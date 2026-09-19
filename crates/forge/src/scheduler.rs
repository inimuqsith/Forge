use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;
use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use crate::{
    builder::RecipeBuilder,
    db::InstalledDatabase,
    merger::MergeTransaction,
    resolver::{DependencyGraph, PackageId, PackageNode, ResolutionPlan},
    ForgeConfig, PackageManifest, PackageMetadata,
};

/// Hasil ringkasan dari eksekusi Wavefront Scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSummary {
    pub total_packages: usize,
    pub compiled_packages: usize,
    pub meta_packages: usize,
    pub skipped_packages: usize,
    pub total_files_installed: usize,
    pub elapsed_seconds: f64,
    pub failed_packages: Vec<String>,
}

/// Konfigurasi eksekusi scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    pub jobs: usize,
    pub clean_staging: bool,
    pub dry_run: bool,
    pub rebuild_deps: bool,
    pub reinstall: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            jobs: Self::detect_default_jobs(None),
            clean_staging: true,
            dry_run: false,
            rebuild_deps: false,
            reinstall: false,
        }
    }
}

impl SchedulerConfig {
    pub fn new(jobs: Option<usize>) -> Self {
        Self {
            jobs: Self::detect_default_jobs(jobs),
            clean_staging: true,
            dry_run: false,
            rebuild_deps: false,
            reinstall: false,
        }
    }

    /// Deteksi jumlah jobs/worker yang optimal dari config, parameter, atau hardware CPU
    pub fn detect_default_jobs(jobs_opt: Option<usize>) -> usize {
        if let Some(j) = jobs_opt {
            if j > 0 {
                return j;
            }
        }
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    }

    /// Parse string konfigurasi jobs seperti "-j8", "4", "j 16"
    pub fn parse_jobs_string(raw: &str) -> usize {
        let cleaned: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
        cleaned.parse::<usize>().unwrap_or_else(|_| {
            thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        })
    }
}

/// Event status yang dikirimkan oleh worker threads ke scheduler controller
enum TaskEvent {
    Started {
        worker_id: usize,
        package_id: PackageId,
        version: String,
        is_meta: bool,
    },
    Skipped {
        worker_id: usize,
        package_id: PackageId,
        version: String,
    },
    Success {
        worker_id: usize,
        package_id: PackageId,
        version: String,
        is_meta: bool,
        staging_dir: Option<PathBuf>,
        duration_ms: u128,
    },
    Failed {
        worker_id: usize,
        package_id: PackageId,
        version: String,
        error: String,
    },
}

/// Engine Wavefront Parallel DAG Scheduler
pub struct WavefrontScheduler {
    config: ForgeConfig,
    scheduler_cfg: SchedulerConfig,
}

impl WavefrontScheduler {
    pub fn new(config: ForgeConfig, jobs: Option<usize>) -> Self {
        Self::with_options(config, jobs, false, false)
    }

    pub fn with_options(
        config: ForgeConfig,
        jobs: Option<usize>,
        rebuild_deps: bool,
        reinstall: bool,
    ) -> Self {
        let effective_jobs = if let Some(j) = jobs {
            j
        } else if !config.build.jobs.is_empty() {
            SchedulerConfig::parse_jobs_string(&config.build.jobs)
        } else {
            SchedulerConfig::detect_default_jobs(None)
        };

        Self {
            config,
            scheduler_cfg: SchedulerConfig {
                jobs: effective_jobs.max(1),
                clean_staging: true,
                dry_run: false,
                rebuild_deps,
                reinstall,
            },
        }
    }

    /// Selesaikan seluruh graf dependensi dan eksekusi kompilasi multi-worker paralel
    pub fn execute(
        &self,
        graph: &DependencyGraph,
        plan: &ResolutionPlan,
        target_root: &Path,
        db: &InstalledDatabase,
    ) -> Result<BuildSummary> {
        let start_time = Instant::now();
        let total_packages = graph.nodes.len();

        println!(
            "\n{} {} (Workers: {}, Target: {})",
            "🚀".green(),
            "Memulai Wavefront Parallel DAG Compilation Engine".bold().cyan(),
            self.scheduler_cfg.jobs.to_string().bold().yellow(),
            plan.target.bold().green()
        );

        if total_packages == 0 {
            return Ok(BuildSummary {
                total_packages: 0,
                compiled_packages: 0,
                meta_packages: 0,
                skipped_packages: 0,
                total_files_installed: 0,
                elapsed_seconds: 0.0,
                failed_packages: Vec::new(),
            });
        }

        // 1. Inisialisasi in-degree runtime untuk semua paket
        let mut in_degrees: HashMap<PackageId, usize> = HashMap::new();
        for id in graph.nodes.keys() {
            let deg = graph
                .dependencies_of
                .get(id)
                .map(|edges| {
                    let unique: HashSet<&PackageId> = edges.iter().map(|e| &e.to).collect();
                    unique.len()
                })
                .unwrap_or(0);
            in_degrees.insert(id.clone(), deg);
        }

        // 2. Siapkan queue node berderajat 0 (Wavefront 0)
        let mut initial_nodes: Vec<PackageId> = in_degrees
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        initial_nodes.sort();
        let mut ready_queue: VecDeque<PackageId> = initial_nodes.into();

        let (tx, rx) = mpsc::channel::<TaskEvent>();
        let mut in_flight: HashSet<PackageId> = HashSet::new();
        let mut completed: HashSet<PackageId> = HashSet::new();
        let mut staged_packages: HashMap<PackageId, (PathBuf, PackageNode)> = HashMap::new();
        let mut worker_counter = 0usize;
        let mut compiled_count = 0usize;
        let mut meta_count = 0usize;
        let mut skipped_count = 0usize;

        // 3. Loop Dispatcher & Event Processor
        while completed.len() < total_packages {
            // Dispatch tugas ke worker selama slot worker masih tersedia
            while in_flight.len() < self.scheduler_cfg.jobs && !ready_queue.is_empty() {
                let pkg_id = match ready_queue.pop_front() {
                    Some(id) => id,
                    None => break,
                };

                let node = match graph.nodes.get(&pkg_id) {
                    Some(n) => n.clone(),
                    None => continue,
                };

                in_flight.insert(pkg_id.clone());
                worker_counter += 1;
                let worker_id = (worker_counter % self.scheduler_cfg.jobs) + 1;

                if node.is_meta {
                    // Meta-paket: instan
                    meta_count += 1;
                    let tx_clone = tx.clone();
                    thread::spawn(move || {
                        let _ = tx_clone.send(TaskEvent::Started {
                            worker_id,
                            package_id: node.id.clone(),
                            version: node.version.clone(),
                            is_meta: true,
                        });
                        let _ = tx_clone.send(TaskEvent::Success {
                            worker_id,
                            package_id: node.id.clone(),
                            version: node.version.clone(),
                            is_meta: true,
                            staging_dir: None,
                            duration_ms: 1,
                        });
                    });
                } else {
                    let is_target = node.id.name == plan.target;
                    let already_installed = is_installed_and_matches(&node, db);
                    let should_skip = if is_target {
                        already_installed && !self.scheduler_cfg.reinstall && !self.scheduler_cfg.rebuild_deps
                    } else {
                        already_installed && !self.scheduler_cfg.rebuild_deps
                    };

                    if should_skip {
                        let tx_clone = tx.clone();
                        thread::spawn(move || {
                            let _ = tx_clone.send(TaskEvent::Skipped {
                                worker_id,
                                package_id: node.id.clone(),
                                version: node.version.clone(),
                            });
                        });
                    } else {
                        // Source package: eksekusi di worker thread
                        compiled_count += 1;
                        let tx_clone = tx.clone();
                        let config_clone = self.config.clone();
                        let node_clone = node.clone();

                        thread::spawn(move || {
                            let task_start = Instant::now();
                            let _ = tx_clone.send(TaskEvent::Started {
                                worker_id,
                                package_id: node_clone.id.clone(),
                                version: node_clone.version.clone(),
                                is_meta: false,
                            });

                            let staging_dir = std::env::temp_dir()
                                .join("forge")
                                .join("stage")
                                .join(format!("{}-{}", node_clone.id.name, node_clone.version));

                            if staging_dir.exists() {
                                let _ = std::fs::remove_dir_all(&staging_dir);
                            }
                            if let Err(e) = std::fs::create_dir_all(&staging_dir) {
                                let _ = tx_clone.send(TaskEvent::Failed {
                                    worker_id,
                                    package_id: node_clone.id.clone(),
                                    version: node_clone.version.clone(),
                                    error: format!("Gagal membuat direktori staging: {:#}", e),
                                });
                                return;
                            }

                            match RecipeBuilder::build(&node_clone.recipe_path, &config_clone, &staging_dir, None) {
                                Ok(_) => {
                                    let elapsed = task_start.elapsed().as_millis();
                                    let _ = tx_clone.send(TaskEvent::Success {
                                        worker_id,
                                        package_id: node_clone.id.clone(),
                                        version: node_clone.version.clone(),
                                        is_meta: false,
                                        staging_dir: Some(staging_dir),
                                        duration_ms: elapsed,
                                    });
                                }
                                Err(e) => {
                                    let _ = tx_clone.send(TaskEvent::Failed {
                                        worker_id,
                                        package_id: node_clone.id.clone(),
                                        version: node_clone.version.clone(),
                                        error: format!("{:#}", e),
                                    });
                                }
                            }
                        });
                    }
                }
            }

            // Jika tidak ada task yang in-flight dan queue kosong padahal belum selesai: deadlock/siklus
            if in_flight.is_empty() && ready_queue.is_empty() && completed.len() < total_packages {
                bail!(
                    "Deadlock terdeteksi dalam DAG scheduler! {} dari {} paket selesai.",
                    completed.len(),
                    total_packages
                );
            }

            // Terima event dari worker
            let event = rx
                .recv()
                .context("Channel worker terputus secara tidak terduga!")?;

            match event {
                TaskEvent::Started {
                    worker_id,
                    package_id,
                    version,
                    is_meta,
                } => {
                    let badge = if is_meta {
                        "[META]".magenta()
                    } else {
                        "[BUILD]".green()
                    };
                    println!(
                        "  [Worker {:02}] {} Memproses {} v{}...",
                        worker_id,
                        badge,
                        package_id.to_string().bold(),
                        version
                    );
                }
                TaskEvent::Skipped {
                    worker_id,
                    package_id,
                    version,
                } => {
                    in_flight.remove(&package_id);
                    completed.insert(package_id.clone());
                    skipped_count += 1;

                    println!(
                        "  [Worker {:02}] {} {} v{} sudah terpasang (Dilewati)",
                        worker_id,
                        "[SKIP]".cyan(),
                        package_id.to_string().bold(),
                        version
                    );

                    // Perambatan Wavefront: kurangi in-degree semua dependent
                    if let Some(dependents) = graph.dependents_of.get(&package_id) {
                        let mut newly_ready = Vec::new();
                        for dep in dependents {
                            if let Some(deg) = in_degrees.get_mut(dep) {
                                if *deg > 0 {
                                    *deg -= 1;
                                    if *deg == 0 && !completed.contains(dep) && !in_flight.contains(dep) {
                                        newly_ready.push(dep.clone());
                                    }
                                }
                            }
                        }
                        newly_ready.sort();
                        for dep in newly_ready {
                            ready_queue.push_back(dep);
                        }
                    }
                }
                TaskEvent::Failed {
                    worker_id,
                    package_id,
                    version,
                    error,
                } => {
                    eprintln!(
                        "\n{} [Worker {:02}] Gagal mengompilasi {} v{}:",
                        "✗".red().bold(),
                        worker_id,
                        package_id.to_string().bold(),
                        version
                    );
                    eprintln!("    {:#}", error.red());
                    bail!("Kompilasi paket '{}' gagal!", package_id);
                }
                TaskEvent::Success {
                    worker_id,
                    package_id,
                    version,
                    is_meta,
                    staging_dir,
                    duration_ms,
                } => {
                    in_flight.remove(&package_id);
                    completed.insert(package_id.clone());

                    let duration_sec = duration_ms as f64 / 1000.0;
                    if is_meta {
                        println!(
                            "  [Worker {:02}] {} Meta-paket {} tercatat ({:.2}s)",
                            worker_id,
                            "✓".green(),
                            package_id.to_string().bold().magenta(),
                            duration_sec
                        );
                    } else {
                        println!(
                            "  [Worker {:02}] {} Selesai mengompilasi {} v{} ({:.2}s, Staged)",
                            worker_id,
                            "✓".green(),
                            package_id.to_string().bold().green(),
                            version,
                            duration_sec
                        );
                        if let Some(stg) = staging_dir {
                            if let Some(node) = graph.nodes.get(&package_id) {
                                staged_packages.insert(package_id.clone(), (stg, node.clone()));
                            }
                        }
                    }

                    // Perambatan Wavefront: kurangi in-degree semua dependent
                    if let Some(dependents) = graph.dependents_of.get(&package_id) {
                        let mut newly_ready = Vec::new();
                        for dep in dependents {
                            if let Some(deg) = in_degrees.get_mut(dep) {
                                if *deg > 0 {
                                    *deg -= 1;
                                    if *deg == 0 && !completed.contains(dep) && !in_flight.contains(dep) {
                                        newly_ready.push(dep.clone());
                                    }
                                }
                            }
                        }
                        newly_ready.sort();
                        for dep in newly_ready {
                            ready_queue.push_back(dep);
                        }
                    }
                }
            }
        }

        // 4. Coordinated Transactional Merger Phase
        println!(
            "\n{}",
            "=== Transaksi Penggabungan Sistem (Topological Coordinated Merge) ==="
                .bold()
                .cyan()
        );
        let mut total_files_installed = 0usize;

        for step in &plan.steps {
            let pkg_id = &step.package_id;
            let node = match graph.nodes.get(pkg_id) {
                Some(n) => n,
                None => continue,
            };

            if node.is_meta {
                let dummy_manifest = PackageManifest {
                    package_name: node.id.name.clone(),
                    package_version: node.version.clone(),
                    release: node.release,
                    slot: node.id.slot.clone(),
                    entries: Vec::new(),
                    metadata: Some(PackageMetadata {
                        name: node.id.name.clone(),
                        version: node.version.clone(),
                        release: node.release,
                        slot: node.id.slot.clone(),
                        description: format!("Kura Linux Meta Package: {}", node.id.name),
                        url: "".to_string(),
                        license: "GPL-3.0".to_string(),
                        upstream: "".to_string(),
                        build_time: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        target_march: self.config.cpu.target_march.clone(),
                        cflags: self.config.build.cflags.clone(),
                        use_flags: self.config.use_flags.flags.clone(),
                        files_count: 0,
                        installed_size: 0,
                        git_commit: None,
                    }),
                    use_flags: Some(self.config.use_flags.flags.clone()),
                    cflags: Some(self.config.build.cflags.clone()),
                };
                if let Err(e) = db.record_package(&dummy_manifest, target_root) {
                    eprintln!("  [!] Gagal mencatat manifest meta-paket: {:#}", e);
                } else {
                    println!("  [✓] Meta-paket '{}' berhasil dicatat di database.", node.id.name.magenta());
                }
                continue;
            }

            if let Some((staging_dir, _)) = staged_packages.get(pkg_id) {
                let mut tx = MergeTransaction::new(
                    &node.id.name,
                    &node.version,
                    &node.id.slot,
                    staging_dir,
                    target_root,
                    PathBuf::from(&self.config.general.db_path),
                );
                tx.cflags = Some(self.config.build.cflags.clone());
                tx.use_flags = Some(self.config.use_flags.flags.clone());

                let manifest = tx.execute_merge(db).with_context(|| {
                    format!("Gagal menggabungkan paket '{}' ke rootfs target", node.id.name)
                })?;

                total_files_installed += manifest.entries.len();
                println!(
                    "  [✓] Paket '{}' v{} sukses digabungkan ({} berkas)",
                    node.id.name.bold().green(),
                    node.version,
                    manifest.entries.len()
                );

                if self.scheduler_cfg.clean_staging && staging_dir.exists() {
                    let _ = std::fs::remove_dir_all(staging_dir);
                }
            }
        }

        let elapsed = start_time.elapsed().as_secs_f64();
        if skipped_count > 0 {
            println!(
                "\n{} Selesai! (Dipasang: {}, Dilewati: {}, Total: {}) dalam {:.2} detik!",
                "✓".green().bold(),
                (compiled_count + meta_count).to_string().bold().green(),
                skipped_count.to_string().bold().cyan(),
                total_packages.to_string().bold().yellow(),
                elapsed
            );
        } else {
            println!(
                "\n{} Seluruh {} paket berhasil dikompilasi & dipasang dalam {:.2} detik!",
                "✓".green().bold(),
                total_packages.to_string().bold().yellow(),
                elapsed
            );
        }

        Ok(BuildSummary {
            total_packages,
            compiled_packages: compiled_count,
            meta_packages: meta_count,
            skipped_packages: skipped_count,
            total_files_installed,
            elapsed_seconds: elapsed,
            failed_packages: Vec::new(),
        })
    }
}

/// Helper untuk memeriksa apakah paket sudah terpasang dan versinya cocok di InstalledDatabase
fn is_installed_and_matches(node: &PackageNode, db: &InstalledDatabase) -> bool {
    if let Ok(Some(installed)) = db.get_package(&node.id.name) {
        if installed.package_version == node.version
            && installed.release == node.release
            && (node.id.slot.is_empty() || installed.slot == node.id.slot)
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    use crate::resolver::{DependencyEdge, DependencyKind};

    #[test]
    fn test_scheduler_jobs_string_parsing() {
        assert_eq!(SchedulerConfig::parse_jobs_string("-j8"), 8);
        assert_eq!(SchedulerConfig::parse_jobs_string("16"), 16);
        assert_eq!(SchedulerConfig::parse_jobs_string("-j 4"), 4);
    }

    #[test]
    fn test_wavefront_diamond_graph_execution() -> Result<()> {
        let temp = tempdir()?;
        let recipes_root = temp.path().join("recipes");
        let target_root = temp.path().join("target_root");
        let db_root = temp.path().join("db");
        fs::create_dir_all(&recipes_root)?;
        fs::create_dir_all(&target_root)?;
        fs::create_dir_all(&db_root)?;

        let mut config = ForgeConfig::default();
        config.general.recipes_path = recipes_root.display().to_string();
        config.general.db_path = db_root.display().to_string();
        config.general.root = target_root.display().to_string();

        let db = InstalledDatabase::new(db_root);

        // Buat DAG Diamond:
        // Root: app
        // app -> lib_a, lib_b
        // lib_a -> lib_core
        // lib_b -> lib_core
        let mut graph = DependencyGraph::new();

        let dummy_recipe = temp.path().join("dummy.toml");
        fs::write(
            &dummy_recipe,
            r#"
[package]
name = "dummy"
version = "1.0.0"
"#,
        )?;

        let node_core = PackageNode {
            id: PackageId::new("lib_core", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "Core library".to_string(),
            recipe_path: dummy_recipe.clone(),
            is_meta: true,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        let node_a = PackageNode {
            id: PackageId::new("lib_a", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "Lib A".to_string(),
            recipe_path: dummy_recipe.clone(),
            is_meta: true,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        let node_b = PackageNode {
            id: PackageId::new("lib_b", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "Lib B".to_string(),
            recipe_path: dummy_recipe.clone(),
            is_meta: true,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        let node_app = PackageNode {
            id: PackageId::new("app", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "Main App".to_string(),
            recipe_path: dummy_recipe.clone(),
            is_meta: true,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        graph.add_node(node_core);
        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_app);

        // lib_a -> lib_core
        graph.add_edge(DependencyEdge {
            from: PackageId::new("lib_a", "0"),
            to: PackageId::new("lib_core", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        // lib_b -> lib_core
        graph.add_edge(DependencyEdge {
            from: PackageId::new("lib_b", "0"),
            to: PackageId::new("lib_core", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        // app -> lib_a
        graph.add_edge(DependencyEdge {
            from: PackageId::new("app", "0"),
            to: PackageId::new("lib_a", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        // app -> lib_b
        graph.add_edge(DependencyEdge {
            from: PackageId::new("app", "0"),
            to: PackageId::new("lib_b", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        let plan = graph.topological_sort("app")?;
        assert_eq!(plan.total_packages, 4);

        let scheduler = WavefrontScheduler::new(config, Some(4));
        let summary = scheduler.execute(&graph, &plan, &target_root, &db)?;

        assert_eq!(summary.total_packages, 4);
        assert_eq!(summary.meta_packages, 4);
        assert_eq!(summary.failed_packages.len(), 0);

        // Pastikan seluruh paket terdaftar di installed database
        let installed = db.list_installed()?;
        assert_eq!(installed.len(), 4);

        Ok(())
    }

    #[test]
    fn test_wavefront_real_source_build_compilation() -> Result<()> {
        let temp = tempdir()?;
        let recipes_root = temp.path().join("recipes");
        let target_root = temp.path().join("target_root");
        let db_root = temp.path().join("db");
        fs::create_dir_all(&recipes_root)?;
        fs::create_dir_all(&target_root)?;
        fs::create_dir_all(&db_root)?;

        let mut config = ForgeConfig::default();
        config.general.recipes_path = recipes_root.display().to_string();
        config.general.db_path = db_root.display().to_string();
        config.general.root = target_root.display().to_string();

        let db = InstalledDatabase::new(db_root);

        // Buat resep nyata yang menulis berkas ke DESTDIR
        let recipe_file = temp.path().join("test_pkg.toml");
        fs::write(
            &recipe_file,
            r#"
[package]
name = "parallel-test-pkg"
version = "2.0.0"

[build]
type = "shell"
script = """
mkdir -p "${DESTDIR}/usr/bin"
echo '#!/bin/sh\necho parallel' > "${DESTDIR}/usr/bin/parallel-test-pkg"
chmod +x "${DESTDIR}/usr/bin/parallel-test-pkg"
"""
"#,
        )?;

        let mut graph = DependencyGraph::new();
        let node = PackageNode {
            id: PackageId::new("parallel-test-pkg", "0"),
            version: "2.0.0".to_string(),
            release: 1,
            description: "Test pkg".to_string(),
            recipe_path: recipe_file,
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };
        graph.add_node(node);

        let plan = graph.topological_sort("parallel-test-pkg")?;
        let scheduler = WavefrontScheduler::new(config, Some(2));
        let summary = scheduler.execute(&graph, &plan, &target_root, &db)?;

        assert_eq!(summary.total_packages, 1);
        assert_eq!(summary.compiled_packages, 1);
        assert!(summary.total_files_installed >= 1);

        // Validasi berkas benar-benar terpasang di target_root
        let bin_path = target_root.join("usr/bin/parallel-test-pkg");
        assert!(bin_path.exists());

        Ok(())
    }

    #[test]
    fn test_wavefront_worker_failure_stops_execution() -> Result<()> {
        let temp = tempdir()?;
        let recipes_root = temp.path().join("recipes");
        let target_root = temp.path().join("target_root");
        let db_root = temp.path().join("db");
        fs::create_dir_all(&recipes_root)?;
        fs::create_dir_all(&target_root)?;
        fs::create_dir_all(&db_root)?;

        let mut config = ForgeConfig::default();
        config.general.recipes_path = recipes_root.display().to_string();
        config.general.db_path = db_root.display().to_string();
        config.general.root = target_root.display().to_string();

        let db = InstalledDatabase::new(db_root);

        // Resep yang sengaja error saat compile
        let bad_recipe = temp.path().join("bad_pkg.toml");
        fs::write(
            &bad_recipe,
            r#"
[package]
name = "bad-pkg"
version = "1.0.0"

[build]
type = "shell"
script = """
exit 42
"""
"#,
        )?;

        let mut graph = DependencyGraph::new();
        let node = PackageNode {
            id: PackageId::new("bad-pkg", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "Bad pkg".to_string(),
            recipe_path: bad_recipe,
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };
        graph.add_node(node);

        let plan = graph.topological_sort("bad-pkg")?;
        let scheduler = WavefrontScheduler::new(config, Some(2));
        let result = scheduler.execute(&graph, &plan, &target_root, &db);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("bad-pkg"));

        Ok(())
    }

    #[test]
    fn test_wavefront_skip_already_installed_dependency() -> Result<()> {
        let temp = tempdir()?;
        let recipes_root = temp.path().join("recipes");
        let target_root = temp.path().join("target_root");
        let db_root = temp.path().join("db");
        fs::create_dir_all(&recipes_root)?;
        fs::create_dir_all(&target_root)?;
        fs::create_dir_all(&db_root)?;

        let mut config = ForgeConfig::default();
        config.general.recipes_path = recipes_root.display().to_string();
        config.general.db_path = db_root.display().to_string();
        config.general.root = target_root.display().to_string();

        let db = InstalledDatabase::new(db_root);

        // Pasang dep-pkg di database dummy
        let dep_manifest = PackageManifest {
            package_name: "dep-pkg".to_string(),
            package_version: "1.0.0".to_string(),
            release: 1,
            slot: "0".to_string(),
            entries: Vec::new(),
            metadata: None,
            use_flags: None,
            cflags: None,
        };
        db.record_package(&dep_manifest, &target_root)?;

        // Buat resep untuk app-pkg (bergantung pada dep-pkg)
        let app_recipe = temp.path().join("app_pkg.toml");
        fs::write(
            &app_recipe,
            r#"
[package]
name = "app-pkg"
version = "1.0.0"

[build]
type = "shell"
script = """
mkdir -p "${DESTDIR}/usr/bin"
echo '#!/bin/sh\necho app' > "${DESTDIR}/usr/bin/app-pkg"
chmod +x "${DESTDIR}/usr/bin/app-pkg"
"""
"#,
        )?;

        let dep_recipe = temp.path().join("dep_pkg.toml");
        fs::write(
            &dep_recipe,
            r#"
[package]
name = "dep-pkg"
version = "1.0.0"

[build]
type = "shell"
script = """
mkdir -p "${DESTDIR}/usr/lib"
touch "${DESTDIR}/usr/lib/libdep.so"
"""
"#,
        )?;

        let mut graph = DependencyGraph::new();
        let dep_node = PackageNode {
            id: PackageId::new("dep-pkg", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "Dep pkg".to_string(),
            recipe_path: dep_recipe,
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };
        let app_node = PackageNode {
            id: PackageId::new("app-pkg", "0"),
            version: "1.0.0".to_string(),
            release: 1,
            description: "App pkg".to_string(),
            recipe_path: app_recipe,
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        graph.add_node(dep_node);
        graph.add_node(app_node);
        graph.add_edge(DependencyEdge {
            from: PackageId::new("app-pkg", "0"),
            to: PackageId::new("dep-pkg", "0"),
            kind: DependencyKind::Runtime,
            condition_flag: None,
        });

        let plan = graph.topological_sort("app-pkg")?;

        // 1. Eksekusi default: dep-pkg harus di-skip karena sudah terpasang
        let scheduler = WavefrontScheduler::new(config.clone(), Some(2));
        let summary = scheduler.execute(&graph, &plan, &target_root, &db)?;

        assert_eq!(summary.total_packages, 2);
        assert_eq!(summary.compiled_packages, 1);
        assert_eq!(summary.skipped_packages, 1);
        assert!(target_root.join("usr/bin/app-pkg").exists());

        // 2. Eksekusi dengan rebuild_deps = true: dep-pkg harus ikut dikompilasi ulang
        let scheduler_rebuild = WavefrontScheduler::with_options(config, Some(2), true, false);
        let summary_rebuild = scheduler_rebuild.execute(&graph, &plan, &target_root, &db)?;

        assert_eq!(summary_rebuild.total_packages, 2);
        assert_eq!(summary_rebuild.compiled_packages, 2);
        assert_eq!(summary_rebuild.skipped_packages, 0);

        Ok(())
    }
}
