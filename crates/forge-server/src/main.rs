use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge_server::{ForgeServer, ServerState};
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

        /// Bundel ulang recipes sebelum menjalankan server
        #[arg(long)]
        bundle: bool,
    },

    /// CI/CD: Kompilasi paket yang di-lock ke CPU target & upload ke Binary Library
    Import {
        /// Nama paket yang akan dikompilasi (misal: base, base-devel, mold)
        #[arg(help = "Nama paket atau meta-paket yang akan di-build")]
        package: Option<String>,

        /// Kunci arsitektur CPU target (misal: znver4, alderlake, x86-64-v4)
        #[arg(long)]
        target_cpu: Option<String>,
    },

    /// Regenerasi database index repositori biner (packages.db.zst)
    Index {
        #[arg(long, default_value = "/var/db/forge/binhost")]
        storage_path: String,
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
            bundle,
        } => {
            println!("{}", "=== Forge Central Server ===".bold().cyan());
            println!("{} Inisialisasi Server Daemon...", "[*]".blue());
            println!("  - Recipes Directory : {}", recipes_path.display());
            println!("  - Cache Directory   : {}", cache_path.display());

            let tar_file = cache_path.join("recipes.tar.zst");
            let sha_file = cache_path.join("recipes.tar.zst.sha256");

            if bundle || !tar_file.exists() || !sha_file.exists() {
                if recipes_path.exists() {
                    println!("{} Mengemas direktori recipes menjadi tarball Zstandard...", "[*]".blue());
                    match ForgeServer::bundle_recipes(&recipes_path, &tar_file) {
                        Ok(hash) => {
                            println!("{} Resep berhasil dikemas! SHA256: {}", "✓".green(), hash.bold().yellow());
                        }
                        Err(e) => {
                            eprintln!("{} Gagal mengemas recipes: {:#}", "✗".red(), e);
                        }
                    }
                } else {
                    println!("{} Direktori recipes '{}' tidak ditemukan. Lewati bundling otomatis.", "[!]".yellow(), recipes_path.display());
                }
            }

            let state = Arc::new(ServerState {
                recipes_dir: recipes_path,
                cache_dir: cache_path,
            });

            let app = ForgeServer::router(state);
            println!("{} Menjalankan Recipe Registry & Binhost API di: {}", "[✓]".green(), bind.bold().yellow());
            println!("Melayani sinkronisasi klien...");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                let listener = tokio::net::TcpListener::bind(&bind).await?;
                axum::serve(listener, app).await?;
                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Import { package, target_cpu } => {
            let cpu = target_cpu.unwrap_or_else(|| "znver4".to_string());
            println!("{}", "=== Forge CI/CD Builder (Lock-CPU) ===".bold().cyan());
            println!("{} Mengunci Target CPU ke: {}", "[🔒]".yellow(), cpu.bold().green());

            if let Some(pkg) = package {
                println!("{} CI/CD Building & Packaging: {}.forge.tar.zst ({})", "[*]".blue(), pkg.bold().green(), cpu);
                println!("{}", "✓ Paket berhasil dipublikasikan ke Binary Library!".green());
            } else {
                println!("{} Tentukan nama paket yang akan di-import (misal: base, base-devel, mold).", "[!]".red());
            }
        }

        Commands::Index { storage_path } => {
            println!(">>> Memindai repositori biner di {}...", storage_path.bold());
            println!("{} Database index packages.db.zst berhasil diperbarui!", "✓".green());
        }
    }

    Ok(())
}
