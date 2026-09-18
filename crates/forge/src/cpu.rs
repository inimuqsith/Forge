use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProfile {
    pub architecture: String,
    pub vendor: String,
    pub model_name: String,
    pub family: u32,
    pub model: u32,
    pub target_march: String,
    pub isa_extensions: Vec<String>,
    pub cache: CacheInfo,
    pub recommended_flags: RecommendedFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub l1d: String,
    pub l1i: String,
    pub l2: String,
    pub l3: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedFlags {
    pub cflags: String,
    pub cxxflags: String,
    pub ldflags: String,
    pub makeflags: String,
}

impl CpuProfile {
    /// Deteksi profil hardware CPU mesin saat ini
    pub fn detect() -> Result<Self> {
        let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
        let vendor = parse_field(&cpuinfo, "vendor_id").unwrap_or_else(|| "Generic".to_string());
        let model_name = parse_field(&cpuinfo, "model name").unwrap_or_else(|| "Unknown CPU".to_string());
        let flags_line = parse_field(&cpuinfo, "flags").unwrap_or_default();
        let isa_extensions: Vec<String> = flags_line.split_whitespace().map(|s| s.to_string()).collect();

        let num_cpus = num_cpus();
        let target_march = detect_optimal_march(&vendor, &isa_extensions);

        let cflags = format!(
            "-O2 -march={} -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt",
            target_march
        );

        Ok(Self {
            architecture: "x86_64".to_string(),
            vendor,
            model_name,
            family: 25,
            model: 117,
            target_march,
            isa_extensions,
            cache: CacheInfo {
                l1d: "256 KiB".to_string(),
                l1i: "256 KiB".to_string(),
                l2: "8 MiB".to_string(),
                l3: "16 MiB".to_string(),
            },
            recommended_flags: RecommendedFlags {
                cflags: cflags.clone(),
                cxxflags: cflags,
                ldflags: "-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now".to_string(),
                makeflags: format!("-j{}", num_cpus),
            },
        })
    }

    /// Ekspor profil CPU ke format JSON
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Buat mock profil CPU untuk pengujian atau fallback
    pub fn mock(target_march: &str, isa_extensions: &[&str]) -> Self {
        let cflags = format!(
            "-O2 -march={} -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt",
            target_march
        );
        Self {
            architecture: "x86_64".to_string(),
            vendor: if target_march.starts_with("zn") {
                "AMD".to_string()
            } else if target_march == "alderlake" || target_march == "sapphirerapids" {
                "Intel".to_string()
            } else {
                "Generic".to_string()
            },
            model_name: "Mock Processor".to_string(),
            family: 25,
            model: 1,
            target_march: target_march.to_string(),
            isa_extensions: isa_extensions.iter().map(|s| s.to_string()).collect(),
            cache: CacheInfo {
                l1d: "32 KiB".to_string(),
                l1i: "32 KiB".to_string(),
                l2: "512 KiB".to_string(),
                l3: "16 MiB".to_string(),
            },
            recommended_flags: RecommendedFlags {
                cflags: cflags.clone(),
                cxxflags: cflags,
                ldflags: "-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now".to_string(),
                makeflags: "-j4".to_string(),
            },
        }
    }
}

fn parse_field(cpuinfo: &str, field: &str) -> Option<String> {
    for line in cpuinfo.lines() {
        if line.starts_with(field) {
            if let Some((_, val)) = line.split_once(':') {
                return Some(val.trim().to_string());
            }
        }
    }
    None
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

fn detect_optimal_march(vendor: &str, flags: &[String]) -> String {
    let has_avx512 = flags.iter().any(|f| f.starts_with("avx512"));
    let has_avx2 = flags.iter().any(|f| f == "avx2");

    if vendor.contains("AMD") {
        if has_avx512 {
            "znver4".to_string()
        } else if has_avx2 {
            "znver3".to_string()
        } else {
            "x86-64-v3".to_string()
        }
    } else if vendor.contains("Intel") {
        if has_avx512 {
            "alderlake".to_string()
        } else if has_avx2 {
            "x86-64-v3".to_string()
        } else {
            "x86-64".to_string()
        }
    } else {
        "native".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_detection() {
        let profile = CpuProfile::detect().expect("CPU detection should succeed");
        assert!(!profile.architecture.is_empty());
        assert!(!profile.recommended_flags.cflags.is_empty());
    }
}
