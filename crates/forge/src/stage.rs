use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Format kompresi yang didukung untuk stage tarball
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageFormat {
    Xz,
    Zstd,
}

pub type CompressionFormat = StageFormat;

impl StageFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            StageFormat::Xz => "tar.xz",
            StageFormat::Zstd => "tar.zst",
        }
    }
}

impl std::fmt::Display for StageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StageFormat::Xz => write!(f, "xz"),
            StageFormat::Zstd => write!(f, "zstd"),
        }
    }
}

impl std::str::FromStr for StageFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "xz" | "tar.xz" | ".tar.xz" => Ok(StageFormat::Xz),
            "zstd" | "zst" | "tar.zst" | ".tar.zst" => Ok(StageFormat::Zstd),
            other => anyhow::bail!(
                "Format kompresi tidak didukung: '{}'. Gunakan 'xz' atau 'zstd'.",
                other
            ),
        }
    }
}

/// Opsi konfigurasi ekspor stage Kura Linux
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageExportOptions {
    pub root_dir: PathBuf,
    pub output_path: PathBuf,
    pub format: StageFormat,
    pub strip_binaries: bool,
    pub verify_usrmerge: bool,
    pub verify_openrc: bool,
    pub clean_temporary: bool,
}

impl Default for StageExportOptions {
    fn default() -> Self {
        let root_dir = std::env::var("FORGE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp/forge/stage"));
        Self {
            root_dir,
            output_path: PathBuf::from("dist/kura-stage.tar.xz"),
            format: StageFormat::Xz,
            strip_binaries: false,
            verify_usrmerge: true,
            verify_openrc: true,
            clean_temporary: true,
        }
    }
}

/// Ringkasan hasil pembuatan artefak stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageExportResult {
    pub output_path: PathBuf,
    pub file_size: u64,
    pub sha256_hash: String,
    pub blake3_hash: String,
    pub sha256_path: PathBuf,
    pub blake3_path: PathBuf,
    pub entries_count: usize,
}

/// Engine Distro Stage Exporter untuk Kura Linux
pub struct StageExporter;

impl StageExporter {
    /// 1. Validasi Pre-Flight Rootfs Kura Linux
    pub fn validate_rootfs(
        root: &Path,
        verify_usrmerge: bool,
        verify_openrc: bool,
    ) -> Result<()> {
        if !root.exists() || !root.is_dir() {
            anyhow::bail!("Direktori rootfs tidak ditemukan: {:?}", root);
        }

        // Validasi struktur direktori dasar FHS (/usr, /etc, /var)
        let usr_dir = root.join("usr");
        let etc_dir = root.join("etc");
        let var_dir = root.join("var");

        if !usr_dir.exists() || !etc_dir.exists() || !var_dir.exists() {
            anyhow::bail!(
                "Struktur direktori FHS dasar tidak lengkap di rootfs {:?}. Wajib memiliki /usr, /etc, dan /var",
                root
            );
        }

        // Validasi UsrMerge (/bin, /sbin, /lib sebagai symlink ke usr/bin atau usr/lib)
        if verify_usrmerge {
            let bin_path = root.join("bin");
            let sbin_path = root.join("sbin");
            let lib_path = root.join("lib");
            let lib64_path = root.join("lib64");

            let validate_symlink = |path: &Path, name: &str, expected_prefix: &str| -> Result<()> {
                let meta = fs::symlink_metadata(path).with_context(|| {
                    format!(
                        "UsrMerge validasi gagal: entri '{}' tidak ditemukan pada rootfs {:?}",
                        name, root
                    )
                })?;

                if !meta.file_type().is_symlink() {
                    anyhow::bail!(
                        "UsrMerge validasi gagal: '/{}' adalah direktori biasa, bukan symlink ke '{}'",
                        name,
                        expected_prefix
                    );
                }

                let target = fs::read_link(path)?;
                let target_str = target.to_string_lossy();
                if !target_str.contains("usr/") && !target_str.starts_with("/usr") {
                    anyhow::bail!(
                        "UsrMerge validasi gagal: symlink '/{}' mengarah ke '{:?}' (harus mengarah ke usr/...)",
                        name,
                        target
                    );
                }

                Ok(())
            };

            validate_symlink(&bin_path, "bin", "usr/bin")?;
            validate_symlink(&sbin_path, "sbin", "usr/bin")?;
            validate_symlink(&lib_path, "lib", "usr/lib")?;

            if lib64_path.exists() || fs::symlink_metadata(&lib64_path).is_ok() {
                validate_symlink(&lib64_path, "lib64", "usr/lib")?;
            }
        }

        // Validasi struktur OpenRC (/etc/init.d/ dan /etc/runlevels/)
        if verify_openrc {
            let init_d = root.join("etc/init.d");
            let runlevels = root.join("etc/runlevels");

            if !init_d.exists() || !init_d.is_dir() {
                anyhow::bail!(
                    "OpenRC validasi gagal: direktori layanan '/etc/init.d' tidak ditemukan di rootfs {:?}",
                    root
                );
            }

            if !runlevels.exists() || !runlevels.is_dir() {
                anyhow::bail!(
                    "OpenRC validasi gagal: direktori runlevel '/etc/runlevels' tidak ditemukan di rootfs {:?}",
                    root
                );
            }
        }

        Ok(())
    }

    /// 2. Sanitasi dan Pembersihan Temporary Files
    pub fn sanitize_staging(staging_copy: &Path) -> Result<()> {
        // Bersihkan isi /tmp/*
        let tmp_dir = staging_copy.join("tmp");
        if tmp_dir.exists() {
            if let Ok(entries) = fs::read_dir(&tmp_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        } else {
            let _ = fs::create_dir_all(&tmp_dir);
        }

        // Bersihkan isi /var/cache/* (misal /var/cache/forge/sync/*)
        let var_cache = staging_copy.join("var/cache");
        if var_cache.exists() {
            if let Ok(entries) = fs::read_dir(&var_cache) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        } else {
            let _ = fs::create_dir_all(&var_cache);
        }

        // Bersihkan isi /var/log/*
        let var_log = staging_copy.join("var/log");
        if var_log.exists() {
            if let Ok(entries) = fs::read_dir(&var_log) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let _ = fs::remove_dir_all(&path);
                    } else {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        } else {
            let _ = fs::create_dir_all(&var_log);
        }

        // Bersihkan isi file lock & PID di /var/lock/*, /var/run/*, /run/*
        for lock_rel in &["var/lock", "var/run", "run", "run/lock"] {
            let lock_dir = staging_copy.join(lock_rel);
            if lock_dir.exists() {
                if let Ok(entries) = fs::read_dir(&lock_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let _ = fs::remove_dir_all(&path);
                        } else {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }

        // Hapus file transien (.tmp, .journal, .lock) di luar area terlindung
        clean_transient_files_recursive(staging_copy, staging_copy)?;

        Ok(())
    }

    /// 3. Jalankan pipeline ekspor penuh: validasi, sanitasi, packaging tarball, dan kalkulasi checksum
    pub fn export(options: &StageExportOptions) -> Result<StageExportResult> {
        // Validasi rootfs
        Self::validate_rootfs(
            &options.root_dir,
            options.verify_usrmerge,
            options.verify_openrc,
        )?;

        // Staging copy di RAM / isolated temporary dir
        let temp_dir = tempfile::tempdir().context("Gagal membuat temporary directory untuk staging stage-export")?;
        let staging_copy = temp_dir.path().join("rootfs");
        fs::create_dir_all(&staging_copy)?;

        let entries_count = copy_dir_preserving(&options.root_dir, &staging_copy)
            .context("Gagal menyalin rootfs ke staging copy")?;

        // Sanitasi jika aktif
        if options.clean_temporary {
            Self::sanitize_staging(&staging_copy)?;
        }

        // Pastikan parent directory output tersedia
        let final_output = if options.output_path.is_absolute() {
            options.output_path.clone()
        } else {
            std::env::current_dir()?.join(&options.output_path)
        };

        if let Some(parent) = final_output.parent() {
            fs::create_dir_all(parent)?;
        }

        // Packaging tarball
        match options.format {
            StageFormat::Zstd => {
                let file = fs::File::create(&final_output)
                    .with_context(|| format!("Gagal membuat file output {:?}", final_output))?;
                let encoder = zstd::Encoder::new(file, 19)
                    .context("Gagal menginisialisasi zstd compression level 19")?;
                let mut tar_builder = tar::Builder::new(encoder);
                tar_builder.mode(tar::HeaderMode::Complete);

                append_tree_to_tar(&mut tar_builder, &staging_copy, Path::new(""))?;

                tar_builder.finish()?;
                let encoder = tar_builder.into_inner()?;
                let mut file = encoder.finish()?;
                file.flush()?;
            }
            StageFormat::Xz => {
                let has_xz_cmd = Command::new("xz").arg("--version").output().is_ok();
                if has_xz_cmd {
                    let out_file = fs::File::create(&final_output)
                        .with_context(|| format!("Gagal membuat file output {:?}", final_output))?;
                    let mut child = Command::new("xz")
                        .arg("-6")
                        .arg("-T0")
                        .stdin(std::process::Stdio::piped())
                        .stdout(out_file)
                        .spawn()
                        .context("Gagal menjalankan proses xz")?;

                    let child_stdin = child
                        .stdin
                        .take()
                        .context("Gagal mengambil stdin xz child process")?;
                    let mut tar_builder = tar::Builder::new(child_stdin);
                    tar_builder.mode(tar::HeaderMode::Complete);

                    append_tree_to_tar(&mut tar_builder, &staging_copy, Path::new(""))?;

                    tar_builder.finish()?;
                    drop(tar_builder);

                    let status = child.wait().context("Gagal menunggu xz compressor selesai")?;
                    if !status.success() {
                        anyhow::bail!("Kompresi XZ gagal untuk {:?}", final_output);
                    }
                } else {
                    let tar_status = Command::new("tar")
                        .arg("-cJf")
                        .arg(&final_output)
                        .arg("-C")
                        .arg(&staging_copy)
                        .arg(".")
                        .status()
                        .context("Gagal mengeksekusi perintah tar -cJf")?;
                    if !tar_status.success() {
                        anyhow::bail!("Gagal mengemas tar.xz ke {:?}", final_output);
                    }
                }
            }
        }

        // Kalkulasi SHA256 & BLAKE3 checksums
        let mut file = fs::File::open(&final_output)
            .with_context(|| format!("Gagal membuka file output {:?}", final_output))?;
        let mut sha256_hasher = Sha256::new();
        let mut blake3_hasher = blake3::Hasher::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            sha256_hasher.update(&buffer[..bytes_read]);
            blake3_hasher.update(&buffer[..bytes_read]);
        }

        let sha256_hash = format!("{:x}", sha256_hasher.finalize());
        let blake3_hash = blake3_hasher.finalize().to_hex().to_string();

        let file_name = final_output
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        let sha256_path = PathBuf::from(format!("{}.sha256", final_output.display()));
        let blake3_path = PathBuf::from(format!("{}.b3sum", final_output.display()));

        fs::write(&sha256_path, format!("{sha256_hash}  {file_name}\n"))?;
        fs::write(&blake3_path, format!("{blake3_hash}  {file_name}\n"))?;

        let file_size = fs::metadata(&final_output)?.len();

        Ok(StageExportResult {
            output_path: final_output,
            file_size,
            sha256_hash,
            blake3_hash,
            sha256_path,
            blake3_path,
            entries_count,
        })
    }
}

/// Helper untuk menyalin direktori secara rekursif dengan preservasi symlink dan permissions
fn copy_dir_preserving(src: &Path, dst: &Path) -> Result<usize> {
    let mut count = 0;
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    let entries = fs::read_dir(src)?;
    for entry in entries.flatten() {
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            let link_target = fs::read_link(&src_path)?;
            if dst_path.exists() || fs::symlink_metadata(&dst_path).is_ok() {
                let _ = fs::remove_file(&dst_path);
            }
            #[cfg(unix)]
            std::os::unix::fs::symlink(&link_target, &dst_path)?;
            count += 1;
        } else if file_type.is_dir() {
            if !dst_path.exists() {
                fs::create_dir_all(&dst_path)?;
            }
            #[cfg(unix)]
            {
                if let Ok(meta) = entry.metadata() {
                    let _ = fs::set_permissions(&dst_path, meta.permissions());
                }
            }
            count += 1;
            count += copy_dir_preserving(&src_path, &dst_path)?;
        } else {
            if dst_path.exists() || fs::symlink_metadata(&dst_path).is_ok() {
                let _ = fs::remove_file(&dst_path);
            }
            fs::copy(&src_path, &dst_path)?;
            #[cfg(unix)]
            {
                if let Ok(meta) = entry.metadata() {
                    let _ = fs::set_permissions(&dst_path, meta.permissions());
                }
            }
            count += 1;
        }
    }

    Ok(count)
}

/// Helper untuk mengemas pohon direktori ke dalam format tar dengan preservasi symlink dan permissions
fn append_tree_to_tar<W: Write>(
    builder: &mut tar::Builder<W>,
    current_dir: &Path,
    rel_prefix: &Path,
) -> Result<()> {
    let entries = fs::read_dir(current_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let rel_path = if rel_prefix.as_os_str().is_empty() {
            PathBuf::from(&file_name)
        } else {
            rel_prefix.join(&file_name)
        };

        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            let link_target = fs::read_link(&path)?;
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_path(&rel_path)?;
            header.set_link_name(&link_target)?;
            header.set_size(0);
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let meta = fs::symlink_metadata(&path)?;
                header.set_mode(meta.mode());
                header.set_mtime(meta.mtime() as u64);
                header.set_uid(meta.uid() as u64);
                header.set_gid(meta.gid() as u64);
            }
            header.set_cksum();
            builder.append(&header, std::io::empty())?;
        } else if file_type.is_dir() {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_path(&rel_path)?;
            header.set_size(0);
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                let meta = fs::metadata(&path)?;
                header.set_mode(meta.mode());
                header.set_mtime(meta.mtime() as u64);
                header.set_uid(meta.uid() as u64);
                header.set_gid(meta.gid() as u64);
            }
            header.set_cksum();
            builder.append(&header, std::io::empty())?;

            append_tree_to_tar(builder, &path, &rel_path)?;
        } else {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_path(&rel_path)?;
            let meta = fs::metadata(&path)?;
            header.set_size(meta.len());
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                header.set_mode(meta.mode());
                header.set_mtime(meta.mtime() as u64);
                header.set_uid(meta.uid() as u64);
                header.set_gid(meta.gid() as u64);
            }
            header.set_cksum();
            let mut file = fs::File::open(&path)?;
            builder.append(&header, &mut file)?;
        }
    }
    Ok(())
}

/// Helper untuk membersihkan file transien tanpa menyentuh area terlindung
fn clean_transient_files_recursive(current: &Path, root: &Path) -> Result<()> {
    if !current.exists() || !current.is_dir() {
        return Ok(());
    }

    // Path terlindung dari pembersihan
    let rel_path = current.strip_prefix(root).unwrap_or(current);
    let rel_str = rel_path.to_string_lossy();
    if rel_str.starts_with("var/db/forge")
        || rel_str.starts_with("etc/forge")
        || rel_str.starts_with("etc/init.d")
    {
        return Ok(());
    }

    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if let Ok(file_type) = entry.file_type() {
                if file_type.is_symlink() {
                    continue;
                } else if file_type.is_dir() {
                    clean_transient_files_recursive(&path, root)?;
                } else if name.ends_with(".tmp") || name.ends_with(".journal") || name.ends_with(".lock") {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_mock_rootfs() -> tempfile::TempDir {
        let temp = tempfile::tempdir().expect("Gagal membuat tempdir untuk mock rootfs");
        let root = temp.path();

        // Buat struktur direktori FHS
        fs::create_dir_all(root.join("usr/bin")).unwrap();
        fs::create_dir_all(root.join("usr/lib")).unwrap();
        fs::create_dir_all(root.join("etc/init.d")).unwrap();
        fs::create_dir_all(root.join("etc/runlevels/default")).unwrap();
        fs::create_dir_all(root.join("etc/forge")).unwrap();
        fs::create_dir_all(root.join("var/db/forge/installed/base-1.0")).unwrap();
        fs::create_dir_all(root.join("tmp")).unwrap();
        fs::create_dir_all(root.join("var/cache/forge/sync")).unwrap();
        fs::create_dir_all(root.join("var/log")).unwrap();
        fs::create_dir_all(root.join("var/lock")).unwrap();

        // Buat file penting
        fs::write(root.join("usr/bin/bash"), b"#!/bin/sh\necho bash").unwrap();
        fs::write(root.join("usr/lib/libc.so"), b"fake libc").unwrap();
        fs::write(root.join("etc/forge/forge.conf"), b"[general]\nroot='/'").unwrap();
        fs::write(root.join("etc/init.d/bootmisc"), b"#!/sbin/openrc-run").unwrap();
        fs::write(
            root.join("var/db/forge/installed/base-1.0/manifest"),
            b"obj /usr/bin/bash",
        )
        .unwrap();

        // Buat symlink UsrMerge
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("usr/bin", root.join("bin")).unwrap();
            std::os::unix::fs::symlink("usr/bin", root.join("sbin")).unwrap();
            std::os::unix::fs::symlink("usr/lib", root.join("lib")).unwrap();
        }

        // Buat file temporary/sampah
        fs::write(root.join("tmp/junk.tmp"), b"temporary junk").unwrap();
        fs::write(
            root.join("var/cache/forge/sync/cache.tar.zst"),
            b"cached sync",
        )
        .unwrap();
        fs::write(root.join("var/log/build.log"), b"build log").unwrap();
        fs::write(root.join("var/lock/forge.lock"), b"1234").unwrap();

        temp
    }

    #[test]
    fn test_stage_exporter_validates_usrmerge() {
        let temp = setup_mock_rootfs();
        let root = temp.path();

        // Validasi pada rootfs normal seharusnya sukses
        assert!(StageExporter::validate_rootfs(root, true, true).is_ok());

        // Hapus symlink bin dan buat sebagai folder biasa (melanggar UsrMerge)
        fs::remove_file(root.join("bin")).unwrap();
        fs::create_dir(root.join("bin")).unwrap();

        let result = StageExporter::validate_rootfs(root, true, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("UsrMerge validasi gagal"));
    }

    #[test]
    fn test_stage_exporter_validates_openrc() {
        let temp = setup_mock_rootfs();
        let root = temp.path();

        // Validasi awal sukses
        assert!(StageExporter::validate_rootfs(root, true, true).is_ok());

        // Hapus direktori /etc/runlevels
        fs::remove_dir_all(root.join("etc/runlevels")).unwrap();

        let result = StageExporter::validate_rootfs(root, false, true);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("OpenRC validasi gagal"));
    }

    #[test]
    fn test_stage_exporter_sanitizes_temporary_files() {
        let temp = setup_mock_rootfs();
        let root = temp.path();

        // Jalankan sanitasi
        StageExporter::sanitize_staging(root).expect("Sanitasi gagal");

        // Temporary files harus terhapus
        assert!(!root.join("tmp/junk.tmp").exists());
        assert!(!root.join("var/cache/forge/sync/cache.tar.zst").exists());
        assert!(!root.join("var/log/build.log").exists());
        assert!(!root.join("var/lock/forge.lock").exists());

        // Direktori penting & konfigurasi harus dipertahankan
        assert!(root.join("tmp").exists());
        assert!(root.join("usr/bin/bash").exists());
        assert!(root.join("etc/forge/forge.conf").exists());
        assert!(root.join("etc/init.d/bootmisc").exists());
        assert!(root.join("var/db/forge/installed/base-1.0/manifest").exists());
    }

    #[test]
    fn test_stage_exporter_full_export_tarball_and_checksums() {
        let temp_root = setup_mock_rootfs();
        let temp_out = tempfile::tempdir().expect("Gagal membuat tempdir output");
        let output_tarball = temp_out.path().join("kura-stage.tar.zst");

        let options = StageExportOptions {
            root_dir: temp_root.path().to_path_buf(),
            output_path: output_tarball.clone(),
            format: StageFormat::Zstd,
            strip_binaries: false,
            verify_usrmerge: true,
            verify_openrc: true,
            clean_temporary: true,
        };

        let result = StageExporter::export(&options).expect("Ekspor stage gagal");

        // Periksa artefak tarball dan checksum
        assert!(result.output_path.exists());
        assert!(result.sha256_path.exists());
        assert!(result.blake3_path.exists());
        assert!(result.file_size > 0);
        assert_eq!(result.sha256_hash.len(), 64);
        assert_eq!(result.blake3_hash.len(), 64);

        let sha_content = fs::read_to_string(&result.sha256_path).unwrap();
        assert!(sha_content.contains(&result.sha256_hash));

        let b3_content = fs::read_to_string(&result.blake3_path).unwrap();
        assert!(b3_content.contains(&result.blake3_hash));

        // Ekstraksi dan verifikasi integritas isi tarball
        let extract_dir = tempfile::tempdir().expect("Gagal membuat extract tempdir");
        let tar_file = fs::File::open(&output_tarball).unwrap();
        let decoder = zstd::Decoder::new(tar_file).unwrap();
        let mut archive = tar::Archive::new(decoder);
        archive.unpack(extract_dir.path()).expect("Ekstraksi tarball gagal");

        let extracted_root = extract_dir.path();
        assert!(extracted_root.join("usr/bin/bash").exists());
        assert!(extracted_root.join("etc/forge/forge.conf").exists());
        assert!(extracted_root.join("var/db/forge/installed/base-1.0/manifest").exists());

        // Verifikasi file temporary telah bersih dari tarball
        assert!(!extracted_root.join("tmp/junk.tmp").exists());
        assert!(!extracted_root.join("var/cache/forge/sync/cache.tar.zst").exists());

        // Verifikasi symlink UsrMerge tetap utuh dalam tarball
        #[cfg(unix)]
        {
            let bin_entry = extracted_root.join("bin");
            let meta = fs::symlink_metadata(&bin_entry).expect("Metadata bin hilang");
            assert!(meta.file_type().is_symlink());
            let target = fs::read_link(&bin_entry).expect("Gagal membaca link bin");
            assert_eq!(target.to_string_lossy(), "usr/bin");
        }
    }

    #[test]
    fn test_stage_exporter_xz_format() {
        let temp_root = setup_mock_rootfs();
        let temp_out = tempfile::tempdir().expect("Gagal membuat tempdir output");
        let output_tarball = temp_out.path().join("kura-stage.tar.xz");

        let options = StageExportOptions {
            root_dir: temp_root.path().to_path_buf(),
            output_path: output_tarball.clone(),
            format: StageFormat::Xz,
            strip_binaries: false,
            verify_usrmerge: true,
            verify_openrc: true,
            clean_temporary: true,
        };

        let result = StageExporter::export(&options).expect("Ekspor stage XZ gagal");

        assert!(result.output_path.exists());
        assert!(result.sha256_path.exists());
        assert!(result.blake3_path.exists());
        assert!(result.file_size > 0);
        assert_eq!(result.sha256_hash.len(), 64);
        assert_eq!(result.blake3_hash.len(), 64);

        let sha_content = fs::read_to_string(&result.sha256_path).unwrap();
        assert!(sha_content.contains(&result.sha256_hash));

        let b3_content = fs::read_to_string(&result.blake3_path).unwrap();
        assert!(b3_content.contains(&result.blake3_hash));
    }
}
