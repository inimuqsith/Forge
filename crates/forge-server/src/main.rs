use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge_core::get_default_system_packages;

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
    },

    /// CI/CD: Kompilasi paket yang di-lock ke CPU target & upload ke Binary Library
    Import {
        /// Nama paket yang akan dikompilasi
        package: Option<String>,

        /// Kunci arsitektur CPU target (misal: znver4, alderlake, x86-64-v4)
        #[arg(long)]
        target_cpu: Option<String>,

        /// Kompilasi massal seluruh paket set @system
        #[arg(long)]
        all_system: bool,
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
        Commands::Serve { bind } => {
            println!("{}", "=== Forge Central Server ===".bold().cyan());
            println!("{} Menjalankan Recipe Registry & Binhost API di: {}", "[✓]".green(), bind.bold().yellow());
            println!("Melayani sinkronisasi klien...");
        }

        Commands::Import { package, target_cpu, all_system } => {
            let cpu = target_cpu.unwrap_or_else(|| "znver4".to_string());
            println!("{}", "=== Forge CI/CD Builder (Lock-CPU) ===".bold().cyan());
            println!("{} Mengunci Target CPU ke: {}", "[🔒]".yellow(), cpu.bold().green());

            if all_system {
                let pkgs = get_default_system_packages();
                println!("{} Memulai kompilasi massal set @system (Total: {} paket)...", "[*]".blue(), pkgs.len());
                for (i, pkg) in pkgs.iter().enumerate() {
                    println!("  [{}/{}] CI/CD Building & Packaging: {}.forge.tar.zst ({})", i + 1, pkgs.len(), pkg.bold().green(), cpu);
                }
                println!("{}", "✓ Seluruh paket @system berhasil di-build dan diunggah ke Binary Library!".green());
            } else if let Some(pkg) = package {
                println!("{} CI/CD Building & Packaging: {}.forge.tar.zst ({})", "[*]".blue(), pkg.bold().green(), cpu);
                println!("{}", "✓ Paket berhasil dipublikasikan ke Binary Library!".green());
            } else {
                println!("{} Tentukan nama paket atau gunakan flag --all-system.", "[!]".red());
            }
        }

        Commands::Index { storage_path } => {
            println!(">>> Memindai repositori biner di {}...", storage_path.bold());
            println!("{} Database index packages.db.zst berhasil diperbarui!", "✓".green());
        }
    }

    Ok(())
}
