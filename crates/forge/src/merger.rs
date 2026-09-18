use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db::{
    InstalledDatabase, ManifestEntry, ManifestEntryType, PackageManifest, PackageMetadata,
};

/// Tipe entri berkas staging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
}

/// Metadata entri berkas dalam direktori staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagedEntry {
    pub relative_path: PathBuf,
    pub file_type: FileType,
    pub size: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub sha256: Option<String>,
    pub symlink_target: Option<PathBuf>,
}

/// Laporan tabrakan berkas (Pre-flight Collision Report)
#[derive(Debug, Clone, Default)]
pub struct CollisionReport {
    pub total_files: usize,
    pub conflicts: Vec<FileConflict>,
}

impl CollisionReport {
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }
}

/// Detail tabrakan satu berkas antar-paket
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileConflict {
    pub target_path: PathBuf,
    pub conflicting_package: String,
}

/// Aksi yang dicatat di Journal transaksi untuk pemulihan (auto-rollback)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalAction {
    CreatedDirectory(PathBuf),
    CreatedFile { path: PathBuf, backup: Option<PathBuf> },
    CreatedSymlink(PathBuf),
    ProtectedConfigCreated(PathBuf),
}

/// Transaksi Penggabungan Berkas (Transactional Merger)
pub struct MergeTransaction {
    pub id: String,
    pub package_name: String,
    pub package_version: String,
    pub slot: String,
    pub release: u32,
    pub staging_dir: PathBuf,
    pub target_root: PathBuf,
    pub db_root: PathBuf,
    pub journal_path: PathBuf,
    pub journal_entries: Vec<JournalAction>,
    pub entries: Vec<StagedEntry>,
    pub config_protect_dirs: Vec<PathBuf>,
    pub metadata: Option<PackageMetadata>,
    pub use_flags: Option<String>,
    pub cflags: Option<String>,
    /// Field opsional untuk simulasi kegagalan I/O pada unit test rollback
    pub simulate_failure_at_step: Option<usize>,
}

impl MergeTransaction {
    /// Inisialisasi transaksi merger baru
    pub fn new(
        package_name: impl Into<String>,
        package_version: impl Into<String>,
        slot: impl Into<String>,
        staging_dir: impl Into<PathBuf>,
        target_root: impl Into<PathBuf>,
        db_root: impl Into<PathBuf>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = format!("{}_{}", timestamp, std::process::id());
        let stg = staging_dir.into();
        let tgt = target_root.into();
        let dbr = db_root.into();
        let journal_path = stg.join(format!("txn_{}.journal", id));

        Self {
            id,
            package_name: package_name.into(),
            package_version: package_version.into(),
            slot: slot.into(),
            release: 1,
            staging_dir: stg,
            target_root: tgt,
            db_root: dbr,
            journal_path,
            journal_entries: Vec::new(),
            entries: Vec::new(),
            config_protect_dirs: vec![PathBuf::from("/etc"), PathBuf::from("etc")],
            metadata: None,
            use_flags: None,
            cflags: None,
            simulate_failure_at_step: None,
        }
    }

    /// Memindai seluruh berkas dan symlink yang berada di direktori staging
    pub fn scan_staging(&mut self) -> Result<&[StagedEntry]> {
        self.entries.clear();
        if !self.staging_dir.exists() {
            bail!("Direktori staging {:?} tidak ditemukan!", self.staging_dir);
        }

        self.scan_dir_recursive(&self.staging_dir.clone())?;
        Ok(&self.entries)
    }

    fn scan_dir_recursive(&mut self, current_dir: &Path) -> Result<()> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let symlink_meta = fs::symlink_metadata(&path)?;
            let file_type = symlink_meta.file_type();

            let relative = path
                .strip_prefix(&self.staging_dir)
                .unwrap_or(&path)
                .to_path_buf();
            let normalized_rel = if relative.starts_with("/") {
                relative
            } else {
                PathBuf::from("/").join(relative)
            };

            let mode = symlink_meta.permissions().mode();
            let size = symlink_meta.len();

            if file_type.is_symlink() {
                let target = fs::read_link(&path)?;
                self.entries.push(StagedEntry {
                    relative_path: normalized_rel,
                    file_type: FileType::Symlink,
                    size: 0,
                    mode,
                    uid: 0,
                    gid: 0,
                    sha256: None,
                    symlink_target: Some(target),
                });
            } else if file_type.is_dir() {
                self.entries.push(StagedEntry {
                    relative_path: normalized_rel,
                    file_type: FileType::Directory,
                    size: 0,
                    mode,
                    uid: 0,
                    gid: 0,
                    sha256: None,
                    symlink_target: None,
                });
                self.scan_dir_recursive(&path)?;
            } else {
                let sha256 = InstalledDatabase::calculate_sha256(&path).ok();
                self.entries.push(StagedEntry {
                    relative_path: normalized_rel,
                    file_type: FileType::Regular,
                    size,
                    mode,
                    uid: 0,
                    gid: 0,
                    sha256,
                    symlink_target: None,
                });
            }
        }
        Ok(())
    }

    /// Melakukan Pre-flight Collision Scan terhadap InstalledDatabase
    pub fn preflight_scan(&self, db: &InstalledDatabase) -> Result<CollisionReport> {
        let mut report = CollisionReport {
            total_files: self.entries.len(),
            conflicts: Vec::new(),
        };

        for entry in &self.entries {
            // Direktori sistem bersama tidak dianggap tabrakan
            if entry.file_type == FileType::Directory {
                continue;
            }

            if let Some((owner_pkg, _owner_ver)) = db.find_owner(&entry.relative_path)? {
                // Tabrakan hanya jika dimiliki oleh paket lain (bukan paket yang sama)
                if owner_pkg != self.package_name {
                    report.conflicts.push(FileConflict {
                        target_path: entry.relative_path.clone(),
                        conflicting_package: owner_pkg,
                    });
                }
            }
        }

        Ok(report)
    }

    fn log_action(&mut self, action: JournalAction) -> Result<()> {
        let mut journal_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.journal_path)?;
        let line = serde_json::to_string(&action)?;
        writeln!(journal_file, "{}", line)?;
        self.journal_entries.push(action);
        Ok(())
    }

    /// Menjalankan transaksi penggabungan (merge) secara atomik
    pub fn execute_merge(&mut self, db: &InstalledDatabase) -> Result<PackageManifest> {
        if self.entries.is_empty() {
            self.scan_staging()?;
        }

        // 1. Pre-flight Collision Check
        let collision_report = self.preflight_scan(db)?;
        if collision_report.has_conflicts() {
            let mut msg = format!(
                "Pre-flight Collision Error: Ditemukan {} konflik berkas dengan paket lain!\n",
                collision_report.conflicts.len()
            );
            for c in &collision_report.conflicts {
                msg.push_str(&format!(
                    "  - {:?} bertabrakan dengan paket '{}'\n",
                    c.target_path, c.conflicting_package
                ));
            }
            bail!(msg);
        }

        // 2. Sortir entri: Direktori dibuat terlebih dahulu, lalu berkas dan symlink
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by_key(|e| match e.file_type {
            FileType::Directory => 0,
            FileType::Regular => 1,
            FileType::Symlink => 2,
        });

        let mut step_counter = 0;

        // 3. Loop eksekusi pemindahan berkas secara transaksional
        for entry in &sorted_entries {
            step_counter += 1;
            if let Some(fail_step) = self.simulate_failure_at_step {
                if step_counter >= fail_step {
                    let _ = self.rollback();
                    bail!("Simulated I/O failure at transaction step {}", step_counter);
                }
            }

            let rel_clean = entry.relative_path.strip_prefix("/").unwrap_or(&entry.relative_path);
            let target_path = self.target_root.join(rel_clean);
            let staged_source = self.staging_dir.join(rel_clean);

            match entry.file_type {
                FileType::Directory => {
                    if !target_path.exists() {
                        if let Err(e) = fs::create_dir_all(&target_path) {
                            let _ = self.rollback();
                            return Err(e).context(format!("Gagal membuat direktori {:?}", target_path));
                        }
                        let _ = fs::set_permissions(&target_path, fs::Permissions::from_mode(entry.mode));
                        self.log_action(JournalAction::CreatedDirectory(target_path))?;
                    }
                }
                FileType::Regular => {
                    // Cek CONFIG_PROTECT
                    let is_config_protected = self.config_protect_dirs.iter().any(|d| {
                        let clean_d = d.strip_prefix("/").unwrap_or(d);
                        rel_clean.starts_with(clean_d) || entry.relative_path.starts_with(d)
                    });

                    if is_config_protected && target_path.exists() {
                        let current_sha = InstalledDatabase::calculate_sha256(&target_path).unwrap_or_default();
                        let staged_sha = entry.sha256.as_deref().unwrap_or_default();

                        if !staged_sha.is_empty() && current_sha != staged_sha {
                            // File konfigurasi telah termodifikasi, simpan sebagai ._cfg0000_<file>
                            let parent = target_path.parent().unwrap_or(&self.target_root);
                            let file_name = target_path.file_name().and_then(|n| n.to_str()).unwrap_or("config");
                            let protected_path = parent.join(format!("._cfg0000_{}", file_name));

                            if let Err(e) = fs::copy(&staged_source, &protected_path) {
                                let _ = self.rollback();
                                return Err(e).context(format!("Gagal menyimpan protected config ke {:?}", protected_path));
                            }
                            let _ = fs::set_permissions(&protected_path, fs::Permissions::from_mode(entry.mode));
                            self.log_action(JournalAction::ProtectedConfigCreated(protected_path))?;
                            continue;
                        }
                    }

                    // Penulisan atomik: Tulis ke temp file lalu rename
                    if let Some(parent) = target_path.parent() {
                        if !parent.exists() {
                            let _ = fs::create_dir_all(parent);
                        }
                    }

                    let tmp_target = target_path.with_extension(format!("forge_tmp.{}", self.id));
                    if let Err(e) = fs::copy(&staged_source, &tmp_target) {
                        let _ = self.rollback();
                        return Err(e).context(format!("Gagal copy ke temporary target {:?}", tmp_target));
                    }

                    let _ = fs::set_permissions(&tmp_target, fs::Permissions::from_mode(entry.mode));

                    // Fsync ke disk
                    if let Ok(file) = File::open(&tmp_target) {
                        let _ = file.sync_all();
                    }

                    // Atomic Rename
                    if let Err(e) = fs::rename(&tmp_target, &target_path) {
                        let _ = self.rollback();
                        return Err(e).context(format!("Gagal atomic rename ke {:?}", target_path));
                    }

                    self.log_action(JournalAction::CreatedFile {
                        path: target_path,
                        backup: None,
                    })?;
                }
                FileType::Symlink => {
                    if let Some(ref sym_target) = entry.symlink_target {
                        if let Some(parent) = target_path.parent() {
                            if !parent.exists() {
                                let _ = fs::create_dir_all(parent);
                            }
                        }

                        if target_path.exists() || target_path.is_symlink() {
                            let _ = fs::remove_file(&target_path);
                        }

                        if let Err(e) = symlink(sym_target, &target_path) {
                            let _ = self.rollback();
                            return Err(e).context(format!("Gagal membuat symlink {:?}", target_path));
                        }

                        self.log_action(JournalAction::CreatedSymlink(target_path))?;
                    }
                }
            }
        }

        // 4. Bangun Manifest dan Catat ke Database
        let manifest_entries: Vec<ManifestEntry> = self
            .entries
            .iter()
            .map(|e| ManifestEntry {
                entry_type: match e.file_type {
                    FileType::Regular => ManifestEntryType::Obj,
                    FileType::Directory => ManifestEntryType::Dir,
                    FileType::Symlink => ManifestEntryType::Sym,
                },
                path: e.relative_path.clone(),
                size: e.size,
                mtime: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                sha256: e.sha256.clone(),
                symlink_target: e.symlink_target.clone(),
            })
            .collect();

        let manifest = PackageManifest {
            package_name: self.package_name.clone(),
            package_version: self.package_version.clone(),
            release: self.release,
            slot: self.slot.clone(),
            entries: manifest_entries,
            metadata: self.metadata.clone(),
            use_flags: self.use_flags.clone(),
            cflags: self.cflags.clone(),
        };

        db.record_package(&manifest, &self.target_root)?;

        // 5. Eksekusi Post-Merge Hooks (OpenRC, ldconfig)
        self.run_post_merge_hooks();

        // 6. Transaksi Berhasil: Hapus file journal
        let _ = fs::remove_file(&self.journal_path);

        Ok(manifest)
    }

    /// Membatalkan seluruh perubahan yang sempat dilakukan (LIFO Rollback)
    pub fn rollback(&mut self) -> Result<()> {
        let mut actions = self.journal_entries.clone();
        actions.reverse();

        for action in actions {
            match action {
                JournalAction::CreatedFile { path, backup } => {
                    let _ = fs::remove_file(&path);
                    if let Some(bak) = backup {
                        let _ = fs::rename(bak, path);
                    }
                }
                JournalAction::CreatedSymlink(path) => {
                    let _ = fs::remove_file(&path);
                }
                JournalAction::ProtectedConfigCreated(path) => {
                    let _ = fs::remove_file(&path);
                }
                JournalAction::CreatedDirectory(path) => {
                    // Hapus direktori hanya jika kosong
                    let _ = fs::remove_dir(&path);
                }
            }
        }

        let _ = fs::remove_file(&self.journal_path);
        self.journal_entries.clear();
        Ok(())
    }

    fn run_post_merge_hooks(&self) {
        // 1. Deteksi service OpenRC (/etc/init.d/)
        let has_openrc_service = self.entries.iter().any(|e| {
            let p_str = e.relative_path.to_string_lossy();
            p_str.starts_with("/etc/init.d/") || p_str.starts_with("etc/init.d/")
        });

        if has_openrc_service {
            println!("  [OpenRC Hook] Layanan OpenRC terdeteksi di /etc/init.d/.");
        }

        // 2. Deteksi pustaka dinamis shared library (/usr/lib/, /lib/)
        let has_libraries = self.entries.iter().any(|e| {
            let p_str = e.relative_path.to_string_lossy();
            p_str.contains(".so") || p_str.starts_with("/usr/lib") || p_str.starts_with("/lib")
        });

        if has_libraries {
            println!("  [Hook] Shared libraries terdeteksi (ldconfig trigger).");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_preflight_collision_detector() {
        let env = tempdir().unwrap();
        let target_root = env.path().join("target");
        let db_root = env.path().join("db");
        let stage_pkg1 = env.path().join("stage_pkg1");
        let stage_pkg2 = env.path().join("stage_pkg2");

        fs::create_dir_all(stage_pkg1.join("usr/bin")).unwrap();
        fs::write(stage_pkg1.join("usr/bin/tool"), b"binary1").unwrap();

        fs::create_dir_all(stage_pkg2.join("usr/bin")).unwrap();
        fs::write(stage_pkg2.join("usr/bin/tool"), b"binary2").unwrap();

        let db = InstalledDatabase::new(&db_root);

        // 1. Merge package 1 (mold)
        let mut txn1 = MergeTransaction::new(
            "mold",
            "2.42.1",
            "0",
            &stage_pkg1,
            &target_root,
            &db_root,
        );
        let res1 = txn1.execute_merge(&db);
        assert!(res1.is_ok(), "Merge package 1 harus sukses");

        // 2. Preflight scan package 2 (fake-mold)
        let mut txn2 = MergeTransaction::new(
            "fake-mold",
            "1.0.0",
            "0",
            &stage_pkg2,
            &target_root,
            &db_root,
        );
        txn2.scan_staging().unwrap();
        let report = txn2.preflight_scan(&db).unwrap();
        assert!(report.has_conflicts());
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].conflicting_package, "mold");

        // 3. Attempt merge package 2 -> should fail with error
        let res2 = txn2.execute_merge(&db);
        assert!(res2.is_err());
        let err_msg = format!("{:#}", res2.err().unwrap());
        assert!(err_msg.contains("Pre-flight Collision Error"));
    }

    #[test]
    fn test_atomic_merge_and_permissions() {
        let env = tempdir().unwrap();
        let target_root = env.path().join("target");
        let db_root = env.path().join("db");
        let staging = env.path().join("staging");

        fs::create_dir_all(staging.join("usr/bin")).unwrap();
        fs::create_dir_all(staging.join("usr/lib")).unwrap();

        let bin_path = staging.join("usr/bin/test-exec");
        fs::write(&bin_path, b"#!/bin/sh\necho test").unwrap();
        fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755)).unwrap();

        let lib_path = staging.join("usr/lib/libtest.so");
        fs::write(&lib_path, b"dummy library content").unwrap();

        let sym_path = staging.join("usr/lib/libtest.so.1");
        symlink(Path::new("libtest.so"), &sym_path).unwrap();

        let db = InstalledDatabase::new(&db_root);
        let mut txn = MergeTransaction::new(
            "test-suite",
            "1.0.0",
            "0",
            &staging,
            &target_root,
            &db_root,
        );

        let manifest = txn.execute_merge(&db).unwrap();
        assert_eq!(manifest.package_name, "test-suite");

        // Verifikasi berkas di target_root
        let target_bin = target_root.join("usr/bin/test-exec");
        assert!(target_bin.exists());
        let mode = fs::metadata(&target_bin).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o755);

        let target_sym = target_root.join("usr/lib/libtest.so.1");
        assert!(target_sym.is_symlink());
        let sym_target = fs::read_link(&target_sym).unwrap();
        assert_eq!(sym_target, PathBuf::from("libtest.so"));

        // Verifikasi database record
        let recorded = db.get_package("test-suite").unwrap();
        assert!(recorded.is_some());
    }

    #[test]
    fn test_config_protect_mechanism() {
        let env = tempdir().unwrap();
        let target_root = env.path().join("target");
        let db_root = env.path().join("db");
        let staging = env.path().join("staging");

        // 1. Simulasikan konfigurasi yang sudah ada di target_root dan diubah oleh pengguna
        fs::create_dir_all(target_root.join("etc")).unwrap();
        let target_cfg = target_root.join("etc/app.conf");
        fs::write(&target_cfg, b"user_custom_setting=true\n").unwrap();

        // 2. Berkas konfigurasi bawaan baru dari staging
        fs::create_dir_all(staging.join("etc")).unwrap();
        let staged_cfg = staging.join("etc/app.conf");
        fs::write(&staged_cfg, b"default_setting=false\n").unwrap();

        let db = InstalledDatabase::new(&db_root);
        let mut txn = MergeTransaction::new(
            "app",
            "2.0.0",
            "0",
            &staging,
            &target_root,
            &db_root,
        );

        let res = txn.execute_merge(&db);
        assert!(res.is_ok());

        // 3. Pastikan config pengguna tidak tertimpa
        let current_cfg_content = fs::read_to_string(&target_cfg).unwrap();
        assert_eq!(current_cfg_content, "user_custom_setting=true\n");

        // 4. Pastikan file protected ._cfg0000_app.conf terbentuk
        let protected_cfg = target_root.join("etc/._cfg0000_app.conf");
        assert!(protected_cfg.exists());
        let protected_content = fs::read_to_string(&protected_cfg).unwrap();
        assert_eq!(protected_content, "default_setting=false\n");
    }

    #[test]
    fn test_transactional_rollback_on_failure() {
        let env = tempdir().unwrap();
        let target_root = env.path().join("target");
        let db_root = env.path().join("db");
        let staging = env.path().join("staging");

        fs::create_dir_all(staging.join("usr/bin")).unwrap();
        fs::create_dir_all(staging.join("usr/share/doc/failpkg")).unwrap();

        fs::write(staging.join("usr/bin/tool1"), b"tool1").unwrap();
        fs::write(staging.join("usr/bin/tool2"), b"tool2").unwrap();
        fs::write(staging.join("usr/share/doc/failpkg/README"), b"doc").unwrap();

        let db = InstalledDatabase::new(&db_root);
        let mut txn = MergeTransaction::new(
            "failpkg",
            "1.0.0",
            "0",
            &staging,
            &target_root,
            &db_root,
        );

        // Simulasi kegagalan pada step ke-3 (di tengah transaksi)
        txn.simulate_failure_at_step = Some(3);

        let res = txn.execute_merge(&db);
        assert!(res.is_err(), "Merge harus gagal karena simulasi error");

        // Verifikasi rollback membersihkan file
        assert!(!target_root.join("usr/bin/tool1").exists());
        assert!(!target_root.join("usr/bin/tool2").exists());
        assert!(!target_root.join("usr/share/doc/failpkg/README").exists());
        assert!(!target_root.join("usr/share/doc/failpkg").exists());
        assert!(!txn.journal_path.exists());
    }

    #[test]
    fn test_unmerge_reverse_pruning() {
        let env = tempdir().unwrap();
        let target_root = env.path().join("target");
        let db_root = env.path().join("db");
        let staging = env.path().join("staging");

        // Buat struktur direktori bersarang
        fs::create_dir_all(staging.join("usr/bin")).unwrap();
        fs::create_dir_all(staging.join("usr/share/unmerge_app/data/sub")).unwrap();
        fs::create_dir_all(staging.join("etc/unmerge_app")).unwrap();

        fs::write(staging.join("usr/bin/unmerge_app"), b"bin").unwrap();
        fs::write(staging.join("usr/share/unmerge_app/data/sub/item.dat"), b"data").unwrap();
        fs::write(staging.join("etc/unmerge_app/config.toml"), b"key=1").unwrap();

        let db = InstalledDatabase::new(&db_root);
        let mut txn = MergeTransaction::new(
            "unmerge_app",
            "1.0.0",
            "0",
            &staging,
            &target_root,
            &db_root,
        );

        txn.execute_merge(&db).unwrap();
        assert!(target_root.join("usr/bin/unmerge_app").exists());

        // Modifikasi config di /etc/ agar terproteksi saat unmerge
        let target_cfg = target_root.join("etc/unmerge_app/config.toml");
        fs::write(&target_cfg, b"key=modified_by_user").unwrap();

        let config_protect = vec![PathBuf::from("/etc"), PathBuf::from("etc")];
        let report = db.unmerge_package("unmerge_app", &target_root, &config_protect).unwrap();

        assert_eq!(report.files_removed, 2); // unmerge_app bin + item.dat
        assert_eq!(report.protected_configs_kept.len(), 1); // config.toml terjaga

        // Verifikasi pruning folder kosong
        assert!(!target_root.join("usr/share/unmerge_app/data/sub").exists());
        assert!(!target_root.join("usr/share/unmerge_app/data").exists());
        assert!(!target_root.join("usr/share/unmerge_app").exists());

        // Folder config tetap ada karena file di dalamnya terlindung
        assert!(target_cfg.exists());

        // Verifikasi database record terhapus
        let check = db.get_package("unmerge_app").unwrap();
        assert!(check.is_none());
    }
}
