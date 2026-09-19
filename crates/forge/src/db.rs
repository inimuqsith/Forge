use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tipe entri sistem berkas dalam manifest
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestEntryType {
    Obj, // Regular file
    Sym, // Symlink
    Dir, // Directory
}

impl ManifestEntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ManifestEntryType::Obj => "obj",
            ManifestEntryType::Sym => "sym",
            ManifestEntryType::Dir => "dir",
        }
    }
}

impl std::str::FromStr for ManifestEntryType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "obj" => Ok(ManifestEntryType::Obj),
            "sym" => Ok(ManifestEntryType::Sym),
            "dir" => Ok(ManifestEntryType::Dir),
            _ => anyhow::bail!("Tipe entri manifest tidak valid: {}", s),
        }
    }
}

/// Entri satu berkas, symlink, atau direktori dalam flat-file manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    pub entry_type: ManifestEntryType,
    pub path: PathBuf,
    pub size: u64,
    pub mtime: u64,
    pub sha256: Option<String>,
    pub symlink_target: Option<PathBuf>,
}

impl ManifestEntry {
    /// Format baris teks flat-file manifest:
    /// obj: `obj <target_path> <size_bytes> <mtime_epoch> <sha256_hash>`
    /// sym: `sym <target_path> <symlink_target> <size_bytes> <mtime_epoch>`
    /// dir: `dir <target_path> <size_bytes> <mtime_epoch>`
    pub fn to_line(&self) -> String {
        match self.entry_type {
            ManifestEntryType::Obj => {
                format!(
                    "obj {} {} {} {}",
                    self.path.display(),
                    self.size,
                    self.mtime,
                    self.sha256.as_deref().unwrap_or("-")
                )
            }
            ManifestEntryType::Sym => {
                format!(
                    "sym {} {} {} {}",
                    self.path.display(),
                    self.symlink_target.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "-".to_string()),
                    self.size,
                    self.mtime
                )
            }
            ManifestEntryType::Dir => {
                format!("dir {} {} {}", self.path.display(), self.size, self.mtime)
            }
        }
    }

    /// Parse baris teks manifest ke `ManifestEntry`
    pub fn parse_line(line: &str) -> Result<ManifestEntry> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            bail!("Baris manifest kosong");
        }

        let entry_type = parts[0]
            .parse::<ManifestEntryType>()
            .with_context(|| format!("Tipe entri manifest tidak valid pada baris: {}", line))?;

        match entry_type {
            ManifestEntryType::Obj => {
                if parts.len() < 5 {
                    bail!("Format baris obj tidak lengkap: {}", line);
                }
                let path = PathBuf::from(parts[1]);
                let size: u64 = parts[2].parse().context("Gagal parse size berkas")?;
                let mtime: u64 = parts[3].parse().context("Gagal parse mtime berkas")?;
                let sha256 = if parts[4] == "-" { None } else { Some(parts[4].to_string()) };

                Ok(ManifestEntry {
                    entry_type,
                    path,
                    size,
                    mtime,
                    sha256,
                    symlink_target: None,
                })
            }
            ManifestEntryType::Sym => {
                if parts.len() < 5 {
                    bail!("Format baris symlink tidak lengkap: {}", line);
                }
                let path = PathBuf::from(parts[1]);
                let target = PathBuf::from(parts[2]);
                let size: u64 = parts[3].parse().context("Gagal parse size symlink")?;
                let mtime: u64 = parts[4].parse().context("Gagal parse mtime symlink")?;

                Ok(ManifestEntry {
                    entry_type,
                    path,
                    size,
                    mtime,
                    sha256: None,
                    symlink_target: Some(target),
                })
            }
            ManifestEntryType::Dir => {
                if parts.len() < 4 {
                    bail!("Format baris dir tidak lengkap: {}", line);
                }
                let path = PathBuf::from(parts[1]);
                let size: u64 = parts[2].parse().context("Gagal parse size dir")?;
                let mtime: u64 = parts[3].parse().context("Gagal parse mtime dir")?;

                Ok(ManifestEntry {
                    entry_type,
                    path,
                    size,
                    mtime,
                    sha256: None,
                    symlink_target: None,
                })
            }
        }
    }
}

/// Metadata paket terpasang (`metadata.json`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    #[serde(default = "default_release")]
    pub release: u32,
    #[serde(default = "default_slot")]
    pub slot: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub upstream: String,
    #[serde(default)]
    pub build_time: u64,
    #[serde(default)]
    pub target_march: String,
    #[serde(default)]
    pub cflags: String,
    #[serde(default)]
    pub use_flags: String,
    #[serde(default)]
    pub files_count: usize,
    #[serde(default)]
    pub installed_size: u64,
    #[serde(default)]
    pub git_commit: Option<String>,
}

fn default_release() -> u32 {
    1
}

fn default_slot() -> String {
    "0".to_string()
}

/// Model data manifes paket terpasang secara komprehensif
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package_name: String,
    pub package_version: String,
    pub release: u32,
    pub slot: String,
    pub entries: Vec<ManifestEntry>,
    pub metadata: Option<PackageMetadata>,
    pub use_flags: Option<String>,
    pub cflags: Option<String>,
}

/// Laporan hasil unmerge paket (`forge remove`)
#[derive(Debug, Clone, Default)]
pub struct UnmergeReport {
    pub package_name: String,
    pub files_removed: usize,
    pub symlinks_removed: usize,
    pub dirs_pruned: usize,
    pub protected_configs_kept: Vec<PathBuf>,
}

/// Database flat-file untuk melacak paket terpasang di `/var/db/forge/installed/`
#[derive(Debug, Clone)]
pub struct InstalledDatabase {
    pub db_root: PathBuf,
}

impl InstalledDatabase {
    /// Inisialisasi koneksi ke InstalledDatabase pada path spesifik
    pub fn new(db_root: impl Into<PathBuf>) -> Self {
        let root = db_root.into();
        Self { db_root: root }
    }

    /// Direktori penyimpanan paket terpasang
    pub fn installed_dir(&self) -> PathBuf {
        if self.db_root.ends_with("installed") {
            self.db_root.clone()
        } else {
            self.db_root.join("installed")
        }
    }

    /// Menghitung SHA256 checksum dari file
    pub fn calculate_sha256(path: &Path) -> Result<String> {
        let mut file = File::open(path)
            .with_context(|| format!("Gagal membuka file untuk hash: {:?}", path))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Memindai dan membaca seluruh paket terpasang di database
    pub fn list_installed(&self) -> Result<Vec<PackageManifest>> {
        let installed = self.installed_dir();
        if !installed.exists() {
            return Ok(Vec::new());
        }

        let mut manifests = Vec::new();
        self.scan_packages_recursive(&installed, &mut manifests)?;
        manifests.sort_by(|a, b| a.package_name.cmp(&b.package_name));
        Ok(manifests)
    }

    fn scan_packages_recursive(&self, dir: &Path, acc: &mut Vec<PackageManifest>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let manifest_file = dir.join("manifest");
        if manifest_file.exists() && manifest_file.is_file() {
            if let Ok(manifest) = self.read_package_dir(dir) {
                acc.push(manifest);
                return Ok(());
            }
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.scan_packages_recursive(&path, acc)?;
            }
        }

        Ok(())
    }

    /// Membaca manifest paket dari direktorinya
    pub fn read_package_dir(&self, pkg_dir: &Path) -> Result<PackageManifest> {
        let manifest_path = pkg_dir.join("manifest");
        let file = File::open(&manifest_path)
            .with_context(|| format!("Gagal membuka manifest di {:?}", manifest_path))?;
        let reader = BufReader::new(file);

        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let entry = ManifestEntry::parse_line(trimmed)?;
                entries.push(entry);
            }
        }

        let metadata_path = pkg_dir.join("metadata.json");
        let metadata: Option<PackageMetadata> = if metadata_path.exists() {
            fs::read_to_string(&metadata_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
        } else {
            None
        };

        let use_flags = pkg_dir.join("USE").exists().then(|| {
            fs::read_to_string(pkg_dir.join("USE")).unwrap_or_default().trim().to_string()
        });

        let cflags = pkg_dir.join("CFLAGS").exists().then(|| {
            fs::read_to_string(pkg_dir.join("CFLAGS")).unwrap_or_default().trim().to_string()
        });

        let dir_name = pkg_dir.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        let (pkg_name, pkg_ver, slot) = if let Some(meta) = &metadata {
            (meta.name.clone(), meta.version.clone(), meta.slot.clone())
        } else {
            // Ekstraksi fallback dari nama folder (misal: mold-2.42.1:0)
            let (name_ver, slot) = dir_name.rsplit_once(':').unwrap_or((dir_name, "0"));
            let (name, ver) = name_ver.rsplit_once('-').unwrap_or((name_ver, "0.1.0"));
            (name.to_string(), ver.to_string(), slot.to_string())
        };

        Ok(PackageManifest {
            package_name: pkg_name,
            package_version: pkg_ver,
            release: metadata.as_ref().map(|m| m.release).unwrap_or(1),
            slot,
            entries,
            metadata,
            use_flags,
            cflags,
        })
    }

    /// Mencari paket berdasarkan nama
    pub fn get_package(&self, pkg_name: &str) -> Result<Option<PackageManifest>> {
        let all = self.list_installed()?;
        for pkg in all {
            if pkg.package_name == pkg_name {
                return Ok(Some(pkg));
            }
        }
        Ok(None)
    }

    /// Cek apakah paket telah terpasang di database
    pub fn is_installed(&self, pkg_name: &str) -> bool {
        self.get_package(pkg_name).map(|opt| opt.is_some()).unwrap_or(false)
    }

    /// Mencari paket pemilik file tertentu di rootfs
    pub fn find_owner(&self, relative_path: &Path) -> Result<Option<(String, String)>> {
        let clean_path = if relative_path.starts_with("/") {
            relative_path.to_path_buf()
        } else {
            PathBuf::from("/").join(relative_path)
        };

        let all = self.list_installed()?;
        for pkg in all {
            for entry in &pkg.entries {
                let entry_clean = if entry.path.starts_with("/") {
                    entry.path.clone()
                } else {
                    PathBuf::from("/").join(&entry.path)
                };

                if entry_clean == clean_path {
                    let version_slot = format!("{}:{}", pkg.package_version, pkg.slot);
                    return Ok(Some((pkg.package_name, version_slot)));
                }
            }
        }

        Ok(None)
    }

    /// Mencatat manifes dan metadata paket yang berhasil di-merge ke database
    pub fn record_package(&self, manifest: &PackageManifest, _target_root: &Path) -> Result<PathBuf> {
        let base_installed = self.installed_dir();
        let pkg_entry_dir = base_installed.join(format!(
            "{}-{}:{}",
            manifest.package_name, manifest.package_version, manifest.slot
        ));

        fs::create_dir_all(&pkg_entry_dir)
            .with_context(|| format!("Gagal membuat direktori manifest di {:?}", pkg_entry_dir))?;

        // 1. Tulis flat-file manifest
        let manifest_file_path = pkg_entry_dir.join("manifest");
        let mut manifest_file = File::create(&manifest_file_path)?;
        for entry in &manifest.entries {
            writeln!(manifest_file, "{}", entry.to_line())?;
        }

        // 2. Tulis metadata.json
        if let Some(ref meta) = manifest.metadata {
            let meta_json = serde_json::to_string_pretty(meta)?;
            fs::write(pkg_entry_dir.join("metadata.json"), meta_json)?;
        } else {
            let total_size: u64 = manifest.entries.iter().map(|e| e.size).sum();
            let fallback_meta = PackageMetadata {
                name: manifest.package_name.clone(),
                version: manifest.package_version.clone(),
                release: manifest.release,
                slot: manifest.slot.clone(),
                description: String::new(),
                url: String::new(),
                license: String::new(),
                upstream: String::new(),
                build_time: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                target_march: "native".to_string(),
                cflags: manifest.cflags.clone().unwrap_or_default(),
                use_flags: manifest.use_flags.clone().unwrap_or_default(),
                files_count: manifest.entries.len(),
                installed_size: total_size,
                git_commit: None,
            };
            let meta_json = serde_json::to_string_pretty(&fallback_meta)?;
            fs::write(pkg_entry_dir.join("metadata.json"), meta_json)?;
        }

        // 3. Tulis USE flags jika ada
        if let Some(ref use_f) = manifest.use_flags {
            fs::write(pkg_entry_dir.join("USE"), use_f)?;
        }

        // 4. Tulis CFLAGS jika ada
        if let Some(ref cf) = manifest.cflags {
            fs::write(pkg_entry_dir.join("CFLAGS"), cf)?;
        }

        // 5. Tulis CONTENTS (daftar path sederhana)
        let contents_path = pkg_entry_dir.join("CONTENTS");
        let mut contents_file = File::create(contents_path)?;
        for entry in &manifest.entries {
            writeln!(contents_file, "{}", entry.path.display())?;
        }

        // 6. Hapus entri versi lama untuk paket dan slot yang sama jika ada (Upgrade cleaner)
        if let Ok(entries) = fs::read_dir(&base_installed) {
            let prefix = format!("{}-", manifest.package_name);
            let slot_suffix = format!(":{}", manifest.slot);
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() && p != pkg_entry_dir {
                    if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                        if file_name.starts_with(&prefix) && file_name.ends_with(&slot_suffix) {
                            let _ = fs::remove_dir_all(&p);
                        }
                    }
                }
            }
        }

        Ok(pkg_entry_dir)
    }

    /// Menghapus paket dari target_root berdasarkan manifest dengan reverse leaf-to-root pruning
    pub fn unmerge_package(
        &self,
        pkg_name: &str,
        target_root: &Path,
        config_protect_dirs: &[PathBuf],
    ) -> Result<UnmergeReport> {
        let manifest = self
            .get_package(pkg_name)?
            .ok_or_else(|| anyhow::anyhow!("Paket '{}' tidak ditemukan dalam database terpasang!", pkg_name))?;

        let mut report = UnmergeReport {
            package_name: pkg_name.to_string(),
            ..Default::default()
        };

        let mut dirs_to_prune = Vec::new();

        // 1. Hapus regular files dan symlinks terlebih dahulu
        for entry in &manifest.entries {
            let rel_clean = entry.path.strip_prefix("/").unwrap_or(&entry.path);
            let target_file_path = target_root.join(rel_clean);

            match entry.entry_type {
                ManifestEntryType::Obj => {
                    if target_file_path.exists() {
                        // Periksa proteksi konfigurasi (CONFIG_PROTECT)
                        let is_protected = config_protect_dirs.iter().any(|d| {
                            let clean_d = d.strip_prefix("/").unwrap_or(d);
                            rel_clean.starts_with(clean_d) || entry.path.starts_with(d)
                        });

                        if is_protected {
                            if let Ok(current_sha) = Self::calculate_sha256(&target_file_path) {
                                if let Some(ref orig_sha) = entry.sha256 {
                                    if &current_sha != orig_sha {
                                        // File konfigurasi telah diubah oleh pengguna, jangan hapus!
                                        report.protected_configs_kept.push(entry.path.clone());
                                        continue;
                                    }
                                }
                            }
                        }

                        if fs::remove_file(&target_file_path).is_ok() {
                            report.files_removed += 1;
                        }
                    }
                }
                ManifestEntryType::Sym => {
                    if (target_file_path.is_symlink() || target_file_path.exists())
                        && fs::remove_file(&target_file_path).is_ok() {
                            report.symlinks_removed += 1;
                        }
                }
                ManifestEntryType::Dir => {
                    dirs_to_prune.push(target_file_path);
                }
            }
        }

        // 2. Reverse Directory Pruning (Leaf-to-Root)
        // Urutkan direktori berdasarkan kedalaman path (komponen terbanyak di awal)
        dirs_to_prune.sort_by_key(|b| std::cmp::Reverse(b.components().count()));
        dirs_to_prune.dedup();

        let essential_system_dirs: std::collections::HashSet<PathBuf> = [
            "", "bin", "sbin", "lib", "lib64", "usr", "usr/bin", "usr/sbin", "usr/lib",
            "usr/lib64", "usr/include", "usr/share", "etc", "var", "var/db", "var/cache",
            "var/log", "tmp", "dev", "proc", "sys", "run", "boot", "home", "root",
        ]
        .iter()
        .map(|d| target_root.join(d))
        .collect();

        for dir in dirs_to_prune {
            if dir.exists() && dir.is_dir() {
                // Jangan pernah hapus direktori fondasi sistem atau target root itu sendiri
                if essential_system_dirs.contains(&dir) || dir == target_root {
                    continue;
                }
                // Cek apakah direktori kosong
                if let Ok(mut read_dir) = fs::read_dir(&dir) {
                    if read_dir.next().is_none() {
                        // Direktori kosong, aman untuk dihapus
                        if fs::remove_dir(&dir).is_ok() {
                            report.dirs_pruned += 1;
                        }
                    }
                }
            }
        }

        // 3. Hapus entri database paket di /var/db/forge/installed/<entry>/
        let pkg_entry_dir = self.installed_dir().join(format!(
            "{}-{}:{}",
            manifest.package_name, manifest.package_version, manifest.slot
        ));
        if pkg_entry_dir.exists() {
            let _ = fs::remove_dir_all(&pkg_entry_dir);
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_entry_serialization() {
        let obj_entry = ManifestEntry {
            entry_type: ManifestEntryType::Obj,
            path: PathBuf::from("/usr/bin/mold"),
            size: 2859384,
            mtime: 1773948201,
            sha256: Some("3f2a5b6c7d8e9f".to_string()),
            symlink_target: None,
        };
        let line = obj_entry.to_line();
        assert_eq!(line, "obj /usr/bin/mold 2859384 1773948201 3f2a5b6c7d8e9f");

        let parsed = ManifestEntry::parse_line(&line).unwrap();
        assert_eq!(parsed, obj_entry);

        let sym_entry = ManifestEntry {
            entry_type: ManifestEntryType::Sym,
            path: PathBuf::from("/usr/bin/ld"),
            size: 0,
            mtime: 1773948201,
            sha256: None,
            symlink_target: Some(PathBuf::from("mold")),
        };
        let sym_line = sym_entry.to_line();
        assert_eq!(sym_line, "sym /usr/bin/ld mold 0 1773948201");
        let sym_parsed = ManifestEntry::parse_line(&sym_line).unwrap();
        assert_eq!(sym_parsed, sym_entry);

        let dir_entry = ManifestEntry {
            entry_type: ManifestEntryType::Dir,
            path: PathBuf::from("/etc/forge"),
            size: 0,
            mtime: 1773948201,
            sha256: None,
            symlink_target: None,
        };
        let dir_line = dir_entry.to_line();
        assert_eq!(dir_line, "dir /etc/forge 0 1773948201");
        let dir_parsed = ManifestEntry::parse_line(&dir_line).unwrap();
        assert_eq!(dir_parsed, dir_entry);
    }

    #[test]
    fn test_installed_database_record_and_get() {
        let tmp = tempdir().unwrap();
        let db = InstalledDatabase::new(tmp.path());

        let manifest = PackageManifest {
            package_name: "test-pkg".to_string(),
            package_version: "1.0.0".to_string(),
            release: 1,
            slot: "0".to_string(),
            entries: vec![
                ManifestEntry {
                    entry_type: ManifestEntryType::Obj,
                    path: PathBuf::from("/usr/bin/test-bin"),
                    size: 1024,
                    mtime: 100,
                    sha256: Some("abcdef123456".to_string()),
                    symlink_target: None,
                },
            ],
            metadata: None,
            use_flags: Some("ssl lto".to_string()),
            cflags: Some("-O3 -march=native".to_string()),
        };

        let target_root = tmp.path().join("rootfs");
        fs::create_dir_all(&target_root).unwrap();

        let pkg_dir = db.record_package(&manifest, &target_root).unwrap();
        assert!(pkg_dir.exists());
        assert!(pkg_dir.join("manifest").exists());
        assert!(pkg_dir.join("metadata.json").exists());
        assert!(pkg_dir.join("USE").exists());
        assert!(pkg_dir.join("CFLAGS").exists());

        let retrieved = db.get_package("test-pkg").unwrap();
        assert!(retrieved.is_some());
        let pkg = retrieved.unwrap();
        assert_eq!(pkg.package_name, "test-pkg");
        assert_eq!(pkg.package_version, "1.0.0");
        assert_eq!(pkg.entries.len(), 1);

        let owner = db.find_owner(Path::new("/usr/bin/test-bin")).unwrap();
        assert_eq!(owner, Some(("test-pkg".to_string(), "1.0.0:0".to_string())));
    }
}
