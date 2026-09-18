use anyhow::{Context, Result};
use colored::*;
use forge::{CpuProfile, ForgeConfig, RecipeBuilder};
use std::fs;
use std::path::{Path, PathBuf};

pub struct ServerBuilder;

impl ServerBuilder {
    /// Worker CI/CD: Menerima CpuProfile yang dimuat, kunci CFLAGS silikon target,
    /// kompilasi paket dari recipes_root ke staging terisolasi, dan kemas menjadi artefak .forge.tar.zst
    pub fn build_package(
        cpu_profile: &CpuProfile,
        package_name: Option<&str>,
        recipes_root: &Path,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let march = &cpu_profile.target_march;
        let cflags = &cpu_profile.recommended_flags.cflags;
        let cxxflags = &cpu_profile.recommended_flags.cxxflags;
        let ldflags = &cpu_profile.recommended_flags.ldflags;
        let makeflags = &cpu_profile.recommended_flags.makeflags;

        // 1. Tampilkan log penguncian CPU silikon
        println!("{}", "=== Forge CI/CD Worker Builder (Lock-CPU) ===".bold().cyan());
        println!(
            "{} Mengunci CI/CD Compiler ke : {} ({})",
            "[🔒]".yellow(),
            cpu_profile.model_name.bold().green(),
            march.bold().yellow()
        );
        println!("{} CFLAGS Injeksi          : {}", "[⚙]".blue(), cflags.cyan());
        println!("{} CXXFLAGS Injeksi        : {}", "[⚙]".blue(), cxxflags.cyan());
        println!("{} LDFLAGS Injeksi         : {}", "[⚙]".blue(), ldflags.cyan());
        println!("{} MAKEFLAGS Injeksi       : {}", "[⚙]".blue(), makeflags.cyan());

        let target_pkg = package_name.unwrap_or("base");
        println!(
            "{} Memulai CI/CD Build untuk paket: {}",
            "[*]".blue(),
            target_pkg.bold().green()
        );

        // 2. Cari resep paket
        let recipe_path = Self::find_recipe(recipes_root, target_pkg)
            .with_context(|| format!("Resep untuk paket '{}' tidak ditemukan di {:?}", target_pkg, recipes_root))?;

        // 3. Konfigurasi ForgeConfig dinamis berdasarkan CPU Profile
        let mut build_config = ForgeConfig::default();
        build_config.build.cflags = cflags.clone();
        build_config.build.cxxflags = cxxflags.clone();
        build_config.build.ldflags = ldflags.clone();
        build_config.build.makeflags = makeflags.clone();
        build_config.cpu.target_march = march.clone();

        // 4. Kompilasi ke staging terisolasi (thread-safe unique temporary directory)
        let temp_stage = tempfile::Builder::new()
            .prefix(&format!("forge-server-stage-{}-", target_pkg))
            .tempdir()?;
        let staging_dir = temp_stage.path().to_path_buf();

        println!("  [🔨] Mengompilasi dari kode sumber di staging terisolasi...");
        RecipeBuilder::build(&recipe_path, &build_config, &staging_dir, None)?;

        // 5. Kemas ke tarball .forge.tar.zst di output_dir
        fs::create_dir_all(output_dir)?;
        let recipe = RecipeBuilder::load_recipe(&recipe_path)?;
        let tarball_filename = format!("{}-{}-{}.forge.tar.zst", recipe.package.name, recipe.package.version, march);
        let tarball_path = output_dir.join(&tarball_filename);

        println!(
            "  [📦] Mengemas artefak biner .forge.tar.zst ke {}",
            tarball_path.display().to_string().yellow()
        );

        let (final_tarball, sha256_hash, size_bytes) = RecipeBuilder::package_staging(
            &staging_dir,
            &tarball_path,
            &recipe,
            march,
            cflags,
            &build_config.use_flags.flags,
        )?;

        println!(
            "  [✓] Build artefak CI/CD sukses! File: {} ({} bytes, SHA256: {})",
            final_tarball.display().to_string().bold().green(),
            size_bytes,
            sha256_hash.green()
        );

        Ok(final_tarball)
    }

    /// Cari file recipe.toml berdasarkan nama paket di recipes_root
    pub fn find_recipe(root: &Path, pkg: &str) -> Option<PathBuf> {
        RecipeBuilder::find_recipe(pkg, Some(root))
    }
}
