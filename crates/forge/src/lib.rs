use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub mod builder;
pub mod toolchain;
pub mod cpu;
pub mod binhost;
pub mod cachyos;
pub mod cascade;
pub mod resolver;
pub mod db;
pub mod merger;
pub mod sync;
pub mod importer;
pub mod lock;
pub mod sandbox;
pub mod stage;
pub mod crypto;
pub mod privilege;
pub mod hooks;
pub mod scheduler;
pub mod downloader;
pub mod menuconfig;

pub use builder::RecipeBuilder;
pub use toolchain::{ToolchainComponent, ToolchainManager, ToolchainStatus};
pub use cpu::{CpuProfile, CacheInfo, RecommendedFlags};
pub use binhost::{BinhostCatalog, BinhostClient, BinhostPackageEntry, DownloadStreamResult, PackageSignature};
pub use cachyos::{CachyOsAdapter, CachyOsTier, CachyOsPackageMeta, parse_alpm_desc, parse_repo_db_tar_zst};
pub use cascade::{CascadeResolution, PackageCascadeResolver, PackageProvider};
pub use resolver::*;
pub use db::*;
pub use merger::*;
pub use sync::SyncClient;
pub use importer::RecipeImporter;
pub use lock::ForgeLockGuard;
pub use sandbox::SandboxRunner;
pub use stage::*;
pub use crypto::*;
pub use privilege::*;
pub use hooks::*;
pub use scheduler::*;
pub use downloader::*;
pub use menuconfig::*;

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
                recipe_server: "https://pkgkura.amqs.net/v1".to_string(),
                binhost_url: "https://pkgkura.amqs.net/v1".to_string(),
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
                cflags: "-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2".to_string(),
                cxxflags: "-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2".to_string(),
                ldflags: "-Wl,-O3 -Wl,--as-needed -Wl,--gc-sections -Wl,--icf=all -Wl,-z,relro -Wl,-z,now".to_string(),
                makeflags: format!("-j{}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(16)),
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
        let env_path = std::env::var("FORGE_CONFIG").ok().map(std::path::PathBuf::from);
        let candidates = [
            path,
            env_path.as_deref(),
            Some(Path::new("/etc/forge/forge.conf")),
            Some(Path::new("config/forge.conf")),
        ];

        for cand in candidates.into_iter().flatten() {
            if let Ok(content) = fs::read_to_string(cand) {
                if let Ok(conf) = toml::from_str::<ForgeConfig>(&content) {
                    return conf;
                }
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
}

