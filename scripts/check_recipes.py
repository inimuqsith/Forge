#!/usr/bin/env python3
"""
Forge Recipe Matrix Linter & Safety Validator
Author: Kura Linux Maintainers
Description:
    Static analysis and matrix validation tool for recipe.toml files.
    Verifies schema validity, dependency graph completeness, cycle freedom,
    build script safety (DESTDIR compliance), and cross-machine portability.
"""

import os
import sys
import re
import argparse
import json
from pathlib import Path
from typing import Dict, List, Set, Tuple, Any, Optional

try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print("Error: Python 3.11+ (with built-in tomllib) or 'tomli' package is required.")
        sys.exit(1)

# ANSI Terminal Colors
RESET = "\033[0m"
BOLD = "\033[1m"
RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
BLUE = "\033[34m"
MAGENTA = "\033[35m"
CYAN = "\033[36m"
GRAY = "\033[90m"

CATEGORIES = ["system", "core", "extra"]
HEX64_REGEX = re.compile(r"^[a-fA-F0-9]{64}$")
USE_COND_REGEX = re.compile(r"^[!]?[a-zA-Z0-9_\-+]+\?\s*\(\s*(.+)\s*\)$")

class RecipeCheckResult:
    def __init__(self, pkg_name: str, category: str, path: Path):
        self.pkg_name = pkg_name
        self.category = category
        self.path = path
        self.errors: List[str] = []
        self.warnings: List[str] = []
        self.info: List[str] = []
        self.is_valid_toml = False
        self.recipe_data: Optional[Dict[str, Any]] = None

    @property
    def status(self) -> str:
        if self.errors:
            return "FAIL"
        if self.warnings:
            return "WARN"
        return "PASS"


def parse_clean_dep_name(dep_token: str) -> str:
    """Normalize dependency string removing slots, version constraints, and USE conditions."""
    token = dep_token.strip()
    # Check conditional USE flag syntax: flag? ( dep )
    match = USE_COND_REGEX.match(token)
    if match:
        token = match.group(1).strip()

    # Strip version comparisons (>=, <=, =, >, <)
    token = re.split(r"[><=~]", token)[0].strip()

    # Strip slot specifications (:slot)
    if ":" in token:
        token = token.split(":")[0].strip()

    return token


class RecipeChecker:
    def __init__(self, recipes_dir: Path):
        self.recipes_dir = recipes_dir
        self.results: Dict[str, RecipeCheckResult] = {}
        self.pkg_index: Dict[str, Path] = {}
        self.pkg_data: Dict[str, Dict[str, Any]] = {}

    def scan_and_load(self) -> None:
        """Scan recipes directory and parse all recipe.toml files."""
        for cat in CATEGORIES:
            cat_dir = self.recipes_dir / cat
            if not cat_dir.is_dir():
                continue

            for pkg_dir in sorted(cat_dir.iterdir()):
                if not pkg_dir.is_dir():
                    continue

                recipe_file = pkg_dir / "recipe.toml"
                pkg_name = pkg_dir.name
                res = RecipeCheckResult(pkg_name, cat, recipe_file)

                if not recipe_file.is_file():
                    res.errors.append(f"Missing recipe.toml in directory {pkg_dir}")
                    self.results[pkg_name] = res
                    continue

                try:
                    with open(recipe_file, "rb") as f:
                        data = tomllib.load(f)
                    res.is_valid_toml = True
                    res.recipe_data = data
                    self.pkg_index[pkg_name] = recipe_file
                    self.pkg_data[pkg_name] = data
                except Exception as e:
                    res.errors.append(f"TOML Syntax Error: {e}")

                self.results[pkg_name] = res

    def validate_schema_and_metadata(self, res: RecipeCheckResult) -> None:
        """Validate recipe schema, naming, versioning, license, and required fields."""
        data = res.recipe_data
        if not data:
            return

        # 1. Package table
        pkg = data.get("package")
        if not isinstance(pkg, dict):
            res.errors.append("Missing required table '[package]'")
            return

        name = pkg.get("name")
        if not name:
            res.errors.append("Missing 'package.name'")
        elif name != res.pkg_name:
            res.errors.append(f"Package name mismatch: folder is '{res.pkg_name}' but recipe defines '{name}'")
        elif not re.match(r"^[a-z0-9][a-z0-9_\-+.]*$", name):
            res.warnings.append(f"Package name '{name}' contains non-standard characters (recommended: lowercase alphanumeric with dash)")

        version = pkg.get("version")
        if not version:
            res.errors.append("Missing 'package.version'")
        elif not isinstance(version, str):
            res.errors.append("'package.version' must be a string")

        release = pkg.get("release")
        if release is None:
            res.warnings.append("Missing 'package.release' (defaulting to 1)")
        elif not isinstance(release, int) or release < 1:
            res.errors.append("'package.release' must be a positive integer >= 1")

        description = pkg.get("description")
        if not description:
            res.warnings.append("Missing 'package.description'")

        license_name = pkg.get("license")
        if not license_name:
            res.warnings.append("Missing 'package.license'")

        # 2. Sources validation
        sources = data.get("sources", {})
        if isinstance(sources, dict):
            urls = sources.get("urls", [])
            sha256_list = sources.get("sha256", [])

            if isinstance(urls, list) and isinstance(sha256_list, list):
                if len(urls) != len(sha256_list):
                    res.errors.append(f"Source count mismatch: {len(urls)} URLs but {len(sha256_list)} SHA256 checksums")

                for idx, sha in enumerate(sha256_list):
                    if not isinstance(sha, str) or not HEX64_REGEX.match(sha):
                        res.errors.append(f"Invalid SHA256 hash at index {idx}: '{sha}' (must be 64 hexadecimal characters)")

        # 3. Build & Package steps safety
        build_tab = data.get("build", {})
        is_meta = pkg.get("meta", False)

        if not is_meta and isinstance(build_tab, dict):
            build_steps = build_tab.get("steps", [])
            pkg_steps = data.get("package", {}).get("steps", [])

            # Check for DESTDIR safety during installation phase
            all_steps_text = "\n".join(build_steps + pkg_steps)

            # Check if installation attempts to write directly to /usr without $DESTDIR
            risky_direct_writes = re.findall(r"(?:cp|install|mv)\s+.*?\s+/(?:usr|etc|bin|lib|opt|sbin)\b", all_steps_text)
            for risky in risky_direct_writes:
                if "$DESTDIR" not in risky and "${DESTDIR}" not in risky and "$FORGE_ROOT" not in risky:
                    res.warnings.append(f"Potential un-staged write detected: '{risky}' (ensure $DESTDIR is used)")

            # Check for hardcoded non-portable CPU flags (except in glibc)
            if res.pkg_name != "glibc":
                if "-march=" in all_steps_text:
                    res.warnings.append("Hardcoded '-march=' compiler flag detected in build script. Leave architecture flags to Forge engine.")
                if "-mtune=" in all_steps_text:
                    res.warnings.append("Hardcoded '-mtune=' compiler flag detected in build script.")

    def validate_dependency_references(self, res: RecipeCheckResult) -> None:
        """Validate that all declared dependencies exist in the recipe tree."""
        data = res.recipe_data
        if not data:
            return

        deps_table = data.get("dependencies", {})
        if not isinstance(deps_table, dict):
            return

        all_declared_deps: List[Tuple[str, str]] = []

        # Run-time dependencies
        runtime_deps = deps_table.get("depends", [])
        if isinstance(runtime_deps, list):
            for d in runtime_deps:
                all_declared_deps.append((str(d), "depends"))

        # Build-time dependencies
        make_deps = deps_table.get("makedepends", [])
        if isinstance(make_deps, list):
            for d in make_deps:
                all_declared_deps.append((str(d), "makedepends"))

        for raw_dep, kind in all_declared_deps:
            clean_name = parse_clean_dep_name(raw_dep)
            if not clean_name:
                continue

            # Virtual / meta package aliases or core providers
            if clean_name not in self.pkg_index:
                res.errors.append(f"Missing {kind} target: '{clean_name}' (from '{raw_dep}') is not defined in any recipe")

    def detect_circular_dependencies(self) -> List[List[str]]:
        """Detect circular dependency cycles in the package graph using DFS tracer."""
        adj: Dict[str, List[str]] = {}
        for pkg, data in self.pkg_data.items():
            deps_table = data.get("dependencies", {})
            pkg_deps = []
            if isinstance(deps_table, dict):
                for d in deps_table.get("depends", []) + deps_table.get("makedepends", []):
                    clean = parse_clean_dep_name(str(d))
                    if clean and clean in self.pkg_data and clean != pkg:
                        pkg_deps.append(clean)
            adj[pkg] = pkg_deps

        visited: Dict[str, int] = {}  # 0: unvisited, 1: visiting, 2: visited
        cycles: List[List[str]] = []

        def dfs(node: str, path: List[str]):
            visited[node] = 1
            path.append(node)

            for neighbor in adj.get(node, []):
                if visited.get(neighbor, 0) == 1:
                    # Found cycle
                    cycle_start = path.index(neighbor)
                    cycles.append(path[cycle_start:] + [neighbor])
                elif visited.get(neighbor, 0) == 0:
                    dfs(neighbor, path)

            path.pop()
            visited[node] = 2

        for node in self.pkg_data:
            if visited.get(node, 0) == 0:
                dfs(node, [])

        return cycles

    def run_all_checks(self, target_pkg: Optional[str] = None) -> bool:
        """Run complete linting suite across recipes."""
        self.scan_and_load()

        if target_pkg:
            if target_pkg not in self.results:
                print(f"{RED}Error: Paket '{target_pkg}' tidak ditemukan di {self.recipes_dir}{RESET}")
                return False
            targets = [target_pkg]
        else:
            targets = list(self.results.keys())

        # Validate each recipe
        for pkg in targets:
            res = self.results[pkg]
            if res.is_valid_toml:
                self.validate_schema_and_metadata(res)
                self.validate_dependency_references(res)

        # Check circular dependencies globally
        cycles = self.detect_circular_dependencies()
        if cycles:
            for cycle in cycles:
                cycle_str = " -> ".join(cycle)
                for node in cycle[:-1]:
                    if node in self.results:
                        self.results[node].errors.append(f"Circular dependency cycle detected: {cycle_str}")

        return True

    def print_terminal_report(self, verbose: bool = False) -> int:
        """Format and print colorized diagnostic summary."""
        print(f"\n{BOLD}{CYAN}══════════════════════════════════════════════════════════════════════{RESET}")
        print(f"{BOLD}{CYAN}   FORGE RECIPE MATRIX LINTER & COMPATIBILITY AUDITOR (Python)        {RESET}")
        print(f"{BOLD}{CYAN}══════════════════════════════════════════════════════════════════════{RESET}\n")

        total = len(self.results)
        passed = sum(1 for r in self.results.values() if r.status == "PASS")
        warned = sum(1 for r in self.results.values() if r.status == "WARN")
        failed = sum(1 for r in self.results.values() if r.status == "FAIL")

        for cat in CATEGORIES:
            cat_pkgs = [r for r in self.results.values() if r.category == cat]
            if not cat_pkgs:
                continue

            print(f"{BOLD}{BLUE}Category: [{cat.upper()}]{RESET} ({len(cat_pkgs)} packages)")
            print(f"{GRAY}{'─' * 70}{RESET}")

            for res in sorted(cat_pkgs, key=lambda x: x.pkg_name):
                status_badge = f"{GREEN}[PASS]{RESET}" if res.status == "PASS" else (f"{YELLOW}[WARN]{RESET}" if res.status == "WARN" else f"{RED}[FAIL]{RESET}")
                
                version_str = ""
                if res.recipe_data and "package" in res.recipe_data:
                    version_str = f"v{res.recipe_data['package'].get('version', '?')}"

                print(f"  {status_badge} {BOLD}{res.pkg_name:<24}{RESET} {GRAY}{version_str:<12}{RESET} ({res.path.relative_to(self.recipes_dir)})")

                if res.errors:
                    for err in res.errors:
                        print(f"       {RED}✗ ERROR:{RESET} {err}")
                if res.warnings and (verbose or res.status != "PASS"):
                    for warn in res.warnings:
                        print(f"       {YELLOW}⚠ WARN:{RESET}  {warn}")

            print()

        # Summary box
        print(f"{BOLD}{CYAN}──────────────────────────────────────────────────────────────────────{RESET}")
        print(f"{BOLD}SUMMARY:{RESET}")
        print(f"  Total Recipes Scanned : {BOLD}{total}{RESET}")
        print(f"  Valid & Safe (PASS)   : {GREEN}{BOLD}{passed}{RESET}")
        print(f"  Warnings (WARN)       : {YELLOW}{BOLD}{warned}{RESET}")
        print(f"  Failed (FAIL)         : {RED}{BOLD}{failed}{RESET}")
        print(f"{BOLD}{CYAN}──────────────────────────────────────────────────────────────────────{RESET}\n")

        if failed > 0:
            print(f"{RED}{BOLD}✗ Status: AUDIT GAGAL! Ditemukan {failed} resep dengan kesalahan fatal.{RESET}\n")
            return 1
        elif warned > 0:
            print(f"{YELLOW}{BOLD}⚠ Status: AUDIT LULUS DENGAN PERINGATAN ({warned} warnings).{RESET}\n")
            return 0
        else:
            print(f"{GREEN}{BOLD}✓ Status: SEMPURNA! 100% Resep valid, aman, dan siap dikompilasi.{RESET}\n")
            return 0

    def export_json(self) -> str:
        """Export analysis report to JSON format."""
        out = {
            "total": len(self.results),
            "passed": sum(1 for r in self.results.values() if r.status == "PASS"),
            "warned": sum(1 for r in self.results.values() if r.status == "WARN"),
            "failed": sum(1 for r in self.results.values() if r.status == "FAIL"),
            "recipes": {}
        }
        for name, res in self.results.items():
            out["recipes"][name] = {
                "category": res.category,
                "status": res.status,
                "path": str(res.path),
                "errors": res.errors,
                "warnings": res.warnings
            }
        return json.dumps(out, indent=2)


def main():
    parser = argparse.ArgumentParser(description="Forge Recipe Matrix Linter & Safety Validator")
    parser.add_argument(
        "--recipes-path",
        type=Path,
        default=Path("recipes"),
        help="Path ke direktori recipes (default: ./recipes)"
    )
    parser.add_argument(
        "--package",
        type=str,
        help="Audit hanya paket tertentu berdasarkan nama"
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Tampilkan seluruh peringatan rinci"
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output dalam format JSON"
    )
    parser.add_argument(
        "--fail-on-warn",
        action="store_true",
        help="Anggap warning sebagai failure (exit code 1)"
    )

    args = parser.parse_args()

    # Resolve recipes path
    recipes_dir = args.recipes_path
    if not recipes_dir.exists():
        # Fallback to absolute or relative repo root
        repo_root = Path(__file__).resolve().parent.parent / "recipes"
        if repo_root.exists():
            recipes_dir = repo_root
        else:
            print(f"{RED}Error: Direktori recipes tidak ditemukan di {args.recipes_path}{RESET}")
            sys.exit(1)

    checker = RecipeChecker(recipes_dir)
    checker.run_all_checks(target_pkg=args.package)

    if args.json:
        print(checker.export_json())
        sys.exit(0)

    exit_code = checker.print_terminal_report(verbose=args.verbose)
    if args.fail_on_warn and sum(1 for r in checker.results.values() if r.status == "WARN") > 0:
        sys.exit(1)

    sys.exit(exit_code)


if __name__ == "__main__":
    main()
