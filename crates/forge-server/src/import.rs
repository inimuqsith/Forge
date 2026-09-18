use anyhow::{bail, Context, Result};
use colored::*;
use forge::{BinhostCatalog, BinhostPackageEntry, PackageMetadata};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub struct ServerImporter;

impl ServerImporter {
    /// Ingestion Biner: Membaca berkas tarball biner .forge.tar.zst yang sudah di-build, mengekstrak metadata,
    /// memindahkannya/menyalinnya ke direktori resmi /var/db/forge/binhost/<march>/, dan memperbarui catalog.json.
    pub fn import_tarball(
        tarball_path: &Path,
        binhost_storage: &Path,
        target_march_override: Option<&str>,
    ) -> Result<BinhostPackageEntry> {
        if !tarball_path.exists() {
            bail!("Berkas tarball biner tidak ditemukan: {:?}", tarball_path);
        }

        println!("{}", "=== Forge Server Binary Ingestion ===".bold().cyan());
        println!("{} Memproses tarball: {}", "[*]".blue(), tarball_path.display().to_string().yellow());

        // 1. Hitung SHA256 & ukuran berkas tarball
        let bytes = fs::read(tarball_path)
            .with_context(|| format!("Gagal membaca tarball biner di {:?}", tarball_path))?;
        let sha256_hash = format!("{:x}", Sha256::digest(&bytes));
        let size_bytes = bytes.len() as u64;

        // 2. Ekstrak metadata dari dalam tarball .forge.tar.zst
        let metadata = Self::extract_metadata_from_tarball(tarball_path)?;

        // 3. Tentukan nama paket, versi, slot, dan target march
        let (pkgname, pkgver, pkgrel, slot, target_march) = if let Some(meta) = metadata {
            let march = if let Some(override_march) = target_march_override {
                override_march.to_string()
            } else if !meta.target_march.is_empty() && meta.target_march != "native" {
                meta.target_march
            } else {
                Self::infer_march_from_filename(tarball_path).unwrap_or_else(|| "generic".to_string())
            };
            (meta.name, meta.version, meta.release, meta.slot, march)
        } else {
            // Fallback parsing dari nama file
            let inferred = Self::infer_metadata_from_filename(tarball_path)?;
            let march = target_march_override
                .map(|s| s.to_string())
                .unwrap_or(inferred.3);
            (inferred.0, inferred.1, 1, "0".to_string(), march)
        };

        println!(
            "  [📦] Paket: {} v{}-r{} (Slot: {}, March: {})",
            pkgname.bold().green(),
            pkgver,
            pkgrel,
            slot.cyan(),
            target_march.yellow()
        );
        println!("  [✓] SHA256 Checksum : {}", sha256_hash.green());
        println!("  [✓] Ukuran File     : {} bytes", size_bytes);

        // 4. Siapkan direktori binhost target: <binhost_storage>/<march>/
        let march_binhost_dir = binhost_storage.join(&target_march);
        fs::create_dir_all(&march_binhost_dir)?;

        // 5. Salin/pindahkan tarball ke binhost storage
        let filename = tarball_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("package.forge.tar.zst");
        let dest_tarball = march_binhost_dir.join(filename);

        let is_same_file = if let (Ok(can_src), Ok(can_dst)) = (tarball_path.canonicalize(), dest_tarball.canonicalize()) {
            can_src == can_dst
        } else {
            false
        };

        if !is_same_file {
            fs::copy(tarball_path, &dest_tarball)
                .with_context(|| format!("Gagal menyalin tarball ke {:?}", dest_tarball))?;
            let sha_source = format!("{}.sha256", tarball_path.display());
            if Path::new(&sha_source).exists() {
                let sha_dest = march_binhost_dir.join(format!("{}.sha256", filename));
                let _ = fs::copy(&sha_source, &sha_dest);
            }
        }

        // 6. Perbarui catalog.json di march_binhost_dir
        let catalog_path = march_binhost_dir.join("catalog.json");
        let mut catalog = if catalog_path.exists() {
            fs::read_to_string(&catalog_path)
                .ok()
                .and_then(|s| serde_json::from_str::<BinhostCatalog>(&s).ok())
                .unwrap_or_else(|| BinhostCatalog {
                    timestamp: 0,
                    server_version: "0.1.0".to_string(),
                    packages: Vec::new(),
                })
        } else {
            BinhostCatalog {
                timestamp: 0,
                server_version: "0.1.0".to_string(),
                packages: Vec::new(),
            }
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        catalog.timestamp = now;

        let entry = BinhostPackageEntry {
            pkgname: pkgname.clone(),
            pkgver: pkgver.clone(),
            pkgrel,
            slot: slot.clone(),
            target_march: target_march.clone(),
            active_use: Vec::new(),
            sha256: sha256_hash,
            size_bytes,
            download_url: filename.to_string(),
        };

        if let Some(pos) = catalog
            .packages
            .iter()
            .position(|p| p.pkgname == entry.pkgname && p.slot == entry.slot)
        {
            catalog.packages[pos] = entry.clone();
        } else {
            catalog.packages.push(entry.clone());
        }

        let catalog_json = serde_json::to_string_pretty(&catalog)?;
        fs::write(&catalog_path, catalog_json)?;

        println!(
            "{} Sukses mengimpor dan mempublikasikan paket {} ke Binary Library ({})",
            "✓".green(),
            pkgname.bold(),
            target_march.bold().yellow()
        );

        Ok(entry)
    }

    /// Ekstraksi metadata.json dari dalam arsip .forge.tar.zst
    pub fn extract_metadata_from_tarball(tarball_path: &Path) -> Result<Option<PackageMetadata>> {
        let file = File::open(tarball_path)?;
        let decoder = zstd::Decoder::new(file)?;
        let mut archive = tar::Archive::new(decoder);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            let path_str = path.to_string_lossy();
            if path_str == "metadata.json" || path_str.ends_with("/metadata.json") {
                let mut content = String::new();
                std::io::Read::read_to_string(&mut entry, &mut content)?;
                if let Ok(meta) = serde_json::from_str::<PackageMetadata>(&content) {
                    return Ok(Some(meta));
                }
            }
        }
        Ok(None)
    }

    /// Ekstraksi mikroarsitektur dari nama file tarball (misal: base-1.0.0-znver4.forge.tar.zst -> znver4)
    fn infer_march_from_filename(path: &Path) -> Option<String> {
        let file_name = path.file_name()?.to_str()?;
        let clean_name = file_name.strip_suffix(".forge.tar.zst")
            .or_else(|| file_name.strip_suffix(".tar.zst"))?;
        let parts: Vec<&str> = clean_name.split('-').collect();
        if parts.len() >= 3 {
            Some(parts[parts.len() - 1].to_string())
        } else {
            None
        }
    }

    /// Fallback ekstraksi metadata dari nama file tarball
    fn infer_metadata_from_filename(path: &Path) -> Result<(String, String, u32, String)> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Nama file tidak valid")?;
        let clean_name = file_name.strip_suffix(".forge.tar.zst")
            .or_else(|| file_name.strip_suffix(".tar.zst"))
            .unwrap_or(file_name);
        let parts: Vec<&str> = clean_name.split('-').collect();
        if parts.len() >= 3 {
            let march = parts[parts.len() - 1].to_string();
            let ver = parts[parts.len() - 2].to_string();
            let name = parts[..parts.len() - 2].join("-");
            Ok((name, ver, 1, march))
        } else if parts.len() == 2 {
            Ok((parts[0].to_string(), parts[1].to_string(), 1, "generic".to_string()))
        } else {
            Ok((parts[0].to_string(), "1.0.0".to_string(), 1, "generic".to_string()))
        }
    }

    /// Cari file recipe.toml berdasarkan nama paket di recipes_root
    pub fn find_recipe(root: &Path, pkg: &str) -> Option<PathBuf> {
        forge::RecipeBuilder::find_recipe(pkg, Some(root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ServerBuilder, ServerProfileManager};
    use forge::CpuProfile;

    #[test]
    fn test_server_import_cpu_profile_saves_active() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();
        let profiles_dir = temp_path.join("profiles");

        // 1. Buat dummy cpu-profile.json
        let src_profile_path = temp_path.join("cpu-profile.json");
        let profile = CpuProfile::mock("znver4", &["avx512f", "avx512dq", "vaes"]);
        fs::write(&src_profile_path, profile.to_json()?)?;

        // 2. Import profil ke server
        let (imported, active_path) = ServerProfileManager::import_profile(
            &src_profile_path,
            Some(&profiles_dir),
            None,
        )?;

        assert_eq!(imported.target_march, "znver4");
        assert_eq!(imported.model_name, "Mock Processor");

        // 3. Verifikasi file <profiles_dir>/znver4.json ada
        let march_json = profiles_dir.join("znver4.json");
        assert!(march_json.exists(), "File znver4.json harus ada di profiles_dir");

        // 4. Verifikasi active.json ada dan isinya cocok
        assert!(active_path.exists(), "File active.json harus ada di profiles_dir");
        let active_content = fs::read_to_string(&active_path)?;
        let active_profile: CpuProfile = serde_json::from_str(&active_content)?;
        assert_eq!(active_profile.target_march, "znver4");
        assert_eq!(active_profile.model_name, "Mock Processor");

        // 5. Import dengan parameter --as custom-name
        let (custom_imported, _) = ServerProfileManager::import_profile(
            &src_profile_path,
            Some(&profiles_dir),
            Some("custom-ryzen"),
        )?;
        assert_eq!(custom_imported.target_march, "znver4");
        assert!(
            profiles_dir.join("custom-ryzen.json").exists(),
            "File custom-ryzen.json harus dibuat"
        );

        Ok(())
    }

    #[test]
    fn test_server_build_uses_imported_active_profile() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        let profiles_dir = temp_path.join("profiles");
        let recipes_dir = temp_path.join("recipes");
        let dist_dir = temp_path.join("dist");
        let binhost_dir = temp_path.join("binhost");

        // 1. Siapkan profil znver4 dan impor sebagai profil aktif
        let profile_path = temp_path.join("cpu-profile.json");
        let profile = CpuProfile::mock("znver4", &["avx512f", "vaes"]);
        fs::write(&profile_path, profile.to_json()?)?;
        ServerProfileManager::import_profile(&profile_path, Some(&profiles_dir), None)?;

        // 2. Siapkan resep base
        let base_dir = recipes_dir.join("system").join("base");
        fs::create_dir_all(&base_dir)?;
        fs::write(
            base_dir.join("recipe.toml"),
            r#"
[package]
name = "base"
version = "1.0.0"
release = 1
slot = "0"
description = "Base Package"

[build]
type = "meta"
script = """
mkdir -p "$DESTDIR/etc"
echo "Kura Linux" > "$DESTDIR/etc/release"
"""
"#,
        )?;

        // 3. Muat profil aktif (tanpa override)
        let (active_profile, source) = ServerProfileManager::load_active_profile(
            None,
            Some(&profiles_dir),
        )?;
        assert_eq!(active_profile.target_march, "znver4");
        assert!(source.contains("active"));

        // 4. Jalankan build menggunakan profil aktif
        let built_tarball = ServerBuilder::build_package(
            &active_profile,
            Some("base"),
            &recipes_dir,
            &dist_dir,
        )?;

        assert!(built_tarball.exists());
        assert_eq!(built_tarball, dist_dir.join("base-1.0.0-znver4.forge.tar.zst"));

        // 5. Publikasikan ke binhost
        let entry = ServerImporter::import_tarball(
            &built_tarball,
            &binhost_dir,
            Some(&active_profile.target_march),
        )?;
        assert_eq!(entry.pkgname, "base");
        assert_eq!(entry.target_march, "znver4");
        assert!(binhost_dir.join("znver4").join("catalog.json").exists());

        Ok(())
    }

    #[test]
    fn test_server_build_with_custom_profile_override() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        let profiles_dir = temp_path.join("profiles");
        let recipes_dir = temp_path.join("recipes");
        let dist_dir = temp_path.join("dist");

        // 1. Set profil aktif sebagai znver4
        let znver4_path = temp_path.join("znver4.json");
        let znver4_profile = CpuProfile::mock("znver4", &["avx512f"]);
        fs::write(&znver4_path, znver4_profile.to_json()?)?;
        ServerProfileManager::import_profile(&znver4_path, Some(&profiles_dir), None)?;

        // 2. Buat berkas profil custom override untuk alderlake
        let custom_path = temp_path.join("custom-alderlake.json");
        let alderlake_profile = CpuProfile::mock("alderlake", &["avx2"]);
        fs::write(&custom_path, alderlake_profile.to_json()?)?;

        // 3. Muat profil dengan flag override --profile custom_path
        let (loaded_profile, source) = ServerProfileManager::load_active_profile(
            Some(&custom_path),
            Some(&profiles_dir),
        )?;
        assert_eq!(loaded_profile.target_march, "alderlake");
        assert!(source.contains("custom"));

        // 4. Siapkan resep base
        let base_dir = recipes_dir.join("system").join("base");
        fs::create_dir_all(&base_dir)?;
        fs::write(
            base_dir.join("recipe.toml"),
            r#"
[package]
name = "base"
version = "1.0.0"
release = 1
slot = "0"
description = "Base Package"
"#,
        )?;

        // 5. Build dengan profil override
        let built_tarball = ServerBuilder::build_package(
            &loaded_profile,
            Some("base"),
            &recipes_dir,
            &dist_dir,
        )?;

        assert!(built_tarball.exists());
        assert_eq!(built_tarball, dist_dir.join("base-1.0.0-alderlake.forge.tar.zst"));

        Ok(())
    }

    #[test]
    fn test_server_list_profiles() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();
        let profiles_dir = temp_path.join("profiles");

        // 1. Impor profil znver4 (menjadi active)
        let znver4_path = temp_path.join("znver4.json");
        let znver4_profile = CpuProfile::mock("znver4", &["avx512f", "vaes"]);
        fs::write(&znver4_path, znver4_profile.to_json()?)?;
        ServerProfileManager::import_profile(&znver4_path, Some(&profiles_dir), None)?;

        // 2. Buat profil x86-64-v3 (hanya simpan di profiles_dir tanpa menimpa active.json)
        let v3_path = profiles_dir.join("x86-64-v3.json");
        let v3_profile = CpuProfile::mock("x86-64-v3", &["avx2", "fma"]);
        fs::write(&v3_path, v3_profile.to_json()?)?;

        // 3. List profiles
        let list = ServerProfileManager::list_profiles(Some(&profiles_dir))?;
        assert_eq!(list.len(), 2);

        let znver4_entry = list.iter().find(|p| p.name == "znver4").expect("znver4 harus ada");
        assert!(znver4_entry.is_active, "znver4 harus ditandai aktif");
        assert_eq!(znver4_entry.march, "znver4");

        let v3_entry = list.iter().find(|p| p.name == "x86-64-v3").expect("x86-64-v3 harus ada");
        assert!(!v3_entry.is_active, "x86-64-v3 TIDAK boleh aktif");
        assert_eq!(v3_entry.march, "x86-64-v3");

        Ok(())
    }

    #[test]
    fn test_forge_server_build_and_import_separation() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        // 1. Siapkan cpu-profile untuk AMD Ryzen 7 8845HS / znver4
        let profile = CpuProfile::mock("znver4", &["avx512f", "avx512dq", "vaes", "sha_ni"]);

        // 2. Siapkan recipes
        let recipes_dir = temp_path.join("recipes");
        let pkg_dir = recipes_dir.join("system").join("base");
        fs::create_dir_all(&pkg_dir)?;

        let recipe_content = r#"
[package]
name = "base"
version = "1.0.0"
release = 1
slot = "0"
description = "Kura Linux Base Meta Package"

[build]
type = "meta"
script = """
mkdir -p "$DESTDIR/etc"
echo "Kura Linux Base v1.0.0" > "$DESTDIR/etc/kura-release"
"""
"#;
        fs::write(pkg_dir.join("recipe.toml"), recipe_content)?;

        let dist_dir = temp_path.join("dist");
        let binhost_dir = temp_path.join("binhost");

        // STEP 1: Run ServerBuilder::build_package (BUILD ONLY)
        let built_tarball = ServerBuilder::build_package(
            &profile,
            Some("base"),
            &recipes_dir,
            &dist_dir,
        )?;

        // Verifikasi hasil BUILD:
        // - Tarball biner harus ada di dist_dir
        assert!(built_tarball.exists(), "Tarball biner harus berhasil dibuat di dist_dir");
        assert_eq!(built_tarball, dist_dir.join("base-1.0.0-znver4.forge.tar.zst"));

        // - Verifikasi SEPARATION: Direktori binhost/znver4 BELUM dibuat atau masih kosong, dan catalog.json BELUM ADA
        let binhost_march_dir = binhost_dir.join("znver4");
        let catalog_path = binhost_march_dir.join("catalog.json");
        assert!(!catalog_path.exists(), "catalog.json TIDAK BOLEH ada sebelum tahap import!");

        // STEP 2: Run ServerImporter::import_tarball (IMPORT ONLY)
        let entry = ServerImporter::import_tarball(
            &built_tarball,
            &binhost_dir,
            None,
        )?;

        // Verifikasi hasil IMPORT:
        // - Binhost sekarang memiliki file tarball dan catalog.json
        assert!(binhost_march_dir.exists(), "Direktori binhost/znver4 harus dibuat setelah import");
        assert!(catalog_path.exists(), "catalog.json harus dibuat setelah import");
        assert_eq!(entry.pkgname, "base");
        assert_eq!(entry.pkgver, "1.0.0");
        assert_eq!(entry.target_march, "znver4");

        // Verifikasi isi catalog.json
        let catalog_content = fs::read_to_string(&catalog_path)?;
        let catalog: BinhostCatalog = serde_json::from_str(&catalog_content)?;
        assert_eq!(catalog.packages.len(), 1);
        assert_eq!(catalog.packages[0].pkgname, "base");
        assert_eq!(catalog.packages[0].sha256, entry.sha256);

        Ok(())
    }

    #[test]
    fn test_forge_server_import_registers_to_catalog() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        let profile = CpuProfile::mock("znver4", &["avx512f", "vaes"]);

        let recipes_dir = temp_path.join("recipes");
        let base_dir = recipes_dir.join("system").join("base");
        let mold_dir = recipes_dir.join("extra").join("mold");
        fs::create_dir_all(&base_dir)?;
        fs::create_dir_all(&mold_dir)?;

        fs::write(
            base_dir.join("recipe.toml"),
            r#"
[package]
name = "base"
version = "1.0.0"
release = 1
slot = "0"
description = "Base Package"
"#,
        )?;

        fs::write(
            mold_dir.join("recipe.toml"),
            r#"
[package]
name = "mold"
version = "2.30.0"
release = 1
slot = "0"
description = "Modern Linker"
"#,
        )?;

        let dist_dir = temp_path.join("dist");
        let binhost_dir = temp_path.join("binhost");

        // Build 1: base
        let base_tarball = ServerBuilder::build_package(
            &profile,
            Some("base"),
            &recipes_dir,
            &dist_dir,
        )?;

        // Build 2: mold
        let mold_tarball = ServerBuilder::build_package(
            &profile,
            Some("mold"),
            &recipes_dir,
            &dist_dir,
        )?;

        // Import 1: base
        ServerImporter::import_tarball(&base_tarball, &binhost_dir, None)?;

        // Import 2: mold
        ServerImporter::import_tarball(&mold_tarball, &binhost_dir, None)?;

        let catalog_path = binhost_dir.join("znver4").join("catalog.json");
        assert!(catalog_path.exists());

        let catalog_content = fs::read_to_string(&catalog_path)?;
        let catalog: BinhostCatalog = serde_json::from_str(&catalog_content)?;

        assert_eq!(catalog.packages.len(), 2);
        assert!(catalog.packages.iter().any(|p| p.pkgname == "base" && p.pkgver == "1.0.0"));
        assert!(catalog.packages.iter().any(|p| p.pkgname == "mold" && p.pkgver == "2.30.0"));

        Ok(())
    }

    #[test]
    fn test_find_recipe_locations() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");

        let system_pkg = recipes_dir.join("system").join("syspkg");
        let core_pkg = recipes_dir.join("core").join("corepkg");
        let extra_pkg = recipes_dir.join("extra").join("extrapkg");
        let custom_pkg = recipes_dir.join("custom_cat").join("custompkg");

        fs::create_dir_all(&system_pkg)?;
        fs::create_dir_all(&core_pkg)?;
        fs::create_dir_all(&extra_pkg)?;
        fs::create_dir_all(&custom_pkg)?;

        fs::write(system_pkg.join("recipe.toml"), "[package]\nname = \"syspkg\"\nversion = \"1.0.0\"")?;
        fs::write(core_pkg.join("recipe.toml"), "[package]\nname = \"corepkg\"\nversion = \"1.0.0\"")?;
        fs::write(extra_pkg.join("recipe.toml"), "[package]\nname = \"extrapkg\"\nversion = \"1.0.0\"")?;
        fs::write(custom_pkg.join("recipe.toml"), "[package]\nname = \"custompkg\"\nversion = \"1.0.0\"")?;

        assert!(ServerImporter::find_recipe(&recipes_dir, "syspkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "corepkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "extrapkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "custompkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "unknown").is_none());

        Ok(())
    }
}
