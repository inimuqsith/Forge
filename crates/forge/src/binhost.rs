use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinhostPackageEntry {
    pub pkgname: String,
    pub pkgver: String,
    pub pkgrel: u32,
    pub slot: String,
    pub target_march: String,
    pub active_use: Vec<String>,
    pub sha256: String,
    pub size_bytes: u64,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinhostCatalog {
    pub timestamp: u64,
    pub server_version: String,
    pub packages: Vec<BinhostPackageEntry>,
}

/// Hasil proses unduh dan ekstraksi streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadStreamResult {
    pub package_name: String,
    pub bytes_downloaded: u64,
    pub sha256_hash: String,
    pub blake3_hash: String,
    pub staging_dir: PathBuf,
}

/// Struktur tanda tangan kriptografis paket biner Kura Linux (Supply Chain Security)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSignature {
    pub package_name: String,
    pub version: String,
    pub sha256: String,
    pub blake3: String,
    pub signature_hex: String,
    pub signer_identity: String,
    pub timestamp: u64,
}

impl PackageSignature {
    /// Validasi integritas hash paket terhadap tanda tangan digital
    pub fn verify_hash(&self, calculated_sha256: &str, calculated_blake3: &str) -> bool {
        self.sha256 == calculated_sha256 && self.blake3 == calculated_blake3
    }
}

pub struct BinhostClient {
    pub server_url: String,
}

impl BinhostClient {
    pub fn new(server_url: &str) -> Self {
        Self {
            server_url: server_url.to_string(),
        }
    }

    pub fn find_matching_package(
        &self,
        catalog: &BinhostCatalog,
        pkgname: &str,
        target_march: &str,
    ) -> Option<BinhostPackageEntry> {
        catalog
            .packages
            .iter()
            .find(|p| p.pkgname == pkgname && (p.target_march == target_march || p.target_march == "generic"))
            .cloned()
    }

    /// Unduh tarball biner (.forge.tar.zst atau .pkg.tar.zst) secara streaming via HTTP GET,
    /// menghitung hash SHA256 & BLAKE3 on-the-fly, dan mengekstrak isi arsip langsung ke staging_dir.
    pub async fn download_and_extract_stream(
        url: &str,
        staging_dir: &Path,
        expected_sha256: Option<&str>,
        show_progress: bool,
    ) -> Result<DownloadStreamResult> {
        let client = reqwest::Client::builder()
            .user_agent("Forge-Client/0.1.0 (KuraLinux Distro Engine)")
            .build()?;

        let response = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Gagal menghubungi server untuk mengunduh {}", url))?;

        if !response.status().is_success() {
            anyhow::bail!("Server mengembalikan status HTTP {}: {}", response.status(), url);
        }

        let total_size = response.content_length().unwrap_or(0);
        let pb = if show_progress && total_size > 0 {
            let p = indicatif::ProgressBar::new(total_size);
            p.set_style(
                indicatif::ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
                    .unwrap()
                    .progress_chars("#>-"),
            );
            p.set_message("Mengunduh paket biner...");
            Some(p)
        } else {
            None
        };

        tokio::fs::create_dir_all(staging_dir)
            .await
            .with_context(|| format!("Gagal membuat direktori staging {:?}", staging_dir))?;

        // Streaming ke file sementara terisolasi
        let temp_archive = tempfile::Builder::new()
            .prefix("forge_stream_")
            .suffix(".tar.zst")
            .tempfile()?;
        let temp_archive_path = temp_archive.path().to_path_buf();

        let mut file = tokio::fs::File::create(&temp_archive_path).await?;
        let mut sha256_hasher = Sha256::new();
        let mut blake3_hasher = blake3::Hasher::new();
        let mut bytes_downloaded = 0u64;

        let mut stream = response;
        while let Some(chunk) = stream.chunk().await? {
            sha256_hasher.update(&chunk);
            blake3_hasher.update(&chunk);
            bytes_downloaded += chunk.len() as u64;
            file.write_all(&chunk).await?;
            if let Some(ref p) = pb {
                p.set_position(bytes_downloaded);
            }
        }

        file.flush().await?;
        drop(file);

        if let Some(ref p) = pb {
            p.finish_with_message("Unduhan biner selesai!");
        }

        let sha256_hash = format!("{:x}", sha256_hasher.finalize());
        let blake3_hash = blake3_hasher.finalize().to_hex().to_string();

        if let Some(expected) = expected_sha256 {
            if !expected.is_empty() && sha256_hash != expected {
                anyhow::bail!(
                    "Integritas biner gagal: Mismatch SHA256! Diharapkan: {}, Ditemukan: {}",
                    expected,
                    sha256_hash
                );
            }
        }

        // Dekompresi Zstd dan ekstrak Tar ke direktori staging (spawn_blocking)
        let staging_dir_buf = staging_dir.to_path_buf();
        let temp_archive_path_buf = temp_archive_path.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            let tar_file = std::fs::File::open(&temp_archive_path_buf)
                .with_context(|| format!("Gagal membuka file arsip sementara {:?}", temp_archive_path_buf))?;
            let decoder = zstd::Decoder::new(tar_file)
                .context("Gagal menginisialisasi dekompresor Zstd")?;
            let mut archive = tar::Archive::new(decoder);
            archive.set_preserve_permissions(true);
            archive.set_unpack_xattrs(true);
            archive.unpack(&staging_dir_buf)
                .with_context(|| format!("Gagal mengekstrak arsip tarball ke {:?}", staging_dir_buf))?;
            Ok(())
        })
        .await
        .context("Gagal mengeksekusi worker thread ekstraksi tarball")??;

        let package_name = url.split('/').last().unwrap_or("package").to_string();

        Ok(DownloadStreamResult {
            package_name,
            bytes_downloaded,
            sha256_hash,
            blake3_hash,
            staging_dir: staging_dir.to_path_buf(),
        })
    }

    /// Verifikasi digital signature Ed25519 untuk file arsip biner
    pub fn verify_archive_signature(
        archive_path: &Path,
        sig_path: &Path,
        verifier: &crate::crypto::PackageVerifier,
    ) -> Result<()> {
        verifier.verify_file_sig_file(archive_path, sig_path)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::crypto::SigningKeyPair;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_binhost_signature_verification() -> Result<()> {
        let temp = tempdir()?;
        let pkg_tar = temp.path().join("ripgrep-14.1.0-1.forge.tar.zst");
        let sig_file = temp.path().join("ripgrep-14.1.0-1.forge.tar.zst.sig");

        fs::write(&pkg_tar, b"Binary payload for ripgrep")?;

        let keypair = SigningKeyPair::generate();
        keypair.sign_file_to_sig_file(&pkg_tar, &sig_file)?;

        let verifier = crate::crypto::PackageVerifier::from_verifying_key(keypair.verifying_key());
        let verify_result = BinhostClient::verify_archive_signature(&pkg_tar, &sig_file, &verifier);
        assert!(verify_result.is_ok(), "Verifikasi signature sah harus berhasil");

        // Modifikasi isi paket untuk memastikan tamper detection
        fs::write(&pkg_tar, b"Tampered binary payload")?;
        let tampered_result = BinhostClient::verify_archive_signature(&pkg_tar, &sig_file, &verifier);
        assert!(tampered_result.is_err(), "Verifikasi signature paket termodifikasi harus gagal");

        Ok(())
    }
}
