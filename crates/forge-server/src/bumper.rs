use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpstreamVersionCheck {
    pub name: String,
    pub category: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub has_update: bool,
    pub source_url: Option<String>,
    pub new_sha256: Option<String>,
    pub provider: String,
}

pub struct RecipeBumper;

impl RecipeBumper {
    /// Periksa versi hulu (upstream) untuk satu resep paket
    pub async fn check_upstream(recipe: &forge::Recipe, category: &str) -> Result<UpstreamVersionCheck> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .user_agent("Forge-Server-Bumper/0.1.0 (KuraLinux Distro Engine)")
            .build()?;

        let mut latest_version = None;
        let mut provider = "Unknown".to_string();

        // 1. Coba deteksi repository GitHub dari upstream atau sources
        let mut github_repo = extract_github_repo(&recipe.package.upstream);
        if github_repo.is_none() {
            if let Some(sources) = &recipe.sources {
                for url in &sources.urls {
                    if let Some(repo) = extract_github_repo(url) {
                        github_repo = Some(repo);
                        break;
                    }
                }
            }
        }

        if let Some((owner, repo_name)) = github_repo {
            if let Ok(Some(ver)) = check_github_latest(&client, &owner, &repo_name, &recipe.package.name).await {
                latest_version = Some(ver);
                provider = format!("GitHub ({}/{})", owner, repo_name);
            }
        }

        // 2. Fallback: Coba periksa database terbuka Anitya / Release-Monitoring.org
        if latest_version.is_none() {
            if let Ok(Some(ver)) = check_anitya_latest(&client, &recipe.package.name, Some(&recipe.package.upstream)).await {
                latest_version = Some(ver);
                provider = "Anitya (Release-Monitoring.org)".to_string();
            }
        }

        let has_update = if let Some(ref latest) = latest_version {
            compare_versions(&recipe.package.version, latest) == Ordering::Less
        } else {
            false
        };

        Ok(UpstreamVersionCheck {
            name: recipe.package.name.clone(),
            category: category.to_string(),
            current_version: recipe.package.version.clone(),
            latest_version,
            has_update,
            source_url: recipe.sources.as_ref().and_then(|s| s.urls.first()).cloned(),
            new_sha256: None,
            provider,
        })
    }

    /// Audit seluruh resep di direktori recipes/ secara paralel (ultra-fast)
    pub async fn audit_all(recipes_dir: &Path) -> Result<Vec<UpstreamVersionCheck>> {
        let recipes_dir_buf = recipes_dir.to_path_buf();
        let loaded_recipes = tokio::task::spawn_blocking(move || {
            let mut list = Vec::new();
            let categories = ["system", "core", "extra"];

            for cat in &categories {
                let cat_dir = recipes_dir_buf.join(cat);
                if !cat_dir.exists() {
                    continue;
                }

                if let Ok(entries) = std::fs::read_dir(&cat_dir) {
                    for entry in entries.flatten() {
                        let pkg_dir = entry.path();
                        let recipe_file = pkg_dir.join("recipe.toml");
                        if recipe_file.is_file() {
                            if let Ok(content) = std::fs::read_to_string(&recipe_file) {
                                if let Ok(recipe) = toml::from_str::<forge::Recipe>(&content) {
                                    list.push((recipe, cat.to_string()));
                                }
                            }
                        }
                    }
                }
            }
            list
        })
        .await
        .context("Gagal membaca daftar resep dari disk")?;

        let mut tasks = Vec::new();
        for (recipe, cat_str) in loaded_recipes {
            tasks.push(tokio::spawn(async move {
                match Self::check_upstream(&recipe, &cat_str).await {
                    Ok(check) => check,
                    Err(_) => UpstreamVersionCheck {
                        name: recipe.package.name,
                        category: cat_str,
                        current_version: recipe.package.version,
                        latest_version: None,
                        has_update: false,
                        source_url: None,
                        new_sha256: None,
                        provider: "Unreachable".to_string(),
                    },
                }
            }));
        }

        let mut results = Vec::new();
        for task in tasks {
            if let Ok(check) = task.await {
                results.push(check);
            }
        }

        results.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(results)
    }

    /// Perbarui berkas recipe.toml ke versi baru secara atomik
    pub async fn bump_recipe_file(
        recipe_path: &Path,
        new_version: &str,
        fetch_sha: bool,
    ) -> Result<(String, Option<String>)> {
        let content = tokio::fs::read_to_string(recipe_path)
            .await
            .with_context(|| format!("Gagal membaca recipe di {:?}", recipe_path))?;

        let recipe: forge::Recipe = toml::from_str(&content)
            .with_context(|| format!("Format recipe tidak valid di {:?}", recipe_path))?;

        let old_version = recipe.package.version.clone();
        let mut new_sha256 = None;

        // 1. Jika diminta fetch_sha, unduh tarball baru dan hitung hash SHA256
        if fetch_sha {
            if let Some(sources) = &recipe.sources {
                if let Some(first_url) = sources.urls.first() {
                    let new_url = first_url
                        .replace("${pkgver}", new_version)
                        .replace(&old_version, new_version);

                    let client = reqwest::Client::builder()
                        .timeout(std::time::Duration::from_secs(30))
                        .user_agent("Forge-Server-Bumper/0.1.0")
                        .build()?;

                    if let Ok(resp) = client.get(&new_url).send().await {
                        if resp.status().is_success() {
                            if let Ok(bytes) = resp.bytes().await {
                                let hash = format!("{:x}", Sha256::digest(&bytes));
                                new_sha256 = Some(hash);
                            }
                        }
                    }
                }
            }
        }

        // 2. Modifikasi teks recipe.toml dengan preservasi format
        let mut modified_lines = Vec::new();
        let mut in_package_section = false;
        let mut in_sources_section = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[package]" {
                in_package_section = true;
                in_sources_section = false;
                modified_lines.push(line.to_string());
                continue;
            } else if trimmed.starts_with('[') {
                if trimmed == "[sources]" {
                    in_sources_section = true;
                } else {
                    in_sources_section = false;
                }
                in_package_section = false;
                modified_lines.push(line.to_string());
                continue;
            }

            if in_package_section {
                if trimmed.starts_with("version") {
                    modified_lines.push(format!("version = \"{}\"", new_version));
                    continue;
                }
                if trimmed.starts_with("release") {
                    modified_lines.push("release = 1".to_string());
                    continue;
                }
            }

            if in_sources_section {
                let mut current_line = line.to_string();
                if current_line.contains(&old_version) {
                    current_line = current_line.replace(&old_version, new_version);
                }
                if let Some(ref sha) = new_sha256 {
                    if trimmed.starts_with("sha256") {
                        modified_lines.push(format!("sha256 = [\"{}\"]", sha));
                        continue;
                    }
                }
                modified_lines.push(current_line);
                continue;
            }

            modified_lines.push(line.to_string());
        }

        let new_content = modified_lines.join("\n") + "\n";
        tokio::fs::write(recipe_path, new_content).await?;

        Ok((old_version, new_sha256))
    }

    /// Cari dan perbarui resep paket berdasarkan nama
    pub async fn bump_package_by_name(
        recipes_dir: &Path,
        pkg_name: &str,
        target_version: Option<&str>,
        fetch_sha: bool,
    ) -> Result<String> {
        let recipe_path = find_recipe_file(recipes_dir, pkg_name)
            .with_context(|| format!("Resep paket '{}' tidak ditemukan di {:?}", pkg_name, recipes_dir))?;

        let content = tokio::fs::read_to_string(&recipe_path).await?;
        let recipe: forge::Recipe = toml::from_str(&content)?;

        let version_to_set = if let Some(ver) = target_version {
            ver.to_string()
        } else {
            let check = Self::check_upstream(&recipe, "extra").await?;
            if let Some(latest) = check.latest_version {
                latest
            } else {
                anyhow::bail!("Tidak dapat menemukan versi hulu terbaru untuk paket '{}'", pkg_name);
            }
        };

        let (old_ver, sha) = Self::bump_recipe_file(&recipe_path, &version_to_set, fetch_sha).await?;
        let sha_info = sha.map(|s| format!(" (SHA256: {})", s)).unwrap_or_default();
        Ok(format!("Paket '{}' berhasil di-bump dari v{} -> v{}{}", pkg_name, old_ver, version_to_set, sha_info))
    }
}

/// Cari lokasi berkas recipe.toml paket
fn find_recipe_file(recipes_dir: &Path, pkg_name: &str) -> Option<PathBuf> {
    for cat in &["system", "core", "extra"] {
        let candidate = recipes_dir.join(cat).join(pkg_name).join("recipe.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Ekstraksi owner dan repo name dari URL GitHub
pub fn extract_github_repo(url: &str) -> Option<(String, String)> {
    if !url.contains("github.com") {
        return None;
    }
    let parts: Vec<&str> = url.split("github.com/").collect();
    if parts.len() < 2 {
        return None;
    }
    let path = parts[1].trim_start_matches('/');
    let segments: Vec<&str> = path.split('/').collect();
    if segments.len() >= 2 {
        let owner = segments[0].to_string();
        let repo = segments[1]
            .trim_end_matches(".git")
            .trim_end_matches('/')
            .to_string();
        if !owner.is_empty() && !repo.is_empty() {
            return Some((owner, repo));
        }
    }
    None
}

/// Bersihkan tag release versi GitHub menjadi format semver standar
pub fn clean_version_tag(tag: &str, pkg_name: &str) -> String {
    let mut cleaned = tag.trim().to_string();

    if cleaned.eq_ignore_ascii_case("nightly")
        || cleaned.eq_ignore_ascii_case("latest")
        || cleaned.eq_ignore_ascii_case("master")
        || cleaned.eq_ignore_ascii_case("main")
    {
        return "".to_string();
    }

    let prefixes = [
        format!("{}-v", pkg_name),
        format!("{}_v", pkg_name),
        format!("{}-", pkg_name),
        format!("{}_", pkg_name),
        "llvmorg-".to_string(),
        "openssl-".to_string(),
        "release-".to_string(),
        "release_".to_string(),
        "v.".to_string(),
        "v".to_string(),
        "V".to_string(),
    ];

    for prefix in &prefixes {
        if cleaned.starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].to_string();
            break;
        }
    }

    let mut trimmed = cleaned.trim_start_matches('.').to_string();
    if trimmed.ends_with("-release") {
        trimmed = trimmed[..trimmed.len() - "-release".len()].to_string();
    } else if trimmed.ends_with("_release") {
        trimmed = trimmed[..trimmed.len() - "_release".len()].to_string();
    }

    // Versi valid harus diawali dengan angka (digit)
    if let Some(first_char) = trimmed.chars().next() {
        if !first_char.is_ascii_digit() {
            return "".to_string();
        }
    } else {
        return "".to_string();
    }

    trimmed
}

/// Bandingkan dua versi secara natural
pub fn compare_versions(v1: &str, v2: &str) -> Ordering {
    let parts1: Vec<&str> = v1.split(|c: char| c == '.' || c == '-' || c == '_').collect();
    let parts2: Vec<&str> = v2.split(|c: char| c == '.' || c == '-' || c == '_').collect();

    let max_len = std::cmp::max(parts1.len(), parts2.len());

    for i in 0..max_len {
        let p1 = parts1.get(i).copied().unwrap_or("0");
        let p2 = parts2.get(i).copied().unwrap_or("0");

        let n1 = p1.parse::<u64>();
        let n2 = p2.parse::<u64>();

        match (n1, n2) {
            (Ok(num1), Ok(num2)) => {
                if num1 != num2 {
                    return num1.cmp(&num2);
                }
            }
            _ => {
                if p1 != p2 {
                    return p1.cmp(p2);
                }
            }
        }
    }

    Ordering::Equal
}

/// Ekstraksi tag rilis dari GitHub Atom Feed (bebas rate-limit)
pub fn extract_tag_from_atom_feed(xml: &str) -> Option<String> {
    if let Some(entry_start) = xml.find("<entry>") {
        let entry_content = &xml[entry_start..];
        // 1. Coba pola link /releases/tag/
        if let Some(link_pos) = entry_content.find("/releases/tag/") {
            let after_tag = &entry_content[link_pos + "/releases/tag/".len()..];
            if let Some(end_pos) = after_tag.find('"') {
                let tag = &after_tag[..end_pos];
                if !tag.is_empty() {
                    return Some(tag.to_string());
                }
            }
        }
        // 2. Coba pola <title> di dalam <entry>
        if let Some(title_start) = entry_content.find("<title>") {
            let after_title = &entry_content[title_start + "<title>".len()..];
            if let Some(title_end) = after_title.find("</title>") {
                let title = &after_title[..title_end];
                let trimmed = title.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
    }
    None
}

/// Cek rilis terbaru dari GitHub API dengan fallback Atom Feed (Zero Rate Limit)
async fn check_github_latest(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
    pkg_name: &str,
) -> Result<Option<String>> {
    let auth_token = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")).ok();

    // 1. Coba endpoint releases/latest (GitHub REST API)
    let rel_url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo);
    let mut req = client.get(&rel_url);
    if let Some(ref token) = auth_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    if let Ok(resp) = req.send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(tag) = val.get("tag_name").and_then(|t| t.as_str()) {
                        let cleaned = clean_version_tag(tag, pkg_name);
                        if !cleaned.is_empty() {
                            return Ok(Some(cleaned));
                        }
                    }
                }
            }
        }
    }

    // 2. Fallback ke endpoint tags (GitHub REST API)
    let tags_url = format!("https://api.github.com/repos/{}/{}/tags?per_page=1", owner, repo);
    let mut req_tags = client.get(&tags_url);
    if let Some(ref token) = auth_token {
        req_tags = req_tags.header("Authorization", format!("Bearer {}", token));
    }

    if let Ok(resp) = req_tags.send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                if let Ok(tags) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                    if let Some(first) = tags.first() {
                        if let Some(tag) = first.get("name").and_then(|t| t.as_str()) {
                            let cleaned = clean_version_tag(tag, pkg_name);
                            if !cleaned.is_empty() {
                                return Ok(Some(cleaned));
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback ke GitHub Atom Feed (Bebas Rate Limit / Zero Quota)
    let atom_url = format!("https://github.com/{}/{}/releases.atom", owner, repo);
    if let Ok(resp) = client.get(&atom_url).send().await {
        if resp.status().is_success() {
            if let Ok(xml) = resp.text().await {
                if let Some(tag) = extract_tag_from_atom_feed(&xml) {
                    let cleaned = clean_version_tag(&tag, pkg_name);
                    if !cleaned.is_empty() {
                        return Ok(Some(cleaned));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Cek versi terbaru dari Anitya API (release-monitoring.org v2 Projects API)
async fn check_anitya_latest(
    client: &reqwest::Client,
    pkg_name: &str,
    upstream_url: Option<&str>,
) -> Result<Option<String>> {
    let lookup_name = match pkg_name {
        "linux-headers" => "linux",
        other => other,
    };
    let url = format!("https://release-monitoring.org/api/v2/projects/?name={}", lookup_name);
    if let Ok(resp) = client.get(&url).send().await {
        if resp.status().is_success() {
            if let Ok(text) = resp.text().await {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(items) = val.get("items").and_then(|i| i.as_array()) {
                        let extract_item_version = |item: &serde_json::Value, pkg: &str| -> Option<String> {
                            if let Some(stables) = item.get("stable_versions").and_then(|s| s.as_array()) {
                                for s in stables {
                                    if let Some(s_str) = s.as_str() {
                                        let cleaned = clean_version_tag(s_str, pkg);
                                        if !cleaned.is_empty() && !cleaned.contains("-rc") && !cleaned.contains("-beta") && !cleaned.contains("-alpha") {
                                            // Pengecualian MPC jika rilis 1.4.x belum tersedia di GNU FTP
                                            if pkg == "mpc" && cleaned.starts_with("1.4") {
                                                continue;
                                            }
                                            return Some(cleaned);
                                        }
                                    }
                                }
                            }
                            if let Some(ver) = item.get("version").and_then(|v| v.as_str()) {
                                let cleaned = clean_version_tag(ver, pkg);
                                if !cleaned.is_empty() {
                                    return Some(cleaned);
                                }
                            }
                            None
                        };

                        // 1. Prioritas tertinggi: Cari item yang homepage atau ecosystem-nya cocok dengan upstream_url
                        if let Some(up_url) = upstream_url {
                            let up_clean = up_url.trim_end_matches('/').to_lowercase();
                            for item in items {
                                let hp = item.get("homepage").and_then(|h| h.as_str()).unwrap_or_default().to_lowercase();
                                let eco = item.get("ecosystem").and_then(|e| e.as_str()).unwrap_or_default().to_lowercase();
                                let be = item.get("backend").and_then(|b| b.as_str()).unwrap_or_default().to_lowercase();
                                
                                if (!hp.is_empty() && (up_clean.contains(&hp) || hp.contains(&up_clean)))
                                    || (!eco.is_empty() && (up_clean.contains(&eco) || eco.contains(&up_clean)))
                                    || (up_clean.contains("gnu.org") && (be.contains("gnu") || be.contains("cgit") || hp.contains("gnu.org") || eco.contains("gnu.org")))
                                    || (up_clean.contains("kernel.org") && (be.contains("cgit") || hp.contains("kernel.org")))
                                {
                                    if let Some(cleaned) = extract_item_version(item, pkg_name) {
                                        return Ok(Some(cleaned));
                                    }
                                }
                            }
                        }

                        // 2. Cari item yang namanya paling cocok
                        for item in items {
                            let item_name = item.get("name").and_then(|n| n.as_str()).unwrap_or_default();
                            if item_name.eq_ignore_ascii_case(pkg_name) {
                                if let Some(cleaned) = extract_item_version(item, pkg_name) {
                                    return Ok(Some(cleaned));
                                }
                            }
                        }

                        // 3. Fallback ke item pertama jika ada
                        if let Some(first) = items.first() {
                            if let Some(cleaned) = extract_item_version(first, pkg_name) {
                                return Ok(Some(cleaned));
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_version_tag_cleaning() {
        assert_eq!(clean_version_tag("v2.38.0", "fastfetch"), "2.38.0");
        assert_eq!(clean_version_tag("fastfetch-2.38.0", "fastfetch"), "2.38.0");
        assert_eq!(clean_version_tag("release-3.8.4", "cowsay"), "3.8.4");
        assert_eq!(clean_version_tag("3.8.4", "cowsay"), "3.8.4");
    }

    #[test]
    fn test_version_comparison() {
        assert_eq!(compare_versions("2.38.0", "2.39.0"), Ordering::Less);
        assert_eq!(compare_versions("2.39.0", "2.38.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.13.2", "1.13.2"), Ordering::Equal);
        assert_eq!(compare_versions("6.13", "6.13.1"), Ordering::Less);
        assert_eq!(compare_versions("15.3.0", "14.2.0"), Ordering::Greater);
    }

    #[test]
    fn test_extract_github_repo() {
        assert_eq!(
            extract_github_repo("https://github.com/fastfetch-cli/fastfetch"),
            Some(("fastfetch-cli".to_string(), "fastfetch".to_string()))
        );
        assert_eq!(
            extract_github_repo("https://github.com/rui314/mold/archive/v2.42.1.tar.gz"),
            Some(("rui314".to_string(), "mold".to_string()))
        );
        assert_eq!(extract_github_repo("https://ftp.gnu.org/gnu/make/make-4.4.1.tar.gz"), None);
    }

    #[test]
    fn test_extract_tag_from_atom_feed() {
        let sample_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Release notes from fastfetch</title>
  <entry>
    <id>tag:github.com,2008:Repository/340181518/2.68.1</id>
    <link rel="alternate" type="text/html" href="https://github.com/fastfetch-cli/fastfetch/releases/tag/2.68.1"/>
    <title>2.68.1</title>
  </entry>
</feed>"#;
        assert_eq!(extract_tag_from_atom_feed(sample_xml), Some("2.68.1".to_string()));
    }

    #[tokio::test]
    async fn test_bump_recipe_toml_manipulation() -> Result<()> {
        let temp = tempdir()?;
        let recipe_file = temp.path().join("recipe.toml");

        let initial_content = r#"[package]
name = "fastfetch"
version = "2.38.0"
release = 2
slot = "0"
description = "Fast neofetch alternative"
license = "MIT"
upstream = "https://github.com/fastfetch-cli/fastfetch"

[sources]
urls = ["https://github.com/fastfetch-cli/fastfetch/archive/refs/tags/2.38.0.tar.gz"]
sha256 = ["old_hash_here"]
"#;
        std::fs::write(&recipe_file, initial_content)?;

        let (old_ver, _) = RecipeBumper::bump_recipe_file(&recipe_file, "2.39.0", false).await?;
        assert_eq!(old_ver, "2.38.0");

        let updated_content = std::fs::read_to_string(&recipe_file)?;
        assert!(updated_content.contains("version = \"2.39.0\""));
        assert!(updated_content.contains("release = 1"));
        assert!(updated_content.contains("sha256 = [\"old_hash_here\"]"));

        Ok(())
    }
}
