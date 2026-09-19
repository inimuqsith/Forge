use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, MultiSelect};
use std::collections::HashSet;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::{ForgeConfig, Recipe};

/// Definisi metadata sebuah USE flag di Kura Linux
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
    pub default_enabled: bool,
}

/// Engine TUI Menuconfig untuk konfigurasi interaktif USE flags
pub struct UseFlagsTui;

impl UseFlagsTui {
    /// Katalog standar USE flags resmi distribusi Kura Linux
    pub fn standard_catalog() -> Vec<FlagDefinition> {
        const FLAGS: &[(&str, &str, &str, bool)] = &[
            ("lto", "Link-Time Optimization (Thin/Full LTO) untuk inlining cross-crate", "Toolchain", true),
            ("pgo", "Profile-Guided Optimization untuk eksekusi branch prediction maksimal", "Toolchain", true),
            ("hardened", "Keamanan ekstra compiler (-fstack-protector-strong, -D_FORTIFY_SOURCE=2)", "Toolchain", true),
            ("wayland", "Dukungan modern Wayland display protocol & compositor", "Graphics", true),
            ("x11", "Dukungan legacy X11 / Xorg display server protocol", "Graphics", false),
            ("vulkan", "Dukungan low-overhead 3D graphics & compute Vulkan API", "Graphics", true),
            ("opengl", "Dukungan standar OpenGL/EGL accelerated rendering", "Graphics", true),
            ("alsa", "Advanced Linux Sound Architecture kernel sound API", "Audio", true),
            ("pipewire", "Next-generation multimedia server & audio graph", "Audio", true),
            ("pulseaudio", "PulseAudio client library compatibility layer", "Audio", false),
            ("openrc", "Integrasi native OpenRC init system scripts (/etc/init.d/)", "System", true),
            ("systemd", "Dukungan unit file systemd (Haram/Dinonaktifkan di Kura Linux)", "System", false),
            ("dbus", "D-Bus inter-process communication message bus system", "System", true),
            ("pam", "Pluggable Authentication Modules untuk otentikasi login", "System", true),
            ("ssl", "Enkripsi TLS/SSL standar (OpenSSL / Rustls backend)", "Security", true),
            ("curl", "Dukungan transfer jaringan via libcurl", "Network", true),
            ("zstd", "Kompresi ultra-cepat Zstandard realtime", "Compression", true),
            ("brotli", "Kompresi web modern Google Brotli", "Compression", false),
            ("qt6", "Dukungan antarmuka Qt6 / KDE Frameworks 6", "GUI Toolkits", true),
            ("gtk4", "Dukungan antarmuka GTK 4 / GNOME toolkit", "GUI Toolkits", false),
        ];

        FLAGS
            .iter()
            .map(|&(name, desc, cat, def)| FlagDefinition {
                name: name.to_string(),
                description: desc.to_string(),
                category: cat.to_string(),
                default_enabled: def,
            })
            .collect()
    }

    /// Ekstraksi seluruh flag bersyarat dari sebuah berkas resep (misal: "wayland? ( dep )")
    pub fn extract_recipe_flags(recipe: &Recipe) -> Vec<String> {
        let mut flags = HashSet::new();

        let parse_token = |token: &str, set: &mut HashSet<String>| {
            let trimmed = token.trim();
            if let Some(pos) = trimmed.find('?') {
                let (flag_part, _) = trimmed.split_at(pos);
                let flag_clean = flag_part.trim().trim_start_matches('!');
                if !flag_clean.is_empty() {
                    set.insert(flag_clean.to_string());
                }
            }
        };

        if let Some(ref deps) = recipe.dependencies {
            for dep in &deps.runtime {
                parse_token(dep, &mut flags);
            }
            for dep in &deps.build {
                parse_token(dep, &mut flags);
            }
        }

        if let Some(ref build) = recipe.build {
            for arg in &build.configure_args {
                parse_token(arg, &mut flags);
            }
        }

        let mut sorted: Vec<String> = flags.into_iter().collect();
        sorted.sort();
        sorted
    }

    /// Gabungkan katalog standar dengan flag dinamis yang ditemukan pada resep
    pub fn build_composite_catalog(recipe: Option<&Recipe>) -> Vec<FlagDefinition> {
        let mut catalog = Self::standard_catalog();
        let mut existing_names: HashSet<String> = catalog.iter().map(|f| f.name.clone()).collect();

        if let Some(r) = recipe {
            let recipe_flags = Self::extract_recipe_flags(r);
            for flag in recipe_flags {
                if !existing_names.contains(&flag) {
                    existing_names.insert(flag.clone());
                    catalog.push(FlagDefinition {
                        name: flag.clone(),
                        description: format!("Resep Khusus: Opsional untuk {}", r.package.name),
                        category: "Package Specific".to_string(),
                        default_enabled: false,
                    });
                }
            }
        }

        catalog
    }

    /// Parsing string flags menjadi set flag aktif (enabled) dan eksplisit non-aktif (-flag)
    pub fn parse_active_flags(raw: &str) -> (HashSet<String>, HashSet<String>) {
        let mut enabled = HashSet::new();
        let mut disabled = HashSet::new();

        for token in raw.split_whitespace() {
            let trimmed = token.trim();
            if let Some(stripped) = trimmed.strip_prefix('-') {
                enabled.remove(stripped);
                disabled.insert(stripped.to_string());
            } else if !trimmed.is_empty() {
                disabled.remove(trimmed);
                enabled.insert(trimmed.to_string());
            }
        }

        (enabled, disabled)
    }

    /// Serialisasi himpunan flag aktif kembali ke string flags standar Forge
    pub fn serialize_flags(enabled_set: &HashSet<String>, catalog: &[FlagDefinition]) -> String {
        let mut tokens = Vec::new();

        for flag in catalog {
            if enabled_set.contains(&flag.name) {
                tokens.push(flag.name.clone());
            } else {
                tokens.push(format!("-{}", flag.name));
            }
        }

        // Sertakan juga flag ekstra di luar katalog jika ada
        for flag in enabled_set {
            if !catalog.iter().any(|f| &f.name == flag) {
                tokens.push(flag.clone());
            }
        }

        tokens.join(" ")
    }

    /// Tampilkan prompt TUI interaktif untuk memilih USE flags
    pub fn prompt_interactive(
        current_flags: &str,
        recipe: Option<&Recipe>,
        pkg_name: Option<&str>,
    ) -> Result<String> {
        // Fallback untuk lingkungan non-TTY (CI / script headless)
        if !std::io::stdin().is_terminal() {
            println!(
                "  {} Lingkungan non-TTY terdeteksi, menggunakan konfigurasi USE flags default.",
                "[i]".blue()
            );
            return Ok(current_flags.to_string());
        }

        let catalog = Self::build_composite_catalog(recipe);
        let (active_set, _) = Self::parse_active_flags(current_flags);

        let target_desc = match pkg_name {
            Some(pkg) => format!("Paket: {}", pkg.bold().green()),
            None => "Global Sistem (/etc/forge/forge.conf)".bold().cyan().to_string(),
        };

        println!("\n{}", "=== Forge TUI Menuconfig — USE Flags Selector ===".bold().cyan());
        println!("  Target Konfigurasi: {}", target_desc);
        println!("  Gunakan tombol [Spasi] untuk toggle, [Panah Atas/Bawah] untuk navigasi, [Enter] untuk Simpan.\n");

        let mut items = Vec::new();
        let mut defaults = Vec::new();

        for flag in &catalog {
            let item_label = format!(
                "[{:<12}] {:<14} — {}",
                flag.category.yellow(),
                flag.name.bold(),
                flag.description.dimmed()
            );
            items.push(item_label);

            let is_selected = if active_set.contains(&flag.name) {
                true
            } else if active_set.is_empty() {
                flag.default_enabled
            } else {
                false
            };
            defaults.push(is_selected);
        }

        let chosen_indices = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Pilih USE Flags yang ingin diaktifkan")
            .items(&items)
            .defaults(&defaults)
            .interact()?;

        let mut new_enabled = HashSet::new();
        for idx in chosen_indices {
            if let Some(flag) = catalog.get(idx) {
                new_enabled.insert(flag.name.clone());
            }
        }

        let serialized = Self::serialize_flags(&new_enabled, &catalog);
        println!(
            "\n{} USE Flags berhasil diperbarui: {}\n",
            "✓".green().bold(),
            serialized.bold().yellow()
        );

        Ok(serialized)
    }

    /// Simpan konfigurasi USE flags global ke /etc/forge/forge.conf
    pub fn save_global_config(config_path: &Path, new_flags: &str) -> Result<()> {
        let mut config = ForgeConfig::load_or_default(Some(config_path));
        config.use_flags.flags = new_flags.to_string();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(&config)
            .context("Gagal melakukan serialisasi ForgeConfig ke TOML")?;
        fs::write(config_path, toml_str)
            .with_context(|| format!("Gagal menyimpan konfigurasi ke {:?}", config_path))?;

        println!(
            "  [✓] Konfigurasi global disimpan di {:?}",
            config_path
        );
        Ok(())
    }

    /// Simpan override USE flags per-paket ke /etc/forge/package.use/{pkgname}
    pub fn save_package_use(etc_dir: &Path, pkg_name: &str, flags: &str) -> Result<PathBuf> {
        let pkg_use_dir = etc_dir.join("package.use");
        fs::create_dir_all(&pkg_use_dir)?;

        let target_file = pkg_use_dir.join(pkg_name);
        let content = format!("# USE Flags override for {}\n{}\n", pkg_name, flags);
        fs::write(&target_file, content)?;

        println!(
            "  [✓] Override per-paket disimpan di {:?}",
            target_file
        );
        Ok(target_file)
    }

    /// Baca override USE flags per-paket jika tersedia
    pub fn load_package_use(etc_dir: &Path, pkg_name: &str) -> Option<String> {
        let target_file = etc_dir.join("package.use").join(pkg_name);
        if target_file.exists() {
            if let Ok(content) = fs::read_to_string(target_file) {
                let lines: Vec<&str> = content
                    .lines()
                    .filter(|l| !l.trim().starts_with('#') && !l.trim().is_empty())
                    .collect();
                return Some(lines.join(" "));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_active_flags_and_serialization() {
        let input = "ssl openrc alsa -systemd lto wayland -x11";
        let (enabled, disabled) = UseFlagsTui::parse_active_flags(input);

        assert!(enabled.contains("ssl"));
        assert!(enabled.contains("openrc"));
        assert!(enabled.contains("wayland"));
        assert!(disabled.contains("systemd"));
        assert!(disabled.contains("x11"));

        let catalog = UseFlagsTui::standard_catalog();
        let serialized = UseFlagsTui::serialize_flags(&enabled, &catalog);

        assert!(serialized.contains("ssl"));
        assert!(serialized.contains("wayland"));
        assert!(serialized.contains("-systemd"));
        assert!(serialized.contains("-x11"));
    }

    #[test]
    fn test_extract_recipe_flags() -> Result<()> {
        let toml_content = r#"
[package]
name = "mpv"
version = "0.38.0"

[dependencies]
runtime = ["alsa? ( alsa-lib )", "wayland? ( wayland )", "!x11? ( pipewire )", "glibc"]
build = ["meson", "ninja", "vulkan? ( vulkan-headers )"]

[build]
type = "meson"
configure_args = ["-Dwayland=enabled", "pipewire? ( -Dpipewire=enabled )"]
"#;

        let recipe = toml::from_str::<Recipe>(toml_content)?;
        let extracted = UseFlagsTui::extract_recipe_flags(&recipe);

        assert!(extracted.contains(&"alsa".to_string()));
        assert!(extracted.contains(&"wayland".to_string()));
        assert!(extracted.contains(&"x11".to_string()));
        assert!(extracted.contains(&"pipewire".to_string()));
        assert!(extracted.contains(&"vulkan".to_string()));

        Ok(())
    }

    #[test]
    fn test_save_and_load_package_use() -> Result<()> {
        let temp = tempdir()?;
        let etc_dir = temp.path().join("etc").join("forge");

        let pkg = "mpv";
        let flags = "wayland pipewire -x11";

        let saved_path = UseFlagsTui::save_package_use(&etc_dir, pkg, flags)?;
        assert!(saved_path.exists());

        let loaded = UseFlagsTui::load_package_use(&etc_dir, pkg);
        assert_eq!(loaded, Some("wayland pipewire -x11".to_string()));

        Ok(())
    }

    #[test]
    fn test_save_global_config_preserves_structure() -> Result<()> {
        let temp = tempdir()?;
        let conf_file = temp.path().join("forge.conf");

        UseFlagsTui::save_global_config(&conf_file, "ssl openrc wayland lto")?;
        assert!(conf_file.exists());

        let loaded = ForgeConfig::load_or_default(Some(&conf_file));
        assert_eq!(loaded.use_flags.flags, "ssl openrc wayland lto");
        assert_eq!(loaded.cpu.target_march, "native");

        Ok(())
    }
}
