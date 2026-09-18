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
        let status = Self::get_status();
        let temp_dir = tempfile::tempdir().context("Gagal membuat temporary directory untuk staging toolchain")?;
        let stage_root = temp_dir.path();

        let usr_bin = stage_root.join("usr/bin");
        let usr_lib = stage_root.join("usr/lib");
        let etc_forge = stage_root.join("etc/forge");

        fs::create_dir_all(&usr_bin)?;
        fs::create_dir_all(&usr_lib)?;
        fs::create_dir_all(&etc_forge)?;

        let mut copied_binaries = Vec::new();
        let components = [
            &status.c_compiler,
            &status.cxx_compiler,
            &status.linker,
            &status.gnu_compiler,
            &status.make,
            &status.ninja,
            &status.pkgconf,
        ];

        // Salin biner dari staging Forge (/tmp/forge/stage/) jika tersedia hasil kompilasi resep
        let stage_pkgs = ["pkgconf", "make", "ninja", "mold"];
        for pkg in stage_pkgs {
            let pkg_stage_bin = PathBuf::from(format!("/tmp/forge/stage/{}/usr/bin", pkg));
            if pkg_stage_bin.exists() {
                if let Ok(entries) = fs::read_dir(&pkg_stage_bin) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() || path.is_symlink() {
                            let file_name = path.file_name().unwrap_or_default();
                            let dest_file = usr_bin.join(file_name);
                            if path.is_symlink() {
                                if let Ok(target) = fs::read_link(&path) {
                                    let _ = make_symlink(&target.to_string_lossy(), &dest_file);
                                }
                            } else {
                                let _ = fs::copy(&path, &dest_file);
                            }
                            copied_binaries.push(pkg.to_string());
                        }
                    }
                }
            }

            let pkg_stage_lib = PathBuf::from(format!("/tmp/forge/stage/{}/usr/lib", pkg));
            if pkg_stage_lib.exists() {
                if let Ok(entries) = fs::read_dir(&pkg_stage_lib) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() || path.is_symlink() {
                            let file_name = path.file_name().unwrap_or_default();
                            let dest_file = usr_lib.join(file_name);
                            if path.is_symlink() {
                                if let Ok(target) = fs::read_link(&path) {
                                    let _ = make_symlink(&target.to_string_lossy(), &dest_file);
                                }
                            } else {
                                let _ = fs::copy(&path, &dest_file);
                            }
                        }
                    }
                }
            }
        }

        // Lengkapi compiler utama dan utilitas LLVM (Clang 22, LLD, llvm-ar, GCC, mold)
        for comp in components {
            if let Some(ref path_str) = comp.path {
                let src_path = Path::new(path_str);
                let dest_name = src_path.file_name().unwrap_or_default();
                let dest_path = usr_bin.join(dest_name);
                if !dest_path.exists() && src_path.exists() {
                    let _ = fs::copy(src_path, &dest_path);
                    copied_binaries.push(comp.name.clone());

                    if comp.name == "clang" {
                        let _ = make_symlink("clang", &usr_bin.join("cc"));
                    } else if comp.name == "clang++" {
                        let _ = make_symlink("clang++", &usr_bin.join("c++"));
                    } else if comp.name == "pkgconf" {
                        let _ = make_symlink("pkgconf", &usr_bin.join("pkg-config"));
                    } else if comp.name == "mold" {
                        let _ = make_symlink("mold", &usr_bin.join("ld"));
                        let _ = make_symlink("mold", &usr_bin.join("ld.mold"));
                    }
                }
            }
        }

        // Salin biner pembantu LLVM jika ada (lld, llvm-ar, llvm-nm, llvm-objdump)
        let llvm_extras = ["lld", "llvm-ar", "llvm-as", "llvm-nm", "llvm-objdump", "llvm-ranlib"];
        for extra in llvm_extras {
            let candidate_paths = [
                format!("/usr/lib/llvm/22/bin/{}", extra),
                format!("/usr/bin/{}", extra),
            ];
            for cand in candidate_paths {
                let p = Path::new(&cand);
                if p.exists() {
                    let dest = usr_bin.join(extra);
                    if !dest.exists() {
                        let _ = fs::copy(p, &dest);
                        if extra == "lld" {
                            let _ = make_symlink("lld", &usr_bin.join("ld.lld"));
                        } else if extra == "llvm-ar" {
                            let _ = make_symlink("llvm-ar", &usr_bin.join("ar"));
                        } else if extra == "llvm-nm" {
                            let _ = make_symlink("llvm-nm", &usr_bin.join("nm"));
                        } else if extra == "llvm-objdump" {
                            let _ = make_symlink("llvm-objdump", &usr_bin.join("objdump"));
                        } else if extra == "llvm-ranlib" {
                            let _ = make_symlink("llvm-ranlib", &usr_bin.join("ranlib"));
                        }
                    }
                    break;
                }
            }
        }

        // Salin header bawaan Clang (/usr/lib/clang) jika ada
        let clang_lib_dir = Path::new("/usr/lib/clang");
        if clang_lib_dir.exists() {
            let _ = Command::new("cp")
                .arg("-r")
                .arg(clang_lib_dir)
                .arg(&usr_lib)
                .status();
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
