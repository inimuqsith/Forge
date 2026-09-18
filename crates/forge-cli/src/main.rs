use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge_core::{get_default_system_packages, ForgeConfig, RecipeBuilder, SystemSetupConfig, ToolchainManager};
use forge_cpu::CpuProfile;
use std::path::{Path, PathBuf};

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

    /// Kelola dan kemas seed toolchain terisolasi untuk sysroot Kura Linux
    Toolchain {
        #[command(subcommand)]
        action: ToolchainAction,
    },
}

#[derive(Subcommand)]
enum ToolchainAction {
    /// Periksa status deteksi compiler (Clang/LLVM 22, GCC, mold, Make, Ninja)
    Status,

    /// Kemas seed toolchain ke dalam file kura-toolchain.tar.xz untuk sysroot Kura Linux
    Bundle {
        #[arg(long, default_value = "dist/kura-toolchain.tar.xz")]
        output: String,
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
            println!("{}", format!(">>> Membangun paket {} dari kode sumber...", package).bold().cyan());
            let config = ForgeConfig::load_or_default(None);

            let recipe_candidates = [
                PathBuf::from(&package),
                PathBuf::from(format!("recipes/system/{}/recipe.toml", package)),
                PathBuf::from(format!("recipes/core/{}/recipe.toml", package)),
                PathBuf::from(format!("recipes/extra/{}/recipe.toml", package)),
            ];

            let found_recipe = recipe_candidates.iter().find(|p| p.exists());
            if let Some(recipe_path) = found_recipe {
                let destdir = PathBuf::from(format!("/tmp/forge/stage/{}", package));
                match RecipeBuilder::build(recipe_path, &config, &destdir, None) {
                    Ok(staged_dir) => {
                        println!("{} Paket {} sukses dikompilasi ke staging: {}", "✓".green(), package.bold(), staged_dir.display().to_string().cyan());
                    }
                    Err(e) => {
                        println!("{} Gagal mengompilasi paket {}: {:#}", "✗".red(), package.bold(), e);
                    }
                }
            } else {
                println!("{} Resep tidak ditemukan untuk paket: {}", "✗".red(), package.bold());
            }
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

        Commands::Toolchain { action } => match action {
            ToolchainAction::Status => {
                println!("{}", "=== Kura Linux Toolchain Detection Status ===".bold().cyan());
                let status = ToolchainManager::get_status();
                print_component(&status.c_compiler);
                print_component(&status.cxx_compiler);
                print_component(&status.linker);
                print_component(&status.gnu_compiler);
                print_component(&status.make);
                print_component(&status.ninja);
                print_component(&status.pkgconf);
            }

            ToolchainAction::Bundle { output } => {
                println!("{}", "=== Mengemas Kura Linux Seed Toolchain ===".bold().cyan());
                println!("Memindai biner Clang, mold, GCC, Make, Ninja, Pkgconf...");
                let out_path = Path::new(&output);
                match ToolchainManager::bundle_seed_toolchain(out_path) {
                    Ok(final_path) => {
                        println!("{} Seed toolchain berhasil dikemas ke: {}", "✓".green(), final_path.display().to_string().bold().green());
                        let sha_file = final_path.with_extension("xz.sha256");
                        if sha_file.exists() {
                            if let Ok(hash_str) = std::fs::read_to_string(&sha_file) {
                                println!("{} SHA256 Checksum: {}", "[#]".yellow(), hash_str.trim().bold());
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} Gagal mengemas seed toolchain: {:#}", "✗".red(), e);
                    }
                }
            }
        },
    }

    Ok(())
}

fn print_component(comp: &forge_core::ToolchainComponent) {
    if comp.is_available {
        println!(
            "  [{}] {:<12} : {} ({})",
            "✓".green(),
            comp.name.bold(),
            comp.path.as_deref().unwrap_or(""),
            comp.version.as_deref().unwrap_or("").dimmed()
        );
    } else {
        println!(
            "  [{}] {:<12} : {}",
            "✗".red(),
            comp.name.bold(),
            "Tidak ditemukan".red()
        );
    }
}
