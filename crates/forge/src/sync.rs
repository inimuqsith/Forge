use anyhow::{Context, Result};
use colored::*;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct SyncClient;

impl SyncClient {
    /// Eksekusi sinkronisasi pohon resep dari server ke target_recipes_dir (/var/db/forge/recipes/)
    pub async fn sync_recipes(
        server_url: &str,
        target_recipes_dir: &Path,
        cache_dir: &Path,
    ) -> Result<bool> {
        let base_url = server_url.trim_end_matches('/');
        println!("  [*] Menghubungi Forge Server di: {}", base_url.cyan());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .context("Gagal menginisialisasi HTTP client")?;

        // 1. Handshake Hash Cepat
        let hash_url = format!("{}/recipes/latest.sha256", base_url);
        let hash_resp = client
            .get(&hash_url)
            .send()
            .await
            .with_context(|| format!("Gagal menghubungi server di {}", hash_url))?;

        if !hash_resp.status().is_success() {
            anyhow::bail!("Server merespon dengan status error: {}", hash_resp.status());
        }

        let remote_hash = hash_resp.text().await?.trim().to_string();
        if remote_hash.is_empty() {
            anyhow::bail!("Server mengembalikan checksum SHA256 kosong.");
        }

        let hash_file = target_recipes_dir.join(".synced_hash");

        if hash_file.exists() {
            if let Ok(local_hash) = std::fs::read_to_string(&hash_file) {
                if local_hash.trim() == remote_hash {
                    println!("  [✓] Pohon resep lokal sudah merupakan versi terkini.");
                    return Ok(false);
                }
            }
        }

        // 2. Download Tarball Zstandard
        let tarball_url = format!("{}/recipes/latest.tar.zst", base_url);
        println!("  [↓] Mengunduh arsip resep: {}", tarball_url);
        let tar_resp = client
            .get(&tarball_url)
            .send()
            .await
            .with_context(|| format!("Gagal mengunduh arsip resep dari {}", tarball_url))?;

        if !tar_resp.status().is_success() {
            anyhow::bail!("Gagal mengunduh arsip resep, status error: {}", tar_resp.status());
        }

        let archive_bytes = tar_resp.bytes().await?;

        // 3. Validasi Hash SHA256
        let calculated_hash = format!("{:x}", Sha256::digest(&archive_bytes));
        if calculated_hash != remote_hash {
            anyhow::bail!(
                "Checksum mismatch! Expected: {}, Found: {}",
                remote_hash,
                calculated_hash
            );
        }

        // 4. Ekstraksi Atomik Zstandard
        if let Err(e) = std::fs::create_dir_all(cache_dir) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let fallback_cache = std::env::temp_dir().join("forge").join("cache").join("sync");
                let fallback_recipes = if std::fs::create_dir_all(target_recipes_dir).is_err() {
                    std::env::temp_dir().join("forge").join("recipes")
                } else {
                    target_recipes_dir.to_path_buf()
                };
                println!("  [!] Izin sistem terbatas di {}. Mengalihkan cache & sync ke: {}", cache_dir.display(), fallback_recipes.display());
                return Box::pin(Self::sync_recipes(server_url, &fallback_recipes, &fallback_cache)).await;
            }
            return Err(e).context(format!("Gagal membuat direktori cache: {}", cache_dir.display()));
        }

        let temp_archive = cache_dir.join("recipes_sync.tar.zst");
        std::fs::write(&temp_archive, &archive_bytes)
            .with_context(|| format!("Gagal menyimpan arsip sementara ke {}", temp_archive.display()))?;

        let decoder = zstd::Decoder::new(std::fs::File::open(&temp_archive)?)
            .context("Gagal menginisialisasi Zstandard decoder")?;
        let mut archive = tar::Archive::new(decoder);
        let staging_extract = cache_dir.join("recipes_staging");
        let _ = std::fs::remove_dir_all(&staging_extract);
        std::fs::create_dir_all(&staging_extract)?;
        archive
            .unpack(&staging_extract)
            .context("Gagal mengekstrak tarball resep ke staging")?;

        // 5. Pindahkan ke direktori resep resmi
        if let Err(e) = std::fs::create_dir_all(target_recipes_dir) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                let fallback_recipes = std::env::temp_dir().join("forge").join("recipes");
                println!("  [!] Izin sistem terbatas di {}. Mengalihkan sync resep ke: {}", target_recipes_dir.display(), fallback_recipes.display());
                return Box::pin(Self::sync_recipes(server_url, &fallback_recipes, cache_dir)).await;
            }
            return Err(e).context(format!("Gagal membuat target direktori resep: {}", target_recipes_dir.display()));
        }

        for entry in std::fs::read_dir(&staging_extract)? {
            let entry = entry?;
            let src_path = entry.path();
            let dest_path = target_recipes_dir.join(entry.file_name());

            if dest_path.exists() {
                if dest_path.is_dir() {
                    let _ = std::fs::remove_dir_all(&dest_path);
                } else {
                    let _ = std::fs::remove_file(&dest_path);
                }
            }

            // Coba rename atomik, jika gagal (misal cross-device) fallback ke recursive copy
            if std::fs::rename(&src_path, &dest_path).is_err() {
                if src_path.is_dir() {
                    copy_dir_recursive(&src_path, &dest_path)?;
                    let _ = std::fs::remove_dir_all(&src_path);
                } else {
                    std::fs::copy(&src_path, &dest_path)?;
                    let _ = std::fs::remove_file(&src_path);
                }
            }
        }

        std::fs::write(&hash_file, &remote_hash)?;
        let _ = std::fs::remove_dir_all(&staging_extract);
        let _ = std::fs::remove_file(&temp_archive);

        println!(
            "  [✓] Pohon resep berhasil disinkronkan ke {}",
            target_recipes_dir.display()
        );
        Ok(true)
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_dir_recursive() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");

        let sub = src.join("system").join("base");
        std::fs::create_dir_all(&sub)?;
        std::fs::write(sub.join("recipe.toml"), "name = \"base\"")?;

        copy_dir_recursive(&src, &dst)?;

        let copied_file = dst.join("system").join("base").join("recipe.toml");
        assert!(copied_file.exists());
        let content = std::fs::read_to_string(copied_file)?;
        assert_eq!(content, "name = \"base\"");

        Ok(())
    }
}
