use anyhow::Result;
use colored::*;
use forge::{BinhostCatalog, BinhostPackageEntry};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::Path;

pub struct ServerIndexer;

impl ServerIndexer {
    /// Regenerasi database index repositori biner (packages.db.zst) dan perbarui seluruh catalog.json di binhost storage
    pub fn regenerate_index(storage_path: &Path) -> Result<usize> {
        println!("{}", "=== Forge Server Binhost Indexer ===".bold().cyan());
        println!(
            "{} Memindai direktori penyimpanan: {}",
            "[*]".blue(),
            storage_path.display().to_string().yellow()
        );

        if !storage_path.exists() {
            fs::create_dir_all(storage_path)?;
        }

        let mut total_packages = 0usize;
        let mut all_catalog_entries: Vec<BinhostPackageEntry> = Vec::new();

        if let Ok(entries) = fs::read_dir(storage_path) {
            for entry in entries.flatten() {
                let march_dir = entry.path();
                if march_dir.is_dir() {
                    let march_name = march_dir
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default();
                    if march_name.starts_with('.') {
                        continue;
                    }

                    println!(
                        "  [🔍] Memindai mikroarsitektur: {}",
                        march_name.bold().green()
                    );
                    let mut march_packages = Vec::new();

                    if let Ok(files) = fs::read_dir(&march_dir) {
                        for file_entry in files.flatten() {
                            let file_path = file_entry.path();
                            let file_name = file_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or_default();

                            if file_name.ends_with(".forge.tar.zst") {
                                if let Ok(bytes) = fs::read(&file_path) {
                                    let sha256_hash = format!("{:x}", Sha256::digest(&bytes));
                                    let size_bytes = bytes.len() as u64;

                                    let meta = crate::ServerImporter::extract_metadata_from_tarball(
                                        &file_path,
                                    )
                                    .ok()
                                    .flatten();
                                    let (pkgname, pkgver, pkgrel, slot) = if let Some(m) = meta {
                                        (m.name, m.version, m.release, m.slot)
                                    } else {
                                        let clean = file_name
                                            .strip_suffix(".forge.tar.zst")
                                            .unwrap_or(file_name);
                                        let parts: Vec<&str> = clean.split('-').collect();
                                        if parts.len() >= 2 {
                                            (
                                                parts[0].to_string(),
                                                parts[1].to_string(),
                                                1,
                                                "0".to_string(),
                                            )
                                        } else {
                                            (
                                                clean.to_string(),
                                                "1.0.0".to_string(),
                                                1,
                                                "0".to_string(),
                                            )
                                        }
                                    };

                                    let pkg_entry = BinhostPackageEntry {
                                        pkgname,
                                        pkgver,
                                        pkgrel,
                                        slot,
                                        target_march: march_name.to_string(),
                                        active_use: Vec::new(),
                                        sha256: sha256_hash,
                                        size_bytes,
                                        download_url: file_name.to_string(),
                                    };

                                    march_packages.push(pkg_entry.clone());
                                    all_catalog_entries.push(pkg_entry);
                                    total_packages += 1;
                                }
                            }
                        }
                    }

                    // Tulis / update catalog.json untuk march tersebut
                    let catalog_path = march_dir.join("catalog.json");
                    let catalog = BinhostCatalog {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        server_version: "0.1.0".to_string(),
                        packages: march_packages,
                    };
                    let catalog_json = serde_json::to_string_pretty(&catalog)?;
                    fs::write(&catalog_path, catalog_json)?;
                    println!(
                        "    [✓] catalog.json untuk {} berhasil diperbarui",
                        march_name
                    );
                }
            }
        }

        // Buat global packages.db.zst di storage_path
        let global_db_path = storage_path.join("packages.db.zst");
        let all_db_json = serde_json::to_string_pretty(&all_catalog_entries)?;
        let file = File::create(&global_db_path)?;
        let mut encoder = zstd::Encoder::new(file, 3)?;
        std::io::Write::write_all(&mut encoder, all_db_json.as_bytes())?;
        let mut finished = encoder.finish()?;
        std::io::Write::flush(&mut finished)?;
        drop(finished);

        println!(
            "{} Indexing selesai! Total {} paket terindeks. Database global: {}",
            "✓".green(),
            total_packages,
            global_db_path.display().to_string().bold().green()
        );

        Ok(total_packages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_indexer_empty_and_populated() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let binhost_dir = temp.path().join("binhost");
        fs::create_dir_all(&binhost_dir)?;

        let count = ServerIndexer::regenerate_index(&binhost_dir)?;
        assert_eq!(count, 0);
        assert!(binhost_dir.join("packages.db.zst").exists());

        Ok(())
    }
}
