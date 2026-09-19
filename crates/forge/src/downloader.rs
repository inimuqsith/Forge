use anyhow::{bail, Context, Result};
use blake3::Hasher as Blake3Hasher;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Opsi konfigurasi untuk proses pengunduhan kode sumber
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    pub expected_sha256: Option<String>,
    pub expected_blake3: Option<String>,
    pub fallback_mirrors: Vec<String>,
    pub retries: usize,
    pub timeout_secs: u64,
    pub show_progress: bool,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            expected_sha256: None,
            expected_blake3: None,
            fallback_mirrors: Vec::new(),
            retries: 3,
            timeout_secs: 30,
            show_progress: true,
        }
    }
}

/// Hasil dari pengunduhan berkas sumber
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub target_file: PathBuf,
    pub bytes_downloaded: u64,
    pub total_size: u64,
    pub sha256_hash: String,
    pub blake3_hash: String,
    pub resumed_from_bytes: u64,
    pub mirror_used: String,
}

/// Engine pengunduh kode sumber hulu dengan dukungan HTTP Range Resumption & Multi-Mirror Failover
pub struct SourceDownloader;

impl SourceDownloader {
    /// Hitung hash SHA256 dan BLAKE3 dari sebuah berkas di disk
    pub fn calculate_hashes(path: &Path) -> Result<(String, String)> {
        let mut file = File::open(path)
            .with_context(|| format!("Gagal membuka berkas {:?} untuk hashing", path))?;
        let mut sha256_hasher = Sha256::new();
        let mut blake3_hasher = Blake3Hasher::new();

        let mut buffer = [0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            sha256_hasher.update(&buffer[..n]);
            blake3_hasher.update(&buffer[..n]);
        }

        let sha256_str = format!("{:x}", sha256_hasher.finalize());
        let blake3_str = blake3_hasher.finalize().to_hex().to_string();
        Ok((sha256_str, blake3_str))
    }

    /// Unduh berkas secara sinkron (blocking wrapper untuk integrasi di pipeline kompilasi)
    pub fn download(
        primary_url: &str,
        target_path: &Path,
        options: &DownloadOptions,
    ) -> Result<DownloadResult> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Gagal menginisialisasi Tokio runtime untuk downloader")?;

        rt.block_on(Self::download_async(primary_url, target_path, options))
    }

    /// Unduh berkas sumber secara asinkron dengan fitur Resumption & Multi-Mirror Failover (Non-Blocking Async I/O)
    pub async fn download_async(
        primary_url: &str,
        target_path: &Path,
        options: &DownloadOptions,
    ) -> Result<DownloadResult> {
        if let Some(parent) = target_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // 1. Cek apakah berkas final sudah ada di disk dan hash-nya valid
        if target_path.exists() {
            let target_path_clone = target_path.to_path_buf();
            if let Ok(Ok((sha, b3))) = tokio::task::spawn_blocking(move || Self::calculate_hashes(&target_path_clone)).await {
                let sha_valid = match &options.expected_sha256 {
                    Some(expected) => !expected.is_empty() && &sha == expected,
                    None => true,
                };
                let b3_valid = match &options.expected_blake3 {
                    Some(expected) => !expected.is_empty() && &b3 == expected,
                    None => true,
                };

                if sha_valid && b3_valid {
                    let metadata = tokio::fs::metadata(target_path).await?;
                    let total_size = metadata.len();
                    if options.show_progress {
                        println!(
                            "  [✓] Cache valid ditemukan di distfiles: {} ({} bytes, SHA256: {})",
                            target_path.display(),
                            total_size,
                            &sha[..12.min(sha.len())]
                        );
                    }
                    return Ok(DownloadResult {
                        target_file: target_path.to_path_buf(),
                        bytes_downloaded: 0,
                        total_size,
                        sha256_hash: sha,
                        blake3_hash: b3,
                        resumed_from_bytes: total_size,
                        mirror_used: "local-cache".to_string(),
                    });
                }
            }
        }

        // 2. Susun daftar URL kandidat (Primary + Fallbacks)
        let mut candidate_urls = vec![primary_url.to_string()];
        for mirror in &options.fallback_mirrors {
            if !candidate_urls.contains(mirror) {
                candidate_urls.push(mirror.clone());
            }
        }

        let part_file = PathBuf::from(format!("{}.part", target_path.display()));
        let mut last_error = None;

        let client = reqwest::Client::builder()
            .user_agent("Forge-Client/0.1.0 (KuraLinux Distro Engine)")
            .connect_timeout(Duration::from_secs(options.timeout_secs.max(15)))
            .build()?;

        // 3. Iterasi setiap kandidat mirror
        for (mirror_idx, url) in candidate_urls.iter().enumerate() {
            let is_fallback = mirror_idx > 0;
            if is_fallback && options.show_progress {
                println!(
                    "  {} Mengalihkan ke mirror cadangan [{}/{}]: {}",
                    "⚠️".yellow(),
                    mirror_idx + 1,
                    candidate_urls.len(),
                    url.cyan()
                );
            }

            // Retry loop per mirror dengan exponential backoff
            for attempt in 1..=options.retries {
                let existing_len = if part_file.exists() {
                    tokio::fs::metadata(&part_file).await.map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                };

                let mut req = client.get(url);
                if existing_len > 0 {
                    req = req.header("Range", format!("bytes={}-", existing_len));
                    if options.show_progress {
                        println!(
                            "  [↺] Melanjutkan unduhan (Resuming) dari offset: {} bytes...",
                            existing_len
                        );
                    }
                } else if options.show_progress && attempt == 1 && !is_fallback {
                    println!("  [↓] Mengunduh sumber dari: {}", url.dimmed());
                }

                match req.send().await {
                    Ok(response) => {
                        let status = response.status();

                        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
                            // File part mungkin corrupt atau server file berubah, hapus part dan ulangi dari 0
                            let _ = tokio::fs::remove_file(&part_file).await;
                            continue;
                        }

                        if !status.is_success() {
                            last_error = Some(format!(
                                "Server mengembalikan HTTP status {} dari {}",
                                status, url
                            ));
                            if attempt < options.retries {
                                tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                            }
                            continue;
                        }

                        let is_partial = status == reqwest::StatusCode::PARTIAL_CONTENT;
                        let mut file = if is_partial && existing_len > 0 {
                            tokio::fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&part_file)
                                .await?
                        } else {
                            tokio::fs::OpenOptions::new()
                                .create(true)
                                .write(true)
                                .truncate(true)
                                .open(&part_file)
                                .await?
                        };

                        let resumed_offset = if is_partial { existing_len } else { 0 };
                        let mut bytes_downloaded_this_session = 0u64;
                        let mut stream_err = false;
                        let mut response = response;
                        let chunk_idle_timeout = Duration::from_secs(options.timeout_secs.max(30));

                        loop {
                            match tokio::time::timeout(chunk_idle_timeout, response.chunk()).await {
                                Ok(Ok(Some(chunk))) => {
                                    if let Err(e) = file.write_all(&chunk).await {
                                        last_error = Some(format!("Gagal menulis chunk ke part file: {:#}", e));
                                        stream_err = true;
                                        break;
                                    }
                                    bytes_downloaded_this_session += chunk.len() as u64;
                                }
                                Ok(Ok(None)) => {
                                    // Selesai membaca seluruh stream secara utuh
                                    break;
                                }
                                Ok(Err(e)) => {
                                    last_error = Some(format!("Error membaca chunk dari stream: {:#}", e));
                                    stream_err = true;
                                    break;
                                }
                                Err(_) => {
                                    last_error = Some(format!("Timeout: tidak ada data diterima selama {} detik", chunk_idle_timeout.as_secs()));
                                    stream_err = true;
                                    break;
                                }
                            }
                        }

                        if stream_err {
                            if attempt < options.retries {
                                tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                                continue;
                            } else {
                                break;
                            }
                        }

                        file.flush().await?;
                        drop(file);

                        // 4. Validasi Checksum Integritas (Offload ke worker thread)
                        let part_file_clone = part_file.clone();
                        let (calc_sha, calc_b3) = tokio::task::spawn_blocking(move || {
                            Self::calculate_hashes(&part_file_clone)
                        })
                        .await
                        .context("Gagal menjalankan hashing task")??;

                        if let Some(ref expected_sha) = options.expected_sha256 {
                            if !expected_sha.is_empty() && &calc_sha != expected_sha {
                                let _ = tokio::fs::remove_file(&part_file).await;
                                last_error = Some(format!(
                                    "Mismatch checksum SHA256! Expected: {}, Found: {}",
                                    expected_sha, calc_sha
                                ));
                                break;
                            }
                        }

                        if let Some(ref expected_b3) = options.expected_blake3 {
                            if !expected_b3.is_empty() && &calc_b3 != expected_b3 {
                                let _ = tokio::fs::remove_file(&part_file).await;
                                last_error = Some(format!(
                                    "Mismatch checksum BLAKE3! Expected: {}, Found: {}",
                                    expected_b3, calc_b3
                                ));
                                break;
                            }
                        }

                        // Promosikan .part secara atomik ke nama file target final
                        tokio::fs::rename(&part_file, target_path).await?;

                        let final_size = tokio::fs::metadata(target_path).await?.len();
                        if options.show_progress {
                            println!(
                                "  [✓] Unduhan selesai ({:.2} MB, SHA256: {})",
                                final_size as f64 / (1024.0 * 1024.0),
                                &calc_sha[..12.min(calc_sha.len())]
                            );
                        }

                        return Ok(DownloadResult {
                            target_file: target_path.to_path_buf(),
                            bytes_downloaded: bytes_downloaded_this_session,
                            total_size: final_size,
                            sha256_hash: calc_sha,
                            blake3_hash: calc_b3,
                            resumed_from_bytes: resumed_offset,
                            mirror_used: url.clone(),
                        });
                    }
                    Err(e) => {
                        last_error = Some(format!("Gagal menghubungi {}: {:#}", url, e));
                        if attempt < options.retries {
                            tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                        }
                    }
                }
            }
        }

        bail!(
            "Gagal mengunduh sumber dari seluruh kandidat mirror. Detail error terakhir: {}",
            last_error.unwrap_or_else(|| "Unknown download error".to_string())
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_calculate_hashes_accuracy() -> Result<()> {
        let temp = tempdir()?;
        let sample_file = temp.path().join("sample.txt");
        fs::write(&sample_file, b"Hello Forge Downloader!")?;

        let (sha, b3) = SourceDownloader::calculate_hashes(&sample_file)?;
        assert!(!sha.is_empty());
        assert!(!b3.is_empty());
        assert_eq!(sha.len(), 64);
        assert_eq!(b3.len(), 64);

        Ok(())
    }

    #[test]
    fn test_local_cache_detection_avoids_re_download() -> Result<()> {
        let temp = tempdir()?;
        let target_file = temp.path().join("existing.tar.gz");
        fs::write(&target_file, b"sample content for testing cache")?;

        let (sha, b3) = SourceDownloader::calculate_hashes(&target_file)?;
        let options = DownloadOptions {
            expected_sha256: Some(sha.clone()),
            expected_blake3: Some(b3.clone()),
            show_progress: false,
            ..Default::default()
        };

        let res = SourceDownloader::download("http://127.0.0.1:9999/dummy.tar.gz", &target_file, &options)?;
        assert_eq!(res.mirror_used, "local-cache");
        assert_eq!(res.bytes_downloaded, 0);
        assert_eq!(res.sha256_hash, sha);

        Ok(())
    }

    #[tokio::test]
    async fn test_http_range_resumption_with_mock_server() -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let server_url = format!("http://127.0.0.1:{}", addr.port());

        let full_payload = b"Hello, this is a 40-byte test payload!!";

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut req_buf = [0u8; 1024];
                let n = socket.read(&mut req_buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&req_buf[..n]);

                if req_str.to_lowercase().contains("range: bytes=10-") {
                    let body = &full_payload[10..];
                    let response = format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes 10-39/40\r\n\r\n",
                        body.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.write_all(body).await;
                } else {
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                        full_payload.len()
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.write_all(full_payload).await;
                }
            }
        });

        let temp = tempdir()?;
        let target_file = temp.path().join("resumed_file.bin");
        let part_file = temp.path().join("resumed_file.bin.part");

        // Tulis 10 byte pertama sebagai sisa unduhan sebelumnya
        fs::write(&part_file, &full_payload[..10])?;

        let options = DownloadOptions {
            expected_sha256: None,
            expected_blake3: None,
            show_progress: false,
            retries: 1,
            timeout_secs: 5,
            fallback_mirrors: Vec::new(),
        };

        let res = SourceDownloader::download_async(&format!("{}/payload.bin", server_url), &target_file, &options).await?;
        assert_eq!(res.resumed_from_bytes, 10);
        assert_eq!(res.total_size, full_payload.len() as u64);
        assert_eq!(fs::read(&target_file)?, full_payload);

        Ok(())
    }

    #[tokio::test]
    async fn test_multi_mirror_fallback_to_second_mirror() -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let working_mirror_url = format!("http://127.0.0.1:{}/valid.tar.gz", addr.port());

        let payload = b"Valid tarball payload from secondary mirror!";

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut req_buf = [0u8; 1024];
                let _ = socket.read(&mut req_buf).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                    payload.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.write_all(payload).await;
            }
        });

        let temp = tempdir()?;
        let target_file = temp.path().join("downloaded.tar.gz");

        let dead_primary_url = "http://127.0.0.1:1/nonexistent.tar.gz";

        let options = DownloadOptions {
            expected_sha256: None,
            expected_blake3: None,
            fallback_mirrors: vec![working_mirror_url.clone()],
            show_progress: false,
            retries: 1,
            timeout_secs: 2,
        };

        let res = SourceDownloader::download_async(dead_primary_url, &target_file, &options).await?;
        assert_eq!(res.mirror_used, working_mirror_url);
        assert_eq!(res.total_size, payload.len() as u64);
        assert_eq!(fs::read(&target_file)?, payload);

        Ok(())
    }
}
