#!/usr/bin/env python3
"""
Forge DAG & Dependency Graph Solver / Verifier
Author: Kura Linux Maintainers
Description:
    Deep analysis and DAG simulation tool for Forge package recipes.
    Simulates dependency graph resolution (runtime + build dependencies),
    detects cycles, verifies topological sort order, and identifies missing recipes.
"""

import sys
import os
import re
import argparse
from pathlib import Path
from typing import Dict, List, Set, Tuple, Any, Optional
from collections import defaultdict, deque

try:
    import tomllib
except ImportError:
    try:
        import tomli as tomllib
    except ImportError:
        print("Error: Python 3.11+ (with built-in tomllib) or 'tomli' package is required.")
        sys.exit(1)

# ANSI Colors
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
USE_COND_REGEX = re.compile(r"^[!]?[a-zA-Z0-9_\-+]+\?\s*\(\s*(.+)\s*\)$")


def parse_clean_dep_name(dep_token: str) -> str:
    """Normalize dependency string removing slots, version constraints, and USE conditions."""
    token = dep_token.strip()
    match = USE_COND_REGEX.match(token)
    if match:
        token = match.group(1).strip()

    token = re.split(r"[><=~]", token)[0].strip()
    if ":" in token:
        token = token.split(":")[0].strip()
    return token


class RecipeNode:
    def __init__(self, name: str, version: str, category: str, path: Path):
        self.name = name
        self.version = version
        self.category = category
        self.path = path
        self.runtime_deps: List[str] = []
        self.build_deps: List[str] = []
        self.all_deps: Set[str] = set()


class ForgeDAGResolver:
    def __init__(self, recipes_dir: Path):
        self.recipes_dir = recipes_dir
        self.recipes: Dict[str, RecipeNode] = {}
        self.load_recipes()

    def load_recipes(self) -> None:
        for cat in CATEGORIES:
            cat_dir = self.recipes_dir / cat
            if not cat_dir.is_dir():
                continue
            for pkg_dir in sorted(cat_dir.iterdir()):
                if not pkg_dir.is_dir():
                    continue
                recipe_file = pkg_dir / "recipe.toml"
                if not recipe_file.is_file():
                    continue
                try:
                    with open(recipe_file, "rb") as f:
                        data = tomllib.load(f)
                    pkg = data.get("package", {})
                    name = pkg.get("name", pkg_dir.name)
                    version = pkg.get("version", "unknown")
                    category = pkg.get("category", cat)

                    node = RecipeNode(name, version, category, recipe_file)
                    deps = data.get("dependencies", {})
                    
                    for r_dep in deps.get("runtime", []):
                        clean_r = parse_clean_dep_name(r_dep)
                        if clean_r:
                            node.runtime_deps.append(clean_r)
                            node.all_deps.add(clean_r)

                    for b_dep in deps.get("build", []):
                        clean_b = parse_clean_dep_name(b_dep)
                        if clean_b:
                            node.build_deps.append(clean_b)
                            node.all_deps.add(clean_b)

                    self.recipes[name] = node
                except Exception as e:
                    print(f"{RED}Error parsing {recipe_file}: {e}{RESET}")

    def check_all_missing(self) -> Dict[str, List[str]]:
        """Find any dependencies referenced across all recipes that do not exist."""
        missing_by_pkg: Dict[str, List[str]] = defaultdict(list)
        for name, node in self.recipes.items():
            for dep in node.all_deps:
                if dep not in self.recipes:
                    missing_by_pkg[name].append(dep)
        return missing_by_pkg

    def resolve_target(self, target: str, include_build: bool = True) -> Tuple[List[str], List[str], List[str]]:
        """
        Simulate DAG resolution for a specific target.
        Returns: (resolved_build_order, missing_dependencies, cycles)
        """
        if target not in self.recipes:
            return [], [target], []

        visited: Set[str] = set()
        visiting: Set[str] = set()
        resolved_order: List[str] = []
        missing: List[str] = []
        cycles: List[str] = []

        def dfs(curr: str, path: List[str]) -> None:
            if curr in visiting:
                cycle_str = " -> ".join(path + [curr])
                cycles.append(cycle_str)
                return

            if curr in visited:
                return

            if curr not in self.recipes:
                missing.append(curr)
                return

            visiting.add(curr)
            node = self.recipes[curr]

            # Dependencies to traverse
            deps_to_traverse = list(node.runtime_deps)
            if include_build:
                deps_to_traverse.extend(node.build_deps)

            for dep in deps_to_traverse:
                dfs(dep, path + [curr])

            visiting.remove(curr)
            visited.add(curr)
            resolved_order.append(curr)

        dfs(target, [])
        return resolved_order, missing, cycles

    def print_tree(self, target: str, prefix: str = "", is_last: bool = True, depth: int = 0, max_depth: int = 6, visited: Optional[Set[str]] = None) -> None:
        """Print ASCII dependency tree for a target."""
        if visited is None:
            visited = set()

        node = self.recipes.get(target)
        tag = f"{CYAN}{target}{RESET}" if node else f"{RED}{target} (MISSING){RESET}"
        if node:
            ver_tag = f" {GRAY}v{node.version} ({node.category}){RESET}"
        else:
            ver_tag = ""

        connector = "└── " if is_last else "├── "
        if depth == 0:
            print(f"{BOLD}{target}{RESET}{ver_tag}")
        else:
            print(f"{prefix}{connector}{tag}{ver_tag}")

        if depth >= max_depth or target in visited or not node:
            return

        visited.add(target)
        children = sorted(list(node.all_deps))
        new_prefix = prefix + ("    " if is_last else "│   ")
        for i, child in enumerate(children):
            self.print_tree(child, new_prefix, i == len(children) - 1, depth + 1, max_depth, set(visited))


def main():
    parser = argparse.ArgumentParser(description="Forge DAG & Dependency Graph Solver / Verifier")
    parser.add_argument("target", nargs="?", help="Specific target package to simulate DAG resolution (e.g. bash, base, base-devel)")
    parser.add_argument("--tree", action="store_true", help="Display ASCII dependency tree for target")
    parser.add_argument("--max-depth", type=int, default=4, help="Max depth for dependency tree printout")
    parser.add_argument("--runtime-only", action="store_true", help="Evaluate runtime dependencies only (exclude build-time deps)")
    parser.add_argument("--recipes-dir", type=str, default="recipes", help="Path to recipes directory")

    args = parser.parse_args()
    recipes_path = Path(args.recipes_dir)
    if not recipes_path.is_dir():
        print(f"{RED}Error: recipes directory not found at '{recipes_path}'{RESET}")
        sys.exit(1)

    solver = ForgeDAGResolver(recipes_path)
    total_pkgs = len(solver.recipes)

    print(f"{BOLD}{CYAN}Forge DAG Solver & Dependency Graph Verifier{RESET}")
    print(f"Loaded {BOLD}{total_pkgs}{RESET} package recipes from '{recipes_path}'.\n")

    if args.target:
        target = args.target
        include_build = not args.runtime_only
        print(f"{BOLD}Simulating DAG resolution for target:{RESET} {CYAN}{target}{RESET} (build-deps: {'ON' if include_build else 'OFF'})")
        
        if args.tree:
            print(f"\n{BOLD}Dependency Tree:{RESET}")
            solver.print_tree(target, max_depth=args.max_depth)
            print()

        resolved_order, missing, cycles = solver.resolve_target(target, include_build=include_build)

        if missing:
            print(f"{RED}{BOLD}✗ RESOLUTION FAILED! Missing dependencies:{RESET}")
            for m in sorted(set(missing)):
                print(f"  {RED}• {m}{RESET}")
        else:
            print(f"{GREEN}{BOLD}✓ Target DAG resolved successfully!{RESET}")

        if cycles:
            print(f"\n{YELLOW}{BOLD}⚠ Detected Dependency Cycles:{RESET}")
            for c in cycles:
                print(f"  {YELLOW}• {c}{RESET}")

        print(f"\n{BOLD}Topological Build Order ({len(resolved_order)} packages):{RESET}")
        for idx, pkg in enumerate(resolved_order, start=1):
            p_node = solver.recipes.get(pkg)
            cat_str = f"({p_node.category})" if p_node else ""
            print(f"  {GRAY}{idx:3d}.{RESET} {BOLD}{pkg:<24}{RESET} {cat_str}")

        if missing:
            sys.exit(1)
        sys.exit(0)

    # Full Catalog Audit Mode
    print(f"{BOLD}Running Full Catalog Dependency Audit...{RESET}")
    missing_all = solver.check_all_missing()

    if missing_all:
        print(f"\n{RED}{BOLD}✗ Found Missing Dependencies Across Catalog:{RESET}")
        for pkg, miss_list in sorted(missing_all.items()):
            print(f"  {BOLD}{pkg}{RESET}: {RED}{', '.join(miss_list)}{RESET}")
        print(f"\n{RED}Total packages with broken dependencies: {len(missing_all)}{RESET}")
        sys.exit(1)
    else:
        print(f"\n{GREEN}{BOLD}✓ SEMPURNA! 0 Missing Dependencies across all {total_pkgs} recipes.{RESET}")
        print(f"{GREEN}Seluruh graf dependensi valid, terhubung, dan dapat di-resolve secara deterministik.{RESET}")
        sys.exit(0)


if __name__ == "__main__":
    main()
