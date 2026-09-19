use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use version_compare::Cmp;

use crate::{ForgeConfig, Recipe, UseFlagsEngine};

/// Batasan versi dependensi paket (misal: ">=2.3.0", "<=5.0", "=1.0", "~1.2")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionConstraint {
    pub raw: String,
    pub op: String,
    pub target_version: String,
}

impl VersionConstraint {
    /// Parsing string dependensi menjadi (PackageId, Option<VersionConstraint>)
    pub fn parse(raw: &str) -> (PackageId, Option<Self>) {
        let trimmed = raw.trim();
        let ops = [">=", "<=", "==", "!=", ">", "<", "=", "~"];

        // Cek operator infix seperti "openssl>=3.0.0"
        for op in ops {
            if let Some(pos) = trimmed.find(op) {
                let pkg_part = trimmed[..pos].trim();
                let ver_part = trimmed[pos + op.len()..].trim();
                if !pkg_part.is_empty() && !ver_part.is_empty() {
                    let pkg_id = PackageId::parse(pkg_part);
                    return (
                        pkg_id,
                        Some(Self {
                            raw: trimmed.to_string(),
                            op: op.to_string(),
                            target_version: ver_part.to_string(),
                        }),
                    );
                }
            }
        }

        // Cek jika diawali operator prefix seperti ">=gcc-15"
        for op in ops {
            if trimmed.starts_with(op) {
                let rest = trimmed[op.len()..].trim();
                let pkg_id = PackageId::parse(rest);
                return (
                    pkg_id,
                    Some(Self {
                        raw: trimmed.to_string(),
                        op: op.to_string(),
                        target_version: rest.to_string(),
                    }),
                );
            }
        }

        (PackageId::parse(trimmed), None)
    }

    /// Evaluasi apakah sebuah versi kandidat memenuhi batasan versi ini
    pub fn is_satisfied_by(&self, candidate_version: &str) -> bool {
        match self.op.as_str() {
            ">=" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Ge).unwrap_or(false),
            "<=" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Le).unwrap_or(false),
            ">" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Gt).unwrap_or(false),
            "<" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Lt).unwrap_or(false),
            "=" | "==" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Eq).unwrap_or(false),
            "!=" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Ne).unwrap_or(false),
            "~" => version_compare::compare_to(candidate_version, &self.target_version, Cmp::Ge).unwrap_or(false),
            _ => true,
        }
    }
}

/// Tipe relasi dependensi antar paket
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Wajib tersedia saat kompilasi (makedepends)
    Build,
    /// Wajib tersedia saat runtime & dynamic linking (depends)
    Runtime,
    /// Dependensi anggota meta-paket
    MetaMember,
}

impl fmt::Display for DependencyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyKind::Build => write!(f, "build"),
            DependencyKind::Runtime => write!(f, "runtime"),
            DependencyKind::MetaMember => write!(f, "meta-member"),
        }
    }
}

/// Identitas unik paket di dalam graf
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PackageId {
    pub name: String,
    pub slot: String,
}

impl PackageId {
    pub fn new(name: impl Into<String>, slot: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            slot: slot.into(),
        }
    }

    /// Parse identifier seperti "glibc", "sys-libs/glibc:0", "gcc:15", atau ">=gcc-15"
    pub fn parse(raw: &str) -> Self {
        let trimmed = raw.trim();
        // Bersihkan version constraint prefix jika ada (misal >=, <=, =, ~, ^)
        let cleaned = trimmed
            .trim_start_matches(|c| c == '>' || c == '<' || c == '=' || c == '~' || c == '^')
            .trim();

        // Ekstraksi jika ada slot (name:slot)
        if let Some((name_part, slot_part)) = cleaned.split_once(':') {
            let pure_name = name_part.split('/').last().unwrap_or(name_part);
            Self::new(pure_name, slot_part)
        } else {
            let pure_name = cleaned.split('/').last().unwrap_or(cleaned);
            // Bersihkan version suffix jika formatnya name-1.0 (kecuali base-devel)
            Self::new(pure_name, "0")
        }
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.slot == "0" || self.slot.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}:{}", self.name, self.slot)
        }
    }
}

/// Node dalam DAG yang merepresentasikan sebuah paket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageNode {
    pub id: PackageId,
    pub version: String,
    pub release: u32,
    pub description: String,
    pub recipe_path: PathBuf,
    pub is_meta: bool,
    pub active_use_flags: HashSet<String>,
    pub is_installed: bool,
    pub installed_version: Option<String>,
}

/// Edge terarah yang merepresentasikan relasi ketergantungan (from membutuhkan to)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: PackageId,
    pub to: PackageId,
    pub kind: DependencyKind,
    pub condition_flag: Option<String>,
}

/// Model Graf Asiklis Terarah (DAG)
#[derive(Debug, Default, Clone)]
pub struct DependencyGraph {
    pub nodes: HashMap<PackageId, PackageNode>,
    /// Daftar tetangga (Package -> Daftar paket yang dibutuhkannya)
    pub dependencies_of: HashMap<PackageId, Vec<DependencyEdge>>,
    /// Daftar kebalikan (Package -> Daftar paket yang bergantung padanya)
    pub dependents_of: HashMap<PackageId, Vec<PackageId>>,
    /// Derajat masuk untuk algoritma Kahn (jumlah dependensi yang belum selesai)
    pub in_degrees: HashMap<PackageId, usize>,
}

/// Satu langkah kompilasi/instalasi dalam antrean eksekusi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    pub step_number: usize,
    pub package_id: PackageId,
    pub version: String,
    pub recipe_path: PathBuf,
    pub is_meta: bool,
    pub build_type: String,
    pub requires_rebuild: bool,
}

/// Rencana eksekusi lengkap hasil resolusi DAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionPlan {
    pub target: String,
    pub steps: Vec<ExecutionStep>,
    pub total_packages: usize,
    pub build_only_count: usize,
    pub runtime_only_count: usize,
}

/// Scanner untuk memindai resep di sistem maupun lokal
#[derive(Debug, Clone, Default)]
pub struct RecipeScanner {
    pub search_roots: Vec<PathBuf>,
    pub recipes_cache: HashMap<PackageId, PathBuf>,
    pub name_index: HashMap<String, Vec<PathBuf>>,
}

impl RecipeScanner {
    /// Inisialisasi scanner dengan search roots standar dan opsional
    pub fn new(custom_roots: Option<Vec<PathBuf>>) -> Self {
        let mut roots = Vec::new();
        if let Some(custom) = custom_roots {
            roots.extend(custom);
        }

        // Lokasi standar sesuai hierarki Forge
        let default_roots = [
            PathBuf::from("/var/db/forge/recipes"),
            PathBuf::from("recipes"),
            PathBuf::from("../recipes"),
            PathBuf::from("../../recipes"),
        ];

        for r in default_roots {
            if r.exists() && !roots.contains(&r) {
                roots.push(r);
            }
        }

        let mut scanner = Self {
            search_roots: roots,
            recipes_cache: HashMap::new(),
            name_index: HashMap::new(),
        };

        let _ = scanner.scan_all();
        scanner
    }

    /// Pindai seluruh subfolder (system, core, extra) dan indeks file recipe.toml secara paralel dengan Rayon
    pub fn scan_all(&mut self) -> Result<()> {
        let categories = ["system", "core", "extra"];
        let mut candidate_files = Vec::new();

        for root in &self.search_roots {
            if !root.exists() {
                continue;
            }

            for cat in &categories {
                let cat_dir = root.join(cat);
                if cat_dir.is_dir() {
                    if let Ok(entries) = fs::read_dir(&cat_dir) {
                        for entry in entries.flatten() {
                            let pkg_dir = entry.path();
                            if pkg_dir.is_dir() {
                                let recipe_file = pkg_dir.join("recipe.toml");
                                if recipe_file.is_file() {
                                    candidate_files.push(recipe_file);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Parsing resep secara paralel di CPU RAM tmpfs menggunakan Rayon
        let parsed_entries: Vec<(PackageId, String, PathBuf)> = candidate_files
            .par_iter()
            .filter_map(|recipe_file| {
                let content = fs::read_to_string(recipe_file).ok()?;
                let parsed = toml::from_str::<Recipe>(&content).ok()?;
                let pkg_id = PackageId::new(&parsed.package.name, &parsed.package.slot);
                Some((pkg_id, parsed.package.name, recipe_file.clone()))
            })
            .collect();

        for (pkg_id, name, recipe_file) in parsed_entries {
            self.recipes_cache.insert(pkg_id, recipe_file.clone());
            self.name_index
                .entry(name)
                .or_default()
                .push(recipe_file);
        }

        Ok(())
    }

    /// Cari lokasi recipe.toml berdasarkan PackageId atau nama string
    pub fn find_recipe(&self, pkg_spec: &str) -> Option<PathBuf> {
        let parsed_id = PackageId::parse(pkg_spec);

        // 1. Cek langsung di cache dengan PackageId
        if let Some(path) = self.recipes_cache.get(&parsed_id) {
            return Some(path.clone());
        }

        // 2. Cek di name index
        if let Some(list) = self.name_index.get(&parsed_id.name) {
            if let Some(path) = list.first() {
                return Some(path.clone());
            }
        }

        // 3. Fallback: Cek path langsung jika target adalah path berkas
        let direct_path = PathBuf::from(pkg_spec);
        if direct_path.is_file() {
            return Some(direct_path);
        }
        if direct_path.is_dir() {
            let r = direct_path.join("recipe.toml");
            if r.is_file() {
                return Some(r);
            }
        }

        // 4. Fallback: Cek kategori umum di search roots
        for root in &self.search_roots {
            for cat in &["system", "core", "extra"] {
                let p = root.join(cat).join(&parsed_id.name).join("recipe.toml");
                if p.is_file() {
                    return Some(p);
                }
            }
        }

        None
    }

    /// Muat dan parsing Recipe dari disk
    pub fn load_recipe(&self, pkg_spec: &str) -> Result<(Recipe, PathBuf)> {
        let recipe_path = self
            .find_recipe(pkg_spec)
            .with_context(|| format!("Resep tidak ditemukan untuk target '{}'", pkg_spec))?;

        let content = fs::read_to_string(&recipe_path)
            .with_context(|| format!("Gagal membaca berkas resep {:?}", recipe_path))?;
        let recipe = toml::from_str::<Recipe>(&content)
            .with_context(|| format!("Gagal mem-parsing sintaks TOML pada {:?}", recipe_path))?;

        Ok((recipe, recipe_path))
    }
}

impl DependencyGraph {
    /// Buat graf dependensi kosong
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluasi USE flags bersyarat pada dependensi
    /// Pola: "flag? ( dep )", "!flag? ( dep )", atau "dep"
    pub fn parse_conditional_dependency(
        dep_str: &str,
        use_engine: &UseFlagsEngine,
    ) -> Option<String> {
        let trimmed = dep_str.trim();
        if let Some(pos) = trimmed.find('?') {
            let (flag, target) = trimmed.split_at(pos);
            let flag = flag.trim();
            let target = target.trim_start_matches('?').trim();
            let target = target.trim_start_matches('(').trim_end_matches(')').trim();

            if let Some(stripped_flag) = flag.strip_prefix('!') {
                if !use_engine.is_enabled(stripped_flag) {
                    return Some(target.to_string());
                }
            } else if use_engine.is_enabled(flag) {
                return Some(target.to_string());
            }
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Tambahkan sebuah node paket ke dalam graf
    pub fn add_node(&mut self, node: PackageNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Tambahkan edge dependensi terarah (from membutuhkan to)
    pub fn add_edge(&mut self, edge: DependencyEdge) {
        self.dependencies_of
            .entry(edge.from.clone())
            .or_default()
            .push(edge.clone());
        self.dependents_of
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    /// Susun urutan eksekusi topologis deterministik menggunakan Algoritma Kahn
    /// Jika terjadi dependensi sirkular, jalankan cycle tracer untuk laporan diagnostik
    pub fn topological_sort(&self, target_name: &str) -> Result<ResolutionPlan> {
        let mut in_degree: HashMap<PackageId, usize> = HashMap::new();

        // Inisialisasi in-degree untuk setiap node.
        // in_degree(u) merepresentasikan jumlah dependensi unik yang harus dibangun sebelum u.
        for id in self.nodes.keys() {
            let count = self
                .dependencies_of
                .get(id)
                .map(|edges| {
                    let unique_deps: HashSet<&PackageId> = edges.iter().map(|e| &e.to).collect();
                    unique_deps.len()
                })
                .unwrap_or(0);
            in_degree.insert(id.clone(), count);
        }

        // Antrean Kahn: Ambil semua node dengan in-degree 0 (tanpa dependensi yang belum selesai)
        let mut queue: VecDeque<PackageId> = VecDeque::new();
        let mut initial_zero_nodes: Vec<PackageId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();
        // Urutkan secara deterministik berdasarkan nama & slot
        initial_zero_nodes.sort();
        for id in initial_zero_nodes {
            queue.push_back(id);
        }

        let mut execution_steps: Vec<ExecutionStep> = Vec::new();
        let mut processed_set: HashSet<PackageId> = HashSet::new();

        while let Some(current_id) = queue.pop_front() {
            if processed_set.contains(&current_id) {
                continue;
            }
            processed_set.insert(current_id.clone());

            let node = match self.nodes.get(&current_id) {
                Some(n) => n,
                None => continue,
            };

            let step = ExecutionStep {
                step_number: execution_steps.len() + 1,
                package_id: node.id.clone(),
                version: node.version.clone(),
                recipe_path: node.recipe_path.clone(),
                is_meta: node.is_meta,
                build_type: if node.is_meta {
                    "meta".to_string()
                } else {
                    "source".to_string()
                },
                requires_rebuild: !node.is_installed,
            };
            execution_steps.push(step);

            // Kurangi in-degree untuk semua paket yang bergantung pada current_id
            if let Some(dependents) = self.dependents_of.get(&current_id) {
                // Kumpulkan dependent yang derajatnya menjadi 0 dan urutkan demi determinisme
                let mut newly_ready = Vec::new();
                for dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        if *deg > 0 {
                            *deg -= 1;
                            if *deg == 0 && !processed_set.contains(dep) {
                                newly_ready.push(dep.clone());
                            }
                        }
                    }
                }
                newly_ready.sort();
                for dep in newly_ready {
                    queue.push_back(dep);
                }
            }
        }

        // Jika tidak semua node berhasil diproses, berarti ada siklus sirkular!
        if execution_steps.len() < self.nodes.len() {
            let unresolved_nodes: HashSet<PackageId> = self
                .nodes
                .keys()
                .filter(|id| !processed_set.contains(*id))
                .cloned()
                .collect();

            let cycle_path = self.find_cycle_path(&unresolved_nodes);
            let diagnostic_msg = self.format_cycle_diagnostic(&cycle_path);
            bail!("{}", diagnostic_msg);
        }

        let total_packages = execution_steps.len();
        let mut build_only_count = 0;
        let mut runtime_only_count = 0;

        for step in &execution_steps {
            if let Some(edges) = self.dependencies_of.get(&step.package_id) {
                for edge in edges {
                    match edge.kind {
                        DependencyKind::Build => build_only_count += 1,
                        DependencyKind::Runtime => runtime_only_count += 1,
                        DependencyKind::MetaMember => {}
                    }
                }
            }
        }

        Ok(ResolutionPlan {
            target: target_name.to_string(),
            steps: execution_steps,
            total_packages,
            build_only_count,
            runtime_only_count,
        })
    }

    /// Telusuri jejak siklus sirkular menggunakan DFS cycle tracer
    fn find_cycle_path(&self, unresolved: &HashSet<PackageId>) -> Vec<PackageId> {
        let mut visited = HashSet::new();
        let mut on_stack = HashSet::new();
        let mut path_stack = Vec::new();

        for start_node in unresolved {
            if !visited.contains(start_node) {
                if let Some(cycle) = self.dfs_cycle(
                    start_node,
                    unresolved,
                    &mut visited,
                    &mut on_stack,
                    &mut path_stack,
                ) {
                    return cycle;
                }
            }
        }

        unresolved.iter().cloned().collect()
    }

    fn dfs_cycle(
        &self,
        current: &PackageId,
        unresolved: &HashSet<PackageId>,
        visited: &mut HashSet<PackageId>,
        on_stack: &mut HashSet<PackageId>,
        path_stack: &mut Vec<PackageId>,
    ) -> Option<Vec<PackageId>> {
        visited.insert(current.clone());
        on_stack.insert(current.clone());
        path_stack.push(current.clone());

        if let Some(edges) = self.dependencies_of.get(current) {
            for edge in edges {
                let neighbor = &edge.to;
                if !unresolved.contains(neighbor) {
                    continue;
                }

                if on_stack.contains(neighbor) {
                    // Siklus terdeteksi! Potong path dari kemunculan neighbor pertama
                    if let Some(pos) = path_stack.iter().position(|id| id == neighbor) {
                        let mut cycle = path_stack[pos..].to_vec();
                        cycle.push(neighbor.clone());
                        return Some(cycle);
                    }
                }

                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_cycle(
                        neighbor,
                        unresolved,
                        visited,
                        on_stack,
                        path_stack,
                    ) {
                        return Some(cycle);
                    }
                }
            }
        }

        path_stack.pop();
        on_stack.remove(current);
        None
    }

    /// Format pesan error diagnostik siklus sirkular yang informatif dan elegan
    fn format_cycle_diagnostic(&self, cycle_path: &[PackageId]) -> String {
        let mut out = String::new();
        out.push_str("\n[!] ERROR: Terdeteksi Siklus Dependensi Sirkular (Circular Dependency Loop)!\n");
        out.push_str("    Pohon dependensi tidak dapat diselesaikan karena terjadi relasi melingkar:\n\n");

        if cycle_path.len() >= 2 {
            let first = &cycle_path[0];
            out.push_str(&format!("    ┌───> [{}]\n", first));

            for i in 0..cycle_path.len().saturating_sub(1) {
                let from = &cycle_path[i];
                let to = &cycle_path[i + 1];

                let kind_str = self
                    .dependencies_of
                    .get(from)
                    .and_then(|edges| edges.iter().find(|e| &e.to == to))
                    .map(|e| e.kind.to_string())
                    .unwrap_or_else(|| "build".to_string());

                let indent = " ".repeat(4 + (i + 1) * 2);
                out.push_str(&format!("    │{}memerlukan ({}): [{}]\n", indent, kind_str, to));
                if i + 2 < cycle_path.len() {
                    out.push_str(&format!("    │{}  └───> [{}]\n", indent, to));
                }
            }

            out.push_str("    └───────────────────────────────────────────────────┘\n\n");
        } else {
            for node in cycle_path {
                out.push_str(&format!("    - Node dalam siklus: [{}]\n", node));
            }
        }

        out.push_str("    Saran Solusi:\n");
        out.push_str("    1. Pastikan dependensi build awal menggunakan compiler seed toolchain (/usr/bin/).\n");
        out.push_str("    2. Pisahkan makedepends sirkular dengan USE flag bootstrap atau pecah resep.\n");

        out
    }
}

/// Engine Resolver Dependensi Utama
pub struct DependencyResolver;

impl DependencyResolver {
    /// Selesaikan seluruh pohon dependensi untuk target yang diminta
    pub fn resolve(
        target: &str,
        config: &ForgeConfig,
        custom_use_engine: Option<&UseFlagsEngine>,
        custom_scanner: Option<&RecipeScanner>,
    ) -> Result<ResolutionPlan> {
        let default_scanner = RecipeScanner::new(Some(vec![PathBuf::from(&config.general.recipes_path)]));
        let scanner = custom_scanner.unwrap_or(&default_scanner);

        let default_use = UseFlagsEngine::new(&config.use_flags.flags, None);
        let use_engine = custom_use_engine.unwrap_or(&default_use);

        let mut graph = DependencyGraph::new();
        let mut visited_packages: HashSet<PackageId> = HashSet::new();

        let initial_target_id = PackageId::parse(target);

        // Rekursif bangun DAG
        Self::build_graph_recursive(
            &initial_target_id,
            &mut graph,
            scanner,
            use_engine,
            &mut visited_packages,
        )?;

        // Lakukan pengurutan topologis
        graph.topological_sort(target)
    }

    /// Rekursif mengurai resep, dependensi build & runtime, dan ekspansi meta-paket
    fn build_graph_recursive(
        pkg_id: &PackageId,
        graph: &mut DependencyGraph,
        scanner: &RecipeScanner,
        use_engine: &UseFlagsEngine,
        visited: &mut HashSet<PackageId>,
    ) -> Result<()> {
        if visited.contains(pkg_id) {
            return Ok(());
        }
        visited.insert(pkg_id.clone());

        let (recipe, recipe_path) = scanner.load_recipe(&pkg_id.name)?;
        let is_meta = recipe.build.as_ref().map(|b| b.r#type == "meta").unwrap_or(false)
            || pkg_id.name == "base"
            || pkg_id.name == "base-devel";

        let mut active_flags = HashSet::new();
        // Cek flag relevan untuk paket jika didefinisikan
        for flag in config_flag_tokens(&recipe) {
            if use_engine.is_enabled(&flag) {
                active_flags.insert(flag);
            }
        }

        let node = PackageNode {
            id: pkg_id.clone(),
            version: recipe.package.version.clone(),
            release: recipe.package.release,
            description: recipe.package.description.clone(),
            recipe_path: recipe_path.clone(),
            is_meta,
            active_use_flags: active_flags,
            is_installed: false,
            installed_version: None,
        };
        graph.add_node(node);

        if let Some(ref deps) = recipe.dependencies {
            // 1. Proses Dependensi Runtime
            for dep_raw in &deps.runtime {
                if let Some(target_dep) = DependencyGraph::parse_conditional_dependency(dep_raw, use_engine) {
                    let dep_id = PackageId::parse(&target_dep);
                    let edge_kind = if is_meta {
                        DependencyKind::MetaMember
                    } else {
                        DependencyKind::Runtime
                    };

                    graph.add_edge(DependencyEdge {
                        from: pkg_id.clone(),
                        to: dep_id.clone(),
                        kind: edge_kind,
                        condition_flag: extract_condition_flag(dep_raw),
                    });

                    Self::build_graph_recursive(&dep_id, graph, scanner, use_engine, visited)?;
                }
            }

            // 2. Proses Dependensi Build (makedepends)
            for dep_raw in &deps.build {
                if let Some(target_dep) = DependencyGraph::parse_conditional_dependency(dep_raw, use_engine) {
                    let dep_id = PackageId::parse(&target_dep);

                    graph.add_edge(DependencyEdge {
                        from: pkg_id.clone(),
                        to: dep_id.clone(),
                        kind: DependencyKind::Build,
                        condition_flag: extract_condition_flag(dep_raw),
                    });

                    Self::build_graph_recursive(&dep_id, graph, scanner, use_engine, visited)?;
                }
            }
        }

        Ok(())
    }
}

fn extract_condition_flag(dep_str: &str) -> Option<String> {
    if let Some(pos) = dep_str.find('?') {
        Some(dep_str[..pos].trim().to_string())
    } else {
        None
    }
}

fn config_flag_tokens(_recipe: &Recipe) -> Vec<String> {
    vec![
        "ssl".to_string(),
        "openrc".to_string(),
        "systemd".to_string(),
        "lto".to_string(),
        "pgo".to_string(),
        "alsa".to_string(),
        "pulseaudio".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_linear_resolution() {
        let mut graph = DependencyGraph::new();

        let node_a = PackageNode {
            id: PackageId::new("A", "0"),
            version: "1.0".into(),
            release: 1,
            description: "Pkg A".into(),
            recipe_path: PathBuf::from("recipes/extra/A/recipe.toml"),
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        let node_b = PackageNode {
            id: PackageId::new("B", "0"),
            version: "1.0".into(),
            release: 1,
            description: "Pkg B".into(),
            recipe_path: PathBuf::from("recipes/extra/B/recipe.toml"),
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        let node_c = PackageNode {
            id: PackageId::new("C", "0"),
            version: "1.0".into(),
            release: 1,
            description: "Pkg C".into(),
            recipe_path: PathBuf::from("recipes/extra/C/recipe.toml"),
            is_meta: false,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_c);

        // A -> B (A butuh B), B -> C (B butuh C)
        // Urutan kompilasi yang benar: C lalu B lalu A
        graph.add_edge(DependencyEdge {
            from: PackageId::new("A", "0"),
            to: PackageId::new("B", "0"),
            kind: DependencyKind::Runtime,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("B", "0"),
            to: PackageId::new("C", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        let plan = graph.topological_sort("A").expect("DAG linear harus valid");
        let step_names: Vec<String> = plan.steps.iter().map(|s| s.package_id.name.clone()).collect();
        assert_eq!(step_names, vec!["C", "B", "A"]);
        assert_eq!(plan.total_packages, 3);
    }

    #[test]
    fn test_dag_diamond_resolution() {
        let mut graph = DependencyGraph::new();

        for name in &["A", "B", "C", "D"] {
            graph.add_node(PackageNode {
                id: PackageId::new(*name, "0"),
                version: "1.0".into(),
                release: 1,
                description: format!("Pkg {}", name),
                recipe_path: PathBuf::from(format!("recipes/extra/{}/recipe.toml", name)),
                is_meta: false,
                active_use_flags: HashSet::new(),
                is_installed: false,
                installed_version: None,
            });
        }

        // Diamond: A -> B, A -> C, B -> D, C -> D
        // Urutan eksekusi: D harus paling pertama, lalu B & C, lalu A paling akhir.
        graph.add_edge(DependencyEdge {
            from: PackageId::new("A", "0"),
            to: PackageId::new("B", "0"),
            kind: DependencyKind::Runtime,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("A", "0"),
            to: PackageId::new("C", "0"),
            kind: DependencyKind::Runtime,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("B", "0"),
            to: PackageId::new("D", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("C", "0"),
            to: PackageId::new("D", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        let plan = graph.topological_sort("A").expect("Diamond DAG harus valid");
        let step_names: Vec<String> = plan.steps.iter().map(|s| s.package_id.name.clone()).collect();

        assert_eq!(step_names[0], "D");
        assert!(step_names[1] == "B" || step_names[1] == "C");
        assert!(step_names[2] == "B" || step_names[2] == "C");
        assert_eq!(step_names[3], "A");
    }

    #[test]
    fn test_dag_cycle_detection() {
        let mut graph = DependencyGraph::new();

        for name in &["glibc", "gcc", "binutils"] {
            graph.add_node(PackageNode {
                id: PackageId::new(*name, "0"),
                version: "1.0".into(),
                release: 1,
                description: format!("Pkg {}", name),
                recipe_path: PathBuf::from(format!("recipes/system/{}/recipe.toml", name)),
                is_meta: false,
                active_use_flags: HashSet::new(),
                is_installed: false,
                installed_version: None,
            });
        }

        // Siklus melingkar: glibc -> gcc -> binutils -> glibc
        graph.add_edge(DependencyEdge {
            from: PackageId::new("glibc", "0"),
            to: PackageId::new("gcc", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("gcc", "0"),
            to: PackageId::new("binutils", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("binutils", "0"),
            to: PackageId::new("glibc", "0"),
            kind: DependencyKind::Runtime,
            condition_flag: None,
        });

        let result = graph.topological_sort("glibc");
        assert!(result.is_err());
        let err_str = format!("{:#}", result.err().unwrap());
        assert!(err_str.contains("Terdeteksi Siklus Dependensi Sirkular"));
    }

    #[test]
    fn test_use_flags_conditional_filtering() {
        let engine_with_ssl = UseFlagsEngine::new("ssl -systemd", None);
        let engine_without_ssl = UseFlagsEngine::new("-ssl systemd", None);

        let dep_ssl = "ssl? ( dev-libs/openssl )";
        let dep_systemd_neg = "!systemd? ( sys-apps/openrc )";

        // Saat ssl aktif:
        assert_eq!(
            DependencyGraph::parse_conditional_dependency(dep_ssl, &engine_with_ssl),
            Some("dev-libs/openssl".to_string())
        );
        // Saat -systemd aktif (!systemd bernilai true):
        assert_eq!(
            DependencyGraph::parse_conditional_dependency(dep_systemd_neg, &engine_with_ssl),
            Some("sys-apps/openrc".to_string())
        );

        // Saat -ssl aktif:
        assert_eq!(
            DependencyGraph::parse_conditional_dependency(dep_ssl, &engine_without_ssl),
            None
        );
        // Saat systemd aktif (!systemd bernilai false):
        assert_eq!(
            DependencyGraph::parse_conditional_dependency(dep_systemd_neg, &engine_without_ssl),
            None
        );
    }

    #[test]
    fn test_meta_package_expansion() {
        let mut graph = DependencyGraph::new();

        // 1. Meta-package node
        let meta_node = PackageNode {
            id: PackageId::new("base-devel", "0"),
            version: "1.0.0".into(),
            release: 1,
            description: "Base development meta-package".into(),
            recipe_path: PathBuf::from("recipes/system/base-devel/recipe.toml"),
            is_meta: true,
            active_use_flags: HashSet::new(),
            is_installed: false,
            installed_version: None,
        };

        // 2. Child nodes
        let toolchain_pkgs = ["linux-headers", "glibc", "binutils", "gcc", "mold"];
        for pkg in &toolchain_pkgs {
            graph.add_node(PackageNode {
                id: PackageId::new(*pkg, "0"),
                version: "1.0.0".into(),
                release: 1,
                description: format!("Toolchain {}", pkg),
                recipe_path: PathBuf::from(format!("recipes/system/{}/recipe.toml", pkg)),
                is_meta: false,
                active_use_flags: HashSet::new(),
                is_installed: false,
                installed_version: None,
            });

            // Hubungkan meta-package ke setiap anggota dengan DependencyKind::MetaMember
            graph.add_edge(DependencyEdge {
                from: PackageId::new("base-devel", "0"),
                to: PackageId::new(*pkg, "0"),
                kind: DependencyKind::MetaMember,
                condition_flag: None,
            });
        }

        // Relasi internal antar anggota: glibc butuh linux-headers, gcc butuh binutils & glibc, mold butuh gcc
        graph.add_edge(DependencyEdge {
            from: PackageId::new("glibc", "0"),
            to: PackageId::new("linux-headers", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("gcc", "0"),
            to: PackageId::new("binutils", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("gcc", "0"),
            to: PackageId::new("glibc", "0"),
            kind: DependencyKind::Runtime,
            condition_flag: None,
        });
        graph.add_edge(DependencyEdge {
            from: PackageId::new("mold", "0"),
            to: PackageId::new("gcc", "0"),
            kind: DependencyKind::Build,
            condition_flag: None,
        });

        graph.add_node(meta_node);

        let plan = graph.topological_sort("base-devel").expect("Meta-package DAG harus valid");
        assert_eq!(plan.target, "base-devel");
        assert_eq!(plan.total_packages, 6);

        // Step terakhir wajib adalah meta-package base-devel itu sendiri
        let last_step = plan.steps.last().unwrap();
        assert_eq!(last_step.package_id.name, "base-devel");
        assert!(last_step.is_meta);

        // linux-headers harus dibangun sebelum glibc
        let idx_headers = plan.steps.iter().position(|s| s.package_id.name == "linux-headers").unwrap();
        let idx_glibc = plan.steps.iter().position(|s| s.package_id.name == "glibc").unwrap();
        let idx_binutils = plan.steps.iter().position(|s| s.package_id.name == "binutils").unwrap();
        let idx_gcc = plan.steps.iter().position(|s| s.package_id.name == "gcc").unwrap();
        let idx_mold = plan.steps.iter().position(|s| s.package_id.name == "mold").unwrap();

        assert!(idx_headers < idx_glibc);
        assert!(idx_binutils < idx_gcc);
        assert!(idx_glibc < idx_gcc);
        assert!(idx_gcc < idx_mold);
    }

    #[test]
    fn test_version_constraint_evaluation() {
        let (id1, c1) = VersionConstraint::parse("openssl>=3.0.0");
        assert_eq!(id1.name, "openssl");
        let constraint1 = c1.expect("Harus menghasilkan VersionConstraint");
        assert_eq!(constraint1.op, ">=");
        assert_eq!(constraint1.target_version, "3.0.0");
        assert!(constraint1.is_satisfied_by("3.0.0"));
        assert!(constraint1.is_satisfied_by("3.4.1"));
        assert!(!constraint1.is_satisfied_by("1.1.1u"));

        let (id2, c2) = VersionConstraint::parse("gcc<=15.0.0");
        assert_eq!(id2.name, "gcc");
        let constraint2 = c2.expect("Harus menghasilkan VersionConstraint");
        assert!(constraint2.is_satisfied_by("14.2.0"));
        assert!(constraint2.is_satisfied_by("15.0.0"));
        assert!(!constraint2.is_satisfied_by("16.0.0"));

        let (id3, c3) = VersionConstraint::parse("zlib!=1.2.11");
        assert_eq!(id3.name, "zlib");
        let constraint3 = c3.expect("Harus menghasilkan VersionConstraint");
        assert!(constraint3.is_satisfied_by("1.3.1"));
        assert!(!constraint3.is_satisfied_by("1.2.11"));

        let (id4, c4) = VersionConstraint::parse("curl");
        assert_eq!(id4.name, "curl");
        assert!(c4.is_none());
    }
}
