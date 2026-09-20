use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use forge::{
    BinhostClient, CpuProfile, DependencyResolver, ForgeConfig, ForgeLockGuard, InstalledDatabase,
    MergeTransaction, PackageCascadeResolver, PackageProvider, RecipeBuilder, RecipeImporter,
    StageExportOptions, StageExporter, StageFormat, SyncClient, ToolchainComponent,
    ToolchainManager, UseFlagsTui, WavefrontScheduler,
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

        /// Jumlah worker kompilasi paralel (misal: -j8 atau --jobs 8)
        #[arg(short = 'j', long)]
        jobs: Option<usize>,

        /// Tampilkan dialog interaktif TUI untuk memilih USE flags sebelum kompilasi
        #[arg(long)]
        interactive_use: bool,

        /// Evaluasi dan kompilasi ulang seluruh rantai dependensi tanpa memedulikan paket yang sudah terpasang
        #[arg(short = 'D', long = "deep", alias = "rebuild-deps", alias = "rebuild-all")]
        deep: bool,

        /// Paksa kompilasi/instalasi ulang paket target utama meskipun sudah terpasang dengan versi yang sama
        #[arg(long = "reinstall", alias = "rebuild")]
        reinstall: bool,
    },

    /// Konfigurasi visual interaktif USE Flags sistem atau per-paket (TUI Menuconfig)
    Menuconfig {
        /// Nama paket opsional jika ingin mengatur override USE flags per-paket
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Hapus paket secara bersih berdasarkan manifest
    Remove {
        package: String,
    },

    /// Kompilasi paket dari source dan kemas ke .forge.tar.zst tanpa merge ke host
    Build {
        /// Nama paket, meta-paket, atau path ke recipe.toml
        package: String,

        /// Direktori keluaran artefak tarball biner .forge.tar.zst
        #[arg(short, long, default_value = "dist")]
        output_dir: PathBuf,
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

        /// Jumlah worker kompilasi paralel (misal: -j8 atau --jobs 8)
        #[arg(short = 'j', long)]
        jobs: Option<usize>,

        /// Evaluasi dan kompilasi ulang seluruh rantai dependensi tanpa memedulikan paket yang sudah terpasang
        #[arg(short = 'D', long = "deep", alias = "rebuild-deps", alias = "rebuild-all")]
        deep: bool,

        /// Paksa kompilasi/instalasi ulang target meskipun versinya sudah terpasang
        #[arg(long = "reinstall", alias = "rebuild")]
        reinstall: bool,
    },

    /// Introspeksi mikroarsitektur CPU hardware dan ekspor cpu-profile.json
    CpuDump {
        #[arg(long, help = "Tampilkan hanya CFLAGS yang direkomendasikan")]
        export_cflags: bool,

        #[arg(long, help = "Simpan ke file spesifik", default_value = "cpu-profile.json")]
        output: String,
    },

    /// Kemas rootfs aktif menjadi tarball stage distribusi (kura-stage.tar.xz / kura-stage.tar.zst)
    StageExport {
        /// Direktori rootfs sumber yang akan dikemas
        #[arg(short, long, default_value = "/tmp/forge/stage")]
        root: PathBuf,

        /// Lokasi file keluaran stage tarball
        #[arg(short, long, default_value = "dist/kura-stage.tar.xz")]
        output: PathBuf,

        /// Format kompresi tarball (xz atau zstd)
        #[arg(short, long)]
        format: Option<StageFormat>,

        /// Lewati validasi UsrMerge dan OpenRC
        #[arg(long)]
        no_verify: bool,

        /// Jangan bersihkan direktori cache dan file sementara sebelum pengemasan
        #[arg(long)]
        no_clean: bool,
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

    /// Impor resep dari PKGBUILD/APKBUILD upstream ke format recipe.toml Kura Linux
    RecipeImport {
        /// Path berkas PKGBUILD lokal atau URL hulu
        source: String,

        /// Path keluaran recipe.toml (opsional)
        #[arg(short, long)]
        output: Option<String>,
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
            let config = ForgeConfig::load_or_default(None);
            let target_root = PathBuf::from(&config.general.root);
            forge::PrivilegeManager::ensure_root_or_escalate(&target_root, "setup")?;

            if defaults {
                println!("{} Menerapkan konfigurasi default Kura Linux ke /etc/forge/forge.conf...", "[✓]".green());
            } else {
                println!("Mode konfigurasi interaktif berjalan...");
            }
            println!("{} Konfigurasi Forge berhasil disimpan!", "✓".green());
        }

        Commands::Menuconfig { package } => {
            let config = ForgeConfig::load_or_default(None);
            let target_root = PathBuf::from(&config.general.root);
            forge::PrivilegeManager::ensure_root_or_escalate(&target_root, "menuconfig")?;

            let config_path = Path::new("/etc/forge/forge.conf");
            let current_flags = match &package {
                Some(pkg) => UseFlagsTui::load_package_use(Path::new("/etc/forge"), pkg)
                    .unwrap_or_else(|| config.use_flags.flags.clone()),
                None => config.use_flags.flags.clone(),
            };

            let recipe = if let Some(ref pkg) = package {
                let scanner = forge::RecipeScanner::new(None);
                scanner.load_recipe(pkg).map(|(r, _)| r).ok()
            } else {
                None
            };

            let new_flags = UseFlagsTui::prompt_interactive(
                &current_flags,
                recipe.as_ref(),
                package.as_deref(),
            )?;

            match package {
                Some(ref pkg) => {
                    UseFlagsTui::save_package_use(Path::new("/etc/forge"), pkg, &new_flags)?;
                }
                None => {
                    UseFlagsTui::save_global_config(config_path, &new_flags)?;
                }
            }
        }

        Commands::Install {
            target,
            binhost,
            native,
            interactive,
            build_source,
            jobs,
            interactive_use,
            deep,
            reinstall,
        } => {
            let mut config = ForgeConfig::load_or_default(None);
            let target_root = PathBuf::from(&config.general.root);
            forge::PrivilegeManager::ensure_root_or_escalate(&target_root, "install")?;

            // Muat override USE flags per-paket jika ada
            if let Some(pkg_flags) = UseFlagsTui::load_package_use(Path::new("/etc/forge"), &target) {
                println!(
                    "  [i] Menggunakan override USE flags per-paket dari /etc/forge/package.use/{}: {}",
                    target.cyan(),
                    pkg_flags.yellow()
                );
                config.use_flags.flags = format!("{} {}", config.use_flags.flags, pkg_flags);
            }

            // Jika interactive_use diminta, buka prompt TUI sebelum resolusi
            if interactive_use {
                let scanner = forge::RecipeScanner::new(None);
                let recipe = scanner.load_recipe(&target).map(|(r, _)| r).ok();
                let selected = UseFlagsTui::prompt_interactive(&config.use_flags.flags, recipe.as_ref(), Some(&target))?;
                config.use_flags.flags = selected;
            }

            let _lock = ForgeLockGuard::acquire("forge", true)?;
            println!(">>> Memproses instalasi: {}", target.bold().green());
            let force_native = native || build_source;
            let db = InstalledDatabase::new(PathBuf::from(&config.general.db_path));

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
                PackageProvider::ForgeBinhost | PackageProvider::CachyOsPrebuilt => {
                    let provider_name = if resolution.provider == PackageProvider::ForgeBinhost {
                        "Forge Native Binhost"
                    } else {
                        "CachyOS Prebuilt Fallback"
                    };
                    println!(
                        "{} Menggunakan provider akselerasi biner: {}",
                        "⚡".cyan(),
                        provider_name.bold().green()
                    );

                    let items_to_install: Vec<(String, String, Option<String>)> = if resolution.provider == PackageProvider::CachyOsPrebuilt {
                        let cpu = forge::CpuProfile::detect().unwrap_or_else(|_| forge::CpuProfile::mock("native", &[]));
                        let cachyos = forge::CachyOsAdapter::auto_detect(&cpu);
                        if let Ok(chain) = rt.block_on(cachyos.resolve_dependency_chain(&target)) {
                            // Filter dependensi yang belum terpasang di database
                            chain.into_iter()
                                .filter(|(pkg, _, _)| !db.is_installed(pkg))
                                .map(|(pkg, url, meta)| (pkg, url, Some(meta.version)))
                                .collect()
                        } else if let Some(ref dl_url) = resolution.download_url {
                            vec![(target.clone(), dl_url.clone(), None)]
                        } else {
                            vec![]
                        }
                    } else if let Some(ref dl_url) = resolution.download_url {
                        vec![(target.clone(), dl_url.clone(), None)]
                    } else {
                        vec![]
                    };

                    if items_to_install.is_empty() {
                        println!("{} Seluruh paket dan dependensi untuk '{}' sudah terpasang!", "✓".green(), target.bold());
                        return Ok(());
                    }

                    println!("\n{}", "=== Rencana Eksekusi Instalasi Biner (Recursive Topological Order) ===".bold().cyan());
                    println!("  Total Paket yang Akan Dipasang: {}", items_to_install.len().to_string().bold().yellow());
                    for (idx, (pkg, _, ver_opt)) in items_to_install.iter().enumerate() {
                        let ver_str = ver_opt.as_deref().unwrap_or("latest");
                        let role = if pkg == &target { "[TARGET]".green() } else { "[DEPEND]".cyan() };
                        println!("  {:>2}. {} {:<20} v{}", idx + 1, role, pkg.bold(), ver_str);
                    }
                    println!();

                    for (pkg, download_url, ver_opt) in items_to_install {
                        let ver_str = ver_opt.as_deref().unwrap_or("latest");
                        println!("\n>>> Memasang paket biner: {} v{}", pkg.bold().green(), ver_str);
                        println!("  [↓] URL: {}", download_url.dimmed());
                        let stream_staging = std::env::temp_dir()
                            .join("forge")
                            .join("stage")
                            .join(format!("stream-{}", pkg));

                        if stream_staging.exists() {
                            let _ = std::fs::remove_dir_all(&stream_staging);
                        }
                        std::fs::create_dir_all(&stream_staging)?;

                        match rt.block_on(BinhostClient::download_and_extract_stream(
                            &download_url,
                            &stream_staging,
                            None,
                            true,
                        )) {
                            Ok(dl_res) => {
                                println!(
                                    "  [✓] Unduhan selesai ({:.2} MB, BLAKE3: {})",
                                    dl_res.bytes_downloaded as f64 / (1024.0 * 1024.0),
                                    &dl_res.blake3_hash[..12.min(dl_res.blake3_hash.len())]
                                );
                                println!("  [📦] Memulai transaksi merger ke target rootfs '{}'...", target_root.display());

                                let mut tx = MergeTransaction::new(
                                    &pkg,
                                    ver_str,
                                    "0",
                                    &stream_staging,
                                    &target_root,
                                    PathBuf::from(&config.general.db_path),
                                );
                                tx.cflags = Some(config.build.cflags.clone());
                                tx.use_flags = Some(config.use_flags.flags.clone());

                                match tx.execute_merge(&db) {
                                    Ok(manifest) => {
                                        println!(
                                            "{} Paket '{}' v{} berhasil dipasang ke sistem! ({} berkas)",
                                            "✓".green(),
                                            manifest.package_name.bold().green(),
                                            manifest.package_version,
                                            manifest.entries.len()
                                        );
                                    }
                                    Err(e) => {
                                        println!("{} Gagal menggabungkan paket ke sistem: {:#}", "✗".red(), e);
                                        std::process::exit(1);
                                    }
                                }
                                let _ = std::fs::remove_dir_all(&stream_staging);
                            }
                            Err(e) => {
                                println!("{} Gagal mengunduh paket biner: {:#}", "✗".red(), e);
                                std::process::exit(1);
                            }
                        }
                    }

                    println!(
                        "\n{} Seluruh target paket '{}' dan dependensinya berhasil dipasang!",
                        "✨".green(),
                        target.bold().green()
                    );
                }
                PackageProvider::ForgeSource => {
                    println!(
                        "{} Mode Kompilasi Native dari Kode Sumber (Gentoo Portage Mode)",
                        "🚀".green()
                    );

                    println!("  [🔍] Menghitung graf dependensi (DAG) & USE flags untuk '{}'...", target.bold().yellow());
                    let graph = match DependencyResolver::build_graph(&target, &config, None, None) {
                        Ok(g) => g,
                        Err(e) => {
                            println!("{} Gagal memetakan graf dependensi target {}: {:#}", "✗".red(), target.bold(), e);
                            return Ok(());
                        }
                    };

                    let plan = match graph.topological_sort(&target) {
                        Ok(p) => p,
                        Err(e) => {
                            println!("{} Gagal menyelesaikan urutan topologis target {}: {:#}", "✗".red(), target.bold(), e);
                            return Ok(());
                        }
                    };

                    println!("\n{}", "=== Rencana Eksekusi Instalasi (Topological Resolution Plan) ===".bold().cyan());
                    println!("  Target Utama     : {}", plan.target.bold().green());
                    println!("  Total Paket      : {}", plan.total_packages.to_string().bold().yellow());
                    println!("  Build Depends    : {}", plan.build_only_count.to_string().cyan());
                    println!("  Runtime Depends  : {}", plan.runtime_only_count.to_string().cyan());
                    println!("\n{}", "Urutan Kompilasi & Staging:".bold());
                    for step in &plan.steps {
                        let is_installed = if let Ok(Some(inst)) = db.get_package(&step.package_id.name) {
                            inst.package_version == step.version
                        } else {
                            false
                        };

                        let kind_badge = if step.is_meta {
                            "[META]".magenta()
                        } else if is_installed && !deep && (step.package_id.name != target || !reinstall) {
                            "[SKIP]".cyan()
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

                    let scheduler = WavefrontScheduler::with_options(config.clone(), jobs, deep, reinstall);
                    if let Err(e) = scheduler.execute(&graph, &plan, &target_root, &db) {
                        println!("{} Eksekusi kompilasi paralel DAG gagal: {:#}", "✗".red(), e);
                        std::process::exit(1);
                    }

                    println!(
                        "\n{} Seluruh target paket '{}' berhasil dipasang dengan sukses!",
                        "✨".green(),
                        target.bold().green()
                    );
                }
            }
        }

        Commands::Remove { package } => {
            let config = ForgeConfig::load_or_default(None);
            let target_root = PathBuf::from(&config.general.root);
            forge::PrivilegeManager::ensure_root_or_escalate(&target_root, "remove")?;

            let _lock = ForgeLockGuard::acquire("forge", true)?;
            println!(">>> Menghapus paket {} berdasarkan manifest...", package.bold().red());
            let db = InstalledDatabase::new(PathBuf::from(&config.general.db_path));
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

        Commands::Build { package, output_dir } => {
            println!("{}", "=== Forge Source Builder ===".bold().cyan());
            println!(">>> Membangun paket {} dari kode sumber...", package.bold().green());
            let config = ForgeConfig::load_or_default(None);

            let recipes_base = PathBuf::from(&config.general.recipes_path);
            let found_recipe = RecipeBuilder::find_recipe(&package, Some(&recipes_base));

            if let Some(recipe_path) = found_recipe {
                println!("  [🔍] Resep ditemukan: {}", recipe_path.display().to_string().cyan());
                match RecipeBuilder::build_and_package(&recipe_path, &config, &output_dir, None) {
                    Ok(tarball) => {
                        println!("\n{} Paket {} sukses dikompilasi & dikemas ke:", "✓".green(), package.bold());
                        println!("  -> {}", tarball.display().to_string().bold().green());
                        println!("{} Sistem host '/' aman tanpa modifikasi.", "🛡️".green());
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
            let target_recipes_dir = PathBuf::from(&config.general.recipes_path);
            if target_recipes_dir.starts_with("/var") || target_recipes_dir.starts_with("/usr") || target_recipes_dir.starts_with("/etc") {
                forge::PrivilegeManager::ensure_root_or_escalate(Path::new("/"), "sync")?;
            }

            let _lock = ForgeLockGuard::acquire("forge", true)?;
            let server_url = server.unwrap_or_else(|| config.server.recipe_server.clone());
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

        Commands::Update {
            target,
            jobs,
            deep,
            reinstall,
        } => {
            let config = ForgeConfig::load_or_default(None);
            let target_root = PathBuf::from(&config.general.root);
            forge::PrivilegeManager::ensure_root_or_escalate(&target_root, "update")?;

            let _lock = ForgeLockGuard::acquire("forge", true)?;
            let db = InstalledDatabase::new(PathBuf::from(&config.general.db_path));
            let recipes_base = PathBuf::from(&config.general.recipes_path);

            println!("{}", "=== Forge System & Package Update Engine ===".bold().cyan());

            if target == "@world" {
                println!(">>> Memindai seluruh paket terpasang di sistem (@world)...");
                let installed_list = db.list_installed()?;
                if installed_list.is_empty() {
                    println!("{} Tidak ada paket yang terpasang di database sistem.", "[i]".yellow());
                    return Ok(());
                }

                println!("  [i] Ditemukan {} paket terpasang di sistem.", installed_list.len().to_string().bold().yellow());
                let mut updates_to_apply = Vec::new();

                for inst in &installed_list {
                    let found_recipe = RecipeBuilder::find_recipe(&inst.package_name, Some(&recipes_base));
                    let recipe_path = match found_recipe {
                        Some(p) => p,
                        None => continue,
                    };

                    let recipe = match RecipeBuilder::load_recipe(&recipe_path) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };

                    let is_git_vcs = recipe.package.version == "git"
                        || recipe
                            .sources
                            .as_ref()
                            .map(|s| s.urls.iter().any(|u| u.contains(".git") || u.starts_with("git://")))
                            .unwrap_or(false);

                    if is_git_vcs {
                        // Periksa hash commit hulu
                        if let Some(ref sources) = recipe.sources {
                            if let Some(first_url) = sources.urls.first() {
                                print!("  [🔍] Memeriksa upstream VCS untuk {}... ", inst.package_name.bold());
                                match RecipeBuilder::probe_git_remote_commit(first_url, None) {
                                    Ok(remote_commit) => {
                                        let installed_commit = inst.metadata.as_ref().and_then(|m| m.git_commit.clone());
                                        let short_remote = &remote_commit[..8.min(remote_commit.len())];
                                        let short_inst = installed_commit
                                            .as_ref()
                                            .map(|c| &c[..8.min(c.len())])
                                            .unwrap_or("none");

                                        if installed_commit.as_deref() != Some(&remote_commit) {
                                            println!(
                                                "{}",
                                                format!("[PEMBARUAN] {} -> {}", short_inst.yellow(), short_remote.bold().green()).cyan()
                                            );
                                            updates_to_apply.push((inst.package_name.clone(), format!("git:{}", short_remote)));
                                        } else {
                                            println!("{}", format!("[TERKINI] ({})", short_inst).dimmed());
                                        }
                                    }
                                    Err(e) => {
                                        println!("{}", format!("[GAGAL CEK: {}]", e).red());
                                    }
                                }
                            }
                        }
                    } else {
                        // Perbandingan versi rilis semver
                        let is_newer_ver = version_compare::compare_to(&recipe.package.version, &inst.package_version, version_compare::Cmp::Gt).unwrap_or(false);
                        let is_same_ver = version_compare::compare_to(&recipe.package.version, &inst.package_version, version_compare::Cmp::Eq).unwrap_or(false);
                        let is_newer_rel = is_same_ver && recipe.package.release > inst.release;

                        if is_newer_ver || is_newer_rel {
                            println!(
                                "  [↑] Pembaruan terdeteksi untuk {}: v{}-r{} -> v{}-r{}",
                                inst.package_name.bold().green(),
                                inst.package_version.yellow(),
                                inst.release,
                                recipe.package.version.bold().green(),
                                recipe.package.release
                            );
                            updates_to_apply.push((inst.package_name.clone(), recipe.package.version.clone()));
                        }
                    }
                }

                if updates_to_apply.is_empty() {
                    println!("\n{} Seluruh paket sistem sudah dalam versi terkini! (@world is up to date)", "✓".green().bold());
                    return Ok(());
                }

                println!(
                    "\n{} Ditemukan {} paket yang memerlukan pembaruan.",
                    "⚡".cyan(),
                    updates_to_apply.len().to_string().bold().yellow()
                );

                for (pkg_name, _new_ver) in updates_to_apply {
                    println!("\n>>> Memperbarui paket: {}", pkg_name.bold().green());
                    let graph = match DependencyResolver::build_graph(&pkg_name, &config, None, None) {
                        Ok(g) => g,
                        Err(e) => {
                            println!("{} Gagal memetakan graf dependensi {}: {:#}", "✗".red(), pkg_name.bold(), e);
                            continue;
                        }
                    };

                    let plan = match graph.topological_sort(&pkg_name) {
                        Ok(p) => p,
                        Err(e) => {
                            println!("{} Gagal menyelesaikan urutan topologis {}: {:#}", "✗".red(), pkg_name.bold(), e);
                            continue;
                        }
                    };

                    let scheduler = WavefrontScheduler::with_options(config.clone(), jobs, deep, true);
                    if let Err(e) = scheduler.execute(&graph, &plan, &target_root, &db) {
                        println!("{} Eksekusi pembaruan paket {} gagal: {:#}", "✗".red(), pkg_name.bold(), e);
                    } else {
                        println!("{} Paket '{}' berhasil diperbarui ke versi terbaru!", "✓".green(), pkg_name.bold());
                    }
                }

                println!("\n{} Proses pembaruan @world selesai!", "✨".green().bold());
            } else {
                // Target spesifik (misal: forge update forge atau forge update bash)
                println!(">>> Memproses pembaruan paket spesifik: {}", target.bold().green());
                let found_recipe = RecipeBuilder::find_recipe(&target, Some(&recipes_base));
                let recipe_path = match found_recipe {
                    Some(p) => p,
                    None => {
                        println!("{} Resep tidak ditemukan untuk paket: {}", "✗".red(), target.bold());
                        return Ok(());
                    }
                };

                let recipe = RecipeBuilder::load_recipe(&recipe_path)?;
                let installed_pkg = db.get_package(&target)?;

                let is_git_vcs = recipe.package.version == "git"
                    || recipe
                        .sources
                        .as_ref()
                        .map(|s| s.urls.iter().any(|u| u.contains(".git") || u.starts_with("git://")))
                        .unwrap_or(false);

                if let Some(ref inst) = installed_pkg {
                    if is_git_vcs {
                        if let Some(ref sources) = recipe.sources {
                            if let Some(first_url) = sources.urls.first() {
                                print!("  [🔍] Memeriksa upstream VCS untuk {}... ", target.bold());
                                match RecipeBuilder::probe_git_remote_commit(first_url, None) {
                                    Ok(remote_commit) => {
                                        let installed_commit = inst.metadata.as_ref().and_then(|m| m.git_commit.clone());
                                        let short_remote = &remote_commit[..8.min(remote_commit.len())];
                                        let short_inst = installed_commit
                                            .as_ref()
                                            .map(|c| &c[..8.min(c.len())])
                                            .unwrap_or("none");

                                        if installed_commit.as_deref() != Some(&remote_commit) {
                                            println!(
                                                "{}",
                                                format!("[PEMBARUAN] {} -> {}", short_inst.yellow(), short_remote.bold().green()).cyan()
                                            );
                                        } else {
                                            println!("{}", format!("[TERKINI] ({}) -> Memaksa re-kompilasi...", short_inst).dimmed());
                                        }
                                    }
                                    Err(e) => {
                                        println!("{}", format!("[GAGAL CEK: {}] -> Melanjutkan kompilasi...", e).yellow());
                                    }
                                }
                            }
                        }
                    } else {
                        println!(
                            "  [i] Paket terpasang: v{}-r{}, Versi resep: v{}-r{}",
                            inst.package_version.yellow(),
                            inst.release,
                            recipe.package.version.green(),
                            recipe.package.release
                        );
                    }
                } else {
                    println!("  [i] Paket '{}' belum terpasang. Memulai instalasi pertama kali...", target.cyan());
                }

                println!("  [🔍] Menghitung graf dependensi (DAG) & USE flags untuk '{}'...", target.bold().yellow());
                let graph = match DependencyResolver::build_graph(&target, &config, None, None) {
                    Ok(g) => g,
                    Err(e) => {
                        println!("{} Gagal memetakan graf dependensi target {}: {:#}", "✗".red(), target.bold(), e);
                        return Ok(());
                    }
                };

                let plan = match graph.topological_sort(&target) {
                    Ok(p) => p,
                    Err(e) => {
                        println!("{} Gagal menyelesaikan urutan topologis target {}: {:#}", "✗".red(), target.bold(), e);
                        return Ok(());
                    }
                };

                let scheduler = WavefrontScheduler::with_options(config.clone(), jobs, deep, reinstall || true);
                if let Err(e) = scheduler.execute(&graph, &plan, &target_root, &db) {
                    println!("{} Pembaruan paket {} gagal: {:#}", "✗".red(), target.bold(), e);
                    std::process::exit(1);
                }

                println!(
                    "\n{} Paket '{}' berhasil diperbarui & dipasang dengan sukses!",
                    "✨".green(),
                    target.bold().green()
                );
            }
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

        Commands::StageExport {
            root,
            output,
            format,
            no_verify,
            no_clean,
        } => {
            println!("{}", "=== Kura Linux Distro Stage Exporter ===".bold().cyan());
            let selected_format = format.unwrap_or_else(|| {
                if output.to_string_lossy().ends_with(".zst") {
                    StageFormat::Zstd
                } else {
                    StageFormat::Xz
                }
            });

            println!("  Rootfs Sumber    : {}", root.display().to_string().yellow());
            println!("  Format Kompresi  : {}", selected_format.to_string().bold().cyan());
            println!("  Target Output    : {}", output.display().to_string().green());
            if no_verify {
                println!("  Validasi Distro  : {}", "Dinonaktifkan (--no-verify)".yellow());
            } else {
                println!("  Validasi Distro  : {}", "UsrMerge & OpenRC diaktifkan".green());
            }

            let options = StageExportOptions {
                root_dir: root,
                output_path: output,
                format: selected_format,
                strip_binaries: false,
                verify_usrmerge: !no_verify,
                verify_openrc: !no_verify,
                clean_temporary: !no_clean,
            };

            println!("\n>>> Memulai proses ekspor distro stage...");
            match StageExporter::export(&options) {
                Ok(result) => {
                    let size_mb = result.file_size as f64 / (1024.0 * 1024.0);
                    println!("\n{} Stage distribusi berhasil diekspor!", "✓".green());
                    println!("  - Artefak Stage  : {}", result.output_path.display().to_string().bold().green());
                    println!("  - Ukuran Berkas  : {:.2} MB ({} bytes)", size_mb, result.file_size);
                    println!("  - SHA256 Checksum: {}", result.sha256_hash.bold().yellow());
                    println!("  - BLAKE3 Checksum: {}", result.blake3_hash.bold().yellow());
                    println!(
                        "  - Berkas Hash    : {} & {}",
                        result.sha256_path.display().to_string().dimmed(),
                        result.blake3_path.display().to_string().dimmed()
                    );
                }
                Err(e) => {
                    println!("{} Gagal mengekspor distro stage: {:#}", "✗".red(), e);
                    std::process::exit(1);
                }
            }
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
                print_component(&status.c_library);
                print_component(&status.dynamic_linker);
                print_component(&status.kernel_headers);
                print_component(&status.binutils);
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

        Commands::RecipeImport { source, output } => {
            println!("{}", "=== Forge Recipe Importer ===".bold().cyan());
            println!("Memproses sumber: {}", source.yellow());

            let recipe = if source.starts_with("http://") || source.starts_with("https://") {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(RecipeImporter::import_from_url(&source))?
            } else {
                let content = std::fs::read_to_string(&source)
                    .with_context(|| format!("Gagal membaca berkas {}", source))?;
                RecipeImporter::parse_pkgbuild(&content)?
            };

            let toml_output = RecipeImporter::to_toml_string(&recipe)?;

            if let Some(out_path_str) = output {
                let out_path = PathBuf::from(&out_path_str);
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&out_path, &toml_output)?;
                println!("{} Resep berhasil disimpan ke: {}", "✓".green(), out_path.display().to_string().bold().green());
            } else {
                let default_dest = PathBuf::from(format!("recipes/extra/{}/recipe.toml", recipe.package.name));
                println!("\n{}", "--- Hasil Transpilasi recipe.toml ---".bold());
                println!("{}", toml_output);
                println!("\n{} Resep berhasil diimpor untuk paket '{}' v{}", "✓".green(), recipe.package.name.bold(), recipe.package.version);
                println!("  Tip: Gunakan flag --output <PATH> untuk menyimpan langsung ke file (misal: {})", default_dest.display().to_string().cyan());
            }
        }
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
