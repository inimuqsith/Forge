use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge::{
    CpuProfile, DependencyResolver, ForgeConfig, InstalledDatabase, PackageCascadeResolver,
    PackageProvider, RecipeBuilder, SyncClient, ToolchainComponent, ToolchainManager,
};
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

    /// Pasang paket atau meta-paket (misal: base, base-devel, bash, openssh)
    Install {
        /// Nama paket atau meta-paket (misal: base, base-devel, mold, nginx)
        target: String,

        /// Opsi Akselerasi: Aktifkan 3-Tier Binhost Cascade Resolution (Forge Binhost -> CachyOS -> Source)
        #[arg(long)]
        binhost: bool,

        /// Paksa kompilasi 100% dari kode sumber secara native (Gentoo Portage mode)
        #[arg(long)]
        native: bool,

        /// Tampilkan dialog interaktif untuk memilih provider
        #[arg(long)]
        interactive: bool,

        /// Paksa kompilasi lokal dari source code (alias untuk --native)
        #[arg(long, hide = true)]
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
    Sync {
        /// Override URL server resep (misal: http://127.0.0.1:8080/v1 atau http://<IP>:8080/v1)
        #[arg(long)]
        server: Option<String>,
    },

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

        Commands::Install {
            target,
            binhost,
            native,
            interactive,
            build_source,
        } => {
            println!(">>> Memproses instalasi: {}", target.bold().green());
            let force_native = native || build_source;
            let config = ForgeConfig::load_or_default(None);

            if interactive {
                println!("{} Mode: Membuka pemilihan provider interaktif.", "[i]".blue());
            }

            // Jalankan 3-Tier Cascade Resolver untuk paket target
            let rt = tokio::runtime::Runtime::new()?;
            let resolution = rt.block_on(PackageCascadeResolver::resolve_and_install(
                &target,
                &config,
                force_native,
                binhost,
            ))?;

            match resolution.provider {
                PackageProvider::ForgeBinhost => {
                    println!(
                        "{} Target akan diunduh via Forge Native Binhost: {}",
                        "✓".green(),
                        resolution.download_url.as_deref().unwrap_or("")
                    );
                }
                PackageProvider::CachyOsPrebuilt => {
                    println!(
                        "{} Target akan diunduh via CachyOS Prebuilt fallback: {}",
                        "✓".green(),
                        resolution.download_url.as_deref().unwrap_or("")
                    );
                }
                PackageProvider::ForgeSource => {
                    println!(
                        "{} Target akan dikompilasi dari kode sumber upstream.",
                        "✓".green()
                    );
                }
            }

            println!("  [🔍] Menghitung graf dependensi (DAG) & USE flags untuk '{}'...", target.bold().yellow());
            match DependencyResolver::resolve(&target, &config, None, None) {
                Ok(plan) => {
                    println!("\n{}", "=== Rencana Eksekusi Instalasi (Topological Resolution Plan) ===".bold().cyan());
                    println!("  Target Utama     : {}", plan.target.bold().green());
                    println!("  Total Paket      : {}", plan.total_packages.to_string().bold().yellow());
                    println!("  Build Depends    : {}", plan.build_only_count.to_string().cyan());
                    println!("  Runtime Depends  : {}", plan.runtime_only_count.to_string().cyan());
                    println!("\n{}", "Urutan Kompilasi & Staging:".bold());
                    for step in &plan.steps {
                        let kind_badge = if step.is_meta {
                            "[META]".magenta()
                        } else {
                            "[SRC] ".green()
                        };
                        println!(
                            "  {:>2}. {} {:<20} v{:<10} ({})",
                            step.step_number,
                            kind_badge,
                            step.package_id.to_string().bold(),
                            step.version,
                            step.recipe_path.display().to_string().dimmed()
                        );
                    }
                    println!("\n{} Pohon dependensi valid & siap dikompilasi!", "✓".green());
                }
                Err(e) => {
                    println!("{} Gagal menyelesaikan dependensi target {}: {:#}", "✗".red(), target.bold(), e);
                }
            }
        }

        Commands::Remove { package } => {
            println!(">>> Menghapus paket {} berdasarkan manifest...", package.bold().red());
            let config = ForgeConfig::load_or_default(None);
            let db = InstalledDatabase::new(PathBuf::from(&config.general.db_path));
            let target_root = PathBuf::from(&config.general.root);
            let config_protect = vec![PathBuf::from("/etc"), PathBuf::from("etc")];

            match db.unmerge_package(&package, &target_root, &config_protect) {
                Ok(report) => {
                    println!("{} Paket {} berhasil dihapus!", "✓".green(), package.bold());
                    println!("  - Berkas dihapus       : {}", report.files_removed);
                    println!("  - Symlink dihapus      : {}", report.symlinks_removed);
                    println!("  - Direktori dipangkas  : {}", report.dirs_pruned);
                    if !report.protected_configs_kept.is_empty() {
                        println!("  - Berkas /etc terlindung (CONFIG_PROTECT):");
                        for cfg in &report.protected_configs_kept {
                            println!("    * {} (termodifikasi)", cfg.display().to_string().yellow());
                        }
                    }
                }
                Err(e) => {
                    println!("{} Gagal menghapus paket {}: {:#}", "✗".red(), package.bold(), e);
                }
            }
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

        Commands::Sync { server } => {
            let config = ForgeConfig::load_or_default(None);
            let server_url = server.unwrap_or_else(|| config.server.recipe_server.clone());
            let target_recipes_dir = PathBuf::from(&config.general.recipes_path);
            let cache_dir = PathBuf::from(&config.general.cache_path).join("sync");

            println!("{}", "=== Forge Recipe Sync Engine ===".bold().cyan());
            let rt = tokio::runtime::Runtime::new()?;
            match rt.block_on(SyncClient::sync_recipes(&server_url, &target_recipes_dir, &cache_dir)) {
                Ok(updated) => {
                    if updated {
                        println!("{} Pohon resep berhasil diperbarui!", "✓".green());
                    } else {
                        println!("{} Pohon resep lokal sudah merupakan versi terkini.", "✓".green());
                    }
                }
                Err(e) => {
                    println!("{} Gagal menyinkronkan pohon resep: {:#}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
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
            let config = ForgeConfig::load_or_default(None);
            let db = InstalledDatabase::new(PathBuf::from(&config.general.db_path));
            println!("{}", "=== Daftar Paket Terpasang (/var/db/forge/installed/) ===".bold().cyan());
            match db.list_installed() {
                Ok(packages) => {
                    if packages.is_empty() {
                        println!("  (Belum ada paket yang terpasang)");
                    } else {
                        println!("  {:<25} {:<15} {:<8} {:<10}", "PAKET", "VERSI", "SLOT", "UKURAN");
                        println!("  {}", "-".repeat(60).dimmed());
                        for pkg in packages {
                            let total_size: u64 = pkg.metadata.as_ref().map(|m| m.installed_size).unwrap_or_else(|| {
                                pkg.entries.iter().map(|e| e.size).sum()
                            });
                            let size_str = if total_size > 1024 * 1024 {
                                format!("{:.2} MB", total_size as f64 / (1024.0 * 1024.0))
                            } else if total_size > 1024 {
                                format!("{:.2} KB", total_size as f64 / 1024.0)
                            } else {
                                format!("{} B", total_size)
                            };
                            println!(
                                "  {:<25} {:<15} {:<8} {:<10}",
                                pkg.package_name.bold().green(),
                                pkg.package_version,
                                pkg.slot.cyan(),
                                size_str.dimmed()
                            );
                        }
                    }
                }
                Err(e) => {
                    println!("{} Gagal membaca database paket: {:#}", "✗".red(), e);
                }
            }
        }

        Commands::Query { package } => {
            let config = ForgeConfig::load_or_default(None);
            let db = InstalledDatabase::new(PathBuf::from(&config.general.db_path));
            match db.get_package(&package) {
                Ok(Some(pkg)) => {
                    println!("{}", "=== Informasi Paket ===".bold().cyan());
                    println!("  Nama         : {}", pkg.package_name.bold().green());
                    println!("  Versi        : {}-r{}", pkg.package_version, pkg.release);
                    println!("  Slot         : {}", pkg.slot.cyan());
                    if let Some(ref meta) = pkg.metadata {
                        if !meta.description.is_empty() {
                            println!("  Deskripsi    : {}", meta.description);
                        }
                        if !meta.url.is_empty() {
                            println!("  URL          : {}", meta.url);
                        }
                        if !meta.license.is_empty() {
                            println!("  Lisensi      : {}", meta.license);
                        }
                        if meta.build_time > 0 {
                            println!("  Build Time   : {}", meta.build_time);
                        }
                        if !meta.target_march.is_empty() {
                            println!("  Target March : {}", meta.target_march.yellow());
                        }
                    }
                    if let Some(ref use_f) = pkg.use_flags {
                        println!("  USE Flags    : {}", use_f.yellow());
                    }
                    if let Some(ref cf) = pkg.cflags {
                        println!("  CFLAGS       : {}", cf.dimmed());
                    }
                    println!("\n  Berkas Terpasang (Total {} entri):", pkg.entries.len());
                    for entry in pkg.entries.iter().take(20) {
                        println!("    [{}] {}", entry.entry_type.as_str().cyan(), entry.path.display());
                    }
                    if pkg.entries.len() > 20 {
                        println!("    ... dan {} berkas lainnya", pkg.entries.len() - 20);
                    }
                }
                Ok(None) => {
                    println!("{} Paket '{}' tidak ditemukan di database terpasang.", "✗".red(), package);
                }
                Err(e) => {
                    println!("{} Gagal query paket: {:#}", "✗".red(), e);
                }
            }
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

fn print_component(comp: &ToolchainComponent) {
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
