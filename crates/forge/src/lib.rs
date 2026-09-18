use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub mod builder;
pub mod toolchain;
pub mod cpu;
pub mod binhost;
pub mod hybrid;

pub use builder::RecipeBuilder;
pub use toolchain::{ToolchainComponent, ToolchainManager, ToolchainStatus};
pub use cpu::{CpuProfile, CacheInfo, RecommendedFlags};
pub use binhost::{BinhostCatalog, BinhostClient, BinhostPackageEntry};
pub use hybrid::HybridAdapter;

/// Konfigurasi Global Forge (/etc/forge/forge.conf)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeConfig {
    pub general: GeneralConfig,
    pub server: ServerConfig,
    pub binhost: BinhostConfig,
    pub hybrid: HybridConfig,
    pub cpu: CpuConfig,
    pub build: BuildConfig,
    pub use_flags: UseConfig,
    pub hooks: HooksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub root: String,
    pub db_path: String,
    pub cache_path: String,
    pub build_path: String,
    pub stage_path: String,
    pub recipes_path: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub recipe_server: String,
    pub binhost_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinhostConfig {
    pub enable_binhost: bool,
    pub auto_match_cpu: bool,
    pub fallback_to_source: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    pub enable_cachyos_fallback: bool,
    pub cachyos_repo_url: String,
    pub enable_arch_fallback: bool,
    pub arch_repo_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    pub target_march: String,
    pub enable_avx512: bool,
    pub enable_avx2: bool,
    pub profile_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub cflags: String,
    pub cxxflags: String,
    pub ldflags: String,
    pub makeflags: String,
    pub jobs: String,
    pub prefix: String,
    #[serde(default = "default_true")]
    pub enable_ccache: bool,
    #[serde(default = "default_ccache_dir")]
    pub ccache_dir: String,
}

fn default_true() -> bool {
    true
}

fn default_ccache_dir() -> String {
    "/var/cache/forge/ccache".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseConfig {
    pub flags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    pub enable_openrc_hooks: bool,
    pub enable_ldconfig_hooks: bool,
    pub enable_mandoc_hooks: bool,
    pub auto_prompt_services: bool,
}

impl Default for ForgeConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                root: "/".to_string(),
                db_path: "/var/db/forge".to_string(),
                cache_path: "/var/cache/forge/distfiles".to_string(),
                build_path: "/tmp/forge/build".to_string(),
                stage_path: "/tmp/forge/stage".to_string(),
                recipes_path: "/var/db/forge/recipes".to_string(),
                mode: "source".to_string(),
            },
            server: ServerConfig {
                recipe_server: "https://recipes.kuralinux.org/v1".to_string(),
                binhost_url: "https://binhost.kuralinux.org/v1".to_string(),
            },
            binhost: BinhostConfig {
                enable_binhost: false,
                auto_match_cpu: true,
                fallback_to_source: true,
            },
            hybrid: HybridConfig {
                enable_cachyos_fallback: false,
                cachyos_repo_url: "https://mirror.cachyos.org/repo/x86_64_v4/cachyos_v4".to_string(),
                enable_arch_fallback: false,
                arch_repo_url: "https://geo.mirror.pkgbuild.com/core/os/x86_64".to_string(),
            },
            cpu: CpuConfig {
                target_march: "native".to_string(),
                enable_avx512: true,
                enable_avx2: true,
                profile_file: "/etc/forge/cpu-profile.json".to_string(),
            },
            build: BuildConfig {
                cflags: "-O3 -march=native -pipe -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt".to_string(),
                cxxflags: "-O3 -march=native -pipe -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt".to_string(),
                ldflags: "-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now".to_string(),
                makeflags: "-j4".to_string(),
                jobs: "auto".to_string(),
                prefix: "/usr".to_string(),
                enable_ccache: true,
                ccache_dir: "/var/cache/forge/ccache".to_string(),
            },
            use_flags: UseConfig {
                flags: "ssl openrc alsa -systemd lto pgo".to_string(),
            },
            hooks: HooksConfig {
                enable_openrc_hooks: true,
                enable_ldconfig_hooks: true,
                enable_mandoc_hooks: true,
                auto_prompt_services: true,
            },
        }
    }
}

impl ForgeConfig {
    pub fn load_or_default(path: Option<&Path>) -> Self {
        let conf_path = path.unwrap_or_else(|| Path::new("/etc/forge/forge.conf"));
        if let Ok(content) = fs::read_to_string(conf_path) {
            if let Ok(conf) = toml::from_str::<ForgeConfig>(&content) {
                return conf;
            }
        }
        Self::default()
    }
}

/// Model Resep Forge (recipe.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: Option<DependenciesMeta>,
    #[serde(default)]
    pub sources: Option<SourcesMeta>,
    #[serde(default)]
    pub build: Option<BuildMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
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
}

fn default_release() -> u32 {
    1
}

fn default_slot() -> String {
    "0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependenciesMeta {
    #[serde(default)]
    pub runtime: Vec<String>,
    #[serde(default)]
    pub build: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourcesMeta {
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub sha256: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildMeta {
    #[serde(default)]
    pub r#type: String,
    #[serde(default)]
    pub configure_args: Vec<String>,
    #[serde(default)]
    pub script: String,
    #[serde(default)]
    pub compiler_override: Option<String>,
    #[serde(default)]
    pub disable_custom_march: bool,
}

/// Evaluator USE Flags ala Portage
pub struct UseFlagsEngine {
    active_flags: HashSet<String>,
}

impl UseFlagsEngine {
    pub fn new(global_flags: &str, package_flags: Option<&str>) -> Self {
        let mut active = HashSet::new();
        for flag in global_flags.split_whitespace() {
            if let Some(stripped) = flag.strip_prefix('-') {
                active.remove(stripped);
            } else {
                active.insert(flag.to_string());
            }
        }
        if let Some(pkg_flags) = package_flags {
            for flag in pkg_flags.split_whitespace() {
                if let Some(stripped) = flag.strip_prefix('-') {
                    active.remove(stripped);
                } else {
                    active.insert(flag.to_string());
                }
            }
        }
        Self { active_flags: active }
    }

    pub fn is_enabled(&self, flag: &str) -> bool {
        self.active_flags.contains(flag)
    }
}

/// Konfigurasi Bootstrap Distro Kura Linux (/etc/forge/system.conf)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSetupConfig {
    pub system: SystemMeta,
    pub target: TargetMeta,
    pub kernel: KernelMeta,
    pub init: InitMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMeta {
    pub profile: String,
    pub hostname: String,
    pub locale: String,
    pub keymap: String,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMeta {
    pub architecture: String,
    pub march: String,
    pub enable_avx512: bool,
    pub enable_avx2: bool,
    pub cflags: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelMeta {
    pub r#type: String,
    pub drivers_builtin: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitMeta {
    pub manager: String,
    pub default_services: Vec<String>,
}

impl Default for SystemSetupConfig {
    fn default() -> Self {
        Self {
            system: SystemMeta {
                profile: "standard".to_string(),
                hostname: "kuralinux".to_string(),
                locale: "en_US.UTF-8".to_string(),
                keymap: "us".to_string(),
                timezone: "UTC".to_string(),
            },
            target: TargetMeta {
                architecture: "x86_64".to_string(),
                march: "native".to_string(),
                enable_avx512: true,
                enable_avx2: true,
                cflags: "-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt".to_string(),
            },
            kernel: KernelMeta {
                r#type: "monolithic".to_string(),
                drivers_builtin: vec![
                    "ext4".to_string(),
                    "nvme".to_string(),
                    "sata_ahci".to_string(),
                    "virtio".to_string(),
                    "virtio_pci".to_string(),
                    "virtio_blk".to_string(),
                    "virtio_net".to_string(),
                ],
            },
            init: InitMeta {
                manager: "openrc".to_string(),
                default_services: vec![
                    "metalog".to_string(),
                    "chronyd".to_string(),
                    "eudev".to_string(),
                    "dhcpcd".to_string(),
                    "acpid".to_string(),
                ],
            },
        }
    }
}

impl SystemSetupConfig {
    pub fn load_or_default(path: Option<&Path>) -> Self {
        let conf_path = path.unwrap_or_else(|| Path::new("/etc/forge/system.conf"));
        if let Ok(content) = fs::read_to_string(conf_path) {
            if let Ok(conf) = toml::from_str::<SystemSetupConfig>(&content) {
                return conf;
            }
        }
        Self::default()
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("Gagal membuat direktori {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).with_context(|| format!("Gagal menulis ke {:?}", path))?;
        Ok(())
    }
}

/// Daftar Paket Default Set @system Kura Linux
pub fn get_default_system_packages() -> Vec<&'static str> {
    vec![
        "glibc", "llvm", "mold", "ninja", "gcc", "binutils", "linux-headers",
        "coreutils", "bash", "sed", "grep", "gawk", "make", "patch",
        "tar", "xz", "zstd", "findutils", "diffutils", "file", "which",
        "linux", "grub", "openrc", "eudev", "acpid", "kmod", "util-linux",
        "shadow", "opendoas", "elogind", "dbus", "dhcpcd", "iwd", "chrony",
        "openssl", "ca-certificates", "curl", "e2fsprogs", "dosfstools", "pkgconf",
        "metalog", "cronie", "earlyoom", "nftables", "mandoc", "nano", "less",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_use_flags_engine() {
        let engine = UseFlagsEngine::new("ssl openrc -systemd lto", Some("-lto pam"));
        assert!(engine.is_enabled("ssl"));
        assert!(engine.is_enabled("openrc"));
        assert!(!engine.is_enabled("systemd"));
        assert!(!engine.is_enabled("lto"));
        assert!(engine.is_enabled("pam"));
    }

    #[test]
    fn test_default_system_packages() {
        let pkgs = get_default_system_packages();
        assert!(pkgs.contains(&"glibc"));
        assert!(pkgs.contains(&"openrc"));
        assert!(pkgs.contains(&"linux"));
    }
}
