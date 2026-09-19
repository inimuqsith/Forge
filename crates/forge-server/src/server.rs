use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServerState {
    pub recipes_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub binhost_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub release: u32,
    pub slot: String,
    pub category: String,
    pub description: String,
    pub license: String,
    pub upstream: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncWebhookResponse {
    pub status: String,
    pub message: String,
    pub package_count: usize,
    pub sha256: Option<String>,
    pub git_updated: bool,
}

pub struct ForgeServer;

impl ForgeServer {
    /// Inisialisasi router HTTP daemon
    pub fn router(state: Arc<ServerState>) -> Router {
        Router::new()
            .route("/", get(index_html_handler))
            .route("/v1/packages", get(packages_json_handler))
            .route("/v1/health", get(health_handler))
            .route("/v1/recipes/latest.sha256", get(recipes_hash_handler))
            .route("/v1/recipes/latest.tar.zst", get(recipes_tarball_handler))
            .route(
                "/v1/binhost/{march}/catalog.json",
                get(binhost_catalog_handler),
            )
            .route("/v1/binhost/{march}/{package}", get(binhost_package_handler))
            .route(
                "/v1/webhook/github",
                post(github_webhook_handler).get(github_webhook_info_handler),
            )
            .route(
                "/v1/recipes/refresh",
                post(recipes_refresh_handler).get(recipes_refresh_handler),
            )
            .with_state(state)
    }

    /// Pindai seluruh resep paket dari direktori recipes
    pub fn scan_packages(recipes_dir: &Path) -> Vec<PackageInfo> {
        let mut packages = Vec::new();
        if !recipes_dir.exists() {
            return packages;
        }

        let mut categories = Vec::new();
        if let Ok(entries) = std::fs::read_dir(recipes_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    categories.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        categories.sort();

        for cat in categories {
            let cat_dir = recipes_dir.join(&cat);
            if let Ok(pkg_entries) = std::fs::read_dir(&cat_dir) {
                let mut pkg_entries: Vec<_> = pkg_entries.flatten().collect();
                pkg_entries.sort_by_key(|e| e.file_name());

                for entry in pkg_entries {
                    let pkg_dir = entry.path();
                    if pkg_dir.is_dir() {
                        let recipe_file = pkg_dir.join("recipe.toml");
                        if recipe_file.is_file() {
                            if let Ok(content) = std::fs::read_to_string(&recipe_file) {
                                if let Ok(recipe) = toml::from_str::<forge::Recipe>(&content) {
                                    packages.push(PackageInfo {
                                        name: recipe.package.name,
                                        version: recipe.package.version,
                                        release: recipe.package.release,
                                        slot: recipe.package.slot,
                                        category: cat.clone(),
                                        description: recipe.package.description,
                                        license: recipe.package.license,
                                        upstream: recipe.package.upstream,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        packages.sort_by(|a, b| a.name.cmp(&b.name));
        packages
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

async fn index_html_handler(State(state): State<Arc<ServerState>>) -> Html<String> {
    let packages = ForgeServer::scan_packages(&state.recipes_dir);
    let html = render_dashboard_html(&packages);
    Html(html)
}

async fn packages_json_handler(
    State(state): State<Arc<ServerState>>,
) -> Json<Vec<PackageInfo>> {
    let recipes_dir = state.recipes_dir.clone();
    let packages = tokio::task::spawn_blocking(move || ForgeServer::scan_packages(&recipes_dir))
        .await
        .unwrap_or_default();
    Json(packages)
}

async fn recipes_hash_handler(State(state): State<Arc<ServerState>>) -> Result<String, StatusCode> {
    let hash_file = state.cache_dir.join("recipes.tar.zst.sha256");
    tokio::fs::read_to_string(hash_file)
        .await
        .map(|s| s.trim().to_string())
        .map_err(|_| StatusCode::NOT_FOUND)
}

async fn recipes_tarball_handler(
    State(state): State<Arc<ServerState>>,
) -> Result<Vec<u8>, StatusCode> {
    let tar_file = state.cache_dir.join("recipes.tar.zst");
    tokio::fs::read(tar_file).await.map_err(|_| StatusCode::NOT_FOUND)
}

async fn binhost_catalog_handler(
    State(state): State<Arc<ServerState>>,
    AxumPath(march): AxumPath<String>,
) -> Result<String, StatusCode> {
    if march.contains("..") || march.contains('/') || march.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }
    let catalog_file = state.binhost_dir.join(&march).join("catalog.json");
    tokio::fs::read_to_string(catalog_file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
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
    tokio::fs::read(package_file)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

/// Sinkronisasi resep dari Git repo jika tersedia dan bungkus ulang ke recipes.tar.zst
pub fn sync_and_rebundle_recipes(recipes_dir: &Path, cache_dir: &Path) -> anyhow::Result<(usize, String, bool)> {
    let mut git_updated = false;
    let git_dir_candidate1 = recipes_dir.join(".git");
    let git_dir_candidate2 = recipes_dir.parent().map(|p| p.join(".git"));

    let target_git_dir = if git_dir_candidate1.exists() {
        Some(recipes_dir)
    } else if let Some(ref p2) = git_dir_candidate2 {
        if p2.exists() {
            recipes_dir.parent()
        } else {
            None
        }
    } else {
        None
    };

    if let Some(target_git) = target_git_dir {
        let output = std::process::Command::new("git")
            .args(["-C", &target_git.to_string_lossy(), "pull", "--rebase"])
            .output();
        if let Ok(out) = output {
            git_updated = out.status.success();
        }
    }

    let tar_file = cache_dir.join("recipes.tar.zst");
    let hash = ForgeServer::bundle_recipes(recipes_dir, &tar_file)?;
    let packages = ForgeServer::scan_packages(recipes_dir);
    Ok((packages.len(), hash, git_updated))
}

async fn handle_sync_rebundle(
    state: Arc<ServerState>,
    success_message: &'static str,
    action_desc: &'static str,
) -> (StatusCode, Json<SyncWebhookResponse>) {
    let recipes_dir = state.recipes_dir.clone();
    let cache_dir = state.cache_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        sync_and_rebundle_recipes(&recipes_dir, &cache_dir)
    })
    .await;

    match result {
        Ok(Ok((count, hash, git_updated))) => (
            StatusCode::OK,
            Json(SyncWebhookResponse {
                status: "ok".to_string(),
                message: success_message.to_string(),
                package_count: count,
                sha256: Some(hash),
                git_updated,
            }),
        ),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SyncWebhookResponse {
                status: "error".to_string(),
                message: format!("Failed to {}: {:#}", action_desc, e),
                package_count: 0,
                sha256: None,
                git_updated: false,
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(SyncWebhookResponse {
                status: "error".to_string(),
                message: format!("Task execution error: {:#}", e),
                package_count: 0,
                sha256: None,
                git_updated: false,
            }),
        ),
    }
}

async fn github_webhook_handler(
    State(state): State<Arc<ServerState>>,
) -> (StatusCode, Json<SyncWebhookResponse>) {
    handle_sync_rebundle(
        state,
        "Recipes successfully synchronized and rebundled from GitHub webhook",
        "synchronize recipes",
    )
    .await
}

async fn github_webhook_info_handler(
    State(state): State<Arc<ServerState>>,
) -> Json<SyncWebhookResponse> {
    let recipes_dir = state.recipes_dir.clone();
    let packages = tokio::task::spawn_blocking(move || ForgeServer::scan_packages(&recipes_dir))
        .await
        .unwrap_or_default();
    let hash_file = state.cache_dir.join("recipes.tar.zst.sha256");
    let sha256 = tokio::fs::read_to_string(hash_file)
        .await
        .ok()
        .map(|s| s.trim().to_string());
    Json(SyncWebhookResponse {
        status: "ready".to_string(),
        message: "Forge GitHub Webhook Receiver is active. Send POST requests from GitHub Webhooks (push event) to trigger automatic recipe rebundle.".to_string(),
        package_count: packages.len(),
        sha256,
        git_updated: false,
    })
}

async fn recipes_refresh_handler(
    State(state): State<Arc<ServerState>>,
) -> (StatusCode, Json<SyncWebhookResponse>) {
    handle_sync_rebundle(
        state,
        "Recipes successfully refreshed and rebundled",
        "refresh recipes",
    )
    .await
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_dashboard_html(packages: &[PackageInfo]) -> String {
    let total_count = packages.len();
    let system_count = packages.iter().filter(|p| p.category == "system").count();
    let core_count = packages.iter().filter(|p| p.category == "core").count();
    let extra_count = packages.iter().filter(|p| p.category == "extra").count();

    let mut pkg_cards_html = String::new();
    for pkg in packages {
        let escaped_name = escape_html(&pkg.name);
        let escaped_version = escape_html(&pkg.version);
        let escaped_category = escape_html(&pkg.category);
        let escaped_desc = if pkg.description.is_empty() {
            "Tidak ada deskripsi tersedia.".to_string()
        } else {
            escape_html(&pkg.description)
        };
        let escaped_slot = escape_html(&pkg.slot);
        let escaped_license = escape_html(&pkg.license);
        let escaped_upstream = escape_html(&pkg.upstream);

        let upstream_btn = if !pkg.upstream.is_empty() {
            format!(
                r#"<a href="{}" target="_blank" rel="noopener noreferrer" class="btn btn-sm btn-outline">🌐 Upstream</a>"#,
                escaped_upstream
            )
        } else {
            String::new()
        };

        let license_meta = if !pkg.license.is_empty() {
            format!(
                r#"<span class="meta-item"><span class="meta-label">License:</span> <span class="meta-val">{}</span></span>"#,
                escaped_license
            )
        } else {
            String::new()
        };

        pkg_cards_html.push_str(&format!(
            r#"<div class="pkg-card" data-name="{name_lower}" data-category="{cat_lower}" data-desc="{desc_lower}">
  <div class="pkg-card-top">
    <span class="tag tag-{cat_lower}">{category}</span>
    <span class="version-badge">v{version}-r{release}</span>
  </div>
  <h3 class="pkg-title">{name}</h3>
  <p class="pkg-description">{description}</p>
  <div class="pkg-meta-row">
    <span class="meta-item"><span class="meta-label">Slot:</span> <span class="meta-val">{slot}</span></span>
    {license_meta}
  </div>
  <div class="pkg-actions">
    <button class="btn btn-sm btn-copy" onclick="copyInstallCmd('{name}')" title="Salin perintah instalasi">📋 forge install {name}</button>
    {upstream_btn}
  </div>
</div>"#,
            name_lower = escaped_name.to_lowercase(),
            cat_lower = escaped_category.to_lowercase(),
            desc_lower = escaped_desc.to_lowercase(),
            category = escaped_category,
            version = escaped_version,
            release = pkg.release,
            name = escaped_name,
            description = escaped_desc,
            slot = escaped_slot,
            license_meta = license_meta,
            upstream_btn = upstream_btn,
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Kura Linux Package &amp; Binhost Repository</title>
  <style>
    :root {{
      --bg-base: #0a0e17;
      --bg-surface: rgba(20, 29, 47, 0.7);
      --bg-card: rgba(26, 38, 62, 0.65);
      --bg-card-hover: rgba(33, 49, 80, 0.85);
      --border-subtle: rgba(255, 255, 255, 0.08);
      --border-focus: rgba(56, 189, 248, 0.5);
      --text-main: #f8fafc;
      --text-muted: #94a3b8;
      --text-dim: #64748b;
      --cyan: #38bdf8;
      --cyan-glow: rgba(56, 189, 248, 0.25);
      --emerald: #10b981;
      --emerald-glow: rgba(16, 185, 129, 0.25);
      --rose: #f43f5e;
      --amber: #f59e0b;
      --purple: #a855f7;
      --font-mono: 'JetBrains Mono', 'Fira Code', ui-monospace, SFMono-Regular, monospace;
    }}

    * {{
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }}

    body {{
      background: radial-gradient(ellipse at 50% 0%, #172554 0%, #0c1527 40%, var(--bg-base) 80%);
      color: var(--text-main);
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      min-height: 100vh;
      line-height: 1.5;
      -webkit-font-smoothing: antialiased;
    }}

    .container {{
      max-width: 1320px;
      margin: 0 auto;
      padding: 2.5rem 1.5rem 4rem;
    }}

    /* Header & Hero */
    header {{
      text-align: center;
      margin-bottom: 2.5rem;
    }}

    .brand-container {{
      display: inline-flex;
      align-items: center;
      gap: 1rem;
      margin-bottom: 0.75rem;
    }}

    .brand-logo {{
      width: 48px;
      height: 48px;
      background: linear-gradient(135deg, #0284c7, #38bdf8);
      border-radius: 14px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 0 24px var(--cyan-glow);
    }}

    .brand-logo svg {{
      width: 28px;
      height: 28px;
      fill: #ffffff;
    }}

    h1 {{
      font-size: 2.25rem;
      font-weight: 800;
      letter-spacing: -0.025em;
      background: linear-gradient(to right, #ffffff, #e2e8f0, #38bdf8);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
    }}

    .subtitle {{
      color: var(--text-muted);
      font-size: 1.05rem;
      max-width: 760px;
      margin: 0 auto 1.5rem;
    }}

    /* Badges Bar */
    .badges-bar {{
      display: flex;
      flex-wrap: wrap;
      justify-content: center;
      gap: 0.6rem;
      margin-bottom: 1.5rem;
    }}

    .badge {{
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      padding: 0.35rem 0.8rem;
      border-radius: 9999px;
      font-size: 0.82rem;
      font-weight: 600;
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      backdrop-filter: blur(8px);
    }}

    .badge-online {{
      color: #34d399;
      border-color: rgba(16, 185, 129, 0.3);
      background: rgba(16, 185, 129, 0.1);
    }}

    .pulse-dot {{
      width: 8px;
      height: 8px;
      background-color: var(--emerald);
      border-radius: 50%;
      box-shadow: 0 0 8px var(--emerald);
      animation: pulse 2s infinite;
    }}

    @keyframes pulse {{
      0% {{ opacity: 1; transform: scale(1); }}
      50% {{ opacity: 0.4; transform: scale(0.85); }}
      100% {{ opacity: 1; transform: scale(1); }}
    }}

    .badge-blue {{ color: #60a5fa; border-color: rgba(96, 165, 250, 0.3); background: rgba(96, 165, 250, 0.1); }}
    .badge-purple {{ color: #c084fc; border-color: rgba(192, 132, 252, 0.3); background: rgba(192, 132, 252, 0.1); }}
    .badge-amber {{ color: #fbbf24; border-color: rgba(251, 191, 36, 0.3); background: rgba(251, 191, 36, 0.1); }}
    .badge-cyan {{ color: #38bdf8; border-color: rgba(56, 189, 248, 0.3); background: rgba(56, 189, 248, 0.1); }}

    /* Quick Links */
    .quick-links {{
      display: flex;
      flex-wrap: wrap;
      justify-content: center;
      gap: 0.75rem;
      margin-top: 1rem;
    }}

    .api-link {{
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      color: var(--cyan);
      text-decoration: none;
      font-size: 0.85rem;
      font-weight: 500;
      padding: 0.3rem 0.75rem;
      border-radius: 6px;
      background: rgba(56, 189, 248, 0.08);
      border: 1px solid rgba(56, 189, 248, 0.2);
      transition: all 0.2s ease;
    }}

    .api-link:hover {{
      background: rgba(56, 189, 248, 0.18);
      border-color: rgba(56, 189, 248, 0.4);
      transform: translateY(-1px);
    }}

    /* Grid layout sections */
    .section-title {{
      font-size: 1.25rem;
      font-weight: 700;
      color: var(--text-main);
      margin-bottom: 1rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }}

    /* CLI Quickstart Card */
    .cli-guide-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
      gap: 1rem;
      margin-bottom: 2.5rem;
    }}

    .cli-card {{
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      padding: 1rem 1.2rem;
      backdrop-filter: blur(12px);
      display: flex;
      flex-direction: column;
      justify-content: space-between;
    }}

    .cli-header {{
      font-size: 0.85rem;
      color: var(--text-muted);
      margin-bottom: 0.5rem;
      font-weight: 500;
    }}

    .code-box {{
      background: rgba(10, 14, 23, 0.85);
      border: 1px solid rgba(255, 255, 255, 0.06);
      border-radius: 8px;
      padding: 0.5rem 0.75rem;
      display: flex;
      align-items: center;
      justify-content: space-between;
      font-family: var(--font-mono);
      font-size: 0.85rem;
      color: var(--cyan);
    }}

    .copy-icon-btn {{
      background: transparent;
      border: none;
      color: var(--text-muted);
      cursor: pointer;
      font-size: 0.8rem;
      padding: 0.2rem 0.4rem;
      border-radius: 4px;
      transition: all 0.15s;
    }}

    .copy-icon-btn:hover {{
      color: #ffffff;
      background: rgba(255, 255, 255, 0.1);
    }}

    /* Binhost Section */
    .binhost-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
      gap: 1rem;
      margin-bottom: 2.5rem;
    }}

    .binhost-card {{
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      padding: 1.25rem;
      backdrop-filter: blur(12px);
    }}

    .binhost-card-title {{
      font-size: 1.05rem;
      font-weight: 700;
      color: #ffffff;
      margin-bottom: 0.35rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }}

    .binhost-card-desc {{
      font-size: 0.85rem;
      color: var(--text-muted);
      margin-bottom: 0.75rem;
    }}

    .binhost-flags {{
      font-family: var(--font-mono);
      font-size: 0.78rem;
      color: #cbd5e1;
      background: rgba(0, 0, 0, 0.3);
      padding: 0.4rem 0.6rem;
      border-radius: 6px;
      border: 1px solid rgba(255, 255, 255, 0.05);
      margin-bottom: 0.75rem;
      word-break: break-all;
    }}

    /* Search & Filter Bar */
    .explorer-toolbar {{
      background: var(--bg-surface);
      border: 1px solid var(--border-subtle);
      border-radius: 14px;
      padding: 1.25rem;
      margin-bottom: 1.5rem;
      backdrop-filter: blur(12px);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.25);
    }}

    .search-input-wrapper {{
      position: relative;
      margin-bottom: 1rem;
    }}

    .search-input {{
      width: 100%;
      background: rgba(10, 14, 23, 0.85);
      border: 1px solid var(--border-subtle);
      border-radius: 10px;
      padding: 0.85rem 1rem 0.85rem 2.75rem;
      font-size: 1rem;
      color: #ffffff;
      outline: none;
      transition: all 0.2s ease;
    }}

    .search-input:focus {{
      border-color: var(--cyan);
      box-shadow: 0 0 0 3px var(--cyan-glow);
    }}

    .search-icon {{
      position: absolute;
      left: 1rem;
      top: 50%;
      transform: translateY(-50%);
      color: var(--text-muted);
      pointer-events: none;
    }}

    .filter-row {{
      display: flex;
      flex-wrap: wrap;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
    }}

    .tabs-group {{
      display: flex;
      gap: 0.4rem;
      flex-wrap: wrap;
    }}

    .tab-btn {{
      background: rgba(255, 255, 255, 0.04);
      border: 1px solid var(--border-subtle);
      color: var(--text-muted);
      padding: 0.45rem 0.9rem;
      border-radius: 8px;
      font-size: 0.85rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s ease;
    }}

    .tab-btn:hover {{
      background: rgba(255, 255, 255, 0.08);
      color: #ffffff;
    }}

    .tab-btn.active {{
      background: var(--cyan);
      color: #04101e;
      border-color: var(--cyan);
      box-shadow: 0 0 12px var(--cyan-glow);
    }}

    .results-count {{
      font-size: 0.88rem;
      color: var(--text-muted);
    }}

    /* Package Cards Grid */
    .packages-grid {{
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
      gap: 1.25rem;
    }}

    .pkg-card {{
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      padding: 1.25rem;
      backdrop-filter: blur(12px);
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      transition: all 0.2s ease;
    }}

    .pkg-card:hover {{
      background: var(--bg-card-hover);
      border-color: rgba(56, 189, 248, 0.3);
      transform: translateY(-2px);
      box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    }}

    .pkg-card-top {{
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.6rem;
    }}

    .tag {{
      padding: 0.2rem 0.55rem;
      border-radius: 6px;
      font-size: 0.72rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }}

    .tag-system {{
      background: rgba(244, 63, 94, 0.15);
      color: #fb7185;
      border: 1px solid rgba(244, 63, 94, 0.3);
    }}

    .tag-core {{
      background: rgba(56, 189, 248, 0.15);
      color: #38bdf8;
      border: 1px solid rgba(56, 189, 248, 0.3);
    }}

    .tag-extra {{
      background: rgba(16, 185, 129, 0.15);
      color: #34d399;
      border: 1px solid rgba(16, 185, 129, 0.3);
    }}

    .version-badge {{
      font-family: var(--font-mono);
      font-size: 0.78rem;
      color: #cbd5e1;
      background: rgba(255, 255, 255, 0.06);
      padding: 0.18rem 0.5rem;
      border-radius: 6px;
    }}

    .pkg-title {{
      font-size: 1.2rem;
      font-weight: 700;
      color: #ffffff;
      margin-bottom: 0.4rem;
    }}

    .pkg-description {{
      font-size: 0.88rem;
      color: var(--text-muted);
      margin-bottom: 1rem;
      line-height: 1.4;
      flex-grow: 1;
    }}

    .pkg-meta-row {{
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      font-size: 0.78rem;
      color: var(--text-dim);
      padding-top: 0.75rem;
      border-top: 1px solid rgba(255, 255, 255, 0.05);
      margin-bottom: 0.75rem;
    }}

    .meta-item {{
      display: inline-flex;
      gap: 0.25rem;
    }}

    .meta-label {{
      color: var(--text-dim);
    }}

    .meta-val {{
      color: var(--text-muted);
      font-weight: 500;
    }}

    .pkg-actions {{
      display: flex;
      gap: 0.5rem;
    }}

    .btn {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.35rem;
      padding: 0.45rem 0.8rem;
      border-radius: 8px;
      font-size: 0.8rem;
      font-weight: 600;
      cursor: pointer;
      text-decoration: none;
      transition: all 0.15s ease;
      border: 1px solid transparent;
    }}

    .btn-sm {{
      padding: 0.35rem 0.65rem;
      font-size: 0.78rem;
    }}

    .btn-copy {{
      background: rgba(56, 189, 248, 0.12);
      color: var(--cyan);
      border-color: rgba(56, 189, 248, 0.25);
      font-family: var(--font-mono);
      flex: 1;
    }}

    .btn-copy:hover {{
      background: rgba(56, 189, 248, 0.22);
      border-color: rgba(56, 189, 248, 0.4);
    }}

    .btn-outline {{
      background: rgba(255, 255, 255, 0.04);
      color: var(--text-muted);
      border-color: var(--border-subtle);
    }}

    .btn-outline:hover {{
      background: rgba(255, 255, 255, 0.08);
      color: #ffffff;
    }}

    /* No results */
    .no-results {{
      grid-column: 1 / -1;
      text-align: center;
      padding: 4rem 1rem;
      color: var(--text-muted);
      background: var(--bg-surface);
      border-radius: 12px;
      border: 1px dashed var(--border-subtle);
      display: none;
    }}

    .no-results h4 {{
      font-size: 1.15rem;
      color: #ffffff;
      margin-bottom: 0.5rem;
    }}

    /* Toast Notification */
    .toast {{
      position: fixed;
      bottom: 2rem;
      right: 2rem;
      background: #0f172a;
      color: #ffffff;
      border: 1px solid var(--cyan);
      box-shadow: 0 0 20px var(--cyan-glow);
      padding: 0.75rem 1.25rem;
      border-radius: 10px;
      font-size: 0.88rem;
      font-weight: 600;
      display: flex;
      align-items: center;
      gap: 0.5rem;
      opacity: 0;
      transform: translateY(12px);
      transition: all 0.25s ease;
      pointer-events: none;
      z-index: 1000;
    }}

    .toast.show {{
      opacity: 1;
      transform: translateY(0);
    }}

    /* Footer */
    footer {{
      margin-top: 4rem;
      text-align: center;
      font-size: 0.85rem;
      color: var(--text-dim);
      border-top: 1px solid var(--border-subtle);
      padding-top: 2rem;
    }}

    footer a {{
      color: var(--cyan);
      text-decoration: none;
    }}

    footer a:hover {{
      text-decoration: underline;
    }}
  </style>
</head>
<body>
  <div class="container">
    <header>
      <div class="brand-container">
        <div class="brand-logo">
          <svg viewBox="0 0 24 24">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>
        <h1>Kura Linux Package &amp; Binhost Repository</h1>
      </div>
      <p class="subtitle">High-Performance Source-First &amp; Silicon-Native Binary Package Explorer untuk Kura Linux</p>

      <div class="badges-bar">
        <span class="badge badge-online"><span class="pulse-dot"></span> Server Online</span>
        <span class="badge badge-blue">📦 <span id="total-badge-count">{total_count}</span> Resep Paket</span>
        <span class="badge badge-purple">⚡ AMD Zen 4 (znver4)</span>
        <span class="badge badge-amber">⚡ Generic (x86-64-v3)</span>
        <span class="badge badge-cyan">🔒 HTTP/2 TLS 1.3</span>
      </div>

      <div class="quick-links">
        <a href="/v1/recipes/latest.tar.zst" class="api-link">📦 Unduh Resep (.tar.zst)</a>
        <a href="/v1/recipes/latest.sha256" class="api-link">🔑 Resep SHA256</a>
        <a href="/v1/packages" class="api-link">📡 REST API (/v1/packages)</a>
        <a href="/v1/health" class="api-link">💚 Server Health</a>
      </div>
    </header>

    <!-- Quickstart CLI Guide -->
    <section>
      <h2 class="section-title">🚀 Panduan Cepat Forge CLI</h2>
      <div class="cli-guide-grid">
        <div class="cli-card">
          <div class="cli-header">1. Inisialisasi &amp; Konfigurasi Toolchain</div>
          <div class="code-box">
            <span>forge setup</span>
            <button class="copy-icon-btn" onclick="copyText('forge setup')" title="Salin">📋</button>
          </div>
        </div>
        <div class="cli-card">
          <div class="cli-header">2. Sinkronisasi Resep Paket Terbaru</div>
          <div class="code-box">
            <span>forge sync</span>
            <button class="copy-icon-btn" onclick="copyText('forge sync')" title="Salin">📋</button>
          </div>
        </div>
        <div class="cli-card">
          <div class="cli-header">3. Deteksi Profil Arsitektur CPU Silikon</div>
          <div class="code-box">
            <span>forge cpu-dump</span>
            <button class="copy-icon-btn" onclick="copyText('forge cpu-dump')" title="Salin">📋</button>
          </div>
        </div>
        <div class="cli-card">
          <div class="cli-header">4. Instalasi Biner Silikon Teroptimasi</div>
          <div class="code-box">
            <span>forge install --binhost curl</span>
            <button class="copy-icon-btn" onclick="copyText('forge install --binhost curl')" title="Salin">📋</button>
          </div>
        </div>
      </div>
    </section>

    <!-- Silicon Binhost Section -->
    <section>
      <h2 class="section-title">⚡ Silicon Binhost &amp; Repositori Biner</h2>
      <div class="binhost-grid">
        <div class="binhost-card">
          <div class="binhost-card-title">🚀 AMD Zen 4 (znver4)</div>
          <div class="binhost-card-desc">Paket biner native teroptimasi instruksi AVX-512, BMI2, FMA, dan tuning cache L1/L2 untuk AMD Ryzen 7000/8000/9000 &amp; EPYC.</div>
          <div class="binhost-flags">-march=znver4 -O3 -mavx512f -mavx512dq -mavx512cd -mavx512bw -mavx512vl</div>
          <a href="/v1/binhost/znver4/catalog.json" class="api-link">Lihat Katalog znver4</a>
        </div>
        <div class="binhost-card">
          <div class="binhost-card-title">💻 Generic Modern (x86-64-v3)</div>
          <div class="binhost-card-desc">Paket biner universal untuk prosesor Intel Haswell+ dan AMD Excavator+ dengan instruksi AVX2, FMA, BMI1/2.</div>
          <div class="binhost-flags">-march=x86-64-v3 -O3 -mavx2 -mfma -mbmi -mbmi2</div>
          <a href="/v1/binhost/x86-64-v3/catalog.json" class="api-link">Lihat Katalog x86-64-v3</a>
        </div>
        <div class="binhost-card">
          <div class="binhost-card-title">🛠️ Source-First Engine</div>
          <div class="binhost-card-desc">Resep kompilasi modular dengan kustomisasi USE flags dan profil perangkat keras lokal.</div>
          <div class="binhost-flags">{total_count} Source Recipes • Bundled as recipes.tar.zst</div>
          <a href="/v1/recipes/latest.tar.zst" class="api-link">Unduh Resep Tarball</a>
        </div>
      </div>
    </section>

    <!-- Package Explorer -->
    <section>
      <h2 class="section-title">📦 Katalog Resep Paket</h2>

      <div class="explorer-toolbar">
        <div class="search-input-wrapper">
          <span class="search-icon">🔍</span>
          <input type="text" id="pkg-search" class="search-input" placeholder="Cari nama paket, deskripsi, atau kategori (tekan '/' untuk mencari)..." autofocus autocomplete="off">
        </div>

        <div class="filter-row">
          <div class="tabs-group">
            <button class="tab-btn active" data-cat="all">Semua ({total_count})</button>
            <button class="tab-btn" data-cat="system">System ({system_count})</button>
            <button class="tab-btn" data-cat="core">Core ({core_count})</button>
            <button class="tab-btn" data-cat="extra">Extra ({extra_count})</button>
          </div>
          <div class="results-count">
            Menampilkan <strong id="shown-count" style="color: var(--cyan);">{total_count}</strong> dari {total_count} paket
          </div>
        </div>
      </div>

      <div class="packages-grid" id="packages-grid">
        {pkg_cards_html}
        <div class="no-results" id="no-results">
          <h4>Tidak ada paket yang ditemukan</h4>
          <p>Coba kata kunci pencarian yang lain atau pilih kategori "Semua".</p>
        </div>
      </div>
    </section>

    <footer>
      <p>Kura Linux Forge Package Manager &amp; Binhost Registry • <a href="https://kuralinux.org">kuralinux.org</a></p>
      <p style="margin-top: 0.35rem; color: var(--text-dim);">Ultra-fast native daemon built with Rust &amp; Axum • Response time &lt; 2ms</p>
    </footer>
  </div>

  <div id="toast" class="toast">
    <span>✓</span>
    <span id="toast-text">Tersalin ke clipboard!</span>
  </div>

  <script>
    const searchInput = document.getElementById('pkg-search');
    const tabBtns = document.querySelectorAll('.tab-btn');
    const cards = document.querySelectorAll('.pkg-card');
    const shownCountEl = document.getElementById('shown-count');
    const noResultsEl = document.getElementById('no-results');
    const toast = document.getElementById('toast');
    const toastText = document.getElementById('toast-text');

    let currentCategory = 'all';
    let searchQuery = '';

    function filterPackages() {{
      let visible = 0;
      const q = searchQuery.toLowerCase().trim();

      cards.forEach(card => {{
        const cat = card.getAttribute('data-category');
        const name = card.getAttribute('data-name');
        const desc = card.getAttribute('data-desc');

        const matchesCat = (currentCategory === 'all' || cat === currentCategory);
        const matchesQuery = (q === '' || name.includes(q) || desc.includes(q) || cat.includes(q));

        if (matchesCat && matchesQuery) {{
          card.style.display = 'flex';
          visible++;
        }} else {{
          card.style.display = 'none';
        }}
      }});

      shownCountEl.textContent = visible;
      noResultsEl.style.display = visible === 0 ? 'block' : 'none';
    }}

    searchInput.addEventListener('input', (e) => {{
      searchQuery = e.target.value;
      filterPackages();
    }});

    tabBtns.forEach(btn => {{
      btn.addEventListener('click', () => {{
        tabBtns.forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        currentCategory = btn.getAttribute('data-cat');
        filterPackages();
      }});
    }});

    window.addEventListener('keydown', (e) => {{
      if (e.key === '/' && document.activeElement !== searchInput) {{
        e.preventDefault();
        searchInput.focus();
      }} else if (e.key === 'Escape' && document.activeElement === searchInput) {{
        searchInput.value = '';
        searchQuery = '';
        filterPackages();
        searchInput.blur();
      }}
    }});

    function showToast(msg) {{
      toastText.textContent = msg;
      toast.classList.add('show');
      setTimeout(() => {{
        toast.classList.remove('show');
      }}, 2200);
    }}

    function copyText(text) {{
      navigator.clipboard.writeText(text).then(() => {{
        showToast('Perintah "' + text + '" berhasil disalin!');
      }}).catch(() => {{
        showToast('Gagal menyalin perintah');
      }});
    }}

    function copyInstallCmd(pkgName) {{
      copyText('forge install ' + pkgName);
    }}
  </script>
</body>
</html>
"#,
        total_count = total_count,
        system_count = system_count,
        core_count = core_count,
        extra_count = extra_count,
        pkg_cards_html = pkg_cards_html,
    )
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

    #[test]
    fn test_scan_packages_parses_all_fields() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");

        let sys_base = recipes_dir.join("system").join("base");
        std::fs::create_dir_all(&sys_base)?;
        std::fs::write(
            sys_base.join("recipe.toml"),
            r#"
[package]
name = "base"
version = "1.0.0"
release = 1
slot = "0"
description = "Kura Linux Base"
license = "GPL-3.0"
upstream = "https://kuralinux.org"
"#,
        )?;

        let core_curl = recipes_dir.join("core").join("curl");
        std::fs::create_dir_all(&core_curl)?;
        std::fs::write(
            core_curl.join("recipe.toml"),
            r#"
[package]
name = "curl"
version = "8.12.1"
release = 2
slot = "0"
description = "Command line tool for URLs"
license = "curl"
upstream = "https://curl.se"
"#,
        )?;

        let packages = ForgeServer::scan_packages(&recipes_dir);
        assert_eq!(packages.len(), 2);

        assert_eq!(packages[0].name, "base");
        assert_eq!(packages[0].version, "1.0.0");
        assert_eq!(packages[0].release, 1);
        assert_eq!(packages[0].category, "system");
        assert_eq!(packages[0].license, "GPL-3.0");

        assert_eq!(packages[1].name, "curl");
        assert_eq!(packages[1].version, "8.12.1");
        assert_eq!(packages[1].release, 2);
        assert_eq!(packages[1].category, "core");
        assert_eq!(packages[1].upstream, "https://curl.se");

        Ok(())
    }

    #[tokio::test]
    async fn test_index_html_endpoint_returns_html_and_contains_packages() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");
        let core_bash = recipes_dir.join("core").join("bash");
        std::fs::create_dir_all(&core_bash)?;
        std::fs::write(
            core_bash.join("recipe.toml"),
            r#"
[package]
name = "bash"
version = "5.2.37"
release = 1
description = "GNU Bourne Again Shell"
license = "GPL-3.0-or-later"
"#,
        )?;

        let cache_dir = temp.path().join("cache");
        let binhost_dir = temp.path().join("binhost");
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
        let resp = client.get(format!("http://{}/", addr)).send().await?;

        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let headers = resp.headers();
        let content_type = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(content_type.contains("text/html"));

        let body = resp.text().await?;
        assert!(body.contains("Kura Linux Package &amp; Binhost Repository"));
        assert!(body.contains("bash"));
        assert!(body.contains("5.2.37"));
        assert!(body.contains("GNU Bourne Again Shell"));
        assert!(body.contains("znver4"));
        assert!(body.contains("x86-64-v3"));
        assert!(body.contains("forge sync"));

        Ok(())
    }

    #[tokio::test]
    async fn test_api_v1_packages_json_endpoint() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let recipes_dir = temp.path().join("recipes");
        let core_sed = recipes_dir.join("core").join("sed");
        std::fs::create_dir_all(&core_sed)?;
        std::fs::write(
            core_sed.join("recipe.toml"),
            r#"
[package]
name = "sed"
version = "4.9"
release = 1
description = "GNU stream editor"
license = "GPL-3.0"
"#,
        )?;

        let cache_dir = temp.path().join("cache");
        let binhost_dir = temp.path().join("binhost");
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
        let resp = client
            .get(format!("http://{}/v1/packages", addr))
            .send()
            .await?;

        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(content_type.contains("application/json"));

        let text = resp.text().await?;
        let packages: Vec<PackageInfo> = serde_json::from_str(&text)?;
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "sed");
        assert_eq!(packages[0].version, "4.9");
        assert_eq!(packages[0].category, "core");
        assert_eq!(packages[0].description, "GNU stream editor");

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

    #[test]
    fn test_scan_actual_workspace_recipes() {
        let scanner_roots = [
            Path::new("recipes"),
            Path::new("../recipes"),
            Path::new("../../recipes"),
        ];

        let found_root = scanner_roots.iter().find(|p| p.is_dir());
        if let Some(root) = found_root {
            let pkgs = ForgeServer::scan_packages(root);
            assert!(!pkgs.is_empty(), "Katalog resep tidak boleh kosong");

            // Hitung secara dinamis jumlah recipe.toml riil di filesystem
            let mut recipe_file_count = 0;
            for category in &["system", "core", "extra"] {
                let cat_dir = root.join(category);
                if cat_dir.is_dir() {
                    for entry in std::fs::read_dir(&cat_dir).unwrap().flatten() {
                        if entry.path().join("recipe.toml").is_file() {
                            recipe_file_count += 1;
                        }
                    }
                }
            }

            assert_eq!(
                pkgs.len(),
                recipe_file_count,
                "Jumlah resep ter-scan ({}) harus cocok dengan jumlah file recipe.toml riil ({})",
                pkgs.len(),
                recipe_file_count
            );
            assert!(
                pkgs.iter().all(|p| !p.name.is_empty() && !p.version.is_empty() && !p.category.is_empty()),
                "Seluruh metadata paket harus valid dan ter-parse lengkap"
            );
        }
    }

    #[tokio::test]
    async fn test_github_webhook_endpoint_triggers_rebundle() -> anyhow::Result<()> {
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

        let state = Arc::new(ServerState {
            recipes_dir: server_recipes_dir.clone(),
            cache_dir: server_cache_dir.clone(),
            binhost_dir: server_binhost_dir,
        });

        let router = ForgeServer::router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });

        let client = reqwest::Client::new();
        let webhook_url = format!("http://{}/v1/webhook/github", addr);
        let resp = client.post(&webhook_url)
            .header("Content-Type", "application/json")
            .body(r#"{"action": "push", "ref": "refs/heads/main"}"#)
            .send()
            .await?;

        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let text = resp.text().await?;
        let json_body: SyncWebhookResponse = serde_json::from_str(&text)?;
        assert_eq!(json_body.status, "ok");
        assert_eq!(json_body.package_count, 1);
        assert!(json_body.sha256.is_some());

        // Verifikasi file tar.zst dan sha256 benar-benar terbuat
        let tar_file = server_cache_dir.join("recipes.tar.zst");
        assert!(tar_file.exists());
        let hash_file = server_cache_dir.join("recipes.tar.zst.sha256");
        assert!(hash_file.exists());

        Ok(())
    }

    #[tokio::test]
    async fn test_recipes_refresh_endpoint() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let server_recipes_dir = temp.path().join("server_recipes");
        let extra_recipe_dir = server_recipes_dir.join("extra").join("htop");
        std::fs::create_dir_all(&extra_recipe_dir)?;
        std::fs::write(
            extra_recipe_dir.join("recipe.toml"),
            "[package]\nname = \"htop\"\nversion = \"3.3.0\"\n",
        )?;

        let server_cache_dir = temp.path().join("server_cache");
        let server_binhost_dir = temp.path().join("server_binhost");

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

        let client = reqwest::Client::new();
        let refresh_url = format!("http://{}/v1/recipes/refresh", addr);
        let resp = client.post(&refresh_url).send().await?;

        assert_eq!(resp.status(), reqwest::StatusCode::OK);
        let text = resp.text().await?;
        let json_body: SyncWebhookResponse = serde_json::from_str(&text)?;
        assert_eq!(json_body.status, "ok");
        assert_eq!(json_body.package_count, 1);

        Ok(())
    }
}
