use serde::{Deserialize, Serialize};

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
}
