use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};

use crate::binhost::{BinhostCatalog, BinhostClient};
use crate::cachyos::CachyOsAdapter;
use crate::cpu::CpuProfile;
use crate::ForgeConfig;

/// Provider penyedia paket dalam hierarki resolusi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageProvider {
    ForgeSource,
    ForgeBinhost,
    CachyOsPrebuilt,
}

/// Hasil resolusi 3-Tier Cascade
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeResolution {
    pub provider: PackageProvider,
    pub package_name: String,
    pub download_url: Option<String>,
    pub target_march: String,
    pub reason: String,
}

/// 3-Tier Package Cascade Resolution Engine
pub struct PackageCascadeResolver;

impl PackageCascadeResolver {
    /// Selesaikan paket berdasarkan hierarki 3-Tier Cascade:
    /// - Mode 1 (--native / default): Source-First Native Compilation (Gentoo Mode).
    /// - Mode 2 (--binhost):
    ///   * Tingkat 1: Forge Native Binhost (.forge.tar.zst)
    ///   * Tingkat 2: Fallback CachyOS Prebuilt (dengan verifikasi Anti-Brick Blacklist)
    ///   * Tingkat 3: Ultimate Fallback ke Source Code Compilation
    pub fn resolve(
        pkg_name: &str,
        config: &ForgeConfig,
        cpu: &CpuProfile,
        force_native: bool,
        enable_binhost: bool,
        binhost_catalog: Option<&BinhostCatalog>,
        cachyos_available: Option<bool>,
    ) -> CascadeResolution {
        // Mode 1: Kompilasi Native dari Source Code (Default / --native)
        if force_native || (!enable_binhost && !config.binhost.enable_binhost) {
            return CascadeResolution {
                provider: PackageProvider::ForgeSource,
                package_name: pkg_name.to_string(),
                download_url: None,
                target_march: cpu.target_march.clone(),
                reason: "Source-First Native Compilation (Gentoo Mode)".to_string(),
            };
        }

        // Mode 2: 3-Tier Cascade Resolution (--binhost)
        // -------------------------------------------------------------
        // Tingkat 1: Periksa Forge Native Binhost (.forge.tar.zst)
        // -------------------------------------------------------------
        let binhost_client = BinhostClient::new(&config.server.binhost_url);
        if let Some(catalog) = binhost_catalog {
            if let Some(entry) = binhost_client.find_matching_package(catalog, pkg_name, &cpu.target_march) {
                return CascadeResolution {
                    provider: PackageProvider::ForgeBinhost,
                    package_name: pkg_name.to_string(),
                    download_url: Some(entry.download_url),
                    target_march: entry.target_march,
                    reason: format!("Found in Forge Native Binhost ({})", config.server.binhost_url),
                };
            }
        }

        // -------------------------------------------------------------
        // Tingkat 2: Fallback CachyOS Prebuilt (Proteksi Anti-Brick)
        // -------------------------------------------------------------
        let cachyos = CachyOsAdapter::auto_detect(cpu);
        if cachyos.is_allowed_package(pkg_name) {
            let is_available = cachyos_available.unwrap_or(true);
            if is_available {
                let download_url = cachyos.get_package_url(pkg_name);
                return CascadeResolution {
                    provider: PackageProvider::CachyOsPrebuilt,
                    package_name: pkg_name.to_string(),
                    download_url: Some(download_url),
                    target_march: cpu.target_march.clone(),
                    reason: format!("Fallback to CachyOS Prebuilt ({:?})", cachyos.tier),
                };
            }
        }

        // -------------------------------------------------------------
        // Tingkat 3: Ultimate Fallback ke Source Code Native Compilation
        // -------------------------------------------------------------
        let reason = if !cachyos.is_allowed_package(pkg_name) {
            format!(
                "Package '{}' is in Core OS blacklist (Anti-Brick protection enforced: bypass CachyOS -> Source)",
                pkg_name
            )
        } else {
            "Package not found in Forge or CachyOS binhosts -> Fallback to Source compilation".to_string()
        };

        CascadeResolution {
            provider: PackageProvider::ForgeSource,
            package_name: pkg_name.to_string(),
            download_url: None,
            target_march: cpu.target_march.clone(),
            reason,
        }
    }

    /// Selesaikan paket dan tampilkan log proses cascade secara interaktif/visual
    pub async fn resolve_and_install(
        pkg_name: &str,
        config: &ForgeConfig,
        force_native: bool,
        enable_binhost: bool,
    ) -> Result<CascadeResolution> {
        let cpu = CpuProfile::detect().unwrap_or_else(|_| CpuProfile::mock("native", &[]));

        if force_native || (!enable_binhost && !config.binhost.enable_binhost) {
            println!("{} Mode: Source-First Native Compilation (Gentoo Mode)", "[*]".blue());
            return Ok(CascadeResolution {
                provider: PackageProvider::ForgeSource,
                package_name: pkg_name.to_string(),
                download_url: None,
                target_march: cpu.target_march,
                reason: "Source-First Native Compilation (Gentoo Mode)".to_string(),
            });
        }

        println!(
            "{} Memulai 3-Tier Binhost Cascade Resolution untuk: {}",
            "[⚡]".cyan(),
            pkg_name.bold().green()
        );

        // 1. Tingkat 1: Forge Native Binhost
        println!(
            "  [1/3] Memeriksa Forge Native Binhost di {}...",
            config.server.binhost_url.cyan()
        );

        // 2. Tingkat 2: Fallback CachyOS (dengan Anti-Brick Blacklist)
        let cachyos = CachyOsAdapter::auto_detect(&cpu);
        if !cachyos.is_allowed_package(pkg_name) {
            println!(
                "  [2/3] {} Paket '{}' adalah Core OS (Anti-Brick Protection). Bypass CachyOS!",
                "[!]".yellow(),
                pkg_name.bold()
            );
        } else {
            println!(
                "  [2/3] Biner belum ada di Forge Server. Memeriksa CachyOS Prebuilt ({:?})...",
                cachyos.tier
            );
            if let Some((download_url, meta)) = cachyos.query_package(pkg_name).await {
                println!(
                    "  [✓] Paket biner ditemukan di CachyOS: {} v{} ({:?})",
                    meta.name.bold().green(),
                    meta.version.cyan(),
                    cachyos.tier
                );
                return Ok(CascadeResolution {
                    provider: PackageProvider::CachyOsPrebuilt,
                    package_name: pkg_name.to_string(),
                    download_url: Some(download_url),
                    target_march: cpu.target_march.clone(),
                    reason: format!("CachyOS Prebuilt ({:?}) v{}", cachyos.tier, meta.version),
                });
            }
        }

        // 3. Tingkat 3: Fallback Source Code
        println!(
            "  [3/3] Fallback otomatis: Mengompilasi dari kode sumber upstream secara native..."
        );

        let resolution = Self::resolve(
            pkg_name,
            config,
            &cpu,
            force_native,
            enable_binhost,
            None,
            Some(false),
        );

        Ok(resolution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binhost::BinhostPackageEntry;

    fn make_test_catalog() -> BinhostCatalog {
        BinhostCatalog {
            timestamp: 1726700000,
            server_version: "0.1.0".to_string(),
            packages: vec![
                BinhostPackageEntry {
                    pkgname: "ripgrep".to_string(),
                    pkgver: "14.1.0".to_string(),
                    pkgrel: 1,
                    slot: "0".to_string(),
                    target_march: "znver4".to_string(),
                    active_use: vec!["pcre2".to_string()],
                    sha256: "abcd1234".to_string(),
                    size_bytes: 2048576,
                    download_url: "https://binhost.kuralinux.org/v1/packages/ripgrep-14.1.0-1-znver4.forge.tar.zst".to_string(),
                },
                BinhostPackageEntry {
                    pkgname: "htop".to_string(),
                    pkgver: "3.3.0".to_string(),
                    pkgrel: 1,
                    slot: "0".to_string(),
                    target_march: "generic".to_string(),
                    active_use: vec![],
                    sha256: "efgh5678".to_string(),
                    size_bytes: 512000,
                    download_url: "https://binhost.kuralinux.org/v1/packages/htop-3.3.0-1-generic.forge.tar.zst".to_string(),
                },
            ],
        }
    }

    #[test]
    fn test_cascade_prefers_forge_binhost() {
        let config = ForgeConfig::default();
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let catalog = make_test_catalog();

        // 1. Package available in Forge Binhost -> ForgeBinhost chosen
        let res = PackageCascadeResolver::resolve(
            "ripgrep",
            &config,
            &cpu,
            false,
            true,
            Some(&catalog),
            Some(true),
        );

        assert_eq!(res.provider, PackageProvider::ForgeBinhost);
        assert_eq!(res.package_name, "ripgrep");
        assert!(res.download_url.unwrap().contains(".forge.tar.zst"));
        assert!(res.reason.contains("Forge Native Binhost"));
    }

    #[test]
    fn test_cascade_fallback_to_cachyos() {
        let config = ForgeConfig::default();
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let catalog = make_test_catalog();

        // 'neovim' is NOT in Forge catalog, but IS an allowed user app in CachyOS
        let res = PackageCascadeResolver::resolve(
            "neovim",
            &config,
            &cpu,
            false,
            true,
            Some(&catalog),
            Some(true),
        );

        assert_eq!(res.provider, PackageProvider::CachyOsPrebuilt);
        assert_eq!(res.package_name, "neovim");
        assert!(res.download_url.unwrap().contains("cachyos.org"));
        assert!(res.reason.contains("CachyOS Prebuilt"));
    }

    #[test]
    fn test_cascade_blocks_core_os_from_cachyos() {
        let config = ForgeConfig::default();
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let catalog = make_test_catalog();

        // Core OS packages MUST be blocked from CachyOS fallback and routed to Source
        let core_pkgs = ["glibc", "openrc", "gcc", "llvm", "base", "base-devel", "systemd"];

        for pkg in core_pkgs {
            let res = PackageCascadeResolver::resolve(
                pkg,
                &config,
                &cpu,
                false,
                true,
                Some(&catalog),
                Some(true), // Even if CachyOS claims to have it!
            );

            assert_eq!(
                res.provider,
                PackageProvider::ForgeSource,
                "Core OS package '{}' must be forced to ForgeSource!",
                pkg
            );
            assert!(res.reason.contains("Core OS blacklist"));
        }
    }

    #[test]
    fn test_cascade_fallback_to_source() {
        let config = ForgeConfig::default();
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let catalog = make_test_catalog();

        // 1. If not in Forge and not available in CachyOS -> Fallback to Source
        let res_missing = PackageCascadeResolver::resolve(
            "my-custom-app",
            &config,
            &cpu,
            false,
            true,
            Some(&catalog),
            Some(false),
        );
        assert_eq!(res_missing.provider, PackageProvider::ForgeSource);
        assert!(res_missing.reason.contains("Fallback to Source"));

        // 2. If force_native is true -> Always Source
        let res_native = PackageCascadeResolver::resolve(
            "ripgrep",
            &config,
            &cpu,
            true,
            true,
            Some(&catalog),
            Some(true),
        );
        assert_eq!(res_native.provider, PackageProvider::ForgeSource);
        assert!(res_native.reason.contains("Gentoo Mode"));
    }
}
