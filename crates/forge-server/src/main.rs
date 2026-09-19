use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge_server::{ForgeServer, RecipeBumper, ServerBuilder, ServerImporter, ServerIndexer, ServerProfileManager, ServerState};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "forge-server")]
#[command(author = "Kura Linux Team <admin@kuralinux.org>")]
#[command(version = "0.1.0")]
#[command(about = "Central Recipe Registry & CI/CD Builder Server for Forge", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Jalankan HTTP / API service untuk sinkronisasi resep & katalog biner
    Serve {
        #[arg(long, default_value = "0.0.0.0:8080")]
        bind: String,

        #[arg(long, default_value = "recipes")]
        recipes_path: PathBuf,

        #[arg(long, default_value = "/var/cache/forge/server")]
        cache_path: PathBuf,

        #[arg(long, default_value = "/var/db/forge/binhost")]
        binhost_path: PathBuf,

        /// Bundel ulang recipes sebelum menjalankan server
        #[arg(long)]
        bundle: bool,
    },

    /// Audit seluruh resep: bandingkan versi lokal dengan versi rilis hulu (upstream)
    Audit {
        /// Path direktori resep
        #[arg(long, default_value = "recipes")]
        recipes_path: PathBuf,
    },

    /// Perbarui (bump) versi resep ke rilis hulu terbaru secara otomatis dan push ke GitHub (SSOT)
    Bump {
        /// Nama paket spesifik yang akan di-bump (gunakan --all untuk bump semua paket yang ada update)
        package: Option<String>,

        /// Bump semua paket yang memiliki rilis hulu baru
        #[arg(long)]
        all: bool,

        /// Override versi target secara manual
        #[arg(long)]
        version: Option<String>,

        /// Path direktori resep
        #[arg(long, default_value = "recipes")]
        recipes_path: PathBuf,

        /// Jangan unduh tarball baru untuk kalkulasi SHA256 checksum
        #[arg(long)]
        no_sha: bool,

        /// Jangan lakukan git push ke GitHub setelah bump
        #[arg(long)]
        no_push: bool,
    },

    /// Impor berkas profil CPU (cpu-profile.json) dan set sebagai profil aktif CI/CD
    Import {
        /// Path ke file profil CPU (cpu-profile.json)
        profile_json: PathBuf,

        /// Direktori penyimpanan profil CPU server
        #[arg(long)]
        profiles_dir: Option<PathBuf>,

        /// Simpan profil dengan nama kustom (misal: znver4, custom-ryzen)
        #[arg(long, name = "as")]
        r#as: Option<String>,
    },

    /// CI/CD Worker: Kompilasi paket menggunakan profil CPU aktif yang tersimpan
    Build {
        /// Nama paket yang akan dikompilasi (misal: mold, base, curl)
        package: String,

        /// Override path ke berkas profil CPU custom (opsional)
        #[arg(long)]
        profile: Option<PathBuf>,

        /// Path direktori resep
        #[arg(long, default_value = "recipes")]
        recipes_path: PathBuf,

        /// Direktori keluaran artefak .forge.tar.zst
        #[arg(long, default_value = "dist")]
        output_dir: PathBuf,

        /// Path direktori penyimpanan binhost resmi
        #[arg(long, default_value = "/var/db/forge/binhost")]
        binhost_path: PathBuf,

        /// Jangan publikasikan secara otomatis ke binhost
        #[arg(long)]
        no_publish: bool,

        /// Direktori profil CPU
        #[arg(long)]
        profiles_dir: Option<PathBuf>,
    },

    /// Tampilkan daftar seluruh profil CPU yang tersimpan di server
    ListProfiles {
        /// Direktori penyimpanan profil CPU server
        #[arg(long)]
        profiles_dir: Option<PathBuf>,
    },

    /// Regenerasi database index repositori biner (packages.db.zst)
    Index {
        #[arg(long, default_value = "/var/db/forge/binhost")]
        storage_path: PathBuf,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            bind,
            recipes_path,
            cache_path,
            binhost_path,
            bundle,
        } => {
            println!("{}", "=== Forge Central Server ===".bold().cyan());
            println!("{} Inisialisasi Server Daemon...", "[*]".blue());
            println!("  - Recipes Directory : {}", recipes_path.display());
            println!("  - Cache Directory   : {}", cache_path.display());
            println!("  - Binhost Directory : {}", binhost_path.display());

            let tar_file = cache_path.join("recipes.tar.zst");
            let sha_file = cache_path.join("recipes.tar.zst.sha256");

            if bundle || !tar_file.exists() || !sha_file.exists() {
                if recipes_path.exists() {
                    println!(
                        "{} Mengemas direktori recipes menjadi tarball Zstandard...",
                        "[*]".blue()
                    );
                    match ForgeServer::bundle_recipes(&recipes_path, &tar_file) {
                        Ok(hash) => {
                            println!(
                                "{} Resep berhasil dikemas! SHA256: {}",
                                "✓".green(),
                                hash.bold().yellow()
                            );
                        }
                        Err(e) => {
                            eprintln!("{} Gagal mengemas recipes: {:#}", "✗".red(), e);
                        }
                    }
                } else {
                    println!(
                        "{} Direktori recipes '{}' tidak ditemukan. Lewati bundling otomatis.",
                        "[!]".yellow(),
                        recipes_path.display()
                    );
                }
            }

            let state = Arc::new(ServerState {
                recipes_dir: recipes_path,
                cache_dir: cache_path,
                binhost_dir: binhost_path,
            });

            let app = ForgeServer::router(state);
            println!(
                "{} Menjalankan Recipe Registry & Binhost API di: {}",
                "[✓]".green(),
                bind.bold().yellow()
            );
            println!("Melayani sinkronisasi klien...");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::bind(&bind).await?;
                axum::serve(listener, app).await?;
                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Import {
            profile_json,
            profiles_dir,
            r#as,
        } => {
            ServerProfileManager::import_profile(
                &profile_json,
                profiles_dir.as_deref(),
                r#as.as_deref(),
            )?;
        }

        Commands::Build {
            package,
            profile,
            recipes_path,
            output_dir,
            binhost_path,
            no_publish,
            profiles_dir,
        } => {
            // 1. Tentukan profil CPU yang digunakan
            let (cpu_profile, source_desc) = ServerProfileManager::load_active_profile(
                profile.as_deref(),
                profiles_dir.as_deref(),
            )?;

            println!(
                "{} Menggunakan Profil CPU [{}]: {} ({})",
                "[*]".blue(),
                source_desc.cyan(),
                cpu_profile.model_name.bold().green(),
                cpu_profile.target_march.bold().yellow()
            );

            // 2. Kompilasi paket di staging terisolasi dan kemas ke .forge.tar.zst
            let built_tarball = ServerBuilder::build_package(
                &cpu_profile,
                Some(&package),
                &recipes_path,
                &output_dir,
            )?;

            // 3. Otomatis publikasikan ke Binhost resmi kecuali --no-publish
            if !no_publish {
                println!(
                    "{} Mempublikasikan biner ke Binhost resmi...",
                    "[*]".blue()
                );
                ServerImporter::import_tarball(
                    &built_tarball,
                    &binhost_path,
                    Some(&cpu_profile.target_march),
                )?;
            }
        }

        Commands::ListProfiles { profiles_dir } => {
            let entries = ServerProfileManager::list_profiles(profiles_dir.as_deref())?;
            let dir = ServerProfileManager::resolve_profiles_dir(profiles_dir.as_deref());

            println!("{}", "=== Forge Server CPU Profiles ===".bold().cyan());
            println!("Direktori Profil: {}\n", dir.display().to_string().yellow());

            if entries.is_empty() {
                println!(
                    "  {} Belum ada profil CPU yang tersimpan.",
                    "[!]".yellow()
                );
                println!(
                    "  Jalankan '{}' untuk mengimpor profil silikon.",
                    "forge-server import <cpu-profile.json>".bold().green()
                );
            } else {
                for entry in &entries {
                    let status = if entry.is_active {
                        "[AKTIF]".bold().green()
                    } else {
                        "[     ]".dimmed()
                    };
                    println!(
                        "  {} {} ({}) - {}",
                        status,
                        entry.name.bold(),
                        entry.march.bold().yellow(),
                        entry.model_name
                    );
                    println!(
                        "       Path: {}",
                        entry.path.display().to_string().cyan()
                    );
                    println!("       ISA : {}", entry.isa_summary.dimmed());
                }
            }
        }

        Commands::Audit { recipes_path } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                println!("{}", "=== Forge Server Upstream Recipe Version Audit ===".bold().cyan());
                println!("Memindai direktori resep: {}\n", recipes_path.display().to_string().yellow());

                let checks = RecipeBumper::audit_all(&recipes_path).await?;
                let mut updates_available = 0;

                println!("{:<22} {:<10} {:<12} {:<14} {:<8} {}", "PAKET".bold(), "KATEGORI".bold(), "VERSI LOKAL".bold(), "VERSI HULU".bold(), "STATUS".bold(), "SUMBER / PROVIDER".bold());
                println!("{}", "-".repeat(95).dimmed());

                for c in &checks {
                    let status_str = if c.has_update {
                        updates_available += 1;
                        "[UPDATE]".bold().yellow()
                    } else if c.latest_version.is_some() {
                        "[LATEST]".green()
                    } else {
                        "[? UNK ]".dimmed()
                    };

                    let latest_str = c.latest_version.as_deref().unwrap_or("-");

                    println!(
                        "{:<22} {:<10} {:<12} {:<14} {:<8} {}",
                        c.name.bold(),
                        c.category.cyan(),
                        c.current_version.white(),
                        if c.has_update { latest_str.bold().yellow() } else { latest_str.dimmed() },
                        status_str,
                        c.provider.dimmed()
                    );
                }

                println!("{}", "-".repeat(95).dimmed());
                println!(
                    "Total paket: {} | Paket mutakhir: {} | Perlu diperbarui: {}",
                    checks.len().to_string().bold(),
                    (checks.len() - updates_available).to_string().bold().green(),
                    updates_available.to_string().bold().yellow()
                );

                if updates_available > 0 {
                    println!("\n{} Jalankan '{}' untuk memperbarui semua resep.", "[i]".blue(), "forge-server bump --all".bold().green());
                }

                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Bump {
            package,
            all,
            version,
            recipes_path,
            no_sha,
            no_push,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let fetch_sha = !no_sha;
                let auto_push = !no_push;

                if all {
                    println!("{}", "=== Memperbarui Seluruh Resep yang Memiliki Versi Hulu Baru ===".bold().cyan());
                    let checks = RecipeBumper::audit_all(&recipes_path).await?;
                    let mut bumped_count = 0;

                    for c in &checks {
                        if c.has_update {
                            if let Some(ref latest) = c.latest_version {
                                println!("{} Memperbarui {} (v{} -> v{})...", "[*]".blue(), c.name.bold(), c.current_version, latest.bold().green());
                                match RecipeBumper::bump_package_by_name(&recipes_path, &c.name, Some(latest), fetch_sha).await {
                                    Ok(msg) => {
                                        println!("  {} {}", "✓".green(), msg);
                                        bumped_count += 1;
                                    }
                                    Err(e) => {
                                        println!("  {} Gagal bump {}: {:#}", "✗".red(), c.name, e);
                                    }
                                }
                            }
                        }
                    }

                    println!("\n{} Selesai memperbarui {} paket.", "[✓]".green(), bumped_count.to_string().bold().green());

                    if auto_push && bumped_count > 0 {
                        println!("{} Mem-push perubahan langsung ke GitHub (Single Source of Truth)...", "[*]".blue());
                        let _ = tokio::process::Command::new("git").args(["add", "recipes"]).status().await;
                        let _ = tokio::process::Command::new("git").args(["commit", "-m", "chore(recipes): automated upstream recipe version bump"]).status().await;
                        let push_status = tokio::process::Command::new("git").args(["push", "origin", "main"]).status().await;
                        if let Ok(st) = push_status {
                            if st.success() {
                                println!("{} Berhasil di-push ke GitHub!", "✓".green());
                            } else {
                                println!("{} Peringatan: Git push gagal. Silakan push manual.", "[!]".yellow());
                            }
                        }
                    }
                } else if let Some(pkg) = package {
                    println!("{} Memperbarui paket '{}'...", "[*]".blue(), pkg.bold().cyan());
                    let msg = RecipeBumper::bump_package_by_name(&recipes_path, &pkg, version.as_deref(), fetch_sha).await?;
                    println!("{} {}", "✓".green(), msg);

                    if auto_push {
                        println!("{} Mem-push perubahan langsung ke GitHub (Single Source of Truth)...", "[*]".blue());
                        let _ = tokio::process::Command::new("git").args(["add", "recipes"]).status().await;
                        let commit_msg = format!("chore(recipes): bump {} to latest version", pkg);
                        let _ = tokio::process::Command::new("git").args(["commit", "-m", &commit_msg]).status().await;
                        let push_status = tokio::process::Command::new("git").args(["push", "origin", "main"]).status().await;
                        if let Ok(st) = push_status {
                            if st.success() {
                                println!("{} Berhasil di-push ke GitHub!", "✓".green());
                            } else {
                                println!("{} Peringatan: Git push gagal. Silakan push manual.", "[!]".yellow());
                            }
                        }
                    }
                } else {
                    anyhow::bail!("Harap tentukan nama paket atau gunakan flag '--all'!");
                }

                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Index { storage_path } => {
            ServerIndexer::regenerate_index(&storage_path)?;
        }
    }

    Ok(())
}
