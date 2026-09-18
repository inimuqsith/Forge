use anyhow::{bail, Context, Result};
use colored::*;
use forge::CpuProfile;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileEntry {
    pub name: String,
    pub march: String,
    pub model_name: String,
    pub path: PathBuf,
    pub is_active: bool,
    pub isa_summary: String,
}

pub struct ServerProfileManager;

impl ServerProfileManager {
    /// Tentukan direktori profil server (prioritas: custom_dir -> /var/db/forge/profiles -> /tmp/forge/profiles -> ./profiles)
    pub fn resolve_profiles_dir(custom_dir: Option<&Path>) -> PathBuf {
        if let Some(dir) = custom_dir {
            return dir.to_path_buf();
        }

        let default_dir = PathBuf::from("/var/db/forge/profiles");
        if default_dir.exists() {
            return default_dir;
        }

        // Cek apakah bisa membuat /var/db/forge/profiles
        if fs::create_dir_all(&default_dir).is_ok()
            && fs::File::create(default_dir.join(".write_test")).is_ok()
        {
            let _ = fs::remove_file(default_dir.join(".write_test"));
            return default_dir;
        }

        // Fallback ke temporary directory
        let tmp_dir = std::env::temp_dir().join("forge").join("profiles");
        if fs::create_dir_all(&tmp_dir).is_ok() {
            return tmp_dir;
        }

        // Fallback lokal
        let local_dir = PathBuf::from("profiles");
        let _ = fs::create_dir_all(&local_dir);
        local_dir
    }

    /// Import berkas profil CPU JSON ke server:
    /// 1. Membaca dan memvalidasi `profile_path` sebagai `CpuProfile`
    /// 2. Menyimpan profil ke `<profiles_dir>/<name>.json` (name = `as_name` atau `target_march`)
    /// 3. Mengesetnya sebagai profil aktif di `<profiles_dir>/active.json`
    /// 4. Mencetak konfirmasi konfirmasi sukses
    pub fn import_profile(
        profile_path: &Path,
        profiles_dir: Option<&Path>,
        as_name: Option<&str>,
    ) -> Result<(CpuProfile, PathBuf)> {
        if !profile_path.exists() {
            bail!("Berkas profil CPU tidak ditemukan: {:?}", profile_path);
        }

        let json_content = fs::read_to_string(profile_path)
            .with_context(|| format!("Gagal membaca berkas profil CPU di {:?}", profile_path))?;

        let cpu_profile: CpuProfile = serde_json::from_str(&json_content)
            .context("Format berkas JSON bukan merupakan CpuProfile yang valid")?;

        let target_dir = Self::resolve_profiles_dir(profiles_dir);
        fs::create_dir_all(&target_dir)
            .with_context(|| format!("Gagal membuat direktori profil di {:?}", target_dir))?;

        let profile_name = as_name
            .map(|s| s.trim_end_matches(".json").to_string())
            .unwrap_or_else(|| cpu_profile.target_march.clone());

        let target_file_name = format!("{}.json", profile_name);
        let saved_profile_path = target_dir.join(&target_file_name);

        let pretty_json = cpu_profile.to_json()?;
        fs::write(&saved_profile_path, &pretty_json)
            .with_context(|| format!("Gagal menyimpan profil ke {:?}", saved_profile_path))?;

        let active_profile_path = target_dir.join("active.json");
        fs::write(&active_profile_path, &pretty_json)
            .with_context(|| format!("Gagal menyimpan profil aktif ke {:?}", active_profile_path))?;

        println!(
            "{} Profil CPU berhasil diimpor: {} ({}) -> Diset sebagai target CI/CD aktif",
            "✓".green(),
            cpu_profile.model_name.bold().green(),
            cpu_profile.target_march.bold().yellow()
        );
        println!(
            "  [📁] Disimpan di : {}",
            saved_profile_path.display().to_string().cyan()
        );
        println!(
            "  [⚡] Profil Aktif: {}",
            active_profile_path.display().to_string().cyan()
        );

        Ok((cpu_profile, active_profile_path))
    }

    /// Muat profil CPU untuk build sesuai hierarki prioritas:
    /// a. Jika `custom_profile_path` diberikan, gunakan file tersebut.
    /// b. Jika tidak, muat profil aktif dari `<profiles_dir>/active.json` atau `./cpu-profile.json`.
    /// c. Jika belum ada profil yang diimpor, fallback ke auto-detect CPU host.
    pub fn load_active_profile(
        custom_profile_path: Option<&Path>,
        profiles_dir: Option<&Path>,
    ) -> Result<(CpuProfile, String)> {
        // a. Custom override via --profile
        if let Some(path) = custom_profile_path {
            if !path.exists() {
                bail!("Berkas profil custom tidak ditemukan: {:?}", path);
            }
            let content = fs::read_to_string(path)
                .with_context(|| format!("Gagal membaca custom profil CPU di {:?}", path))?;
            let profile: CpuProfile = serde_json::from_str(&content)
                .context("Format berkas custom JSON bukan merupakan CpuProfile yang valid")?;
            return Ok((profile, format!("custom ({})", path.display())));
        }

        // b. Cek active.json di profiles_dir yang ditentukan
        let target_dir = Self::resolve_profiles_dir(profiles_dir);
        let active_path = target_dir.join("active.json");
        if active_path.exists() {
            if let Ok(content) = fs::read_to_string(&active_path) {
                if let Ok(profile) = serde_json::from_str::<CpuProfile>(&content) {
                    return Ok((profile, format!("active ({})", active_path.display())));
                }
            }
        }

        // Cek fallback lokasi lain
        let fallback_locations = [
            PathBuf::from("/var/db/forge/profiles/active.json"),
            std::env::temp_dir().join("forge").join("profiles").join("active.json"),
            PathBuf::from("profiles/active.json"),
            PathBuf::from("cpu-profile.json"),
        ];

        for loc in &fallback_locations {
            if loc.exists() {
                if let Ok(content) = fs::read_to_string(loc) {
                    if let Ok(profile) = serde_json::from_str::<CpuProfile>(&content) {
                        return Ok((profile, format!("active ({})", loc.display())));
                    }
                }
            }
        }

        // c. Fallback ke auto-detect CPU host
        let detected = CpuProfile::detect().unwrap_or_else(|_| {
            CpuProfile::mock("native", &["avx2", "sse4_2"])
        });
        Ok((detected, "auto-detect host CPU".to_string()))
    }

    /// Tampilkan daftar seluruh profil CPU yang tersimpan dan tandai profil yang sedang aktif
    pub fn list_profiles(profiles_dir: Option<&Path>) -> Result<Vec<ProfileEntry>> {
        let target_dir = Self::resolve_profiles_dir(profiles_dir);
        let mut entries = Vec::new();

        if !target_dir.exists() {
            return Ok(entries);
        }

        let active_profile: Option<CpuProfile> = {
            let active_path = target_dir.join("active.json");
            if active_path.exists() {
                fs::read_to_string(&active_path)
                    .ok()
                    .and_then(|s| serde_json::from_str::<CpuProfile>(&s).ok())
            } else {
                None
            }
        };

        if let Ok(read_dir) = fs::read_dir(&target_dir) {
            for dir_entry in read_dir.flatten() {
                let path = dir_entry.path();
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default();

                if file_name.ends_with(".json") && file_name != "active.json" {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(profile) = serde_json::from_str::<CpuProfile>(&content) {
                            let name = file_name.trim_end_matches(".json").to_string();
                            let is_active = if let Some(ref active) = active_profile {
                                active.target_march == profile.target_march
                                    && active.model_name == profile.model_name
                                    && active.recommended_flags.cflags == profile.recommended_flags.cflags
                            } else {
                                false
                            };

                            let isa_summary = if profile.isa_extensions.is_empty() {
                                "none".to_string()
                            } else {
                                let take_count = profile.isa_extensions.len().min(6);
                                let mut s = profile.isa_extensions[..take_count].join(", ");
                                if profile.isa_extensions.len() > 6 {
                                    s.push_str(", ...");
                                }
                                s
                            };

                            entries.push(ProfileEntry {
                                name,
                                march: profile.target_march.clone(),
                                model_name: profile.model_name.clone(),
                                path,
                                is_active,
                                isa_summary,
                            });
                        }
                    }
                }
            }
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }
}
