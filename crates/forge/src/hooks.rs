use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use crate::merger::StagedEntry;

/// Kategori pemicu file post-merge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerCategory {
    DynamicLinker,
    DesktopDatabase,
    IconCache,
    GlibSchemas,
    MimeDatabase,
    KernelModules,
    OpenRcServices,
}

/// Engine untuk mengevaluasi dan menjalankan generic post-merge hook triggers
pub struct HookEngine;

impl HookEngine {
    /// Evaluasi daftar file yang di-merge dan jalankan hook yang sesuai
    pub fn run_post_merge_hooks(entries: &[StagedEntry], target_root: &Path) -> Vec<String> {
        let mut executed_hooks = Vec::new();
        let triggers = Self::evaluate_triggers(entries);

        for cat in triggers {
            match cat {
                TriggerCategory::DynamicLinker => {
                    if let Some(msg) = Self::run_ldconfig(target_root) {
                        executed_hooks.push(msg);
                    }
                }
                TriggerCategory::DesktopDatabase => {
                    if let Some(msg) = Self::run_desktop_database(target_root) {
                        executed_hooks.push(msg);
                    }
                }
                TriggerCategory::IconCache => {
                    if let Some(msg) = Self::run_icon_cache(target_root, entries) {
                        executed_hooks.push(msg);
                    }
                }
                TriggerCategory::GlibSchemas => {
                    if let Some(msg) = Self::run_glib_schemas(target_root) {
                        executed_hooks.push(msg);
                    }
                }
                TriggerCategory::MimeDatabase => {
                    if let Some(msg) = Self::run_mime_database(target_root) {
                        executed_hooks.push(msg);
                    }
                }
                TriggerCategory::KernelModules => {
                    if let Some(msg) = Self::run_depmod(target_root) {
                        executed_hooks.push(msg);
                    }
                }
                TriggerCategory::OpenRcServices => {
                    if let Some(msg) = Self::detect_openrc_services(target_root, entries) {
                        executed_hooks.push(msg);
                    }
                }
            }
        }

        executed_hooks
    }

    /// Pindai entries dan tentukan kategori trigger apa saja yang cocok
    pub fn evaluate_triggers(entries: &[StagedEntry]) -> HashSet<TriggerCategory> {
        let mut matched = HashSet::new();

        for entry in entries {
            let path_str = entry.relative_path.to_string_lossy();
            let clean = path_str.trim_start_matches('/');

            if clean.starts_with("usr/lib") || clean.starts_with("lib") {
                if clean.contains(".so") {
                    matched.insert(TriggerCategory::DynamicLinker);
                }
                if clean.starts_with("usr/lib/modules") || clean.starts_with("lib/modules") {
                    matched.insert(TriggerCategory::KernelModules);
                }
            }

            if clean.starts_with("usr/share/applications") && clean.ends_with(".desktop") {
                matched.insert(TriggerCategory::DesktopDatabase);
            }

            if clean.starts_with("usr/share/icons") {
                matched.insert(TriggerCategory::IconCache);
            }

            if clean.starts_with("usr/share/glib-2.0/schemas") && clean.ends_with(".xml") {
                matched.insert(TriggerCategory::GlibSchemas);
            }

            if clean.starts_with("usr/share/mime") && clean.ends_with(".xml") {
                matched.insert(TriggerCategory::MimeDatabase);
            }

            if clean.starts_with("etc/init.d") {
                matched.insert(TriggerCategory::OpenRcServices);
            }
        }

        matched
    }

    fn run_ldconfig(target_root: &Path) -> Option<String> {
        if target_root == Path::new("/") {
            if Self::command_exists("ldconfig") {
                if let Ok(st) = Command::new("ldconfig").status() {
                    if st.success() {
                        return Some("Menjalankan ldconfig untuk memperbarui dynamic library cache".to_string());
                    }
                }
            }
        } else if target_root.join("etc").exists() {
            let ldconfig_bin = target_root.join("sbin/ldconfig");
            if Self::command_exists_or_in_sysroot(&ldconfig_bin, "ldconfig") {
                let mut cmd = Command::new("ldconfig");
                cmd.arg("-r").arg(target_root);
                if let Ok(st) = cmd.status() {
                    if st.success() {
                        return Some("Menjalankan ldconfig untuk memperbarui dynamic library cache".to_string());
                    }
                }
            }
        }
        None
    }

    fn run_desktop_database(target_root: &Path) -> Option<String> {
        if Self::command_exists("update-desktop-database") {
            let app_dir = target_root.join("usr/share/applications");
            if app_dir.exists() {
                let mut cmd = Command::new("update-desktop-database");
                cmd.arg("-q").arg(&app_dir);
                if let Ok(st) = cmd.status() {
                    if st.success() {
                        return Some("Memperbarui database desktop menu entry (/usr/share/applications)".to_string());
                    }
                }
            }
        }
        None
    }

    fn run_icon_cache(target_root: &Path, entries: &[StagedEntry]) -> Option<String> {
        if Self::command_exists("gtk-update-icon-cache") {
            // Temukan direktori theme icon terpengaruh
            let mut icon_themes = HashSet::new();
            for entry in entries {
                let path_str = entry.relative_path.to_string_lossy();
                let clean = path_str.trim_start_matches('/');
                if clean.starts_with("usr/share/icons/") {
                    let parts: Vec<&str> = clean.split('/').collect();
                    if parts.len() >= 4 {
                        icon_themes.insert(parts[3].to_string());
                    }
                }
            }

            for theme in icon_themes {
                let theme_path = target_root.join("usr/share/icons").join(&theme);
                if theme_path.exists() {
                    let _ = Command::new("gtk-update-icon-cache")
                        .args(["-q", "-t", "-f"])
                        .arg(&theme_path)
                        .status();
                }
            }
            return Some("Memperbarui cache icon GTK/KDE (/usr/share/icons)".to_string());
        }
        None
    }

    fn run_glib_schemas(target_root: &Path) -> Option<String> {
        if Self::command_exists("glib-compile-schemas") {
            let schema_dir = target_root.join("usr/share/glib-2.0/schemas");
            if schema_dir.exists() {
                let mut cmd = Command::new("glib-compile-schemas");
                cmd.arg(&schema_dir);
                if let Ok(st) = cmd.status() {
                    if st.success() {
                        return Some("Mengompilasi schema GSettings Glib (/usr/share/glib-2.0/schemas)".to_string());
                    }
                }
            }
        }
        None
    }

    fn run_mime_database(target_root: &Path) -> Option<String> {
        if Self::command_exists("update-mime-database") {
            let mime_dir = target_root.join("usr/share/mime");
            if mime_dir.exists() {
                let mut cmd = Command::new("update-mime-database");
                cmd.arg(&mime_dir);
                if let Ok(st) = cmd.status() {
                    if st.success() {
                        return Some("Memperbarui database MIME types (/usr/share/mime)".to_string());
                    }
                }
            }
        }
        None
    }

    fn run_depmod(target_root: &Path) -> Option<String> {
        if Self::command_exists("depmod") {
            let mut cmd = Command::new("depmod");
            cmd.arg("-a");
            if target_root != Path::new("/") {
                cmd.arg("-b").arg(target_root);
            }
            if let Ok(st) = cmd.status() {
                if st.success() {
                    return Some("Menjalankan depmod untuk memperbarui indeks dependensi modul kernel".to_string());
                }
            }
        }
        None
    }

    fn detect_openrc_services(_target_root: &Path, entries: &[StagedEntry]) -> Option<String> {
        let mut services = Vec::new();
        for entry in entries {
            let path_str = entry.relative_path.to_string_lossy();
            let clean = path_str.trim_start_matches('/');
            if clean.starts_with("etc/init.d/") {
                if let Some(service_name) = clean.strip_prefix("etc/init.d/") {
                    if !service_name.is_empty() && !service_name.contains('/') {
                        services.push(service_name.to_string());
                    }
                }
            }
        }

        if !services.is_empty() {
            Some(format!(
                "Layanan OpenRC terdeteksi: {}. Aktifkan dengan 'rc-update add <service> default'",
                services.join(", ")
            ))
        } else {
            None
        }
    }

    fn command_exists(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn command_exists_or_in_sysroot(path: &Path, cmd: &str) -> bool {
        path.exists() || Self::command_exists(cmd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::merger::FileType;

    #[test]
    fn test_evaluate_triggers_desktop_and_so() {
        let entries = vec![
            StagedEntry {
                relative_path: PathBuf::from("/usr/bin/fastfetch"),
                file_type: FileType::Regular,
                size: 100,
                mode: 0o755,
                uid: 0,
                gid: 0,
                sha256: None,
                symlink_target: None,
            },
            StagedEntry {
                relative_path: PathBuf::from("/usr/share/applications/fastfetch.desktop"),
                file_type: FileType::Regular,
                size: 200,
                mode: 0o644,
                uid: 0,
                gid: 0,
                sha256: None,
                symlink_target: None,
            },
            StagedEntry {
                relative_path: PathBuf::from("/usr/lib/libcurl.so.4.8.0"),
                file_type: FileType::Regular,
                size: 500000,
                mode: 0o755,
                uid: 0,
                gid: 0,
                sha256: None,
                symlink_target: None,
            },
            StagedEntry {
                relative_path: PathBuf::from("/etc/init.d/sshd"),
                file_type: FileType::Regular,
                size: 500,
                mode: 0o755,
                uid: 0,
                gid: 0,
                sha256: None,
                symlink_target: None,
            },
        ];

        let triggers = HookEngine::evaluate_triggers(&entries);
        assert!(triggers.contains(&TriggerCategory::DesktopDatabase));
        assert!(triggers.contains(&TriggerCategory::DynamicLinker));
        assert!(triggers.contains(&TriggerCategory::OpenRcServices));
        assert!(!triggers.contains(&TriggerCategory::GlibSchemas));
        assert!(!triggers.contains(&TriggerCategory::MimeDatabase));
    }

    #[test]
    fn test_evaluate_triggers_glib_and_icons() {
        let entries = vec![
            StagedEntry {
                relative_path: PathBuf::from("/usr/share/glib-2.0/schemas/org.gnome.shell.gschema.xml"),
                file_type: FileType::Regular,
                size: 300,
                mode: 0o644,
                uid: 0,
                gid: 0,
                sha256: None,
                symlink_target: None,
            },
            StagedEntry {
                relative_path: PathBuf::from("/usr/share/icons/hicolor/scalable/apps/org.kura.forge.svg"),
                file_type: FileType::Regular,
                size: 400,
                mode: 0o644,
                uid: 0,
                gid: 0,
                sha256: None,
                symlink_target: None,
            },
        ];

        let triggers = HookEngine::evaluate_triggers(&entries);
        assert!(triggers.contains(&TriggerCategory::GlibSchemas));
        assert!(triggers.contains(&TriggerCategory::IconCache));
        assert!(!triggers.contains(&TriggerCategory::DynamicLinker));
    }
}
