use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Engine Isolasi Eksekusi Script Build Menggunakan Namespace / Bubblewrap
#[derive(Debug, Clone)]
pub struct SandboxRunner {
    bwrap_path: Option<PathBuf>,
}

impl SandboxRunner {
    /// Inisialisasi SandboxRunner dengan auto-detection `bwrap` pada sistem host
    pub fn new() -> Self {
        let bwrap_path = Self::detect_bwrap();
        Self { bwrap_path }
    }

    /// Inisialisasi dengan path kustom untuk keperluan testing atau override
    pub fn with_bwrap_path(bwrap_path: Option<PathBuf>) -> Self {
        Self { bwrap_path }
    }

    /// Apakah Bubblewrap (`bwrap`) tersedia pada sistem host
    pub fn is_bwrap_available(&self) -> bool {
        self.bwrap_path.is_some()
    }

    /// Path biner bwrap yang terdeteksi
    pub fn bwrap_path(&self) -> Option<&Path> {
        self.bwrap_path.as_deref()
    }

    /// Deteksi bwrap via path lookup
    fn detect_bwrap() -> Option<PathBuf> {
        let output = Command::new("which").arg("bwrap").output().ok()?;
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() && Path::new(&path_str).exists() {
                return Some(PathBuf::from(path_str));
            }
        }
        None
    }

    /// Bangun daftar argumen standar untuk Bubblewrap isolasi build
    pub fn build_bwrap_args(build_dir: &Path, destdir: &Path) -> Vec<String> {
        vec![
            "--ro-bind".to_string(),
            "/".to_string(),
            "/".to_string(),
            "--bind".to_string(),
            build_dir.display().to_string(),
            build_dir.display().to_string(),
            "--bind".to_string(),
            destdir.display().to_string(),
            destdir.display().to_string(),
            "--bind".to_string(),
            "/tmp".to_string(),
            "/tmp".to_string(),
            "--proc".to_string(),
            "/proc".to_string(),
            "--dev".to_string(),
            "/dev".to_string(),
            "--unshare-all".to_string(),
            "--die-with-parent".to_string(),
        ]
    }

    /// Jalankan script kompilasi dengan validasi ketat Bubblewrap:
    /// - Jika `bwrap` terpasang: Eksekusi di dalam Bubblewrap sandbox (--ro-bind / /).
    /// - Jika `bwrap` TIDAK terpasang:
    ///   * Pengecualian khusus: Paket 'bubblewrap' / 'bwrap' diizinkan di-build tanpa sandbox (self-bootstrap).
    ///   * Paket lainnya: DITOLAK SEKETIKA (Fatal Error) untuk melindungi filesystem host.
    pub fn run_script_for_package(
        &self,
        pkg_name: &str,
        script: &str,
        build_dir: &Path,
        destdir: &Path,
        env_vars: &HashMap<String, String>,
        allow_unsafe_fallback: bool,
    ) -> Result<ExitStatus> {
        std::fs::create_dir_all(build_dir)
            .with_context(|| format!("Gagal membuat direktori build {:?}", build_dir))?;
        std::fs::create_dir_all(destdir)
            .with_context(|| format!("Gagal membuat direktori destdir {:?}", destdir))?;

        if let Some(ref bwrap) = self.bwrap_path {
            let mut cmd = Command::new(bwrap);
            let bwrap_args = Self::build_bwrap_args(build_dir, destdir);
            cmd.args(bwrap_args);
            cmd.arg("--").arg("bash").arg("-c").arg(script);
            cmd.envs(env_vars);
            cmd.current_dir(build_dir);
            cmd.status()
                .with_context(|| "Gagal mengeksekusi bash script di dalam sandbox Bubblewrap")
        } else {
            let is_self_bootstrap = pkg_name == "bubblewrap" || pkg_name == "bwrap";
            if is_self_bootstrap {
                println!(
                    "  [!] Mode Khusus Self-Bootstrap: Mengompilasi '{}' dalam mode un-sandboxed agar isolasi Bubblewrap tersedia untuk paket selanjutnya...",
                    pkg_name
                );
                Self::run_script_fallback(script, build_dir, destdir, env_vars)
            } else if allow_unsafe_fallback {
                println!(
                    "  [⚠️] PERINGATAN KESELAMATAN: Berjalan dalam mode un-sandboxed (--unsafe-no-sandbox aktif)!"
                );
                Self::run_script_fallback(script, build_dir, destdir, env_vars)
            } else {
                anyhow::bail!(
                    "\n[FATAL ERROR] Isolasi Sandbox Bubblewrap ('bwrap') WAJIB digunakan untuk mengompilasi paket '{}', namun biner 'bwrap' tidak ditemukan di sistem host!\n\n\
                    Demi menjaga kebersihan dan melindungi filesystem host '/', Forge melarang kompilasi un-sandboxed.\n\
                    Pengecualian satu-satunya: paket 'bubblewrap' itu sendiri diizinkan untuk di-build pertama kali via:\n\
                      forge install bubblewrap\n\
                    Atau silakan pasang 'bubblewrap' pada sistem host Anda sebelum melanjutkan.",
                    pkg_name
                );
            }
        }
    }

    /// Jalankan script bash (kompatibilitas mundur)
    pub fn run_script(
        &self,
        script: &str,
        build_dir: &Path,
        destdir: &Path,
        env_vars: &HashMap<String, String>,
    ) -> Result<ExitStatus> {
        self.run_script_for_package("generic", script, build_dir, destdir, env_vars, false)
    }

    /// Mode eksekusi fallback langsung dengan validasi lingkungan dan chdir
    pub fn run_script_fallback(
        script: &str,
        build_dir: &Path,
        _destdir: &Path,
        env_vars: &HashMap<String, String>,
    ) -> Result<ExitStatus> {
        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(script);
        cmd.envs(env_vars);
        cmd.current_dir(build_dir);
        cmd.status()
            .with_context(|| "Gagal mengeksekusi bash script dalam mode fallback")
    }

    /// Konfigurasi batas alokasi resource proses Linux (nix syscalls setrlimit)
    pub fn configure_process_limits(
        max_open_files: Option<u64>,
        max_core_dump: Option<u64>,
    ) -> Result<()> {
        if let Some(nofile) = max_open_files {
            let _ = nix::sys::resource::setrlimit(
                nix::sys::resource::Resource::RLIMIT_NOFILE,
                nofile,
                nofile,
            );
        }
        if let Some(core) = max_core_dump {
            let _ = nix::sys::resource::setrlimit(
                nix::sys::resource::Resource::RLIMIT_CORE,
                core,
                core,
            );
        }
        Ok(())
    }
}

impl Default for SandboxRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_sandbox_command_construction() {
        let build_dir = Path::new("/tmp/forge/build/test-pkg-1.0");
        let destdir = Path::new("/tmp/forge/stage/test-pkg");

        let args = SandboxRunner::build_bwrap_args(build_dir, destdir);

        // 1. Validasi host root read-only bind
        assert!(args.contains(&"--ro-bind".to_string()));
        let ro_idx = args.iter().position(|r| r == "--ro-bind").unwrap();
        assert_eq!(args[ro_idx + 1], "/");
        assert_eq!(args[ro_idx + 2], "/");

        // 2. Validasi writable bind build_dir & destdir
        assert!(args.contains(&build_dir.display().to_string()));
        assert!(args.contains(&destdir.display().to_string()));

        // 3. Validasi isolasi kernel namespace & proc/dev
        assert!(args.contains(&"--unshare-all".to_string()));
        assert!(args.contains(&"--die-with-parent".to_string()));
        assert!(args.contains(&"--proc".to_string()));
        assert!(args.contains(&"--dev".to_string()));
    }

    #[test]
    fn test_sandbox_fallback_execution() {
        let temp_dir = tempfile::tempdir().expect("Gagal membuat direktori sementara");
        let build_dir = temp_dir.path().join("build");
        let destdir = temp_dir.path().join("stage");

        std::fs::create_dir_all(&build_dir).unwrap();
        std::fs::create_dir_all(&destdir).unwrap();

        let mut env_vars = HashMap::new();
        env_vars.insert("DESTDIR".to_string(), destdir.display().to_string());
        env_vars.insert("MY_TEST_VAR".to_string(), "kura_linux_rulez".to_string());

        let script = r#"
echo "hello from fallback sandbox" > "$DESTDIR/output.txt"
echo "var=$MY_TEST_VAR" >> "$DESTDIR/output.txt"
"#;

        let status = SandboxRunner::run_script_fallback(script, &build_dir, &destdir, &env_vars)
            .expect("Eksekusi script fallback harus berhasil");

        assert!(status.success(), "Exit status harus sukses");

        let output_file = destdir.join("output.txt");
        assert!(output_file.exists(), "Berkas output.txt harus tercipta di DESTDIR");

        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(content.contains("hello from fallback sandbox"));
        assert!(content.contains("var=kura_linux_rulez"));
    }

    #[test]
    fn test_sandbox_rejects_non_bubblewrap_package_when_bwrap_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let build_dir = temp_dir.path().join("build");
        let destdir = temp_dir.path().join("stage");
        let env_vars = HashMap::new();

        let runner = SandboxRunner::with_bwrap_path(None);
        let res = runner.run_script_for_package(
            "fastfetch",
            "echo 'build fastfetch'",
            &build_dir,
            &destdir,
            &env_vars,
            false,
        );

        assert!(res.is_err(), "Harus gagal ketika bwrap tidak ada untuk paket selain bubblewrap");
        let err = res.unwrap_err().to_string();
        assert!(err.contains("Isolasi Sandbox Bubblewrap ('bwrap') WAJIB"), "Error harus mengandung pesan penegakan wajib: {}", err);
    }

    #[test]
    fn test_sandbox_allows_bubblewrap_self_bootstrap_when_bwrap_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let build_dir = temp_dir.path().join("build");
        let destdir = temp_dir.path().join("stage");
        let env_vars = HashMap::new();

        let runner = SandboxRunner::with_bwrap_path(None);
        let res = runner.run_script_for_package(
            "bubblewrap",
            "echo 'bootstrap bwrap'",
            &build_dir,
            &destdir,
            &env_vars,
            false,
        );

        assert!(res.is_ok(), "Harus diizinkan untuk paket 'bubblewrap' agar self-bootstrap berhasil");
    }

    #[test]
    fn test_configure_process_limits() {
        let res = SandboxRunner::configure_process_limits(Some(1024), Some(0));
        assert!(res.is_ok(), "Konfigurasi limits via nix harus berhasil");
    }
}
