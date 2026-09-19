use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use forge_server::{ForgeServer, ServerBuilder, ServerImporter, ServerIndexer, ServerProfileManager, ServerState};
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

    /// Generate pasangan kunci kriptografis Ed25519 untuk digital signing
    Keygen {
        /// Direktori penyimpanan kunci
        #[arg(long, default_value = "/etc/forge/keys")]
        output_dir: PathBuf,

        /// Nama file kunci privat
        #[arg(long, default_value = "kura_builder.priv")]
        priv_name: String,

        /// Nama file kunci publik
        #[arg(long, default_value = "kura.pub")]
        pub_name: String,
    },

    /// Tandatangani paket biner .forge.tar.zst dengan kunci Ed25519
    Sign {
        /// File paket biner .forge.tar.zst yang akan ditandatangani
        package_file: PathBuf,

        /// Path ke berkas private key
        #[arg(long, default_value = "/etc/forge/keys/kura_builder.priv")]
        key: PathBuf,

        /// Path file output signature (opsional, default: <package_file>.sig)
        #[arg(long)]
        sig_output: Option<PathBuf>,
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

        Commands::Index { storage_path } => {
            ServerIndexer::regenerate_index(&storage_path)?;
        }

        Commands::Keygen {
            output_dir,
            priv_name,
            pub_name,
        } => {
            println!("{}", "=== Forge Key Generator (Ed25519) ===".bold().cyan());
            let priv_path = output_dir.join(&priv_name);
            let pub_path = output_dir.join(&pub_name);

            let keypair = forge::SigningKeyPair::generate();
            keypair.save_to_files(&priv_path, Some(&pub_path))?;

            println!("{} Berhasil membuat pasangan kunci digital signing!", "✓".green());
            println!("  - Secret Key (Private) : {}", priv_path.display().to_string().bold().yellow());
            println!("  - Public Key (Distro)  : {}", pub_path.display().to_string().bold().green());
            println!("  - Public Key Hex       : {}", keypair.public_key_hex().bold().cyan());
        }

        Commands::Sign {
            package_file,
            key,
            sig_output,
        } => {
            println!("{}", "=== Forge Package Signer ===".bold().cyan());
            if !package_file.exists() {
                anyhow::bail!("Berkas paket biner {:?} tidak ditemukan!", package_file);
            }
            if !key.exists() {
                anyhow::bail!(
                    "Berkas private key {:?} tidak ditemukan! Buat kunci dengan 'forge-server keygen'.",
                    key
                );
            }

            let keypair = forge::SigningKeyPair::load_from_file(&key)?;
            let target_sig = sig_output.unwrap_or_else(|| {
                let mut p = package_file.clone().into_os_string();
                p.push(".sig");
                PathBuf::from(p)
            });

            keypair.sign_file_to_sig_file(&package_file, &target_sig)?;
            println!("{} Paket biner berhasil ditandatangani!", "✓".green());
            println!("  - Package   : {}", package_file.display());
            println!("  - Signature : {}", target_sig.display().to_string().bold().yellow());
        }
    }

    Ok(())
}
