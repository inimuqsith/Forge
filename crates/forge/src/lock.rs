use anyhow::{bail, Context, Result};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// RAII Global Concurrency Guard untuk mencegah tabrakan transaksi package manager
#[derive(Debug)]
pub struct ForgeLockGuard {
    file: File,
    lock_path: PathBuf,
}

impl ForgeLockGuard {
    /// Dapatkan lock eksklusif dengan nama kunci tertentu.
    ///
    /// Jika `non_blocking` bernilai true, fungsi akan segera gagal jika lock sedang
    /// dipegang oleh proses lain.
    pub fn acquire(lock_name: &str, non_blocking: bool) -> Result<Self> {
        let primary_path = PathBuf::from(format!("/var/lock/{}.lock", lock_name));
        let fallback_path = std::env::temp_dir().join(format!("{}.lock", lock_name));

        // Coba peroleh lock di /var/lock, fallback ke temporary dir jika izin non-root ditolak
        match Self::acquire_path(&primary_path, non_blocking) {
            Ok(guard) => Ok(guard),
            Err(e) => {
                let err_msg = e.to_string();
                // Jika error adalah karena proses lain sedang berjalan, jangan fallback ke temp
                if err_msg.contains("Proses forge lain sedang berjalan") {
                    return Err(e);
                }
                // Jika kegagalan disebabkan permission denied / path creation, coba fallback
                Self::acquire_path(&fallback_path, non_blocking)
                    .with_context(|| format!("Gagal memperoleh file lock di {:?} dan {:?}", primary_path, fallback_path))
            }
        }
    }

    /// Dapatkan lock eksklusif pada path berkas tertentu.
    pub fn acquire_path(path: &Path, non_blocking: bool) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .with_context(|| format!("Gagal membuka atau membuat berkas lock di {:?}", path))?;

        if non_blocking {
            match file.try_lock_exclusive() {
                Ok(()) => {
                    Self::record_pid(&mut file)?;
                    Ok(Self {
                        file,
                        lock_path: path.to_path_buf(),
                    })
                }
                Err(_e) => {
                    let existing_pid = fs::read_to_string(path).unwrap_or_default();
                    let pid_info = if let Ok(pid_num) = existing_pid.trim().parse::<i32>() {
                        let alive = Self::is_pid_alive(pid_num);
                        format!(
                            " (PID: {}, status: {})",
                            pid_num,
                            if alive { "active" } else { "stale/dead" }
                        )
                    } else if !existing_pid.trim().is_empty() {
                        format!(" (PID: {})", existing_pid.trim())
                    } else {
                        String::new()
                    };
                    bail!(
                        "Proses forge lain sedang berjalan{}. Mohon tunggu hingga proses tersebut selesai atau hentikan proses terkait sebelum melanjutkan.",
                        pid_info
                    );
                }
            }
        } else {
            file.lock_exclusive()
                .with_context(|| format!("Gagal memperoleh exclusive lock pada {:?}", path))?;
            Self::record_pid(&mut file)?;
            Ok(Self {
                file,
                lock_path: path.to_path_buf(),
            })
        }
    }

    /// Periksa apakah proses dengan PID tertentu masih hidup menggunakan sinyal kernel 0 (nix)
    pub fn is_pid_alive(pid: i32) -> bool {
        if pid <= 0 {
            return false;
        }
        match nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), None) {
            Ok(()) => true,
            Err(nix::errno::Errno::EPERM) => true,
            Err(nix::errno::Errno::ESRCH) => false,
            Err(_) => false,
        }
    }

    /// Tulis PID proses aktif ke dalam file lock
    fn record_pid(file: &mut File) -> Result<()> {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        writeln!(file, "{}", std::process::id())?;
        file.flush()?;
        Ok(())
    }

    /// Path berkas lock aktif
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }
}

impl Drop for ForgeLockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_acquire_and_release_lock() {
        let temp_dir = tempfile::tempdir().expect("Gagal membuat direktori sementara");
        let lock_file = temp_dir.path().join("forge_test.lock");

        // 1. Acquire lock pertama kali
        {
            let guard = ForgeLockGuard::acquire_path(&lock_file, true)
                .expect("Seharusnya berhasil memperoleh lock");
            assert_eq!(guard.lock_path(), lock_file);

            // Verifikasi PID tercatat di dalam berkas
            let content = fs::read_to_string(&lock_file).expect("Gagal membaca lock file");
            assert_eq!(content.trim(), std::process::id().to_string());
        } // guard di-drop di sini (RAII unlock)

        // 2. Acquire lock kedua kali setelah yang pertama dilepas
        let second_guard = ForgeLockGuard::acquire_path(&lock_file, true)
            .expect("Seharusnya berhasil memperoleh lock kembali setelah drop");
        assert_eq!(second_guard.lock_path(), lock_file);
    }

    #[test]
    fn test_contended_lock_rejection() {
        let temp_dir = tempfile::tempdir().expect("Gagal membuat direktori sementara");
        let lock_file = temp_dir.path().join("forge_contended.lock");

        // 1. Acquire lock pertama
        let _guard = ForgeLockGuard::acquire_path(&lock_file, true)
            .expect("Lock pertama harus berhasil didapat");

        // 2. Coba acquire lock kedua saat yang pertama masih aktif (non-blocking)
        let second_attempt = ForgeLockGuard::acquire_path(&lock_file, true);
        assert!(second_attempt.is_err(), "Seharusnya gagal saat lock sedang dipegang");

        let err_msg = second_attempt.unwrap_err().to_string();
        assert!(
            err_msg.contains("Proses forge lain sedang berjalan"),
            "Pesan error harus informatif: {}",
            err_msg
        );
        assert!(
            err_msg.contains(&std::process::id().to_string()),
            "Pesan error harus mencantumkan PID aktif: {}",
            err_msg
        );
    }

    #[test]
    fn test_is_pid_alive_detection() {
        let current_pid = std::process::id() as i32;
        assert!(ForgeLockGuard::is_pid_alive(current_pid));
        // PID 99999999 kemungkinan besar tidak ada di Linux
        assert!(!ForgeLockGuard::is_pid_alive(99_999_999));
        assert!(!ForgeLockGuard::is_pid_alive(-1));
    }
}
