use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::io::Read;
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

/// Metadata Paket ALPM CachyOS dari database repo (.db.tar.zst / desc)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CachyOsPackageMeta {
    pub name: String,
    pub version: String,
    pub desc: String,
    pub depends: Vec<String>,
    pub provides: Vec<String>,
    pub filename: String,
    pub csize: u64,
    pub isize: u64,
    pub sha256sum: Option<String>,
}

/// Bersihkan nama dependensi dari versi (misal: "glibc>=2.38" -> "glibc")
pub fn clean_package_name(dep: &str) -> &str {
    dep.split(['>', '<', '=', ':'])
        .next()
        .unwrap_or(dep)
        .trim()
}

/// Parser format entri `desc` pada ALPM sync database
pub fn parse_alpm_desc(content: &str) -> Result<CachyOsPackageMeta> {
    let mut name = String::new();
    let mut version = String::new();
    let mut desc = String::new();
    let mut filename = String::new();
    let mut csize = 0u64;
    let mut isize = 0u64;
    let mut sha256sum = None;
    let mut depends = Vec::new();
    let mut provides = Vec::new();

    let mut current_section = "";

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('%') && trimmed.ends_with('%') && trimmed.len() >= 2 {
            current_section = &trimmed[1..trimmed.len() - 1];
            continue;
        }

        match current_section {
            "NAME" => name = trimmed.to_string(),
            "VERSION" => version = trimmed.to_string(),
            "DESC" => {
                if desc.is_empty() {
                    desc = trimmed.to_string();
                } else {
                    desc.push(' ');
                    desc.push_str(trimmed);
                }
            }
            "FILENAME" => filename = trimmed.to_string(),
            "CSIZE" => csize = trimmed.parse::<u64>().unwrap_or(0),
            "ISIZE" => isize = trimmed.parse::<u64>().unwrap_or(0),
            "SHA256SUM" => sha256sum = Some(trimmed.to_string()),
            "DEPENDS" => depends.push(trimmed.to_string()),
            "PROVIDES" => provides.push(trimmed.to_string()),
            _ => {}
        }
    }

    if name.is_empty() {
        anyhow::bail!("Format berkas ALPM desc tidak valid: %NAME% tidak ditemukan");
    }

    Ok(CachyOsPackageMeta {
        name,
        version,
        desc,
        depends,
        provides,
        filename,
        csize,
        isize,
        sha256sum,
    })
}

/// Parser arsip database repositori CachyOS (`.db.tar.zst`)
pub fn parse_repo_db_tar_zst(archive_bytes: &[u8]) -> Result<HashMap<String, CachyOsPackageMeta>> {
    let mut decoder = zstd::stream::read::Decoder::new(archive_bytes)
        .context("Gagal menginisialisasi dekompresor Zstd untuk database repositori")?;
    let mut archive = tar::Archive::new(&mut decoder);
    let mut result = HashMap::new();

    for entry in archive.entries().context("Gagal membaca entri tarball database")? {
        let mut entry = entry.context("Gagal membaca header entri tar")?;
        let path = entry.path().context("Gagal membaca path entri tar")?.to_path_buf();

        if path.file_name().and_then(|s| s.to_str()) == Some("desc") {
            let mut content = String::new();
            entry.read_to_string(&mut content)
                .context("Gagal membaca konten berkas desc dari database")?;
            if let Ok(meta) = parse_alpm_desc(&content) {
                result.insert(meta.name.clone(), meta);
            }
        }
    }

    Ok(result)
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
            "znver4" => (CachyOsTier::Znver4, "cachyos-znver4"),
            "x86-64-v4" | "alderlake" | "sapphirerapids" => (CachyOsTier::X86_64_V4, "cachyos-v4"),
            _ if cpu.isa_extensions.iter().any(|f| f.starts_with("avx512")) => {
                (CachyOsTier::X86_64_V4, "cachyos-v4")
            }
            "x86-64-v3" | "znver3" => (CachyOsTier::X86_64_V3, "cachyos-v3"),
            _ if cpu.isa_extensions.iter().any(|f| f == "avx2") => {
                (CachyOsTier::X86_64_V3, "cachyos-v3")
            }
            _ => (CachyOsTier::Generic, "cachyos"),
        };

        let base_url = match tier {
            CachyOsTier::Znver4 => format!("https://cdn77.cachyos.org/repo/x86_64_v4/{}", repo_name),
            CachyOsTier::X86_64_V4 => format!("https://cdn77.cachyos.org/repo/x86_64_v4/{}", repo_name),
            CachyOsTier::X86_64_V3 => format!("https://cdn77.cachyos.org/repo/x86_64_v3/{}", repo_name),
            CachyOsTier::Generic => format!("https://cdn77.cachyos.org/repo/x86_64/{}", repo_name),
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

    /// Daftar repositori aktif untuk tier arsitektur saat ini
    pub fn get_repo_urls(&self) -> Vec<(String, String)> {
        match self.tier {
            CachyOsTier::Znver4 => vec![
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-extra-v4".to_string(), "cachyos-extra-v4".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-znver4".to_string(), "cachyos-znver4".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-v4".to_string(), "cachyos-v4".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-core-v4".to_string(), "cachyos-core-v4".to_string()),
            ],
            CachyOsTier::X86_64_V4 => vec![
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-extra-v4".to_string(), "cachyos-extra-v4".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-v4".to_string(), "cachyos-v4".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v4/cachyos-core-v4".to_string(), "cachyos-core-v4".to_string()),
            ],
            CachyOsTier::X86_64_V3 => vec![
                ("https://cdn77.cachyos.org/repo/x86_64_v3/cachyos-extra-v3".to_string(), "cachyos-extra-v3".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v3/cachyos-v3".to_string(), "cachyos-v3".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64_v3/cachyos-core-v3".to_string(), "cachyos-core-v3".to_string()),
            ],
            CachyOsTier::Generic => vec![
                ("https://cdn77.cachyos.org/repo/x86_64/cachyos-extra".to_string(), "cachyos-extra".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64/cachyos".to_string(), "cachyos".to_string()),
                ("https://cdn77.cachyos.org/repo/x86_64/cachyos-core".to_string(), "cachyos-core".to_string()),
            ],
        }
    }

    /// Query repository databases secara asinkron untuk mencari URL biner dan metadata paket
    pub async fn query_package(&self, pkg_name: &str) -> Option<(String, CachyOsPackageMeta)> {
        let clean = clean_package_name(pkg_name);
        if !self.is_allowed_package(clean) {
            return None;
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .ok()?;

        for (base_repo_url, repo_name) in self.get_repo_urls() {
            let db_url = format!("{}/{}.db.tar.zst", base_repo_url, repo_name);
            if let Ok(resp) = client.get(&db_url).send().await {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        if let Ok(db) = parse_repo_db_tar_zst(&bytes) {
                            if let Some(meta) = db.get(clean) {
                                let download_url = format!("{}/{}", base_repo_url, meta.filename);
                                return Some((download_url, meta.clone()));
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Periksa apakah paket diizinkan diambil dari prebuilt CachyOS (Anti-Brick check)
    pub fn is_allowed_package(&self, pkgname: &str) -> bool {
        let clean = clean_package_name(pkgname);
        !self.protected_blacklist.contains(clean)
    }

    /// Dapatkan URL download paket biner CachyOS dengan versi spesifik
    pub fn get_download_url(&self, pkgname: &str, pkgver: &str, pkgrel: &str) -> String {
        format!("{}/{}-{}-{}-x86_64.pkg.tar.zst", self.base_url, pkgname, pkgver, pkgrel)
    }

    /// Dapatkan URL umum paket biner CachyOS
    pub fn get_package_url(&self, pkgname: &str) -> String {
        format!("{}/{}.pkg.tar.zst", self.base_url, pkgname)
    }

    /// Lakukan penelusuran graf dependensi rekursif (DFS Topological Sort)
    /// dengan menerapkan filter Anti-Brick perlindungan Core OS.
    pub fn resolve_dependencies_recursive(
        &self,
        root_pkgs: &[&str],
        db: &HashMap<String, CachyOsPackageMeta>,
    ) -> Result<Vec<String>> {
        let mut resolved_order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn dfs(
            adapter: &CachyOsAdapter,
            pkg_name: &str,
            db: &HashMap<String, CachyOsPackageMeta>,
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
            order: &mut Vec<String>,
        ) -> Result<()> {
            let clean_pkg = clean_package_name(pkg_name);
            if clean_pkg.is_empty() {
                return Ok(());
            }

            // 1. Anti-Brick check: jika dependensi dilarang, catat peringatan dan jangan masukkan ke CachyOS chain
            if !adapter.is_allowed_package(clean_pkg) {
                tracing::warn!(
                    "Anti-Brick: dependensi '{}' dilarang dari CachyOS, harus dipenuhi oleh resep native Kura Linux.",
                    clean_pkg
                );
                return Ok(());
            }

            if visited.contains(clean_pkg) {
                return Ok(());
            }

            if visiting.contains(clean_pkg) {
                // Siklus dependensi sirkular terdeteksi, lewati untuk mencegah loop tanpa akhir
                return Ok(());
            }

            visiting.insert(clean_pkg.to_string());

            // 2. Cari metadata paket di database dan telusuri dependensinya
            if let Some(meta) = db.get(clean_pkg) {
                for dep in &meta.depends {
                    dfs(adapter, dep, db, visited, visiting, order)?;
                }
            }

            visiting.remove(clean_pkg);
            visited.insert(clean_pkg.to_string());
            order.push(clean_pkg.to_string());

            Ok(())
        }

        for root in root_pkgs {
            let clean_root = clean_package_name(root);
            dfs(self, clean_root, db, &mut visited, &mut visiting, &mut resolved_order)?;
        }

        Ok(resolved_order)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_cachyos_tier_auto_detection() {
        // 1. AMD Zen 4 -> Znver4
        let zen4_cpu = CpuProfile::mock("znver4", &["avx512f", "avx512vl", "avx2"]);
        let adapter_zen4 = CachyOsAdapter::auto_detect(&zen4_cpu);
        assert_eq!(adapter_zen4.tier, CachyOsTier::Znver4);
        assert_eq!(adapter_zen4.repo_name, "cachyos-znver4");
        assert!(adapter_zen4.base_url.contains("cachyos-znver4"));

        // 2. Intel / AVX-512 -> X86_64_V4
        let v4_cpu = CpuProfile::mock("alderlake", &["avx512f", "avx2"]);
        let adapter_v4 = CachyOsAdapter::auto_detect(&v4_cpu);
        assert_eq!(adapter_v4.tier, CachyOsTier::X86_64_V4);
        assert_eq!(adapter_v4.repo_name, "cachyos-v4");
        assert!(adapter_v4.base_url.contains("cachyos-v4"));

        // 3. AVX2 -> X86_64_V3
        let v3_cpu = CpuProfile::mock("x86-64-v3", &["avx2", "sse4_2"]);
        let adapter_v3 = CachyOsAdapter::auto_detect(&v3_cpu);
        assert_eq!(adapter_v3.tier, CachyOsTier::X86_64_V3);
        assert_eq!(adapter_v3.repo_name, "cachyos-v3");
        assert!(adapter_v3.base_url.contains("cachyos-v3"));

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

    #[test]
    fn test_parse_cachyos_desc_format() {
        let desc_content = r#"
%FILENAME%
ffmpeg-2:7.1-2-x86_64_v4.pkg.tar.zst

%NAME%
ffmpeg

%BASE%
ffmpeg

%VERSION%
2:7.1-2

%DESC%
Complete solution to record, convert and stream audio and video

%CSIZE%
36700160

%ISIZE%
104857600

%SHA256SUM%
e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855

%DEPENDS%
alsa-lib
libvpx
x264
glibc>=2.38
openssl

%PROVIDES%
libavcodec.so=61-64
ffmpeg=7.1
"#;

        let meta = parse_alpm_desc(desc_content).expect("Gagal mem-parsing desc CachyOS");
        assert_eq!(meta.name, "ffmpeg");
        assert_eq!(meta.version, "2:7.1-2");
        assert_eq!(meta.filename, "ffmpeg-2:7.1-2-x86_64_v4.pkg.tar.zst");
        assert_eq!(meta.csize, 36700160);
        assert_eq!(meta.isize, 104857600);
        assert_eq!(
            meta.sha256sum.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert_eq!(
            meta.depends,
            vec!["alsa-lib", "libvpx", "x264", "glibc>=2.38", "openssl"]
        );
        assert_eq!(
            meta.provides,
            vec!["libavcodec.so=61-64", "ffmpeg=7.1"]
        );
    }

    #[test]
    fn test_cachyos_recursive_dependency_chain() {
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let adapter = CachyOsAdapter::auto_detect(&cpu);

        let mut db = HashMap::new();
        db.insert(
            "ffmpeg".to_string(),
            CachyOsPackageMeta {
                name: "ffmpeg".to_string(),
                version: "7.1".to_string(),
                depends: vec!["libvpx".to_string(), "x264".to_string()],
                ..Default::default()
            },
        );
        db.insert(
            "libvpx".to_string(),
            CachyOsPackageMeta {
                name: "libvpx".to_string(),
                version: "1.14".to_string(),
                depends: vec!["libogg".to_string()],
                ..Default::default()
            },
        );
        db.insert(
            "x264".to_string(),
            CachyOsPackageMeta {
                name: "x264".to_string(),
                version: "164".to_string(),
                depends: vec![],
                ..Default::default()
            },
        );
        db.insert(
            "libogg".to_string(),
            CachyOsPackageMeta {
                name: "libogg".to_string(),
                version: "1.3.5".to_string(),
                depends: vec![],
                ..Default::default()
            },
        );

        let order = adapter
            .resolve_dependencies_recursive(&["ffmpeg"], &db)
            .expect("Resolusi dependensi rekursif gagal");

        // Memastikan urutan topologis yang benar: dependensi lebih dulu sebelum paket induk
        let pos_ogg = order.iter().position(|p| p == "libogg").unwrap();
        let pos_vpx = order.iter().position(|p| p == "libvpx").unwrap();
        let pos_x264 = order.iter().position(|p| p == "x264").unwrap();
        let pos_ffmpeg = order.iter().position(|p| p == "ffmpeg").unwrap();

        assert!(pos_ogg < pos_vpx, "libogg harus diunduh sebelum libvpx");
        assert!(pos_vpx < pos_ffmpeg, "libvpx harus diunduh sebelum ffmpeg");
        assert!(pos_x264 < pos_ffmpeg, "x264 harus diunduh sebelum ffmpeg");
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn test_cachyos_recursive_respects_blacklist() {
        let cpu = CpuProfile::mock("znver4", &["avx512f", "avx2"]);
        let adapter = CachyOsAdapter::auto_detect(&cpu);

        let mut db = HashMap::new();
        db.insert(
            "ffmpeg".to_string(),
            CachyOsPackageMeta {
                name: "ffmpeg".to_string(),
                version: "7.1".to_string(),
                depends: vec![
                    "x264".to_string(),
                    "glibc>=2.38".to_string(),
                    "openrc".to_string(),
                    "systemd".to_string(),
                ],
                ..Default::default()
            },
        );
        db.insert(
            "x264".to_string(),
            CachyOsPackageMeta {
                name: "x264".to_string(),
                version: "164".to_string(),
                depends: vec!["gcc-libs".to_string()],
                ..Default::default()
            },
        );

        let order = adapter
            .resolve_dependencies_recursive(&["ffmpeg"], &db)
            .expect("Resolusi harus berhasil");

        // Paket-paket yang ada di blacklist tidak boleh masuk ke dalam chain CachyOS
        assert!(order.contains(&"x264".to_string()));
        assert!(order.contains(&"ffmpeg".to_string()));
        assert!(!order.contains(&"glibc".to_string()));
        assert!(!order.contains(&"openrc".to_string()));
        assert!(!order.contains(&"systemd".to_string()));
        assert!(!order.contains(&"gcc-libs".to_string()));
        assert_eq!(order, vec!["x264", "ffmpeg"]);
    }

    #[test]
    fn test_parse_repo_db_tar_zst() {
        // Buat mock tarball in-memory dengan kompresi zstd
        let mut tar_builder = tar::Builder::new(Vec::new());

        let desc_ffmpeg = b"%NAME%\nffmpeg\n%VERSION%\n7.1\n%FILENAME%\nffmpeg.pkg.tar.zst\n";
        let mut header_ffmpeg = tar::Header::new_gnu();
        header_ffmpeg.set_size(desc_ffmpeg.len() as u64);
        header_ffmpeg.set_mode(0o644);
        header_ffmpeg.set_cksum();
        tar_builder
            .append_data(&mut header_ffmpeg, "ffmpeg-7.1/desc", &desc_ffmpeg[..])
            .unwrap();

        let desc_x264 = b"%NAME%\nx264\n%VERSION%\n164\n%FILENAME%\nx264.pkg.tar.zst\n";
        let mut header_x264 = tar::Header::new_gnu();
        header_x264.set_size(desc_x264.len() as u64);
        header_x264.set_mode(0o644);
        header_x264.set_cksum();
        tar_builder
            .append_data(&mut header_x264, "x264-164/desc", &desc_x264[..])
            .unwrap();

        let tar_bytes = tar_builder.into_inner().unwrap();
        let zstd_bytes = zstd::encode_all(&tar_bytes[..], 3).unwrap();

        let db = parse_repo_db_tar_zst(&zstd_bytes).expect("Gagal mem-parsing mock db.tar.zst");
        assert_eq!(db.len(), 2);
        assert!(db.contains_key("ffmpeg"));
        assert!(db.contains_key("x264"));
        assert_eq!(db["ffmpeg"].version, "7.1");
        assert_eq!(db["x264"].version, "164");
    }
}
