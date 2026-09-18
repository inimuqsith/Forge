use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use crate::cpu::CpuProfile;

/// Tingkatan Mikroarsitektur Repositori CachyOS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum CachyOsTier {
    Znver4,
    X86_64_V4,
    X86_64_V3,
    Generic,
}

/// Adapter Repositori CachyOS dengan Proteksi Anti-Brick Core OS
#[derive(Debug, Clone)]
pub struct CachyOsAdapter {
    pub tier: CachyOsTier,
    pub repo_name: String,
    pub base_url: String,
    pub protected_blacklist: HashSet<String>,
}

impl CachyOsAdapter {
    /// Inisialisasi adapter CachyOS dengan auto-detection mikroarsitektur CPU
    /// dan pemuatan blacklist perlindungan anti-brick sistem inti.
    pub fn auto_detect(cpu: &CpuProfile) -> Self {
        let (tier, repo_name) = match cpu.target_march.as_str() {
            "znver4" => (CachyOsTier::Znver4, "cachyos_znver4"),
            "x86-64-v4" | "alderlake" | "sapphirerapids" => (CachyOsTier::X86_64_V4, "cachyos_v4"),
            _ if cpu.isa_extensions.iter().any(|f| f.starts_with("avx512")) => {
                (CachyOsTier::X86_64_V4, "cachyos_v4")
            }
            "x86-64-v3" | "znver3" => (CachyOsTier::X86_64_V3, "cachyos_v3"),
            _ if cpu.isa_extensions.iter().any(|f| f == "avx2") => {
                (CachyOsTier::X86_64_V3, "cachyos_v3")
            }
            _ => (CachyOsTier::Generic, "cachyos"),
        };

        let base_url = match tier {
            CachyOsTier::Znver4 => format!("https://mirror.cachyos.org/repo/x86_64_v4/{}", repo_name),
            CachyOsTier::X86_64_V4 => format!("https://mirror.cachyos.org/repo/x86_64_v4/{}", repo_name),
            CachyOsTier::X86_64_V3 => format!("https://mirror.cachyos.org/repo/x86_64_v3/{}", repo_name),
            CachyOsTier::Generic => format!("https://mirror.cachyos.org/repo/x86_64/{}", repo_name),
        };

        // DAFTAR PAKET YANG HARAM DIAMBIL DARI CACHYOS DEMI MENCEGAH BRICK
        let mut blacklist = HashSet::new();
        for pkg in [
            "glibc", "linux-headers", "gcc", "gcc-libs", "llvm", "clang",
            "mold", "binutils", "openrc", "eudev", "kmod", "shadow",
            "util-linux", "base", "base-devel", "forge", "sysvinit", "systemd"
        ] {
            blacklist.insert(pkg.to_string());
        }

        Self {
            tier,
            repo_name: repo_name.to_string(),
            base_url,
            protected_blacklist: blacklist,
        }
    }

    /// Periksa apakah paket diizinkan diambil dari prebuilt CachyOS (Anti-Brick check)
    pub fn is_allowed_package(&self, pkgname: &str) -> bool {
        !self.protected_blacklist.contains(pkgname)
    }

    /// Dapatkan URL download paket biner CachyOS dengan versi spesifik
    pub fn get_download_url(&self, pkgname: &str, pkgver: &str, pkgrel: &str) -> String {
        format!("{}/{}-{}-{}-x86_64.pkg.tar.zst", self.base_url, pkgname, pkgver, pkgrel)
    }

    /// Dapatkan URL umum paket biner CachyOS
    pub fn get_package_url(&self, pkgname: &str) -> String {
        format!("{}/{}.pkg.tar.zst", self.base_url, pkgname)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cachyos_tier_auto_detection() {
        // 1. AMD Zen 4 -> Znver4
        let zen4_cpu = CpuProfile::mock("znver4", &["avx512f", "avx512vl", "avx2"]);
        let adapter_zen4 = CachyOsAdapter::auto_detect(&zen4_cpu);
        assert_eq!(adapter_zen4.tier, CachyOsTier::Znver4);
        assert_eq!(adapter_zen4.repo_name, "cachyos_znver4");
        assert!(adapter_zen4.base_url.contains("cachyos_znver4"));

        // 2. Intel / AVX-512 -> X86_64_V4
        let v4_cpu = CpuProfile::mock("alderlake", &["avx512f", "avx2"]);
        let adapter_v4 = CachyOsAdapter::auto_detect(&v4_cpu);
        assert_eq!(adapter_v4.tier, CachyOsTier::X86_64_V4);
        assert_eq!(adapter_v4.repo_name, "cachyos_v4");
        assert!(adapter_v4.base_url.contains("cachyos_v4"));

        // 3. AVX2 -> X86_64_V3
        let v3_cpu = CpuProfile::mock("x86-64-v3", &["avx2", "sse4_2"]);
        let adapter_v3 = CachyOsAdapter::auto_detect(&v3_cpu);
        assert_eq!(adapter_v3.tier, CachyOsTier::X86_64_V3);
        assert_eq!(adapter_v3.repo_name, "cachyos_v3");
        assert!(adapter_v3.base_url.contains("cachyos_v3"));

        // 4. Generic -> Generic
        let generic_cpu = CpuProfile::mock("x86-64", &["sse2"]);
        let adapter_generic = CachyOsAdapter::auto_detect(&generic_cpu);
        assert_eq!(adapter_generic.tier, CachyOsTier::Generic);
        assert_eq!(adapter_generic.repo_name, "cachyos");
        assert!(adapter_generic.base_url.contains("repo/x86_64/cachyos"));
    }

    #[test]
    fn test_core_os_blacklist_protection() {
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let adapter = CachyOsAdapter::auto_detect(&cpu);

        // Core OS and toolchain packages MUST be blacklisted
        let blacklisted = [
            "glibc", "linux-headers", "gcc", "gcc-libs", "llvm", "clang",
            "mold", "binutils", "openrc", "eudev", "kmod", "shadow",
            "util-linux", "base", "base-devel", "forge", "sysvinit", "systemd",
        ];

        for pkg in blacklisted {
            assert!(
                !adapter.is_allowed_package(pkg),
                "Paket inti '{}' HARUS dilarang (blacklisted) dari CachyOS!",
                pkg
            );
        }

        // Standard user packages/libraries MUST be allowed
        let allowed = ["ffmpeg", "ripgrep", "htop", "neovim", "nginx", "zstd", "curl"];
        for pkg in allowed {
            assert!(
                adapter.is_allowed_package(pkg),
                "Paket aplikasi '{}' harus diizinkan dari CachyOS!",
                pkg
            );
        }
    }
}
