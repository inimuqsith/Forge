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
    pub c_library: ToolchainComponent,
    pub dynamic_linker: ToolchainComponent,
    pub kernel_headers: ToolchainComponent,
    pub binutils: ToolchainComponent,
    pub make: ToolchainComponent,
    pub ninja: ToolchainComponent,
    pub pkgconf: ToolchainComponent,
}

pub struct ToolchainManager;

impl ToolchainManager {
    /// Periksa status seluruh komponen toolchain pada host termasuk Glibc dan Kernel Headers
    pub fn get_status() -> ToolchainStatus {
        ToolchainStatus {
            c_compiler: detect_component("clang", &["clang-22", "clang", "/usr/lib/llvm/22/bin/clang", "/usr/bin/clang"]),
            cxx_compiler: detect_component("clang++", &["clang++-22", "clang++", "/usr/lib/llvm/22/bin/clang++", "/usr/bin/clang++"]),
            linker: detect_component("mold", &["mold", "/usr/bin/mold"]),
            gnu_compiler: detect_component("gcc", &["gcc", "/usr/bin/gcc"]),
            c_library: detect_glibc(),
            dynamic_linker: detect_dynamic_linker(),
            kernel_headers: detect_kernel_headers(),
            binutils: detect_component("binutils (as/ar)", &["as", "/usr/bin/as", "ar", "/usr/bin/ar"]),
            make: detect_component("make", &["make", "/usr/bin/make"]),
            ninja: detect_component("ninja", &["ninja", "/usr/bin/ninja"]),
            pkgconf: detect_component("pkgconf", &["pkgconf", "pkg-config", "/usr/bin/pkgconf", "/usr/bin/pkg-config"]),
        }
    }

    /// Kemas seed toolchain dari default staging path (/tmp/forge/stage) ke dalam file tar.xz atau tar.zst
    pub fn bundle_seed_toolchain(output_path: &Path) -> Result<PathBuf> {
        Self::bundle_seed_toolchain_with_stage(output_path, Path::new("/tmp/forge/stage"))
    }

    /// Kemas seed toolchain dari direktori staging tertentu (memudahkan unit testing & isolasi)
    pub fn bundle_seed_toolchain_with_stage(output_path: &Path, stage_root_src: &Path) -> Result<PathBuf> {
        let temp_dir = tempfile::tempdir().context("Gagal membuat temporary directory untuk staging toolchain")?;
        let stage_root = temp_dir.path();

        let usr_bin = stage_root.join("usr/bin");
        let usr_lib = stage_root.join("usr/lib");
        let usr_inc = stage_root.join("usr/include");
        let etc_forge = stage_root.join("etc/forge");

        fs::create_dir_all(&usr_bin)?;
        fs::create_dir_all(&usr_lib)?;
        fs::create_dir_all(&usr_inc)?;
        fs::create_dir_all(&etc_forge)?;

        // 1. Buat hierarki direktori UsrMerge standar & skeleton direktori OS
        let skeleton_dirs = [
            "var/db/forge/recipes",
            "var/db/forge/installed",
            "var/cache/forge/ccache",
            "var/cache/forge/distfiles",
            "var/log/forge",
            "tmp",
            "dev",
            "proc",
            "sys",
            "root",
            "home",
            "etc/conf.d",
            "etc/init.d",
        ];

        for dir in &skeleton_dirs {
            fs::create_dir_all(stage_root.join(dir))?;
        }


        // 2. Buat symlink UsrMerge di root level
        let _ = make_symlink("usr/bin", &stage_root.join("bin"));
        let _ = make_symlink("usr/bin", &stage_root.join("sbin"));
        let _ = make_symlink("usr/lib", &stage_root.join("lib"));
        let _ = make_symlink("usr/lib", &stage_root.join("lib64"));
        let _ = make_symlink("lib", &usr_lib.parent().unwrap().join("usr/lib64"));
        let _ = make_symlink("bin", &usr_bin.parent().unwrap().join("usr/sbin"));

        let mut copied_packages = Vec::new();
        let mut has_glibc = false;

        // 3. HARAM AMBIL DARI HOST: Salin HANYA biner, library, dan header yang 100% dikompilasi dari source code oleh Forge di stage_root_src
        if stage_root_src.exists() {
            if let Ok(pkg_dirs) = fs::read_dir(stage_root_src) {
                for pkg_dir in pkg_dirs.flatten() {
                    let pkg_path = pkg_dir.path();
                    let pkg_name = pkg_path.file_name().unwrap_or_default().to_string_lossy().to_string();

                    if pkg_name.contains("glibc") {
                        has_glibc = true;
                    }

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
                            }
                        }
                    }

                    // Salin usr/lib dari staging paket (termasuk Glibc libc.so, ld-linux, CRT objects)
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

                    // Salin usr/include dari staging paket (termasuk C & Linux headers)
                    let stage_include = pkg_path.join("usr/include");
                    if stage_include.exists() {
                        let _ = Command::new("cp").arg("-r").arg(&stage_include).arg(usr_inc.parent().unwrap()).status();
                    }

                    copied_packages.push(pkg_name);
                }
            }
        }

        if copied_packages.is_empty() {
            anyhow::bail!("Tidak ada paket hasil kompilasi source di {:?}. Silakan kompilasi paket toolchain via 'forge build <pkg>' terlebih dahulu! (Haram ambil dari host)", stage_root_src);
        }

        // 4. Terapkan symlink standar pada biner compiler & build tools
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

        // 5. Sertakan biner Forge ke dalam seed toolchain agar siap pakai di dalam chroot
        let forge_bin_candidates = [
            PathBuf::from("target/release/forge"),
            PathBuf::from("target/debug/forge"),
        ];
        if let Some(forge_bin) = forge_bin_candidates.iter().find(|p| p.exists()) {
            let dest_forge = usr_bin.join("forge");
            let _ = fs::copy(forge_bin, &dest_forge);
            let _ = Command::new("chmod").arg("755").arg(&dest_forge).status();
        }

        // 6. Injeksi Konfigurasi Tunggal Package Manager (/etc/forge/forge.conf)
        let forge_conf_content = r#"# /etc/forge/forge.conf
# Kura Linux Package Manager Configuration

[general]
root = "/"
cache_dir = "/var/cache/forge"
db_dir = "/var/db/forge"
server_url = "https://pkgkura.amqs.net"
jobs = 0

[build]
cflags = "-O3 -march=native -pipe -flto=thin -fstack-protector-strong -fuse-ld=/usr/bin/mold"
cxxflags = "-O3 -march=native -pipe -flto=thin -fstack-protector-strong -fuse-ld=/usr/bin/mold"
use_flags = []
"#;
        fs::write(etc_forge.join("forge.conf"), forge_conf_content)?;

        // 7. Tulis environment loader Kura Linux dengan flag optimasi native silikon
        let toolchain_env_content = r#"# /etc/forge/toolchain.conf
# Kura Linux Seed Toolchain Environment (LLVM 22 + mold + Native Silicon Optimization)
export CC="/usr/bin/clang"
export CXX="/usr/bin/clang++"
export LD="/usr/bin/mold"
export AR="/usr/bin/llvm-ar"
export NM="/usr/bin/llvm-nm"
export RANLIB="/usr/bin/llvm-ranlib"
export CFLAGS="-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2"
export CXXFLAGS="${CFLAGS}"
export LDFLAGS="-Wl,-O3 -Wl,--as-needed -Wl,--gc-sections -Wl,--icf=all -fuse-ld=/usr/bin/mold"
export MAKEFLAGS="-j$(nproc)"
"#;
        fs::write(etc_forge.join("toolchain.conf"), toolchain_env_content)?;

        // 8. Buat parent direktori jika belum ada
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 9. Jalankan pengemasan tarball lengkap dengan seluruh root UsrMerge
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
            .arg("bin")
            .arg("sbin")
            .arg("lib")
            .arg("lib64")
            .arg("usr")
            .arg("etc")
            .arg("var")
            .arg("tmp")
            .arg("dev")
            .arg("proc")
            .arg("sys")
            .arg("root")
            .arg("home")
            .status()
            .context("Gagal mengeksekusi perintah tar")?;

        if !tar_status.success() {
            anyhow::bail!("Gagal mengompresi seed toolchain ke {:?}", final_output);
        }

        // 10. Buat file checksum sha256
        let sha256_output = Command::new("sha256sum")
            .arg(&final_output)
            .output();

        if let Ok(output) = sha256_output {
            let sha_file = final_output.with_extension("xz.sha256");
            let _ = fs::write(sha_file, output.stdout);
        }

        if !has_glibc {
            eprintln!("[PERINGATAN] Toolchain dikemas tanpa staging Glibc di {:?}. Pastikan Glibc sudah terpasang jika menargetkan sistem bootable penuh!", stage_root_src);
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

fn detect_glibc() -> ToolchainComponent {
    let getconf_output = Command::new("getconf").arg("GNU_LIBC_VERSION").output();
    if let Ok(out) = getconf_output {
        if out.status.success() {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            return ToolchainComponent {
                name: "glibc (C Library)".to_string(),
                path: Some("/usr/lib/libc.so.6".to_string()),
                version: Some(ver),
                is_available: true,
            };
        }
    }

    // Fallback pencarian file library libc.so.6
    let candidates = [
        "/usr/lib/libc.so.6",
        "/lib/x86_64-linux-gnu/libc.so.6",
        "/usr/lib64/libc.so.6",
        "/lib64/libc.so.6",
    ];

    for candidate in &candidates {
        if Path::new(candidate).exists() {
            return ToolchainComponent {
                name: "glibc (C Library)".to_string(),
                path: Some(candidate.to_string()),
                version: Some("Detected via libc.so.6".to_string()),
                is_available: true,
            };
        }
    }

    ToolchainComponent {
        name: "glibc (C Library)".to_string(),
        path: None,
        version: None,
        is_available: false,
    }
}

fn detect_dynamic_linker() -> ToolchainComponent {
    let candidates = [
        "/lib64/ld-linux-x86-64.so.2",
        "/usr/lib/ld-linux-x86-64.so.2",
        "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
    ];

    for candidate in &candidates {
        if Path::new(candidate).exists() {
            return ToolchainComponent {
                name: "ld-linux (Dynamic Linker)".to_string(),
                path: Some(candidate.to_string()),
                version: Some("x86_64 ELF Interpreter".to_string()),
                is_available: true,
            };
        }
    }

    ToolchainComponent {
        name: "ld-linux (Dynamic Linker)".to_string(),
        path: None,
        version: None,
        is_available: false,
    }
}

fn detect_kernel_headers() -> ToolchainComponent {
    let candidates = [
        "/usr/include/linux/version.h",
        "/usr/include/linux/types.h",
    ];

    for candidate in &candidates {
        if Path::new(candidate).exists() {
            return ToolchainComponent {
                name: "linux-headers (Kernel API)".to_string(),
                path: Some(candidate.to_string()),
                version: Some("Linux Kernel C Headers Available".to_string()),
                is_available: true,
            };
        }
    }

    ToolchainComponent {
        name: "linux-headers (Kernel API)".to_string(),
        path: None,
        version: None,
        is_available: false,
    }
}

#[cfg(unix)]
fn make_symlink(target: &str, link: &Path) -> std::io::Result<()> {
    if link.exists() || link.is_symlink() {
        let _ = fs::remove_file(link);
    }
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_symlink(_target: &str, _link: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_toolchain_status_detection_including_glibc() {
        let status = ToolchainManager::get_status();
        // Pada host linux, libc dan dynamic linker semestinya terdeteksi
        assert!(status.c_library.is_available);
        assert!(status.dynamic_linker.is_available);
        assert_eq!(status.c_library.name, "glibc (C Library)");
        assert_eq!(status.dynamic_linker.name, "ld-linux (Dynamic Linker)");
    }

    #[test]
    fn test_bundle_seed_toolchain_fails_when_empty_stage() {
        let temp_stage = tempdir().unwrap();
        let temp_out = tempdir().unwrap();
        let out_tar = temp_out.path().join("empty-toolchain.tar.xz");

        let result = ToolchainManager::bundle_seed_toolchain_with_stage(&out_tar, temp_stage.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Tidak ada paket hasil kompilasi source"));
    }

    #[test]
    fn test_bundle_seed_toolchain_usrmerge_structure_and_config() {
        let temp_stage = tempdir().unwrap();
        let temp_out = tempdir().unwrap();
        let out_tar = temp_out.path().join("kura-toolchain.tar.xz");

        // Simulasikan staging paket llvm & glibc
        let glibc_stage = temp_stage.path().join("glibc");
        let glibc_lib = glibc_stage.join("usr/lib");
        let glibc_inc = glibc_stage.join("usr/include");
        fs::create_dir_all(&glibc_lib).unwrap();
        fs::create_dir_all(&glibc_inc).unwrap();
        fs::write(glibc_lib.join("libc.so.6"), b"mock-libc").unwrap();
        fs::write(glibc_inc.join("stdio.h"), b"/* mock stdio */").unwrap();

        let llvm_stage = temp_stage.path().join("llvm");
        let llvm_bin = llvm_stage.join("usr/bin");
        fs::create_dir_all(&llvm_bin).unwrap();
        fs::write(llvm_bin.join("clang"), b"#!/bin/sh\necho clang").unwrap();

        let result = ToolchainManager::bundle_seed_toolchain_with_stage(&out_tar, temp_stage.path());
        assert!(result.is_ok());
        assert!(out_tar.exists());
        assert!(out_tar.with_extension("xz.sha256").exists());

        // Verifikasi isi tarball menggunakan command tar -tf
        let tar_list = Command::new("tar")
            .arg("-tf")
            .arg(&out_tar)
            .output()
            .expect("Gagal menjalankan tar -tf");

        let content = String::from_utf8_lossy(&tar_list.stdout);
        assert!(content.contains("bin"));
        assert!(content.contains("lib64"));
        assert!(content.contains("usr/bin/clang"));
        assert!(content.contains("usr/lib/libc.so.6"));
        assert!(content.contains("etc/forge/forge.conf"));
        assert!(content.contains("etc/forge/toolchain.conf"));
        assert!(content.contains("var/db/forge/recipes"));
    }
}
