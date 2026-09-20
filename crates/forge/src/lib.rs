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
pub use sandbox::{SandboxRunner, SeccompFilterBuilder, SockFilter};
pub use stage::*;
pub use crypto::*;
pub use privilege::*;
pub use hooks::*;
pub use scheduler::*;
pub use downloader::*;
pub use menuconfig::*;

/// Konfigurasi Global Forge (/etc/forge/forge.conf)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ForgeConfig {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub binhost: BinhostConfig,
    #[serde(default)]
    pub hybrid: HybridConfig,
    #[serde(default)]
    pub cpu: CpuConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default, alias = "use")]
    pub use_flags: UseConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_cache_path")]
    pub cache_path: String,
    #[serde(default = "default_build_path")]
    pub build_path: String,
    #[serde(default = "default_stage_path")]
    pub stage_path: String,
    #[serde(default = "default_recipes_path")]
    pub recipes_path: String,
    #[serde(default = "default_mode")]
    pub mode: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            root: default_root(),
            db_path: default_db_path(),
            cache_path: default_cache_path(),
            build_path: default_build_path(),
            stage_path: default_stage_path(),
            recipes_path: default_recipes_path(),
            mode: default_mode(),
        }
    }
}

fn default_root() -> String { "/".to_string() }
fn default_db_path() -> String { "/var/db/forge".to_string() }
fn default_cache_path() -> String { "/var/cache/forge/distfiles".to_string() }
fn default_build_path() -> String { "/tmp/forge/build".to_string() }
fn default_stage_path() -> String { "/tmp/forge/stage".to_string() }
fn default_recipes_path() -> String { "/var/db/forge/recipes".to_string() }
fn default_mode() -> String { "source".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_recipe_server")]
    pub recipe_server: String,
    #[serde(default = "default_binhost_url")]
    pub binhost_url: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            recipe_server: default_recipe_server(),
            binhost_url: default_binhost_url(),
        }
    }
}

fn default_recipe_server() -> String { "https://pkgkura.amqs.net/v1".to_string() }
fn default_binhost_url() -> String { "https://pkgkura.amqs.net/v1".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinhostConfig {
    #[serde(default)]
    pub enable_binhost: bool,
    #[serde(default = "default_true")]
    pub auto_match_cpu: bool,
    #[serde(default = "default_true")]
    pub fallback_to_source: bool,
}

impl Default for BinhostConfig {
    fn default() -> Self {
        Self {
            enable_binhost: false,
            auto_match_cpu: true,
            fallback_to_source: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfig {
    #[serde(default)]
    pub enable_cachyos_fallback: bool,
    #[serde(default = "default_cachyos_repo_url")]
    pub cachyos_repo_url: String,
    #[serde(default)]
    pub enable_arch_fallback: bool,
    #[serde(default = "default_arch_repo_url")]
    pub arch_repo_url: String,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            enable_cachyos_fallback: false,
            cachyos_repo_url: default_cachyos_repo_url(),
            enable_arch_fallback: false,
            arch_repo_url: default_arch_repo_url(),
        }
    }
}

fn default_cachyos_repo_url() -> String { "https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-v4".to_string() }
fn default_arch_repo_url() -> String { "https://geo.mirror.pkgbuild.com/core/os/x86_64".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuConfig {
    #[serde(default = "default_target_march")]
    pub target_march: String,
    #[serde(default = "default_true")]
    pub enable_avx512: bool,
    #[serde(default = "default_true")]
    pub enable_avx2: bool,
    #[serde(default = "default_profile_file")]
    pub profile_file: String,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            target_march: default_target_march(),
            enable_avx512: true,
            enable_avx2: true,
            profile_file: default_profile_file(),
        }
    }
}

fn default_target_march() -> String { "native".to_string() }
fn default_profile_file() -> String { "/etc/forge/cpu-profile.json".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    #[serde(default = "default_cflags")]
    pub cflags: String,
    #[serde(default = "default_cxxflags")]
    pub cxxflags: String,
    #[serde(default = "default_ldflags")]
    pub ldflags: String,
    #[serde(default = "default_makeflags")]
    pub makeflags: String,
    #[serde(default = "default_jobs")]
    pub jobs: String,
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_true")]
    pub enable_ccache: bool,
    #[serde(default = "default_ccache_dir")]
    pub ccache_dir: String,
    #[serde(default = "default_cargo_cache_dir")]
    pub cargo_cache_dir: String,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            cflags: default_cflags(),
            cxxflags: default_cxxflags(),
            ldflags: default_ldflags(),
            makeflags: default_makeflags(),
            jobs: default_jobs(),
            prefix: default_prefix(),
            enable_ccache: true,
            ccache_dir: default_ccache_dir(),
            cargo_cache_dir: default_cargo_cache_dir(),
        }
    }
}

fn default_cflags() -> String { "-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2".to_string() }
fn default_cxxflags() -> String { "-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2".to_string() }
fn default_ldflags() -> String { "-Wl,-O3 -Wl,--as-needed -Wl,--gc-sections -Wl,--icf=all -Wl,-z,relro -Wl,-z,now".to_string() }
fn default_makeflags() -> String { format!("-j{}", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(16)) }
fn default_jobs() -> String { "auto".to_string() }
fn default_prefix() -> String { "/usr".to_string() }
fn default_true() -> bool { true }
fn default_ccache_dir() -> String { "/var/cache/forge/ccache".to_string() }
fn default_cargo_cache_dir() -> String { "/var/cache/forge/cargo".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseConfig {
    #[serde(default = "default_use_flags")]
    pub flags: String,
}

impl Default for UseConfig {
    fn default() -> Self {
        Self {
            flags: default_use_flags(),
        }
    }
}

fn default_use_flags() -> String { "ssl openrc alsa -systemd lto pgo".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    #[serde(default = "default_true")]
    pub enable_openrc_hooks: bool,
    #[serde(default = "default_true")]
    pub enable_ldconfig_hooks: bool,
    #[serde(default = "default_true")]
    pub enable_mandoc_hooks: bool,
    #[serde(default = "default_true")]
    pub auto_prompt_services: bool,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            enable_openrc_hooks: true,
            enable_ldconfig_hooks: true,
            enable_mandoc_hooks: true,
            auto_prompt_services: true,
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
            Some(Path::new("/usr/share/forge/forge.conf.default")),
        ];

        for cand in candidates.into_iter().flatten() {
            if let Ok(content) = fs::read_to_string(cand) {
                match toml::from_str::<ForgeConfig>(&content) {
                    Ok(conf) => return conf,
                    Err(e) => {
                        eprintln!("[!] Peringatan: Gagal mem-parse konfigurasi di {:?}: {}", cand, e);
                    }
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

    #[test]
    fn test_partial_config_layered_fallback_with_defaults() {
        let partial_toml = r#"
[general]
root = "/tmp/custom_root"

[use]
flags = "ssl -systemd custom_flag"
"#;
        let config: ForgeConfig = toml::from_str(partial_toml).expect("Gagal mem-parse partial TOML");
        assert_eq!(config.general.root, "/tmp/custom_root");
        assert_eq!(config.general.db_path, "/var/db/forge");
        assert_eq!(config.use_flags.flags, "ssl -systemd custom_flag");
        assert_eq!(config.build.prefix, "/usr");
        assert_eq!(config.cpu.target_march, "native");
        assert!(config.hooks.enable_openrc_hooks);
    }
}

