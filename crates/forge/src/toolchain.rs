use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainComponent {
    pub name: String,
    pub path: Option<String>,
    pub version: Option<String>,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainStatus {
    pub c_compiler: ToolchainComponent,
    pub cxx_compiler: ToolchainComponent,
    pub linker: ToolchainComponent,
    pub gnu_compiler: ToolchainComponent,
    pub make: ToolchainComponent,
    pub ninja: ToolchainComponent,
    pub pkgconf: ToolchainComponent,
}

pub struct ToolchainManager;

impl ToolchainManager {
    /// Periksa status seluruh komponen toolchain pada host
    pub fn get_status() -> ToolchainStatus {
        ToolchainStatus {
            c_compiler: detect_component("clang", &["clang-22", "clang", "/usr/lib/llvm/22/bin/clang", "/usr/bin/clang"]),
            cxx_compiler: detect_component("clang++", &["clang++-22", "clang++", "/usr/lib/llvm/22/bin/clang++", "/usr/bin/clang++"]),
            linker: detect_component("mold", &["mold", "/usr/bin/mold"]),
            gnu_compiler: detect_component("gcc", &["gcc", "/usr/bin/gcc"]),
            make: detect_component("make", &["make", "/usr/bin/make"]),
            ninja: detect_component("ninja", &["ninja", "/usr/bin/ninja"]),
            pkgconf: detect_component("pkgconf", &["pkgconf", "pkg-config", "/usr/bin/pkgconf", "/usr/bin/pkg-config"]),
        }
    }

    /// Kemas seed toolchain ke dalam file tar.xz atau tar.zst
    pub fn bundle_seed_toolchain(output_path: &Path) -> Result<PathBuf> {
        let _status = Self::get_status();
        let temp_dir = tempfile::tempdir().context("Gagal membuat temporary directory untuk staging toolchain")?;
        let stage_root = temp_dir.path();

        let usr_bin = stage_root.join("usr/bin");
        let usr_lib = stage_root.join("usr/lib");
        let etc_forge = stage_root.join("etc/forge");

        fs::create_dir_all(&usr_bin)?;
        fs::create_dir_all(&usr_lib)?;
        fs::create_dir_all(&etc_forge)?;

        let mut copied_binaries = Vec::new();

        // HARAM AMBIL DARI HOST: Salin HANYA biner, library, dan header yang 100% dikompilasi dari source code oleh Forge ke /tmp/forge/stage/
        let stage_root_src = Path::new("/tmp/forge/stage");
        if stage_root_src.exists() {
            if let Ok(pkg_dirs) = fs::read_dir(stage_root_src) {
                for pkg_dir in pkg_dirs.flatten() {
                    let pkg_path = pkg_dir.path();
                    let pkg_name = pkg_path.file_name().unwrap_or_default().to_string_lossy().to_string();

                    // Salin usr/bin dari staging paket
                    let stage_bin = pkg_path.join("usr/bin");
                    if stage_bin.exists() {
                        if let Ok(entries) = fs::read_dir(&stage_bin) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                let file_name = path.file_name().unwrap_or_default();
                                let dest_file = usr_bin.join(file_name);
                                if path.is_symlink() {
                                    if let Ok(target) = fs::read_link(&path) {
                                        let _ = make_symlink(&target.to_string_lossy(), &dest_file);
                                    }
                                } else {
                                    let _ = fs::copy(&path, &dest_file);
                                }
                                copied_binaries.push(pkg_name.clone());
                            }
                        }
                    }

                    // Salin usr/lib dari staging paket
                    let stage_lib = pkg_path.join("usr/lib");
                    if stage_lib.exists() {
                        if let Ok(entries) = fs::read_dir(&stage_lib) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                let file_name = path.file_name().unwrap_or_default();
                                let dest_file = usr_lib.join(file_name);
                                if path.is_symlink() {
                                    if let Ok(target) = fs::read_link(&path) {
                                        let _ = make_symlink(&target.to_string_lossy(), &dest_file);
                                    }
                                } else if path.is_dir() {
                                    let _ = Command::new("cp").arg("-r").arg(&path).arg(&dest_file).status();
                                } else {
                                    let _ = fs::copy(&path, &dest_file);
                                }
                            }
                        }
                    }

                    // Salin usr/include dari staging paket
                    let stage_include = pkg_path.join("usr/include");
                    if stage_include.exists() {
                        let usr_inc = stage_root.join("usr/include");
                        fs::create_dir_all(&usr_inc)?;
                        let _ = Command::new("cp").arg("-r").arg(&stage_include).arg(&usr_inc).status();
                    }
                }
            }
        }

        if copied_binaries.is_empty() {
            anyhow::bail!("Tidak ada paket hasil kompilasi source di /tmp/forge/stage/. Silakan kompilasi paket toolchain via 'forge build <pkg>' terlebih dahulu! (Haram ambil dari host)");
        }

        // Terapkan symlink standar pada biner yang terkompilasi dari source
        if usr_bin.join("clang").exists() {
            let _ = make_symlink("clang", &usr_bin.join("cc"));
        }
        if usr_bin.join("clang++").exists() {
            let _ = make_symlink("clang++", &usr_bin.join("c++"));
        }
        if usr_bin.join("pkgconf").exists() {
            let _ = make_symlink("pkgconf", &usr_bin.join("pkg-config"));
        }
        if usr_bin.join("mold").exists() {
            let _ = make_symlink("mold", &usr_bin.join("ld"));
            let _ = make_symlink("mold", &usr_bin.join("ld.mold"));
        }

        // Tulis environment loader Kura Linux dengan flag optimasi native silikon
        let env_content = r#"# /etc/forge/toolchain.conf
# Kura Linux Seed Toolchain Environment (LLVM 22 + mold + Native Silicon Optimization)
export CC="/usr/bin/clang"
export CXX="/usr/bin/clang++"
export LD="/usr/bin/mold"
export AR="/usr/bin/llvm-ar"
export NM="/usr/bin/llvm-nm"
export RANLIB="/usr/bin/llvm-ranlib"
export CFLAGS="-O3 -march=native -pipe -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"
export CXXFLAGS="${CFLAGS}"
export LDFLAGS="-Wl,-O1 -Wl,--as-needed -fuse-ld=/usr/bin/mold"
export MAKEFLAGS="-j$(nproc)"
"#;
        fs::write(etc_forge.join("toolchain.conf"), env_content)?;

        // Buat parent direktori jika belum ada
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Jalankan pengemasan tarball
        let final_output = if output_path.is_absolute() {
            output_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(output_path)
        };

        let tar_status = Command::new("tar")
            .arg("-cJf")
            .arg(&final_output)
            .arg("-C")
            .arg(stage_root)
            .arg("usr")
            .arg("etc")
            .status()
            .context("Gagal mengeksekusi perintah tar")?;

        if !tar_status.success() {
            anyhow::bail!("Gagal mengompresi seed toolchain ke {:?}", final_output);
        }

        // Buat file checksum sha256
        let sha256_output = Command::new("sha256sum")
            .arg(&final_output)
            .output();

        if let Ok(output) = sha256_output {
            let sha_file = final_output.with_extension("xz.sha256");
            let _ = fs::write(sha_file, output.stdout);
        }

        Ok(final_output)
    }
}

fn detect_component(name: &str, candidates: &[&str]) -> ToolchainComponent {
    for candidate in candidates {
        let which_output = Command::new("which").arg(candidate).output();
        let path = if let Ok(out) = which_output {
            if out.status.success() {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else if Path::new(candidate).exists() {
                candidate.to_string()
            } else {
                continue;
            }
        } else if Path::new(candidate).exists() {
            candidate.to_string()
        } else {
            continue;
        };

        let ver_output = Command::new(&path).arg("--version").output();
        let version = ver_output.ok().and_then(|out| {
            if out.status.success() {
                let first_line = String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string();
                Some(first_line)
            } else {
                None
            }
        });

        return ToolchainComponent {
            name: name.to_string(),
            path: Some(path),
            version,
            is_available: true,
        };
    }

    ToolchainComponent {
        name: name.to_string(),
        path: None,
        version: None,
        is_available: false,
    }
}

#[cfg(unix)]
fn make_symlink(target: &str, link: &Path) -> std::io::Result<()> {
    if link.exists() {
        let _ = fs::remove_file(link);
    }
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_symlink(_target: &str, _link: &Path) -> std::io::Result<()> {
    Ok(())
}
