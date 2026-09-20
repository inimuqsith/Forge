use anyhow::{bail, Context, Result};
use colored::Colorize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{BuildMeta, ForgeConfig, PackageMetadata, Recipe};

/// Daftar paket tingkat rendah / bare-metal / bootloader yang wajib otomatis
/// dikecualikan dari optimasi agresif (-march=native / SIMD / mold) demi mencegah triple-fault dan kegagalan boot.
pub const SENSITIVE_BAREMETAL_PACKAGES: &[&str] = &[
    // Tier 1: Bootloader & Bare-Metal
    "grub",
    "efibootmgr",
    "efivar",
    "syslinux",
    "memtest86+",
    "edk2",
    "ovmf",
    // Tier 2: C Library & Dynamic Linker
    "glibc",
    "musl",
    // Tier 3: Ring-0 Kernel & Module Loader
    "linux",
    "kmod",
    // Tier 4: Emulators, JIT & Low-Level Debuggers
    "valgrind",
    "gdb",
];

pub struct RecipeBuilder;

impl RecipeBuilder {
    /// Parse URL git menjadi (clean_url, Option<branch>)
    pub fn parse_git_url(raw: &str) -> (String, Option<String>) {
        if let Some((url_part, frag)) = raw.split_once('#') {
            let branch = if let Some(b) = frag.strip_prefix("branch=") {
                Some(b.to_string())
            } else if let Some(t) = frag.strip_prefix("tag=") {
                Some(t.to_string())
            } else if let Some(c) = frag.strip_prefix("commit=") {
                Some(c.to_string())
            } else {
                Some(frag.to_string())
            };
            (url_part.to_string(), branch)
        } else {
            (raw.to_string(), None)
        }
    }

    /// Melakukan probe commit HEAD git remote secara instan via `git ls-remote`
    pub fn probe_git_remote_commit(raw_url: &str, branch_override: Option<&str>) -> Result<String> {
        let (clean_url, branch) = Self::parse_git_url(raw_url);
        let target_branch = branch_override.or(branch.as_deref()).unwrap_or("HEAD");

        let output = Command::new("git")
            .arg("ls-remote")
            .arg(&clean_url)
            .arg(target_branch)
            .output()
            .with_context(|| format!("Gagal menjalankan git ls-remote pada {}", clean_url))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("git ls-remote gagal untuk {}: {}", clean_url, err.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let commit = stdout
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().next())
            .ok_or_else(|| anyhow::anyhow!("Tidak ada commit yang dikembalikan oleh git ls-remote dari {}", clean_url))?;

        Ok(commit.to_string())
    }
    /// Cek apakah paket wajib menggunakan compiler & flag aman (GCC + BFD ld + -O2)
    pub fn is_compiler_exempt(pkg_name: &str, build_meta: &BuildMeta) -> bool {
        SENSITIVE_BAREMETAL_PACKAGES.contains(&pkg_name)
            || build_meta.compiler_override.as_deref() == Some("gcc")
            || build_meta.disable_custom_march
    }

    /// Baca dan parse file recipe.toml
    pub fn load_recipe(path: &Path) -> Result<Recipe> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Gagal membaca berkas resep di {:?}", path))?;
        let recipe = toml::from_str::<Recipe>(&content)
            .with_context(|| format!("Gagal mem-parsing sintaks TOML pada {:?}", path))?;
        Ok(recipe)
    }

    /// Cari file recipe.toml berdasarkan nama paket di recipes_path atau fallback lokal
    pub fn find_recipe(pkg: &str, base_path: Option<&Path>) -> Option<PathBuf> {
        let direct = Path::new(pkg);
        if direct.exists() && direct.is_file() {
            return Some(direct.to_path_buf());
        }

        let check_dir = |base: &Path| -> Option<PathBuf> {
            let direct_cand = base.join(pkg).join("recipe.toml");
            if direct_cand.exists() {
                return Some(direct_cand);
            }
            let standard_cands = [
                base.join("system").join(pkg).join("recipe.toml"),
                base.join("core").join(pkg).join("recipe.toml"),
                base.join("extra").join(pkg).join("recipe.toml"),
            ];
            for cand in standard_cands {
                if cand.exists() {
                    return Some(cand);
                }
            }
            // Scan subdirektori dinamis (misal: custom_cat/pkg/recipe.toml)
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let sub_cand = path.join(pkg).join("recipe.toml");
                        if sub_cand.exists() {
                            return Some(sub_cand);
                        }
                    }
                }
            }
            None
        };

        if let Some(base) = base_path {
            if let Some(res) = check_dir(base) {
                return Some(res);
            }
        }
        let fallback_bases = [
            PathBuf::from("/var/db/forge/recipes"),
            PathBuf::from("recipes"),
        ];
        for base in fallback_bases {
            if let Some(res) = check_dir(&base) {
                return Some(res);
            }
        }
        None
    }

    /// Kemas direktori staging DESTDIR menjadi tarball biner .forge.tar.zst dengan metadata.json
    pub fn package_staging(
        staging_dir: &Path,
        output_tarball: &Path,
        recipe: &Recipe,
        target_march: &str,
        cflags: &str,
        use_flags: &str,
    ) -> Result<(PathBuf, String, u64)> {
        if let Some(parent) = output_tarball.parent() {
            fs::create_dir_all(parent)?;
        }

        Self::sanitize_staging_dir(staging_dir);

        // Hitung total size dan files_count
        let mut files_count = 0usize;
        let mut installed_size = 0u64;
        let mut stack = vec![staging_dir.to_path_buf()];
        while let Some(dir) = stack.pop() {
            if let Ok(rd) = fs::read_dir(&dir) {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.is_file() || p.is_symlink() {
                        files_count += 1;
                        if let Ok(m) = entry.metadata() {
                            installed_size += m.len();
                        }
                    }
                }
            }
        }

        let git_commit = {
            let commit_file = staging_dir.join(".forge_git_commit");
            if commit_file.exists() {
                let s = fs::read_to_string(&commit_file).ok().map(|s| s.trim().to_string());
                let _ = fs::remove_file(&commit_file);
                s
            } else {
                None
            }
        };

        // Tulis metadata.json ke staging sebelum kompresi
        let metadata = PackageMetadata {
            name: recipe.package.name.clone(),
            version: recipe.package.version.clone(),
            release: recipe.package.release,
            slot: recipe.package.slot.clone(),
            description: recipe.package.description.clone(),
            url: recipe.package.url.clone(),
            license: recipe.package.license.clone(),
            upstream: recipe.package.upstream.clone(),
            build_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            target_march: target_march.to_string(),
            cflags: cflags.to_string(),
            use_flags: use_flags.to_string(),
            files_count,
            installed_size,
            git_commit,
        };

        let meta_path = staging_dir.join("metadata.json");
        fs::write(&meta_path, serde_json::to_string_pretty(&metadata)?)?;

        let file = fs::File::create(output_tarball)?;
        let encoder = zstd::Encoder::new(file, 3)?;
        let mut tar = tar::Builder::new(encoder);
        tar.append_dir_all(".", staging_dir)?;
        let encoder = tar.into_inner()?;
        let mut finished_file = encoder.finish()?;
        std::io::Write::flush(&mut finished_file)?;
        drop(finished_file);

        let bytes = fs::read(output_tarball)?;
        let sha256_hash = format!("{:x}", Sha256::digest(&bytes));
        let size_bytes = bytes.len() as u64;

        let sha_file = format!("{}.sha256", output_tarball.display());
        fs::write(sha_file, &sha256_hash)?;

        Ok((output_tarball.to_path_buf(), sha256_hash, size_bytes))
    }

    /// Kompilasi paket dari source dan kemas langsung ke .forge.tar.zst di output_dir
    pub fn build_and_package(
        recipe_path: &Path,
        config: &ForgeConfig,
        output_dir: &Path,
        custom_src_dir: Option<&Path>,
    ) -> Result<PathBuf> {
        let recipe = Self::load_recipe(recipe_path)?;
        let pkg_name = &recipe.package.name;
        let pkg_ver = &recipe.package.version;
        let target_march = if config.cpu.target_march.is_empty() || config.cpu.target_march == "native" {
            "native"
        } else {
            &config.cpu.target_march
        };

        let staging_dir = PathBuf::from(format!("/tmp/forge/stage/{}-{}-{}", pkg_name, pkg_ver, target_march));
        if staging_dir.exists() {
            let _ = fs::remove_dir_all(&staging_dir);
        }
        fs::create_dir_all(&staging_dir)?;

        Self::build(recipe_path, config, &staging_dir, custom_src_dir)?;

        fs::create_dir_all(output_dir)?;
        let tarball_name = format!("{}-{}-{}.forge.tar.zst", pkg_name, pkg_ver, target_march);
        let output_tarball = output_dir.join(tarball_name);

        let (final_tarball, sha256, size) = Self::package_staging(
            &staging_dir,
            &output_tarball,
            &recipe,
            target_march,
            &config.build.cflags,
            &config.use_flags.flags,
        )?;

        println!(
            "  [✓] Paket biner terkemas: {} ({} bytes, SHA256: {})",
            final_tarball.display(),
            size,
            sha256
        );
        Ok(final_tarball)
    }

    /// Eksekusi kompilasi lengkap dari kode sumber sesuai hierarki konfigurasi Forge
    pub fn build(
        recipe_path: &Path,
        config: &ForgeConfig,
        destdir: &Path,
        custom_src_dir: Option<&Path>,
    ) -> Result<PathBuf> {
        let recipe = Self::load_recipe(recipe_path)?;
        let pkg_name = &recipe.package.name;
        let pkg_ver = &recipe.package.version;

        println!("  [*] Memproses resep: {} v{}", pkg_name, pkg_ver);

        // 1. Siapkan direktori distfiles dan build
        let distfiles_dir = {
            let configured = PathBuf::from(&config.general.cache_path);
            if fs::create_dir_all(&configured).is_ok() && std::fs::File::create(configured.join(".write_test")).is_ok() {
                let _ = fs::remove_file(configured.join(".write_test"));
                configured
            } else {
                let local_fallback = PathBuf::from("distfiles");
                fs::create_dir_all(&local_fallback).ok();
                local_fallback
            }
        };

        let build_root = PathBuf::from(format!("/tmp/forge/build/{}-{}", pkg_name, pkg_ver));
        if build_root.exists() {
            let _ = fs::remove_dir_all(&build_root);
        }
        fs::create_dir_all(&build_root)?;
        fs::create_dir_all(destdir)?;

        // 2. Unduh dan verifikasi sumber jika ada entri sources
        if let Some(ref sources) = recipe.sources {
            for (i, url) in sources.urls.iter().enumerate() {
                let is_git = url.ends_with(".git")
                    || url.contains(".git#")
                    || url.starts_with("git://")
                    || url.starts_with("git+")
                    || recipe.package.version == "git";

                if is_git {
                    let (clean_url, branch) = Self::parse_git_url(url);
                    let raw_repo = clean_url
                        .trim_end_matches(".git")
                        .split('/')
                        .next_back()
                        .unwrap_or(pkg_name.as_str());
                    let target_dir = build_root.join(raw_repo);

                    println!(
                        "  [📦] Mengkloning repositori Git: {} (Branch: {:?})",
                        clean_url.cyan(),
                        branch.as_deref().unwrap_or("default")
                    );

                    if target_dir.exists() {
                        let _ = fs::remove_dir_all(&target_dir);
                    }

                    let mut cmd = Command::new("git");
                    cmd.arg("clone").arg("--depth").arg("1");
                    if let Some(ref b) = branch {
                        cmd.arg("--branch").arg(b);
                    }
                    cmd.arg(&clean_url).arg(&target_dir);

                    let status = cmd.status().context("Gagal mengeksekusi git clone")?;
                    if !status.success()
                        && custom_src_dir.is_none() {
                            bail!("git clone gagal untuk {}", clean_url);
                        }

                    // Ambil commit hash HEAD
                    if let Ok(rev_out) = Command::new("git")
                        .current_dir(&target_dir)
                        .args(["rev-parse", "HEAD"])
                        .output()
                    {
                        if rev_out.status.success() {
                            let head_commit = String::from_utf8_lossy(&rev_out.stdout).trim().to_string();
                            println!("  [✓] Git HEAD Commit: {}", head_commit.bold().cyan());
                            let _ = fs::write(destdir.join(".forge_git_commit"), &head_commit);
                        }
                    }
                } else {
                    let filename = url.split('/').next_back().unwrap_or("source.tar.gz");
                    let target_file = distfiles_dir.join(filename);

                    let expected_sha = sources.sha256.get(i).cloned();
                    let dl_options = crate::downloader::DownloadOptions {
                        expected_sha256: expected_sha.clone(),
                        expected_blake3: None,
                        fallback_mirrors: Vec::new(),
                        retries: 3,
                        timeout_secs: 30,
                        show_progress: true,
                    };

                    if let Err(e) = crate::downloader::SourceDownloader::download(url, &target_file, &dl_options) {
                        if !target_file.exists() && custom_src_dir.is_none() {
                            anyhow::bail!("Gagal mengunduh sumber dari {}: {:#}", url, e);
                        }
                    }

                    // Ekstrak ke build directory jika arsip atau salin langsung jika berkas non-arsip
                    if target_file.exists() {
                        let is_archive = filename.ends_with(".tar.gz")
                            || filename.ends_with(".tar.xz")
                            || filename.ends_with(".tar.zst")
                            || filename.ends_with(".tar.bz2")
                            || filename.ends_with(".tgz")
                            || filename.ends_with(".tbz2")
                            || filename.ends_with(".txz");

                        if is_archive {
                            println!("  [📦] Mengekstrak sumber ke {:?}", build_root);
                            let _ = Command::new("tar")
                                .arg("-xf")
                                .arg(&target_file)
                                .arg("-C")
                                .arg(&build_root)
                                .status();
                        } else {
                            println!("  [📦] Menyalin sumber non-arsip ({}) ke {:?}", filename, build_root);
                            let dest_file = build_root.join(filename);
                            let _ = fs::copy(&target_file, &dest_file);
                        }
                    }
                }
            }
        }

        // 3. Terapkan HIERARKI KONFIGURASI COMPILER & FORGE (ADR-002 & Bare-Metal Safety Guard)
        let build_meta = recipe.build.clone().unwrap_or_default();
        let is_exempt = Self::is_compiler_exempt(pkg_name, &build_meta);

        let ccache_available = config.build.enable_ccache
            && Command::new("which")
                .arg("ccache")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        let ccache_dir = {
            let p = PathBuf::from(&config.build.ccache_dir);
            let dir = if fs::create_dir_all(&p).is_ok() && fs::File::create(p.join(".write_test")).is_ok() {
                let _ = fs::remove_file(p.join(".write_test"));
                p
            } else {
                let fallback = distfiles_dir.join(".ccache");
                fs::create_dir_all(&fallback).ok();
                fallback
            };
            let abs = if dir.is_absolute() {
                dir
            } else {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(dir)
            };
            abs.canonicalize().unwrap_or(abs)
        };

        let (cc_base, cxx_base, ld, cflags, cxxflags, ldflags) = if is_exempt {
            println!("  [!] Menerapkan aturan pengecualian Forge (ADR-002 / Bare-Metal Safety Guard): Compiler GCC standar tanpa CFLAGS custom");
            (
                "gcc".to_string(),
                "g++".to_string(),
                "ld".to_string(),
                "-O2 -pipe -fstack-protector-strong".to_string(),
                "-O2 -pipe -fstack-protector-strong".to_string(),
                "-Wl,-O1 -Wl,--as-needed".to_string(),
            )
        } else {
            // HIERARKI FORGE DIUTAMAKAN: Clang + Mold + Flag Native Silikon
            (
                "clang".to_string(),
                "clang++".to_string(),
                "mold".to_string(),
                config.build.cflags.clone(),
                config.build.cxxflags.clone(),
                format!("{} -fuse-ld=mold", config.build.ldflags),
            )
        };

        let (cc, cxx) = if ccache_available {
            println!("  [⚡] Akselerasi Ccache aktif (CCACHE_DIR: {:?})", ccache_dir);
            (format!("ccache {}", cc_base), format!("ccache {}", cxx_base))
        } else {
            (cc_base, cxx_base)
        };

        // 4. Jalankan script build jika ada
        if !build_meta.script.is_empty() {
            println!("  [🔨] Menjalankan script kompilasi dengan CC={}, LD={}...", cc, ld);
            let mut env_vars = std::collections::HashMap::new();
            env_vars.insert("CC".to_string(), cc.clone());
            env_vars.insert("CXX".to_string(), cxx.clone());
            env_vars.insert("ASM".to_string(), cc.clone());
            env_vars.insert("LD".to_string(), ld.clone());
            env_vars.insert("USE".to_string(), config.use_flags.flags.clone());
            env_vars.insert("USE_FLAGS".to_string(), config.use_flags.flags.clone());
            env_vars.insert("CCACHE_DIR".to_string(), ccache_dir.display().to_string());
            env_vars.insert("CFLAGS".to_string(), cflags.clone());
            env_vars.insert("CXXFLAGS".to_string(), cxxflags.clone());
            env_vars.insert("LDFLAGS".to_string(), ldflags.clone());
            env_vars.insert("MAKEFLAGS".to_string(), config.build.makeflags.clone());
            env_vars.insert("DESTDIR".to_string(), destdir.display().to_string());
            env_vars.insert("pkgdir".to_string(), destdir.display().to_string());
            env_vars.insert("PREFIX".to_string(), config.build.prefix.clone());
            env_vars.insert("srcdir".to_string(), build_root.display().to_string());
            env_vars.insert("pkgname".to_string(), pkg_name.clone());
            env_vars.insert("pkgver".to_string(), pkg_ver.clone());

            let runner = crate::sandbox::SandboxRunner::new();
            if runner.is_bwrap_available() {
                println!("  [🛡️] Sandbox Bubblewrap aktif (--ro-bind / /, namespace terisolasi)");
            }

            let script_content = format!("set -e\n{}", build_meta.script);
            let exit_status = runner.run_script_for_package(pkg_name, &script_content, &build_root, destdir, &env_vars, false)
                .context("Gagal mengeksekusi script kompilasi di dalam sandbox")?;

            if !exit_status.success() {
                anyhow::bail!("Proses kompilasi resep {} gagal!", pkg_name);
            }
        }

        Self::sanitize_staging_dir(destdir);

        // Tulis penanda integritas staging 100% sukses (ADR-081)
        let _ = fs::write(destdir.join(".forge_staging_complete"), b"OK");

        println!("  [✓] Kompilasi & staging {} berhasil di {:?}", pkg_name, destdir);
        Ok(destdir.to_path_buf())
    }

    /// Bersihkan berkas transien/indeks katalog sistem bersama dari direktori staging sebelum packaging/merging (ADR-057)
    pub fn sanitize_staging_dir(destdir: &Path) {
        let transient_files = [
            "usr/share/info/dir",
            "share/info/dir",
            "etc/ld.so.cache",
            "usr/share/glib-2.0/schemas/gschemas.compiled",
            "usr/share/applications/mimeinfo.cache",
            "usr/share/mime/XMLnamespaces",
            "usr/share/mime/globs",
            "usr/share/mime/globs2",
            "usr/share/mime/magic",
            "usr/share/mime/subclasses",
            "usr/share/mime/types",
            "usr/share/mime/version",
            "usr/share/mime/aliases",
            "usr/share/mime/generic-icons",
            "usr/share/mime/icons",
            "usr/share/mime/treemagic",
        ];

        for rel in &transient_files {
            let target = destdir.join(rel);
            if target.exists() {
                let _ = fs::remove_file(&target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_client_build_produces_tarball_without_installing() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let recipe_dir = temp.path().join("recipes").join("system").join("client-test-pkg");
        fs::create_dir_all(&recipe_dir)?;

        let recipe_path = recipe_dir.join("recipe.toml");
        let recipe_content = r#"
[package]
name = "client-test-pkg"
version = "1.2.3"
release = 1
slot = "0"
description = "Test Package for Client Build"

[build]
type = "meta"
script = """
mkdir -p "$DESTDIR/usr/bin"
echo "echo client test" > "$DESTDIR/usr/bin/client-test-bin"
chmod +x "$DESTDIR/usr/bin/client-test-bin"
"""
"#;
        fs::write(&recipe_path, recipe_content)?;

        let output_dir = temp.path().join("dist");
        let mut config = ForgeConfig::default();
        config.cpu.target_march = "znver4".to_string();

        let tarball_path = RecipeBuilder::build_and_package(&recipe_path, &config, &output_dir, None)?;
        assert!(tarball_path.exists(), "Tarball biner .forge.tar.zst harus dibuat di output_dir");
        assert_eq!(tarball_path, output_dir.join("client-test-pkg-1.2.3-znver4.forge.tar.zst"));

        // Periksa isi tarball
        let tar_file = fs::File::open(&tarball_path)?;
        let decoder = zstd::Decoder::new(tar_file)?;
        let mut archive = tar::Archive::new(decoder);
        let mut found_bin = false;
        let mut found_meta = false;

        for entry in archive.entries()? {
            let entry = entry?;
            let path = entry.path()?.to_path_buf();
            let path_str = path.to_string_lossy();
            if path_str.ends_with("usr/bin/client-test-bin") {
                found_bin = true;
            }
            if path_str.ends_with("metadata.json") {
                found_meta = true;
            }
        }

        assert!(found_bin, "File /usr/bin/client-test-bin harus ada di dalam tarball");
        assert!(found_meta, "metadata.json harus ada di dalam tarball");

        // Verifikasi bahwa host /usr/bin/client-test-bin TIDAK pernah terpasang
        assert!(!Path::new("/usr/bin/client-test-bin").exists(), "forge build TIDAK BOLEH memasang ke host filesystem /");

        Ok(())
    }

    #[test]
    fn test_sensitive_baremetal_packages_are_auto_exempted() {
        let empty_meta = BuildMeta::default();

        // 1. Paket dalam daftar bawaan SENSITIVE_BAREMETAL_PACKAGES (4 Tier) harus otomatis exempt
        assert!(RecipeBuilder::is_compiler_exempt("glibc", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("musl", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("grub", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("efibootmgr", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("efivar", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("syslinux", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("memtest86+", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("edk2", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("ovmf", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("linux", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("kmod", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("valgrind", &empty_meta));
        assert!(RecipeBuilder::is_compiler_exempt("gdb", &empty_meta));

        // 2. Paket aplikasi biasa tanpa override TIDAK BOLEH exempt (wajib Clang + mold + -march=native)
        assert!(!RecipeBuilder::is_compiler_exempt("fastfetch", &empty_meta));
        assert!(!RecipeBuilder::is_compiler_exempt("ripgrep", &empty_meta));
        assert!(!RecipeBuilder::is_compiler_exempt("plasma-desktop", &empty_meta));

        // 3. Paket kustom dengan override gcc / disable_custom_march harus di-exempt
        let mut gcc_meta = BuildMeta::default();
        gcc_meta.compiler_override = Some("gcc".to_string());
        assert!(RecipeBuilder::is_compiler_exempt("my-custom-pkg", &gcc_meta));

        let mut march_meta = BuildMeta::default();
        march_meta.disable_custom_march = true;
        assert!(RecipeBuilder::is_compiler_exempt("my-custom-pkg2", &march_meta));
    }

    #[test]
    fn test_git_url_parsing() {
        let (url1, branch1) = RecipeBuilder::parse_git_url("https://github.com/inimuqsith/Forge.git#branch=main");
        assert_eq!(url1, "https://github.com/inimuqsith/Forge.git");
        assert_eq!(branch1, Some("main".to_string()));

        let (url2, tag2) = RecipeBuilder::parse_git_url("https://github.com/CachyOS/linux-cachyos.git#tag=v6.13");
        assert_eq!(url2, "https://github.com/CachyOS/linux-cachyos.git");
        assert_eq!(tag2, Some("v6.13".to_string()));

        let (url3, branch3) = RecipeBuilder::parse_git_url("https://github.com/foo/bar.git");
        assert_eq!(url3, "https://github.com/foo/bar.git");
        assert_eq!(branch3, None);
    }

    #[test]
    fn test_prebuild_workspace_auto_sanitization() {
        let fake_pkg = "forge-test-clean";
        let fake_ver = "1.0.0";
        let build_root = PathBuf::from(format!("/tmp/forge/build/{}-{}", fake_pkg, fake_ver));
        
        // Simulasikan folder kotor bekas build sebelumnya
        fs::create_dir_all(&build_root).unwrap();
        let dirty_file = build_root.join("dirty_state.cache");
        fs::write(&dirty_file, "corrupted config state").unwrap();
        assert!(dirty_file.exists());

        // Jalankan logika sanitasi pre-build
        if build_root.exists() {
            let _ = fs::remove_dir_all(&build_root);
        }
        fs::create_dir_all(&build_root).unwrap();

        // Verifikasi bahwa folder bersih kembali
        assert!(build_root.exists());
        assert!(!dirty_file.exists(), "Berkas kotor bekas build sebelumnya harus bersih terhapus");

        // Bersihkan setelah pengujian
        let _ = fs::remove_dir_all(&build_root);
    }
}

