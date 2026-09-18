use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{ForgeConfig, Recipe};

pub struct RecipeBuilder;

impl RecipeBuilder {
    /// Baca dan parse file recipe.toml
    pub fn load_recipe(path: &Path) -> Result<Recipe> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Gagal membaca berkas resep di {:?}", path))?;
        let recipe = toml::from_str::<Recipe>(&content)
            .with_context(|| format!("Gagal mem-parsing sintaks TOML pada {:?}", path))?;
        Ok(recipe)
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
        fs::create_dir_all(&build_root)?;
        fs::create_dir_all(destdir)?;

        // 2. Unduh dan verifikasi sumber jika ada entri sources
        if let Some(ref sources) = recipe.sources {
            for (i, url) in sources.urls.iter().enumerate() {
                let filename = url.split('/').last().unwrap_or("source.tar.gz");
                let target_file = distfiles_dir.join(filename);

                if !target_file.exists() {
                    println!("  [↓] Mengunduh sumber: {}", url);
                    let curl_status = Command::new("curl")
                        .arg("-sSL")
                        .arg("-o")
                        .arg(&target_file)
                        .arg(url)
                        .status();

                    if curl_status.is_err() || !curl_status.unwrap().success() {
                        // Fallback jika tidak ada akses internet saat test, buat dummy file jika custom_src_dir tidak ada
                        if custom_src_dir.is_none() {
                            anyhow::bail!("Gagal mengunduh sumber dari {}", url);
                        }
                    }
                }

                // Verifikasi SHA256 jika checksum tersedia
                if let Some(expected_sha) = sources.sha256.get(i) {
                    if target_file.exists() {
                        println!("  [✓] Memvalidasi hash SHA256 untuk {}...", filename);
                        let sha_out = Command::new("sha256sum").arg(&target_file).output();
                        if let Ok(out) = sha_out {
                            let calculated = String::from_utf8_lossy(&out.stdout)
                                .split_whitespace()
                                .next()
                                .unwrap_or_default()
                                .to_string();
                            if !expected_sha.is_empty() && calculated != *expected_sha {
                                anyhow::bail!(
                                    "Mismatch checksum SHA256! Expected: {}, Found: {}",
                                    expected_sha,
                                    calculated
                                );
                            }
                        }
                    }
                }

                // Ekstrak ke build directory
                if target_file.exists() {
                    println!("  [📦] Mengekstrak sumber ke {:?}", build_root);
                    let _ = Command::new("tar")
                        .arg("-xf")
                        .arg(&target_file)
                        .arg("-C")
                        .arg(&build_root)
                        .status();
                }
            }
        }

        // 3. Terapkan HIERARKI KONFIGURASI COMPILER & FORGE
        let build_meta = recipe.build.clone().unwrap_or_default();
        let is_glibc_or_exempt = pkg_name == "glibc"
            || build_meta.compiler_override.as_deref() == Some("gcc")
            || build_meta.disable_custom_march;

        let ccache_available = config.build.enable_ccache
            && Command::new("which")
                .arg("ccache")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

        let ccache_dir = {
            let p = PathBuf::from(&config.build.ccache_dir);
            if fs::create_dir_all(&p).is_ok() && fs::File::create(p.join(".write_test")).is_ok() {
                let _ = fs::remove_file(p.join(".write_test"));
                p
            } else {
                let fallback = distfiles_dir.join(".ccache");
                fs::create_dir_all(&fallback).ok();
                fallback
            }
        };

        let (cc_base, cxx_base, ld, cflags, cxxflags, ldflags) = if is_glibc_or_exempt {
            println!("  [!] Menerapkan aturan pengecualian Forge (ADR-002): Compiler GCC standar tanpa CFLAGS custom");
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
            let script_runner = format!(
                r#"
set -e
export CC="{cc}"
export CXX="{cxx}"
export LD="{ld}"
export CCACHE_DIR="{ccache_dir}"
export CFLAGS="{cflags}"
export CXXFLAGS="{cxxflags}"
export LDFLAGS="{ldflags}"
export MAKEFLAGS="{makeflags}"
export DESTDIR="{destdir}"
export PREFIX="{prefix}"
export srcdir="{srcdir}"
export pkgname="{pkgname}"
export pkgver="{pkgver}"

{script}
"#,
                cc = cc,
                cxx = cxx,
                ld = ld,
                ccache_dir = ccache_dir.display(),
                cflags = cflags,
                cxxflags = cxxflags,
                ldflags = ldflags,
                makeflags = config.build.makeflags,
                destdir = destdir.display(),
                prefix = config.build.prefix,
                srcdir = build_root.display(),
                pkgname = pkg_name,
                pkgver = pkg_ver,
                script = build_meta.script
            );

            let bash_status = Command::new("bash")
                .arg("-c")
                .arg(&script_runner)
                .status()
                .context("Gagal mengeksekusi bash script kompilasi")?;

            if !bash_status.success() {
                anyhow::bail!("Proses kompilasi resep {} gagal!", pkg_name);
            }
        }

        println!("  [✓] Kompilasi & staging {} berhasil di {:?}", pkg_name, destdir);
        Ok(destdir.to_path_buf())
    }
}
