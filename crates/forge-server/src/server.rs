use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    routing::get,
    Router,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServerState {
    pub recipes_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub binhost_dir: PathBuf,
}

pub struct ForgeServer;

impl ForgeServer {
    /// Inisialisasi router HTTP daemon
    pub fn router(state: Arc<ServerState>) -> Router {
        Router::new()
            .route("/v1/health", get(health_handler))
            .route("/v1/recipes/latest.sha256", get(recipes_hash_handler))
            .route("/v1/recipes/latest.tar.zst", get(recipes_tarball_handler))
            .route(
                "/v1/binhost/{march}/catalog.json",
                get(binhost_catalog_handler),
            )
            .route("/v1/binhost/{march}/{package}", get(binhost_package_handler))
            .with_state(state)
    }

    /// Kemas direktori recipes/ menjadi recipes.tar.zst dan simpan hash SHA256-nya
    pub fn bundle_recipes(recipes_dir: &Path, output_tar_zst: &Path) -> anyhow::Result<String> {
        if let Some(parent) = output_tar_zst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(output_tar_zst)?;
        let encoder = zstd::Encoder::new(file, 3)?;
        let mut tar_builder = tar::Builder::new(encoder);
        tar_builder.append_dir_all(".", recipes_dir)?;
        let encoder = tar_builder.into_inner()?;
        let mut file = encoder.finish()?;
        std::io::Write::flush(&mut file)?;
        drop(file);

        let bytes = std::fs::read(output_tar_zst)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let hash_file = format!("{}.sha256", output_tar_zst.display());
        std::fs::write(&hash_file, &hash)?;
        Ok(hash)
    }
}

async fn health_handler() -> &'static str {
    "OK - Forge Central Server Active"
}

async fn recipes_hash_handler(State(state): State<Arc<ServerState>>) -> Result<String, StatusCode> {
    let hash_file = state.cache_dir.join("recipes.tar.zst.sha256");
    std::fs::read_to_string(hash_file)
        .map(|s| s.trim().to_string())
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn recipes_tarball_handler(
    State(state): State<Arc<ServerState>>,
) -> Result<Vec<u8>, StatusCode> {
    let tar_file = state.cache_dir.join("recipes.tar.zst");
    std::fs::read(tar_file).map_err(|_| StatusCode::NOT_FOUND)
}

async fn binhost_catalog_handler(
    State(state): State<Arc<ServerState>>,
    AxumPath(march): AxumPath<String>,
) -> Result<String, StatusCode> {
    if march.contains("..") || march.contains('/') || march.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }
    let catalog_file = state.binhost_dir.join(&march).join("catalog.json");
    std::fs::read_to_string(catalog_file).map_err(|_| StatusCode::NOT_FOUND)
}

async fn binhost_package_handler(
    State(state): State<Arc<ServerState>>,
    AxumPath((march, package)): AxumPath<(String, String)>,
) -> Result<Vec<u8>, StatusCode> {
    if march.contains("..")
        || march.contains('/')
        || march.contains('\\')
        || package.contains("..")
        || package.contains('/')
        || package.contains('\\')
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let package_file = state.binhost_dir.join(&march).join(&package);
    std::fs::read(package_file).map_err(|_| StatusCode::NOT_FOUND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge::SyncClient;

    #[test]
    fn test_bundle_recipes_and_hash_generation() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");
        let base_recipe_dir = recipes_dir.join("system").join("base");
        std::fs::create_dir_all(&base_recipe_dir)?;

        let recipe_content = r#"
[package]
name = "base"
version = "1.0.0"
release = 1
slot = "0"
description = "Kura Linux Base Meta Package"
"#;
        std::fs::write(base_recipe_dir.join("recipe.toml"), recipe_content)?;

        let output_tar_zst = temp.path().join("cache").join("recipes.tar.zst");
        let hash = ForgeServer::bundle_recipes(&recipes_dir, &output_tar_zst)?;

        assert!(output_tar_zst.exists());
        let hash_file = PathBuf::from(format!("{}.sha256", output_tar_zst.display()));
        assert!(hash_file.exists());

        let saved_hash = std::fs::read_to_string(hash_file)?.trim().to_string();
        assert_eq!(hash, saved_hash);

        let bytes = std::fs::read(&output_tar_zst)?;
        let calculated_hash = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(hash, calculated_hash);
        assert!(!hash.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_server_health_and_endpoints() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");
        let base_recipe_dir = recipes_dir.join("system").join("base");
        std::fs::create_dir_all(&base_recipe_dir)?;
        std::fs::write(
            base_recipe_dir.join("recipe.toml"),
            "[package]\nname = \"base\"\nversion = \"1.0.0\"",
        )?;

        let cache_dir = temp.path().join("cache");
        let binhost_dir = temp.path().join("binhost");
        let tar_file = cache_dir.join("recipes.tar.zst");
        let bundle_hash = ForgeServer::bundle_recipes(&recipes_dir, &tar_file)?;

        let state = Arc::new(ServerState {
            recipes_dir: recipes_dir.clone(),
            cache_dir: cache_dir.clone(),
            binhost_dir: binhost_dir.clone(),
        });

        let router = ForgeServer::router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let client = reqwest::Client::new();

        // 1. Health check
        let health_resp = client
            .get(format!("http://{}/v1/health", addr))
            .send()
            .await?;
        assert_eq!(health_resp.status(), reqwest::StatusCode::OK);
        let health_text = health_resp.text().await?;
        assert!(health_text.contains("OK"));

        // 2. Hash endpoint
        let hash_resp = client
            .get(format!("http://{}/v1/recipes/latest.sha256", addr))
            .send()
            .await?;
        assert_eq!(hash_resp.status(), reqwest::StatusCode::OK);
        let hash_text = hash_resp.text().await?;
        assert_eq!(hash_text.trim(), bundle_hash);

        // 3. Tarball endpoint
        let tar_resp = client
            .get(format!("http://{}/v1/recipes/latest.tar.zst", addr))
            .send()
            .await?;
        assert_eq!(tar_resp.status(), reqwest::StatusCode::OK);
        let tar_bytes = tar_resp.bytes().await?;
        let fetched_hash = format!("{:x}", Sha256::digest(&tar_bytes));
        assert_eq!(fetched_hash, bundle_hash);

        Ok(())
    }

    #[tokio::test]
    async fn test_binhost_catalog_and_package_endpoints() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");
        let cache_dir = temp.path().join("cache");
        let binhost_dir = temp.path().join("binhost");

        let znver4_dir = binhost_dir.join("znver4");
        std::fs::create_dir_all(&znver4_dir)?;

        let catalog_sample = r#"{
  "timestamp": 1726700000,
  "server_version": "0.1.0",
  "packages": [
    {
      "pkgname": "base",
      "pkgver": "1.0.0",
      "pkgrel": 1,
      "slot": "0",
      "target_march": "znver4",
      "active_use": [],
      "sha256": "abcdef1234567890",
      "size_bytes": 1024,
      "download_url": "base.forge.tar.zst"
    }
  ]
}"#;
        std::fs::write(znver4_dir.join("catalog.json"), catalog_sample)?;

        let fake_package_bytes = b"kura-linux-package-binary-data";
        std::fs::write(
            znver4_dir.join("base.forge.tar.zst"),
            fake_package_bytes,
        )?;

        let state = Arc::new(ServerState {
            recipes_dir,
            cache_dir,
            binhost_dir,
        });

        let router = ForgeServer::router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let client = reqwest::Client::new();

        // 1. Valid catalog endpoint
        let catalog_resp = client
            .get(format!("http://{}/v1/binhost/znver4/catalog.json", addr))
            .send()
            .await?;
        assert_eq!(catalog_resp.status(), reqwest::StatusCode::OK);
        let catalog_text = catalog_resp.text().await?;
        assert!(catalog_text.contains("znver4"));
        assert!(catalog_text.contains("base"));

        // 2. Valid package binary endpoint
        let pkg_resp = client
            .get(format!("http://{}/v1/binhost/znver4/base.forge.tar.zst", addr))
            .send()
            .await?;
        assert_eq!(pkg_resp.status(), reqwest::StatusCode::OK);
        let pkg_bytes = pkg_resp.bytes().await?;
        assert_eq!(&pkg_bytes[..], fake_package_bytes);

        // 3. Nonexistent catalog endpoint
        let non_cat_resp = client
            .get(format!("http://{}/v1/binhost/intel_core/catalog.json", addr))
            .send()
            .await?;
        assert_eq!(non_cat_resp.status(), reqwest::StatusCode::NOT_FOUND);

        // 4. Nonexistent package endpoint
        let non_pkg_resp = client
            .get(format!(
                "http://{}/v1/binhost/znver4/nonexistent.forge.tar.zst",
                addr
            ))
            .send()
            .await?;
        assert_eq!(non_pkg_resp.status(), reqwest::StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_recipes_client_full_cycle() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let server_recipes_dir = temp.path().join("server_recipes");
        let base_recipe_dir = server_recipes_dir.join("system").join("base");
        std::fs::create_dir_all(&base_recipe_dir)?;
        let original_recipe_content =
            "[package]\nname = \"base\"\nversion = \"1.0.0\"\nrelease = 1\n";
        std::fs::write(
            base_recipe_dir.join("recipe.toml"),
            original_recipe_content,
        )?;

        let server_cache_dir = temp.path().join("server_cache");
        let server_binhost_dir = temp.path().join("server_binhost");
        let tar_file = server_cache_dir.join("recipes.tar.zst");
        let bundle_hash = ForgeServer::bundle_recipes(&server_recipes_dir, &tar_file)?;

        let state = Arc::new(ServerState {
            recipes_dir: server_recipes_dir,
            cache_dir: server_cache_dir,
            binhost_dir: server_binhost_dir,
        });

        let router = ForgeServer::router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let client_recipes_dir = temp.path().join("client_recipes");
        let client_cache_dir = temp.path().join("client_cache");

        let server_url = format!("http://{}/v1", addr);
        let updated =
            SyncClient::sync_recipes(&server_url, &client_recipes_dir, &client_cache_dir).await?;
        assert!(updated, "Harusnya mengembalikan true saat sinkronisasi pertama");

        let synced_recipe = client_recipes_dir
            .join("system")
            .join("base")
            .join("recipe.toml");
        assert!(
            synced_recipe.exists(),
            "Berkas recipe.toml harus ada setelah disinkronkan"
        );

        let content = std::fs::read_to_string(synced_recipe)?;
        assert_eq!(content, original_recipe_content);

        let synced_hash_file = client_recipes_dir.join(".synced_hash");
        assert!(synced_hash_file.exists(), ".synced_hash harus dibuat");
        let saved_hash = std::fs::read_to_string(synced_hash_file)?
            .trim()
            .to_string();
        assert_eq!(saved_hash, bundle_hash);

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_noop_when_up_to_date() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let server_recipes_dir = temp.path().join("server_recipes");
        let base_recipe_dir = server_recipes_dir.join("system").join("base");
        std::fs::create_dir_all(&base_recipe_dir)?;
        std::fs::write(
            base_recipe_dir.join("recipe.toml"),
            "[package]\nname = \"base\"\nversion = \"1.0.0\"\n",
        )?;

        let server_cache_dir = temp.path().join("server_cache");
        let server_binhost_dir = temp.path().join("server_binhost");
        let tar_file = server_cache_dir.join("recipes.tar.zst");
        ForgeServer::bundle_recipes(&server_recipes_dir, &tar_file)?;

        let state = Arc::new(ServerState {
            recipes_dir: server_recipes_dir,
            cache_dir: server_cache_dir,
            binhost_dir: server_binhost_dir,
        });

        let router = ForgeServer::router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let client_recipes_dir = temp.path().join("client_recipes");
        let client_cache_dir = temp.path().join("client_cache");
        let server_url = format!("http://{}/v1", addr);

        // First sync -> true
        let first_sync =
            SyncClient::sync_recipes(&server_url, &client_recipes_dir, &client_cache_dir).await?;
        assert!(first_sync);

        // Second sync without server change -> false (noop)
        let second_sync =
            SyncClient::sync_recipes(&server_url, &client_recipes_dir, &client_cache_dir).await?;
        assert!(
            !second_sync,
            "Harusnya mengembalikan false karena hash sama (noop)"
        );

        Ok(())
    }
}
