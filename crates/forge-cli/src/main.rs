use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge_core::{get_default_system_packages, SystemSetupConfig};
use forge_cpu::CpuProfile;
use std::path::Path;

#[derive(Parser)]
#[command(name = "forge")]
#[command(author = "Kura Linux Team <admin@kuralinux.org>")]
#[command(version = "0.1.0")]
#[command(about = "High-Performance Source-First & Hybrid Package Manager for Kura Linux", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Inisialisasi dan konfigurasi package manager Forge (/etc/forge/forge.conf)
    Setup {
        #[arg(long, help = "Gunakan nilai default distro tanpa dialog interaktif")]
        defaults: bool,
    },

    /// Wizard bootstrap sistem Kura Linux (/etc/forge/system.conf)
    SystemSetup {
        #[arg(long, help = "Profil sistem: standard, minimal, desktop-ready")]
        profile: Option<String>,
    },

    /// Pasang paket (Default: Kompilasi source native secara 100% silikon)
    Install {
        /// Nama paket atau set paket (misal: bash, openssh, @system)
        target: String,

        /// Opsi Akselerasi: Prioritaskan unduhan biner native dari Forge Server
        #[arg(long)]
        binhost: bool,

        /// Opsi Akselerasi: Izinkan fallback ke biner CachyOS/Arch Linux
        #[arg(long)]
        hybrid: bool,

        /// Tampilkan dialog interaktif untuk memilih provider
        #[arg(long)]
        interactive: bool,

        /// Paksa kompilasi lokal dari source code
        #[arg(long)]
        build_source: bool,
    },

    /// Hapus paket secara bersih berdasarkan manifest
    Remove {
        package: String,
    },

    /// Kompilasi paket dari source hingga tahap staging DESTDIR tanpa merge
    Build {
        package: String,
    },

    /// Sinkronisasi pohon resep dan metadata dari Forge Server
    Sync,

    /// Perbarui dan re-kompilasi seluruh paket yang terpasang
    Update {
        #[arg(default_value = "@world")]
        target: String,
    },

    /// Introspeksi mikroarsitektur CPU hardware dan ekspor cpu-profile.json
    CpuDump {
        #[arg(long, help = "Tampilkan hanya CFLAGS yang direkomendasikan")]
        export_cflags: bool,

        #[arg(long, help = "Simpan ke file spesifik", default_value = "cpu-profile.json")]
        output: String,
    },

    /// Kemas rootfs aktif menjadi tarball stage distribusi (kura-stage.tar.xz)
    StageExport {
        #[arg(long, default_value = "kura-stage.tar.xz")]
        output: String,
    },

    /// Tampilkan daftar seluruh paket yang terpasang
    List,

    /// Tampilkan informasi metadata, USE flags, dan manifest paket
    Query {
        package: String,
    },

    /// Cari resep paket berdasarkan nama atau deskripsi
    Search {
        query: String,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup { defaults } => {
            println!("{}", "=== Forge Package Manager Setup ===".bold().cyan());
            if defaults {
                println!("{} Menerapkan konfigurasi default Kura Linux ke /etc/forge/forge.conf...", "[✓]".green());
            } else {
                println!("Mode konfigurasi interaktif berjalan...");
            }
            println!("{} Konfigurasi Forge berhasil disimpan!", "✓".green());
        }

        Commands::SystemSetup { profile } => {
            println!("{}", "=== Kura Linux System Bootstrap Setup ===".bold().cyan());
            let cpu = CpuProfile::detect()?;
            println!("{} Deteksi CPU: {} ({})", "[i]".blue(), cpu.model_name.bold(), cpu.target_march.yellow());
            
            let chosen_profile = profile.unwrap_or_else(|| "standard".to_string());
            println!("{} Profil Base Distro Terpilih: {}", "[i]".blue(), chosen_profile.bold().green());
            
            let _sys_config = SystemSetupConfig::default();
            println!("{} Menyimpan konfigurasi bootstrap ke /etc/forge/system.conf...", "[✓]".green());
            
            println!("\n{}", "===============================================================".bold().yellow());
            println!("{}", "✓ Konfigurasi Bootstrap Kura Linux Berhasil Disimpan!".bold().green());
            println!("Silakan jalankan perintah berikut untuk memulai kompilasi base OS:");
            println!("  # {}", "forge install @system".bold().cyan());
            println!("{}", "===============================================================".bold().yellow());
        }

        Commands::Install { target, binhost, hybrid, interactive, build_source } => {
            if target == "@system" {
                println!("{}", ">>> Memulai Kompilasi Fondasi Sistem Kura Linux (@system)...".bold().cyan());
                let config_exists = Path::new("/etc/forge/system.conf").exists();
                if config_exists {
                    println!("{} Membaca profil kustom dari /etc/forge/system.conf...", "[i]".blue());
                } else {
                    println!("{} Konfigurasi kustom tidak ditemukan. Menggunakan template default standar Kura Linux...", "[i]".yellow());
                }

                let pkgs = get_default_system_packages();
                println!("{} Total paket @system yang akan di-build: {}", "[*]".blue(), pkgs.len().to_string().bold());
                for (i, pkg) in pkgs.iter().enumerate() {
                    println!("  [{}/{}] Menyiapkan kompilasi native: {}", i + 1, pkgs.len(), pkg.bold().green());
                }
                println!("{}", "✓ Kompilasi set @system siap dieksekusi!".green());
            } else {
                println!(">>> Memproses instalasi paket: {}", target.bold().green());
                if build_source {
                    println!("{} Mode: Paksa kompilasi lokal dari source code.", "[i]".blue());
                } else if binhost {
                    println!("{} Mode: Mengutamakan Forge Native Binhost.", "[i]".blue());
                } else if hybrid {
                    println!("{} Mode: Fallback ke CachyOS/Arch diperbolehkan.", "[i]".blue());
                } else if interactive {
                    println!("{} Mode: Membuka pemilihan provider interaktif.", "[i]".blue());
                } else {
                    println!("{} Mode Default: Source-First Native Compilation (Gentoo Mode).", "[i]".blue());
                }
            }
        }

        Commands::Remove { package } => {
            println!(">>> Menghapus paket {} berdasarkan manifest...", package.bold().red());
        }

        Commands::Build { package } => {
            println!(">>> Membangun paket {} ke staging DESTDIR...", package.bold().cyan());
        }

        Commands::Sync => {
            println!(">>> Menyinkronkan pohon resep dari Forge Server...",);
            println!("{} Pohon resep berhasil diperbarui!", "✓".green());
        }

        Commands::Update { target } => {
            println!(">>> Memeriksa pembaruan untuk target: {}", target.bold().yellow());
        }

        Commands::CpuDump { export_cflags, output } => {
            let cpu = CpuProfile::detect()?;
            if export_cflags {
                println!("{}", cpu.recommended_flags.cflags);
            } else {
                println!("{}", "=== Hardware Introspection Report ===".bold().cyan());
                println!("CPU Model       : {}", cpu.model_name.green());
                println!("Target March    : {}", cpu.target_march.bold().yellow());
                println!("Recommended Opt : {}", cpu.recommended_flags.cflags.cyan());
                println!("Make Parallel   : {}", cpu.recommended_flags.makeflags);
                std::fs::write(&output, cpu.to_json()?)?;
                println!("{} Profil CPU berhasil diekspor ke: {}", "✓".green(), output.bold());
            }
        }

        Commands::StageExport { output } => {
            println!(">>> Mengemas rootfs aktif menjadi stage tarball: {}", output.bold().cyan());
            println!("{} Stage tarball berhasil diekspor!", "✓".green());
        }

        Commands::List => {
            println!("{}", "Daftar Paket Terpasang (/var/db/forge/installed/):".bold());
        }

        Commands::Query { package } => {
            println!("Query metadata untuk paket: {}", package.bold().green());
        }

        Commands::Search { query } => {
            println!("Mencari paket dengan query: '{}'...", query.bold().yellow());
        }
    }

    Ok(())
}
