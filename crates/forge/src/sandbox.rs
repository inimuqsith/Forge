use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

// ============================================================================
// Konstanta Seccomp Berkeley Packet Filter (BPF)
// ============================================================================

pub const BPF_LD: u16 = 0x00;
pub const BPF_W: u16 = 0x00;
pub const BPF_ABS: u16 = 0x20;
pub const BPF_JMP: u16 = 0x05;
pub const BPF_JEQ: u16 = 0x10;
pub const BPF_K: u16 = 0x00;
pub const BPF_RET: u16 = 0x06;

pub const AUDIT_ARCH_X86_64: u32 = 0xc000003e;
pub const AUDIT_ARCH_AARCH64: u32 = 0xc00000b7;

pub const SECCOMP_RET_KILL_PROCESS: u32 = 0x80000000;
pub const SECCOMP_RET_KILL_THREAD: u32 = 0x00000000;
pub const SECCOMP_RET_TRAP: u32 = 0x03000000;
pub const SECCOMP_RET_ERRNO: u32 = 0x00050000;
pub const SECCOMP_RET_ALLOW: u32 = 0x7fff0000;
pub const SECCOMP_RET_LOG: u32 = 0x7ffc0000;

pub const SECCOMP_DATA_NR_OFFSET: u32 = 0;
pub const SECCOMP_DATA_ARCH_OFFSET: u32 = 4;

/// Representasi instruksi Berkeley Packet Filter (BPF) 8-byte
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SockFilter {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

impl SockFilter {
    pub fn new(code: u16, jt: u8, jf: u8, k: u32) -> Self {
        Self { code, jt, jf, k }
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..2].copy_from_slice(&self.code.to_ne_bytes());
        bytes[2] = self.jt;
        bytes[3] = self.jf;
        bytes[4..8].copy_from_slice(&self.k.to_ne_bytes());
        bytes
    }
}

/// Builder untuk menyusun program Seccomp Berkeley Packet Filter (BPF)
/// guna memblokir syscall berbahaya pada proses build kompilasi.
#[derive(Debug, Clone)]
pub struct SeccompFilterBuilder {
    blocked_syscalls: Vec<u32>,
    default_action: u32,
    deny_action: u32,
    arch: u32,
}

impl SeccompFilterBuilder {
    /// Inisialisasi builder dengan daftar syscall berbahaya default x86_64
    pub fn new() -> Self {
        Self {
            blocked_syscalls: Self::default_blocked_syscalls_x86_64(),
            default_action: SECCOMP_RET_ALLOW,
            deny_action: SECCOMP_RET_ERRNO | (nix::libc::EPERM as u32),
            arch: AUDIT_ARCH_X86_64,
        }
    }

    /// Inisialisasi untuk arsitektur CPU tertentu
    pub fn for_arch(arch: u32) -> Self {
        let mut builder = Self::new();
        builder.arch = arch;
        builder
    }

    /// Daftar syscall berbahaya default pada arsitektur x86_64
    pub fn default_blocked_syscalls_x86_64() -> Vec<u32> {
        vec![
            169, // sys_reboot
            246, // sys_kexec_load
            320, // sys_kexec_file_load
            175, // sys_init_module
            313, // sys_finit_module
            176, // sys_delete_module
            101, // sys_ptrace
            310, // sys_process_vm_readv
            311, // sys_process_vm_writev
            172, // sys_iopl
            173, // sys_ioperm
            164, // sys_settimeofday
            227, // sys_clock_settime
            250, // sys_keyctl
            248, // sys_add_key
            249, // sys_request_key
            321, // sys_bpf
            323, // sys_userfaultfd
            298, // sys_perf_event_open
            212, // sys_lookup_dcookie
            167, // sys_swapon
            168, // sys_swapoff
            155, // sys_pivot_root
        ]
    }

    /// Tambahkan syscall number untuk diblokir
    pub fn add_blocked_syscall(&mut self, nr: u32) -> &mut Self {
        if !self.blocked_syscalls.contains(&nr) {
            self.blocked_syscalls.push(nr);
        }
        self
    }

    /// Tambahkan beberapa syscall number sekaligus
    pub fn add_blocked_syscalls(&mut self, nrs: &[u32]) -> &mut Self {
        for &nr in nrs {
            self.add_blocked_syscall(nr);
        }
        self
    }

    /// Set aksi default untuk syscall yang tidak diblokir (default: SECCOMP_RET_ALLOW)
    pub fn with_default_action(&mut self, action: u32) -> &mut Self {
        self.default_action = action;
        self
    }

    /// Set aksi saat syscall terblokir dieksekusi (default: SECCOMP_RET_ERRNO | EPERM)
    pub fn with_deny_action(&mut self, action: u32) -> &mut Self {
        self.deny_action = action;
        self
    }

    /// Dapatkan daftar nomor syscall yang diblokir
    pub fn blocked_syscalls(&self) -> &[u32] {
        &self.blocked_syscalls
    }

    /// Kompilasi aturan ke instruksi BPF
    pub fn compile_bpf(&self) -> Vec<SockFilter> {
        let mut filter = Vec::new();
        let n = self.blocked_syscalls.len();

        // 1. Load architecture: [BPF_LD | BPF_W | BPF_ABS, 0, 0, 4]
        filter.push(SockFilter::new(BPF_LD | BPF_W | BPF_ABS, 0, 0, SECCOMP_DATA_ARCH_OFFSET));

        // 2. Check architecture: if arch == self.arch jump 1 (skip deny at inst 2), else fallthrough
        filter.push(SockFilter::new(BPF_JMP | BPF_JEQ | BPF_K, 1, 0, self.arch));

        // 3. Deny mismatched arch: [BPF_RET | BPF_K, 0, 0, deny_action]
        filter.push(SockFilter::new(BPF_RET | BPF_K, 0, 0, self.deny_action));

        // 4. Load syscall number: [BPF_LD | BPF_W | BPF_ABS, 0, 0, 0]
        filter.push(SockFilter::new(BPF_LD | BPF_W | BPF_ABS, 0, 0, SECCOMP_DATA_NR_OFFSET));

        // 5. Check each blocked syscall
        // When a match occurs, jump forward to the deny action at (4 + N + 1).
        // Distance from next instruction (4 + i + 1) to deny instruction (4 + N + 1) is (N - i).
        for (i, &nr) in self.blocked_syscalls.iter().enumerate() {
            let jt = (n - i) as u8;
            filter.push(SockFilter::new(BPF_JMP | BPF_JEQ | BPF_K, jt, 0, nr));
        }

        // 6. Default allow: [BPF_RET | BPF_K, 0, 0, default_action]
        filter.push(SockFilter::new(BPF_RET | BPF_K, 0, 0, self.default_action));

        // 7. Deny blocked syscall: [BPF_RET | BPF_K, 0, 0, deny_action]
        filter.push(SockFilter::new(BPF_RET | BPF_K, 0, 0, self.deny_action));

        filter
    }

    /// Kompilasi BPF instructions ke byte array biner
    pub fn compile_to_bytes(&self) -> Vec<u8> {
        let instructions = self.compile_bpf();
        let mut bytes = Vec::with_capacity(instructions.len() * 8);
        for inst in instructions {
            bytes.extend_from_slice(&inst.to_bytes());
        }
        bytes
    }
}

impl Default for SeccompFilterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Sandbox Runner (Bubblewrap + Seccomp BPF + Resource Limits)
// ============================================================================

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

    /// Bangun daftar argumen standar untuk Bubblewrap isolasi build (termasuk --cap-drop ALL)
    pub fn build_bwrap_args(build_dir: &Path, destdir: &Path) -> Vec<String> {
        Self::build_bwrap_args_with_seccomp(build_dir, destdir, None)
    }

    /// Bangun daftar argumen Bubblewrap dengan opsi Seccomp File Descriptor & Capability Dropping
    pub fn build_bwrap_args_with_seccomp(
        build_dir: &Path,
        destdir: &Path,
        seccomp_fd: Option<i32>,
    ) -> Vec<String> {
        let mut args = vec![
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
            "--cap-drop".to_string(),
            "ALL".to_string(),
            "--die-with-parent".to_string(),
        ];

        if let Some(fd) = seccomp_fd {
            args.push("--seccomp".to_string());
            args.push(fd.to_string());
        }

        args
    }

    /// Jalankan script kompilasi dengan validasi ketat Bubblewrap + Seccomp BPF:
    /// - Jika `bwrap` terpasang: Eksekusi di dalam Bubblewrap sandbox (--ro-bind / /, --cap-drop ALL, --seccomp FD).
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
            // Persiapkan filter Seccomp BPF
            let seccomp_builder = SeccompFilterBuilder::new();
            let bpf_bytes = seccomp_builder.compile_to_bytes();

            // Tulis BPF filter ke temporary file yang akan dibaca oleh bwrap via FD
            let mut seccomp_file = tempfile::NamedTempFile::new()
                .with_context(|| "Gagal membuat berkas sementara untuk Seccomp BPF filter")?;
            seccomp_file
                .write_all(&bpf_bytes)
                .with_context(|| "Gagal menulis bytecode Seccomp BPF")?;
            seccomp_file.flush()?;

            // Buka file descriptor read-only untuk di-pass ke bwrap
            let read_file = std::fs::File::open(seccomp_file.path())
                .with_context(|| "Gagal membuka Seccomp filter file untuk dibaca")?;
            let fd = read_file.as_raw_fd();

            let mut cmd = Command::new(bwrap);
            let bwrap_args = Self::build_bwrap_args_with_seccomp(build_dir, destdir, Some(fd));
            cmd.args(bwrap_args);
            cmd.arg("--").arg("bash").arg("-c").arg(script);
            cmd.envs(env_vars);
            cmd.current_dir(build_dir);

            // Bersihkan flag FD_CLOEXEC agar child process bwrap dapat membaca file descriptor seccomp
            unsafe {
                cmd.pre_exec(move || {
                    let flags = nix::libc::fcntl(fd, nix::libc::F_GETFD);
                    if flags != -1 {
                        nix::libc::fcntl(fd, nix::libc::F_SETFD, flags & !nix::libc::FD_CLOEXEC);
                    }
                    Ok(())
                });
            }

            match cmd.status() {
                Ok(status) => Ok(status),
                Err(e) => {
                    // Fallback jika lingkungan host melarang seccomp (misal nested unprivileged container)
                    eprintln!("  [!] Warning: Seccomp BPF filter gagal dimuat ({}), mengulang tanpa filter seccomp...", e);
                    let mut fallback_cmd = Command::new(bwrap);
                    let fallback_args = Self::build_bwrap_args(build_dir, destdir);
                    fallback_cmd.args(fallback_args);
                    fallback_cmd.arg("--").arg("bash").arg("-c").arg(script);
                    fallback_cmd.envs(env_vars);
                    fallback_cmd.current_dir(build_dir);
                    fallback_cmd.status()
                        .with_context(|| "Gagal mengeksekusi bash script di dalam sandbox Bubblewrap")
                }
            }
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
    fn test_seccomp_filter_bytecode_generation() {
        let mut builder = SeccompFilterBuilder::new();
        builder.with_deny_action(SECCOMP_RET_ERRNO | 1); // EPERM

        let filter = builder.compile_bpf();
        assert!(!filter.is_empty(), "BPF filter bytecode tidak boleh kosong");

        // 1. Verifikasi Header Arch Checking
        assert_eq!(filter[0].code, BPF_LD | BPF_W | BPF_ABS);
        assert_eq!(filter[0].k, SECCOMP_DATA_ARCH_OFFSET);
        assert_eq!(filter[1].code, BPF_JMP | BPF_JEQ | BPF_K);
        assert_eq!(filter[1].k, AUDIT_ARCH_X86_64);
        assert_eq!(filter[1].jt, 1);
        assert_eq!(filter[2].code, BPF_RET | BPF_K);
        assert_eq!(filter[2].k, SECCOMP_RET_ERRNO | 1);

        // 2. Verifikasi Syscall Loading
        assert_eq!(filter[3].code, BPF_LD | BPF_W | BPF_ABS);
        assert_eq!(filter[3].k, SECCOMP_DATA_NR_OFFSET);

        // 3. Verifikasi jumlah instruksi total = 4 (header) + N (syscall checks) + 2 (allow + deny)
        let n = builder.blocked_syscalls().len();
        assert_eq!(filter.len(), 4 + n + 2);

        // 4. Verifikasi allow & deny terminal instructions
        assert_eq!(filter[4 + n].code, BPF_RET | BPF_K);
        assert_eq!(filter[4 + n].k, SECCOMP_RET_ALLOW);
        assert_eq!(filter[4 + n + 1].code, BPF_RET | BPF_K);
        assert_eq!(filter[4 + n + 1].k, SECCOMP_RET_ERRNO | 1);

        // 5. Verifikasi serialisasi binary byte representation (8 bytes per instruction)
        let bytes = builder.compile_to_bytes();
        assert_eq!(bytes.len(), filter.len() * 8);
    }

    #[test]
    fn test_seccomp_blocked_syscall_table() {
        let builder = SeccompFilterBuilder::new();
        let blocked = builder.blocked_syscalls();

        // Verifikasi syscall krusial diblokir
        assert!(blocked.contains(&169), "reboot (169) harus diblokir");
        assert!(blocked.contains(&246), "kexec_load (246) harus diblokir");
        assert!(blocked.contains(&175), "init_module (175) harus diblokir");
        assert!(blocked.contains(&101), "ptrace (101) harus diblokir");
        assert!(blocked.contains(&172), "iopl (172) harus diblokir");
        assert!(blocked.contains(&164), "settimeofday (164) harus diblokir");
        assert!(blocked.contains(&321), "bpf (321) harus diblokir");
    }

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

        // 4. Validasi capability dropping
        assert!(args.contains(&"--cap-drop".to_string()));
        let cap_idx = args.iter().position(|r| r == "--cap-drop").unwrap();
        assert_eq!(args[cap_idx + 1], "ALL");
    }

    #[test]
    fn test_sandbox_command_construction_with_seccomp() {
        let build_dir = Path::new("/tmp/forge/build/test-pkg-1.0");
        let destdir = Path::new("/tmp/forge/stage/test-pkg");

        let args = SandboxRunner::build_bwrap_args_with_seccomp(build_dir, destdir, Some(42));
        assert!(args.contains(&"--seccomp".to_string()));
        let sec_idx = args.iter().position(|r| r == "--seccomp").unwrap();
        assert_eq!(args[sec_idx + 1], "42");
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
