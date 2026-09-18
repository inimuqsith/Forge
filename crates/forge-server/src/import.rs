use anyhow::{Context, Result};
use colored::*;
use forge::{BinhostCatalog, BinhostPackageEntry, CpuProfile, ForgeConfig, RecipeBuilder};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct ServerImporter;

impl ServerImporter {
    /// Impor file cpu-profile.json dan kompilasi paket khusus CPU tersebut
    pub fn import_and_build(
        profile_path: &Path,
        package_name: Option<&str>,
        recipes_root: &Path,
        binhost_storage: &Path,
    ) -> Result<()> {
        // 1. Baca dan parse cpu-profile.json
        let json_content = std::fs::read_to_string(profile_path)
            .with_context(|| format!("Gagal membaca file profil CPU di {:?}", profile_path))?;
        let cpu_profile: CpuProfile = serde_json::from_str(&json_content)
            .context("Format file JSON bukan merupakan CpuProfile yang valid")?;

        let march = &cpu_profile.target_march;
        let cflags = &cpu_profile.recommended_flags.cflags;
        let cxxflags = &cpu_profile.recommended_flags.cxxflags;
        let ldflags = &cpu_profile.recommended_flags.ldflags;
        let makeflags = &cpu_profile.recommended_flags.makeflags;

        // 2. Tampilkan log penguncian CPU silikon
        println!("{}", "=== Forge CI/CD Builder (Lock-CPU) ===".bold().cyan());
        println!(
            "{} Mengunci CI/CD Compiler ke : {} ({})",
            "[🔒]".yellow(),
            cpu_profile.model_name.bold().green(),
            march.bold().yellow()
        );
        println!("{} CFLAGS Injeksi          : {}", "[⚙]".blue(), cflags.cyan());
        println!("{} CXXFLAGS Injeksi        : {}", "[⚙]".blue(), cxxflags.cyan());
        println!("{} LDFLAGS Injeksi         : {}", "[⚙]".blue(), ldflags.cyan());
        println!("{} MAKEFLAGS Injeksi       : {}", "[⚙]".blue(), makeflags.cyan());

        let target_pkg = package_name.unwrap_or("base");
        println!(
            "{} Memulai CI/CD Build untuk paket: {}",
            "[*]".blue(),
            target_pkg.bold().green()
        );

        // 3. Cari resep paket
        let recipe_path = Self::find_recipe(recipes_root, target_pkg)
            .with_context(|| format!("Resep untuk paket '{}' tidak ditemukan di {:?}", target_pkg, recipes_root))?;

        // 4. Konfigurasi ForgeConfig dinamis berdasarkan CPU Profile
        let mut build_config = ForgeConfig::default();
        build_config.build.cflags = cflags.clone();
        build_config.build.cxxflags = cxxflags.clone();
        build_config.build.ldflags = ldflags.clone();
        build_config.build.makeflags = makeflags.clone();
        build_config.cpu.target_march = march.clone();

        // 5. Kompilasi ke staging
        let staging_dir = std::env::temp_dir()
            .join("forge")
            .join("server_stage")
            .join(target_pkg);
        if staging_dir.exists() {
            let _ = std::fs::remove_dir_all(&staging_dir);
        }
        std::fs::create_dir_all(&staging_dir)?;

        println!("  [🔨] Mengompilasi dari kode sumber...");
        RecipeBuilder::build(&recipe_path, &build_config, &staging_dir, None)?;

        // 6. Kemas ke .forge.tar.zst
        let march_binhost_dir = binhost_storage.join(march);
        std::fs::create_dir_all(&march_binhost_dir)?;

        let package_tarball = march_binhost_dir.join(format!("{}.forge.tar.zst", target_pkg));
        println!(
            "  [📦] Mengemas biner native ke {}",
            package_tarball.display().to_string().yellow()
        );

        let file = std::fs::File::create(&package_tarball)?;
        let encoder = zstd::Encoder::new(file, 3)?;
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(".", &staging_dir)?;
        let encoder = tar.into_inner()?;
        let mut finished_file = encoder.finish()?;
        std::io::Write::flush(&mut finished_file)?;
        drop(finished_file);

        // 7. Hitung hash SHA256 & ukuran
        let bytes = std::fs::read(&package_tarball)?;
        let sha256_hash = format!("{:x}", Sha256::digest(&bytes));
        let size_bytes = bytes.len() as u64;
        println!(
            "  [✓] Paket biner berhasil dibuat! SHA256: {}",
            sha256_hash.green()
        );

        // 8. Generate / Update catalog.json
        let catalog_path = march_binhost_dir.join("catalog.json");
        let mut catalog = if catalog_path.exists() {
            std::fs::read_to_string(&catalog_path)
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

        let recipe = RecipeBuilder::load_recipe(&recipe_path)?;
        let entry = BinhostPackageEntry {
            pkgname: recipe.package.name.clone(),
            pkgver: recipe.package.version.clone(),
            pkgrel: recipe.package.release,
            slot: recipe.package.slot.clone(),
            target_march: march.clone(),
            active_use: Vec::new(),
            sha256: sha256_hash,
            size_bytes,
            download_url: format!("{}.forge.tar.zst", recipe.package.name),
        };

        if let Some(pos) = catalog
            .packages
            .iter()
            .position(|p| p.pkgname == entry.pkgname && p.slot == entry.slot)
        {
            catalog.packages[pos] = entry;
        } else {
            catalog.packages.push(entry);
        }

        let catalog_json = serde_json::to_string_pretty(&catalog)?;
        std::fs::write(&catalog_path, catalog_json)?;

        println!(
            "{} Sukses mempublikasikan paket {} ke Binary Library ({})",
            "✓".green(),
            target_pkg.bold(),
            march.bold()
        );

        Ok(())
    }

    /// Cari file recipe.toml berdasarkan nama paket di recipes_root
    pub fn find_recipe(root: &Path, pkg: &str) -> Option<PathBuf> {
        let direct_candidates = [
            root.join("system").join(pkg).join("recipe.toml"),
            root.join("core").join(pkg).join("recipe.toml"),
            root.join("extra").join(pkg).join("recipe.toml"),
            root.join(pkg).join("recipe.toml"),
        ];
        for cand in direct_candidates {
            if cand.exists() {
                return Some(cand);
            }
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let candidate = path.join(pkg).join("recipe.toml");
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_and_build_lock_cpu() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        // 1. Siapkan cpu-profile.json untuk AMD Ryzen 7 8845HS / znver4
        let profile_json_path = temp_path.join("cpu-profile.json");
        let profile = CpuProfile::mock("znver4", &["avx512f", "avx512dq", "vaes", "sha_ni"]);
        std::fs::write(&profile_json_path, profile.to_json()?)?;

        // 2. Siapkan recipes
        let recipes_dir = temp_path.join("recipes");
        let pkg_dir = recipes_dir.join("system").join("base");
        std::fs::create_dir_all(&pkg_dir)?;

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
        std::fs::write(pkg_dir.join("recipe.toml"), recipe_content)?;

        // 3. Jalankan ServerImporter::import_and_build
        let binhost_dir = temp_path.join("binhost");
        ServerImporter::import_and_build(
            &profile_json_path,
            Some("base"),
            &recipes_dir,
            &binhost_dir,
        )?;

        // 4. Verifikasi hasil binary tarball & catalog.json
        let march_dir = binhost_dir.join("znver4");
        let tarball_path = march_dir.join("base.forge.tar.zst");
        let catalog_path = march_dir.join("catalog.json");

        assert!(tarball_path.exists(), "Tarball biner harus berhasil dibuat");
        assert!(catalog_path.exists(), "catalog.json harus berhasil dibuat");

        // Verifikasi catalog.json
        let catalog_content = std::fs::read_to_string(&catalog_path)?;
        let catalog: BinhostCatalog = serde_json::from_str(&catalog_content)?;

        assert_eq!(catalog.packages.len(), 1);
        let entry = &catalog.packages[0];
        assert_eq!(entry.pkgname, "base");
        assert_eq!(entry.pkgver, "1.0.0");
        assert_eq!(entry.pkgrel, 1);
        assert_eq!(entry.slot, "0");
        assert_eq!(entry.target_march, "znver4");
        assert_eq!(entry.download_url, "base.forge.tar.zst");

        let tarball_bytes = std::fs::read(&tarball_path)?;
        let expected_sha = format!("{:x}", Sha256::digest(&tarball_bytes));
        assert_eq!(entry.sha256, expected_sha);
        assert_eq!(entry.size_bytes, tarball_bytes.len() as u64);

        Ok(())
    }

    #[test]
    fn test_import_multiple_packages_and_updates() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let temp_path = temp.path();

        let profile_json_path = temp_path.join("cpu-profile.json");
        let profile = CpuProfile::mock("znver4", &["avx512f", "vaes"]);
        std::fs::write(&profile_json_path, profile.to_json()?)?;

        let recipes_dir = temp_path.join("recipes");
        let base_dir = recipes_dir.join("system").join("base");
        let mold_dir = recipes_dir.join("extra").join("mold");
        std::fs::create_dir_all(&base_dir)?;
        std::fs::create_dir_all(&mold_dir)?;

        std::fs::write(
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

        std::fs::write(
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

        let binhost_dir = temp_path.join("binhost");

        // Build 1: base (using None as package_name -> defaults to "base")
        ServerImporter::import_and_build(
            &profile_json_path,
            None,
            &recipes_dir,
            &binhost_dir,
        )?;

        // Build 2: mold
        ServerImporter::import_and_build(
            &profile_json_path,
            Some("mold"),
            &recipes_dir,
            &binhost_dir,
        )?;

        let catalog_path = binhost_dir.join("znver4").join("catalog.json");
        assert!(catalog_path.exists());

        let catalog_content = std::fs::read_to_string(&catalog_path)?;
        let catalog: BinhostCatalog = serde_json::from_str(&catalog_content)?;

        assert_eq!(catalog.packages.len(), 2);
        assert!(catalog.packages.iter().any(|p| p.pkgname == "base"));
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

        std::fs::create_dir_all(&system_pkg)?;
        std::fs::create_dir_all(&core_pkg)?;
        std::fs::create_dir_all(&extra_pkg)?;
        std::fs::create_dir_all(&custom_pkg)?;

        std::fs::write(system_pkg.join("recipe.toml"), "[package]\nname = \"syspkg\"\nversion = \"1.0.0\"")?;
        std::fs::write(core_pkg.join("recipe.toml"), "[package]\nname = \"corepkg\"\nversion = \"1.0.0\"")?;
        std::fs::write(extra_pkg.join("recipe.toml"), "[package]\nname = \"extrapkg\"\nversion = \"1.0.0\"")?;
        std::fs::write(custom_pkg.join("recipe.toml"), "[package]\nname = \"custompkg\"\nversion = \"1.0.0\"")?;

        assert!(ServerImporter::find_recipe(&recipes_dir, "syspkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "corepkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "extrapkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "custompkg").is_some());
        assert!(ServerImporter::find_recipe(&recipes_dir, "unknown").is_none());

        Ok(())
    }
}
