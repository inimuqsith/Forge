use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::{BuildMeta, DependenciesMeta, PackageMeta, Recipe, SourcesMeta};

/// Engine Pengimpor Resep dari Upstream (Arch Linux PKGBUILD / Alpine APKBUILD) ke Kura Linux (recipe.toml)
pub struct RecipeImporter;

impl RecipeImporter {
    /// Impor dari berkas lokal atau string konten langsung
    pub fn import_from_file_or_content(input: &str) -> Result<String> {
        let content = if Path::new(input).exists() {
            fs::read_to_string(input)
                .with_context(|| format!("Gagal membaca berkas sumber dari {}", input))?
        } else {
            input.to_string()
        };

        let recipe = Self::parse_pkgbuild(&content)?;
        Self::to_toml_string(&recipe)
    }

    /// Impor dari berkas PKGBUILD lokal dan simpan ke file tujuan recipe.toml
    pub fn import_and_save(pkgbuild_path: &Path, output_recipe_path: &Path) -> Result<()> {
        let content = fs::read_to_string(pkgbuild_path)
            .with_context(|| format!("Gagal membaca PKGBUILD dari {:?}", pkgbuild_path))?;
        let recipe = Self::parse_pkgbuild(&content)?;
        let toml_str = Self::to_toml_string(&recipe)?;

        if let Some(parent) = output_recipe_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_recipe_path, toml_str)
            .with_context(|| format!("Gagal menyimpan recipe.toml ke {:?}", output_recipe_path))?;
        Ok(())
    }

    /// Impor dari URL hulu (misal GitLab Arch Linux / Alpine aports)
    pub async fn import_from_url(url: &str) -> Result<Recipe> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let resp = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Gagal mengunduh PKGBUILD dari {}", url))?;
        let text = resp
            .text()
            .await
            .with_context(|| "Gagal membaca body response PKGBUILD")?;
        Self::parse_pkgbuild(&text)
    }

    /// Parse konten deklaratif PKGBUILD / APKBUILD ke struktur data Recipe Kura Linux
    pub fn parse_pkgbuild(content: &str) -> Result<Recipe> {
        let mut vars = HashMap::new();
        let mut arrays = HashMap::new();

        // 1. Ekstraksi variabel dan array bash dari konten
        Self::extract_variables_and_arrays(content, &mut vars, &mut arrays);

        // 2. Ekstraksi metadata dasar paket
        let raw_pkgname = vars
            .get("pkgname")
            .cloned()
            .or_else(|| arrays.get("pkgname").and_then(|a| a.first().cloned()))
            .unwrap_or_else(|| "unknown".to_string());
        let pkgname = Self::clean_value(&raw_pkgname);

        let raw_pkgver = vars.get("pkgver").cloned().unwrap_or_else(|| "1.0.0".to_string());
        let pkgver = Self::clean_value(&raw_pkgver);

        let pkgrel = vars
            .get("pkgrel")
            .and_then(|r| Self::clean_value(r).parse::<u32>().ok())
            .unwrap_or(1);

        let raw_pkgdesc = vars
            .get("pkgdesc")
            .cloned()
            .unwrap_or_else(|| format!("Package {} for Kura Linux", pkgname));
        let pkgdesc = Self::expand_variables(&Self::clean_value(&raw_pkgdesc), &vars);

        let raw_url = vars
            .get("url")
            .cloned()
            .unwrap_or_else(|| format!("https://archlinux.org/packages/{}", pkgname));
        let url = Self::expand_variables(&Self::clean_value(&raw_url), &vars);

        let license_raw = vars
            .get("license")
            .cloned()
            .or_else(|| arrays.get("license").and_then(|a| a.first().cloned()))
            .unwrap_or_else(|| "GPL-3.0-or-later".to_string());
        let license = Self::clean_value(&license_raw);

        let slot = Self::determine_slot(&pkgname, &pkgver);

        // 3. Ekstraksi & Normalisasi Dependensi
        let mut runtime_deps = Vec::new();
        if let Some(deps) = arrays.get("depends") {
            for dep in deps {
                let expanded = Self::expand_variables(dep, &vars);
                let normalized = Self::normalize_dependency(&expanded);
                if !normalized.is_empty() && !runtime_deps.contains(&normalized) {
                    runtime_deps.push(normalized);
                }
            }
        }

        let mut build_deps = Vec::new();
        if let Some(mdeps) = arrays.get("makedepends") {
            for mdep in mdeps {
                let expanded = Self::expand_variables(mdep, &vars);
                let normalized = Self::normalize_dependency(&expanded);
                if !normalized.is_empty() && !build_deps.contains(&normalized) {
                    build_deps.push(normalized);
                }
            }
        }

        // 4. Ekstraksi Sources & Checksum
        let mut source_urls = Vec::new();
        if let Some(srcs) = arrays.get("source").or_else(|| arrays.get("sources")) {
            for src in srcs {
                let expanded = Self::expand_variables(src, &vars);
                let cleaned = Self::clean_value(&expanded);
                // Filter hanya URL HTTP/HTTPS/FTP atau tarball
                if cleaned.starts_with("http://")
                    || cleaned.starts_with("https://")
                    || cleaned.starts_with("ftp://")
                {
                    // Tangani format "filename::url"
                    let actual_url = if let Some((_, u)) = cleaned.split_once("::") {
                        u.to_string()
                    } else {
                        cleaned
                    };
                    source_urls.push(actual_url);
                }
            }
        }

        let mut sha256_list = Vec::new();
        if let Some(sums) = arrays.get("sha256sums") {
            for sum in sums {
                let cleaned = Self::clean_value(sum);
                if cleaned != "SKIP" && cleaned.len() == 64 {
                    sha256_list.push(cleaned);
                }
            }
        }

        // 5. Ekstraksi & Transpilasi Fungsi Bash (prepare, build, package)
        let functions = Self::extract_bash_functions(content);
        let (build_type, transpiled_script) =
            Self::transpile_build_functions(&pkgname, &pkgver, &functions);

        let recipe = Recipe {
            package: PackageMeta {
                name: pkgname,
                version: pkgver,
                release: pkgrel,
                slot,
                description: pkgdesc,
                url: url.clone(),
                license,
                upstream: url,
            },
            dependencies: Some(DependenciesMeta {
                runtime: runtime_deps,
                build: build_deps,
            }),
            sources: Some(SourcesMeta {
                urls: source_urls,
                sha256: sha256_list,
            }),
            build: Some(BuildMeta {
                r#type: build_type,
                configure_args: vec![],
                script: transpiled_script,
                compiler_override: None,
                disable_custom_march: false,
            }),
        };

        Ok(recipe)
    }

    /// Format struktur data Recipe menjadi string TOML yang valid dan rapi
    pub fn to_toml_string(recipe: &Recipe) -> Result<String> {
        let mut out = String::new();

        out.push_str("[package]\n");
        out.push_str(&format!("name = \"{}\"\n", recipe.package.name));
        out.push_str(&format!("version = \"{}\"\n", recipe.package.version));
        out.push_str(&format!("release = {}\n", recipe.package.release));
        out.push_str(&format!("slot = \"{}\"\n", recipe.package.slot));
        out.push_str(&format!("description = \"{}\"\n", recipe.package.description.replace('\"', "\\\"")));
        out.push_str(&format!("license = \"{}\"\n", recipe.package.license));
        out.push_str(&format!("upstream = \"{}\"\n\n", recipe.package.upstream));

        if let Some(ref deps) = recipe.dependencies {
            out.push_str("[dependencies]\n");
            out.push_str("runtime = [\n");
            for r in &deps.runtime {
                out.push_str(&format!("    \"{}\",\n", r));
            }
            out.push_str("]\n");
            out.push_str("build = [\n");
            for b in &deps.build {
                out.push_str(&format!("    \"{}\",\n", b));
            }
            out.push_str("]\n\n");
        }

        if let Some(ref srcs) = recipe.sources {
            out.push_str("[sources]\n");
            out.push_str("urls = [\n");
            for u in &srcs.urls {
                out.push_str(&format!("    \"{}\",\n", u));
            }
            out.push_str("]\n");
            out.push_str("sha256 = [\n");
            for s in &srcs.sha256 {
                out.push_str(&format!("    \"{}\",\n", s));
            }
            out.push_str("]\n\n");
        }

        if let Some(ref bld) = recipe.build {
            out.push_str("[build]\n");
            out.push_str(&format!("type = \"{}\"\n", bld.r#type));
            if let Some(ref comp) = bld.compiler_override {
                out.push_str(&format!("compiler_override = \"{}\"\n", comp));
            }
            if bld.disable_custom_march {
                out.push_str("disable_custom_march = true\n");
            }
            out.push_str("\nscript = \"\"\"\n");
            let clean_script = bld.script.trim();
            out.push_str(clean_script);
            out.push_str("\n\"\"\"\n");
        }

        Ok(out)
    }

    /// Ekstraksi variabel skalar dan array bash dari teks PKGBUILD
    fn extract_variables_and_arrays(
        content: &str,
        vars: &mut HashMap<String, String>,
        arrays: &mut HashMap<String, Vec<String>>,
    ) {
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        let var_re = Regex::new(r#"^([a-zA-Z_][a-zA-Z0-9_]*)=(.*)$"#).unwrap();
        let array_start_re = Regex::new(r#"^([a-zA-Z_][a-zA-Z0-9_]*)=\((.*)$"#).unwrap();

        while i < lines.len() {
            let line = lines[i].trim();
            // Lewati komentar dan baris kosong
            if line.starts_with('#') || line.is_empty() {
                i += 1;
                continue;
            }

            // Cek apakah deklarasi fungsi bash, jika ya lewati hingga kurung kurawal selesai
            if (line.contains("() {") || line.contains("() ") || line.starts_with("function ")) && line.contains('{') {
                let mut brace_depth = 1;
                i += 1;
                while i < lines.len() && brace_depth > 0 {
                    let sub = lines[i];
                    brace_depth += sub.matches('{').count();
                    brace_depth = brace_depth.saturating_sub(sub.matches('}').count());
                    i += 1;
                }
                continue;
            }

            // Cek deklarasi array: var=(...)
            if let Some(caps) = array_start_re.captures(line) {
                let name = caps.get(1).unwrap().as_str().to_string();
                let rest = caps.get(2).unwrap().as_str();

                let mut array_content = String::new();
                if let Some(end_pos) = rest.rfind(')') {
                    array_content.push_str(&rest[..end_pos]);
                    i += 1;
                } else {
                    array_content.push_str(rest);
                    array_content.push(' ');
                    i += 1;
                    while i < lines.len() {
                        let sub = lines[i].trim();
                        if let Some(end_pos) = sub.find(')') {
                            array_content.push_str(&sub[..end_pos]);
                            i += 1;
                            break;
                        } else {
                            if !sub.starts_with('#') {
                                array_content.push_str(sub);
                                array_content.push(' ');
                            }
                            i += 1;
                        }
                    }
                }

                let items = Self::tokenize_bash_array(&array_content);
                arrays.insert(name, items);
                continue;
            }

            // Cek variabel skalar: var=val
            if let Some(caps) = var_re.captures(line) {
                let name = caps.get(1).unwrap().as_str().to_string();
                let mut val = caps.get(2).unwrap().as_str().trim().to_string();

                // Bersihkan quote penutup jika ada
                val = Self::clean_value(&val);
                vars.insert(name, val);
                i += 1;
                continue;
            }

            i += 1;
        }
    }

    /// Tokenisasi elemen-elemen di dalam array bash `('elem1' "elem2" elem3)`
    fn tokenize_bash_array(raw: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut cur = String::new();
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut chars = raw.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '\'' if !in_double_quote => {
                    in_single_quote = !in_single_quote;
                }
                '"' if !in_single_quote => {
                    in_double_quote = !in_double_quote;
                }
                ' ' | '\t' | '\n' if !in_single_quote && !in_double_quote => {
                    if !cur.is_empty() {
                        tokens.push(cur.clone());
                        cur.clear();
                    }
                }
                '#' if !in_single_quote && !in_double_quote => {
                    // Abaikan sisa komentar baris
                    while let Some(&next_c) = chars.peek() {
                        chars.next();
                        if next_c == '\n' {
                            break;
                        }
                    }
                }
                _ => {
                    cur.push(c);
                }
            }
        }
        if !cur.is_empty() {
            tokens.push(cur);
        }

        tokens
            .into_iter()
            .map(|s| Self::clean_value(&s))
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Ekstraksi isi fungsi-fungsi bash (prepare, build, package)
    fn extract_bash_functions(content: &str) -> HashMap<String, String> {
        let mut functions = HashMap::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        let func_re = Regex::new(r#"^(?:function\s+)?([a-zA-Z0-9_]+)\s*\(\)\s*\{?"#).unwrap();

        while i < lines.len() {
            let line = lines[i].trim();
            if let Some(caps) = func_re.captures(line) {
                let name = caps.get(1).unwrap().as_str().to_string();
                let mut body = String::new();
                let mut brace_depth = line.matches('{').count();

                if brace_depth == 0 && i + 1 < lines.len() && lines[i + 1].trim().starts_with('{') {
                    i += 1;
                    brace_depth = 1;
                }

                i += 1;
                while i < lines.len() && brace_depth > 0 {
                    let cur_line = lines[i];
                    brace_depth += cur_line.matches('{').count();
                    let close_count = cur_line.matches('}').count();
                    if brace_depth <= close_count {
                        // Ambil baris sebelum kurung tutup terakhir
                        if let Some(pos) = cur_line.rfind('}') {
                            let prefix = &cur_line[..pos];
                            if !prefix.trim().is_empty() {
                                body.push_str(prefix);
                                body.push('\n');
                            }
                        }
                        i += 1;
                        break;
                    } else {
                        brace_depth -= close_count;
                        body.push_str(cur_line);
                        body.push('\n');
                        i += 1;
                    }
                }

                functions.insert(name, body);
            } else {
                i += 1;
            }
        }

        functions
    }

    /// Transpilasi fungsi build, prepare, package menjadi script tunggal Forge
    fn transpile_build_functions(
        pkgname: &str,
        pkgver: &str,
        functions: &HashMap<String, String>,
    ) -> (String, String) {
        let mut combined_script = String::new();

        if let Some(prep) = functions.get("prepare") {
            combined_script.push_str("# --- Prepare Step ---\n");
            combined_script.push_str(&Self::transpile_script_block(prep));
            combined_script.push('\n');
        }

        if let Some(bld) = functions.get("build") {
            combined_script.push_str("# --- Build Step ---\n");
            combined_script.push_str(&Self::transpile_script_block(bld));
            combined_script.push('\n');
        }

        if let Some(pkg) = functions.get("package") {
            combined_script.push_str("# --- Package Step ---\n");
            combined_script.push_str(&Self::transpile_script_block(pkg));
            combined_script.push('\n');
        }

        // Jika tidak ada fungsi yang diekstraksi, berikan template standar
        if combined_script.trim().is_empty() {
            combined_script = format!(
                r#"cd "${{srcdir}}/{pkgname}-{pkgver}"
./configure --prefix=/usr
make ${{MAKEFLAGS}}
make DESTDIR="${{DESTDIR}}" install"#
            );
        }

        // Tentukan build system type
        let script_lower = combined_script.to_lowercase();
        let build_type = if script_lower.contains("meson setup") || script_lower.contains("meson compile") {
            "meson".to_string()
        } else if script_lower.contains("cmake") {
            "cmake".to_string()
        } else if script_lower.contains("cargo build") {
            "cargo".to_string()
        } else if script_lower.contains("./configure") || script_lower.contains("configure") {
            "autotools".to_string()
        } else if script_lower.contains("make") || script_lower.contains("ninja") {
            "make".to_string()
        } else {
            "custom".to_string()
        };

        (build_type, combined_script.trim().to_string())
    }

    /// Transpilasi baris skrip: transposisi $pkgdir -> ${DESTDIR}, $srcdir -> ${srcdir}
    fn transpile_script_block(script: &str) -> String {
        let mut result = String::new();
        for line in script.lines() {
            let mut transpiled = line.to_string();

            // Ganti $pkgdir dan ${pkgdir} menjadi "${DESTDIR}" atau $FORGE_DESTDIR
            transpiled = transpiled.replace("\"$pkgdir\"", "\"${DESTDIR}\"");
            transpiled = transpiled.replace("\"${pkgdir}\"", "\"${DESTDIR}\"");
            transpiled = transpiled.replace("$pkgdir", "${DESTDIR}");
            transpiled = transpiled.replace("${pkgdir}", "${DESTDIR}");

            // Ganti $FORGE_DESTDIR jika ada
            transpiled = transpiled.replace("$FORGE_DESTDIR", "${DESTDIR}");

            result.push_str(&transpiled);
            result.push('\n');
        }
        result
    }

    /// Normalisasi nama dependensi (membersihkan versi dan soname library)
    pub fn normalize_dependency(raw: &str) -> String {
        let cleaned = Self::clean_value(raw);
        if cleaned.is_empty() {
            return String::new();
        }

        // 1. Bersihkan batasan versi: >=8.0, <=5.2, =1.0, ~2.0, dsb.
        let name_part = cleaned
            .split(&['>', '<', '=', '~', ':'][..])
            .next()
            .unwrap_or(&cleaned)
            .trim();

        // 2. Pemetaan soname library ke paket riil Kura Linux
        let mapped = match name_part {
            "libssl.so" | "libcrypto.so" => "openssl",
            "libcurl.so" => "curl",
            "libz.so" => "zlib",
            "libzstd.so" => "zstd",
            "liblzma.so" => "xz",
            "libxml2.so" => "libxml2",
            "libxslt.so" => "libxslt",
            "libsqlite3.so" => "sqlite",
            "libreadline.so" => "readline",
            "libncurses.so" | "libncursesw.so" => "ncurses",
            "libarchive.so" => "libarchive",
            "libffi.so" => "libffi",
            "libpcre2-8.so" | "libpcre2.so" => "pcre2",
            "libglib-2.0.so" => "glib2",
            "libgmp.so" => "gmp",
            "libmpfr.so" => "mpfr",
            "libmpc.so" => "mpc",
            "libcap.so" => "libcap",
            "libseccomp.so" => "libseccomp",
            "gcc-libs" => "gcc",
            "python-setuptools" | "python-pip" => "python",
            "coreutils-single" => "coreutils",
            other => other,
        };

        mapped.to_string()
    }

    /// Ekspansi variabel $var dan ${var} di dalam string
    fn expand_variables(input: &str, vars: &HashMap<String, String>) -> String {
        let mut result = input.to_string();
        for (k, v) in vars {
            result = result.replace(&format!("${{{}}}", k), v);
            result = result.replace(&format!("${}", k), v);
        }
        result
    }

    /// Bersihkan tanda kutip ganda atau tunggal di sekitar string
    fn clean_value(s: &str) -> String {
        let trimmed = s.trim();
        let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            if trimmed.len() >= 2 {
                &trimmed[1..trimmed.len() - 1]
            } else {
                trimmed
            }
        } else {
            trimmed
        };
        unquoted.trim().to_string()
    }

    /// Tentukan slot default berdasarkan nama dan versi
    fn determine_slot(pkgname: &str, pkgver: &str) -> String {
        match pkgname {
            "llvm" | "clang" => {
                pkgver.split('.').next().unwrap_or("0").to_string()
            }
            "gcc" => {
                pkgver.split('.').next().unwrap_or("0").to_string()
            }
            "python" => {
                let parts: Vec<&str> = pkgver.split('.').collect();
                if parts.len() >= 2 {
                    format!("{}.{}", parts[0], parts[1])
                } else {
                    "3".to_string()
                }
            }
            _ => "0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pkgbuild_metadata() {
        let sample_pkgbuild = r#"
# Maintainer: Arch Linux
pkgname=bash
pkgver=5.2.37
pkgrel=1
pkgdesc="GNU Bourne Again SHell"
arch=('x86_64')
url="https://www.gnu.org/software/bash/"
license=('GPL-3.0-or-later')
depends=('readline>=8.2' 'ncurses')
makedepends=('bison' 'make')
source=("https://ftp.gnu.org/gnu/bash/bash-${pkgver}.tar.gz")
sha256sums=('9547b85e054bfd27572714c68ff0cfd66eb1f3c311400ba601736b7a5a80d463')

build() {
    cd "$pkgname-$pkgver"
    ./configure --prefix=/usr --with-installed-readline
    make
}

package() {
    cd "$pkgname-$pkgver"
    make DESTDIR="$pkgdir" install
}
"#;

        let recipe = RecipeImporter::parse_pkgbuild(sample_pkgbuild).expect("Gagal mem-parsing PKGBUILD");
        assert_eq!(recipe.package.name, "bash");
        assert_eq!(recipe.package.version, "5.2.37");
        assert_eq!(recipe.package.release, 1);
        assert_eq!(recipe.package.license, "GPL-3.0-or-later");
        assert_eq!(recipe.package.upstream, "https://www.gnu.org/software/bash/");

        let deps = recipe.dependencies.expect("Dependencies kosong");
        assert!(deps.runtime.contains(&"readline".to_string()));
        assert!(deps.runtime.contains(&"ncurses".to_string()));
        assert!(deps.build.contains(&"bison".to_string()));

        let srcs = recipe.sources.expect("Sources kosong");
        assert_eq!(srcs.urls.len(), 1);
        assert_eq!(srcs.urls[0], "https://ftp.gnu.org/gnu/bash/bash-5.2.37.tar.gz");
        assert_eq!(srcs.sha256[0], "9547b85e054bfd27572714c68ff0cfd66eb1f3c311400ba601736b7a5a80d463");
    }

    #[test]
    fn test_transpile_build_package_steps() {
        let sample_pkgbuild = r#"
pkgname=nano
pkgver=8.2
pkgrel=1
pkgdesc="Pico editor clone with enhancements"
url="https://nano-editor.org"
license=('GPL-3.0-or-later')
depends=('ncurses')

build() {
    cd "${pkgname}-${pkgver}"
    ./configure --prefix=/usr --sysconfdir=/etc --enable-utf8
    make
}

package() {
    cd "${pkgname}-${pkgver}"
    make DESTDIR="${pkgdir}" install
    install -Dm644 doc/sample.nanorc "${pkgdir}/etc/nanorc"
}
"#;

        let recipe = RecipeImporter::parse_pkgbuild(sample_pkgbuild).expect("Gagal parse nano PKGBUILD");
        let build = recipe.build.expect("Build meta kosong");
        assert_eq!(build.r#type, "autotools");
        assert!(build.script.contains("make DESTDIR=\"${DESTDIR}\" install"));
        assert!(build.script.contains("install -Dm644 doc/sample.nanorc \"${DESTDIR}/etc/nanorc\""));
        assert!(!build.script.contains("$pkgdir"));
        assert!(!build.script.contains("${pkgdir}"));
    }

    #[test]
    fn test_dependency_normalization() {
        assert_eq!(RecipeImporter::normalize_dependency("readline>=8.2"), "readline");
        assert_eq!(RecipeImporter::normalize_dependency("libssl.so"), "openssl");
        assert_eq!(RecipeImporter::normalize_dependency("libcrypto.so"), "openssl");
        assert_eq!(RecipeImporter::normalize_dependency("gcc-libs=15.2.0"), "gcc");
        assert_eq!(RecipeImporter::normalize_dependency("python-setuptools"), "python");
        assert_eq!(RecipeImporter::normalize_dependency("coreutils-single"), "coreutils");
    }

    #[test]
    fn test_validate_all_recipes_in_repo_are_valid_toml() {
        let scanner_roots = [
            Path::new("recipes"),
            Path::new("../recipes"),
            Path::new("../../recipes"),
        ];

        let found_root = scanner_roots.iter().find(|p| p.is_dir());
        if let Some(root) = found_root {
            let mut validated_count = 0;
            for category in &["system", "core", "extra"] {
                let cat_dir = root.join(category);
                if cat_dir.is_dir() {
                    for entry in fs::read_dir(&cat_dir).unwrap().flatten() {
                        let recipe_file = entry.path().join("recipe.toml");
                        if recipe_file.is_file() {
                            let content = fs::read_to_string(&recipe_file)
                                .unwrap_or_else(|_| panic!("Gagal membaca {:?}", recipe_file));
                            let recipe: Recipe = toml::from_str(&content)
                                .unwrap_or_else(|e| panic!("Sintaks recipe.toml invalid di {:?}: {}", recipe_file, e));
                            assert!(!recipe.package.name.is_empty());
                            assert!(!recipe.package.version.is_empty());
                            validated_count += 1;
                        }
                    }
                }
            }
            println!("Validasi sukses: {} berkas recipe.toml valid.", validated_count);
            assert!(validated_count > 0);
        }
    }
}
