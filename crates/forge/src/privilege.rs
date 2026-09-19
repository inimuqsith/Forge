use anyhow::{bail, Result};
use colored::*;
use nix::unistd::geteuid;
use std::io::IsTerminal;
use std::path::Path;
use std::process::Command;

/// Manajer Hak Akses & Eskalasi Root Package Manager Forge
pub struct PrivilegeManager;

impl PrivilegeManager {
    /// Cek apakah proses saat ini berjalan dengan hak akses root (UID 0)
    pub fn is_root() -> bool {
        geteuid().is_root()
    }

    /// Apakah path yang dituju merupakan rootfs utama sistem ('/' atau empty)
    pub fn is_system_root(target_root: &Path) -> bool {
        target_root == Path::new("/") || target_root.as_os_str().is_empty()
    }

    /// Pastikan proses memiliki hak akses root sebelum menjalankan operasi mutatif ke rootfs sistem.
    ///
    /// - Jika `target_root` BUKAN sistem ('/'), misal `--root /tmp/stage`: diizinkan berjalan non-root.
    /// - Jika `target_root` ADALAH sistem ('/') dan proses sudah root (UID 0): langsung lanjut.
    /// - Jika `target_root` ADALAH sistem ('/') dan proses BUKAN root:
    ///   * Jika di terminal interaktif (TTY) dan `sudo`/`doas` tersedia: otomatis re-exec via `sudo`/`doas` (prompt password).
    ///   * Jika non-interaktif (script/CI-CD) atau `sudo` tidak ada: berhenti dengan pesan error yang jelas.
    pub fn ensure_root_or_escalate(target_root: &Path, op_name: &str) -> Result<()> {
        if !Self::is_system_root(target_root) {
            // Target adalah direktori kustom/staging, tidak wajib root
            return Ok(());
        }

        if Self::is_root() {
            // Sudah root (UID 0)
            return Ok(());
        }

        let is_tty = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
        let tool_opt = if Self::has_command("sudo") {
            Some("sudo")
        } else if Self::has_command("doas") {
            Some("doas")
        } else {
            None
        };

        // Jika interaktif dan alat eskalasi tersedia, re-exec otomatis
        if is_tty {
            if let Some(tool) = tool_opt {
                println!(
                    "{} Operasi '{}' ke rootfs sistem ('/') memerlukan hak Administrator (root).",
                    "[!]".yellow().bold(),
                    op_name.bold()
                );
                println!(
                    "{} Mengalihkan eksekusi secara otomatis menggunakan '{}'...",
                    "[*]".blue(),
                    tool.bold().green()
                );

                // Dapatkan argumen proses saat ini
                let raw_args: Vec<String> = std::env::args().collect();
                // Eksekusi tool (sudo/doas) dengan seluruh argumen
                let status = Command::new(tool)
                    .args(&raw_args)
                    .status();

                match status {
                    Ok(exit_status) => {
                        let code = exit_status.code().unwrap_or(1);
                        std::process::exit(code);
                    }
                    Err(e) => {
                        bail!("Gagal menjalankan '{}': {:#}", tool, e);
                    }
                }
            }
        }

        // Pesan error ramah pengguna jika non-interaktif atau tanpa sudo
        let current_cmd = std::env::args().collect::<Vec<_>>().join(" ");
        let bin_name = std::env::args().next().unwrap_or_else(|| "forge".to_string());

        bail!(
            "Akses Ditolak: Operasi '{}' ke rootfs sistem ('/') memerlukan hak Administrator (root).\n\n\
            {} Silakan jalankan perintah dengan hak root:\n\
            {} sudo {}\n\n\
            {} Atau tentukan root kustom non-root:\n\
            {} {} --root /path/to/custom/sysroot",
            op_name,
            "💡".cyan(),
            "->".green().bold(),
            current_cmd.bold(),
            "💡".cyan(),
            "->".green().bold(),
            bin_name
        );
    }

    /// Deteksi ketersediaan executable di sistem host
    fn has_command(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_system_root_detection() {
        assert!(PrivilegeManager::is_system_root(Path::new("/")));
        assert!(PrivilegeManager::is_system_root(Path::new("")));
        assert!(!PrivilegeManager::is_system_root(Path::new("/mnt/kura")));
        assert!(!PrivilegeManager::is_system_root(Path::new("/tmp/staging")));
    }

    #[test]
    fn test_ensure_root_allows_non_system_root() {
        let custom_root = Path::new("/tmp/test_sysroot_non_root");
        // Harus selalu Ok meskipun proses non-root
        assert!(PrivilegeManager::ensure_root_or_escalate(custom_root, "test").is_ok());
    }
}
