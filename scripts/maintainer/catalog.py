#!/usr/bin/env python3
"""
Maintainer Catalog - In-memory recipe catalog and reverse-dependency indexer.
"""

import os
import re
import tomllib
from collections import defaultdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

# Base paths
WORKSPACE_ROOT = Path(__file__).resolve().parent.parent.parent
RECIPES_DIR = WORKSPACE_ROOT / "recipes"
PACKAGE_STATUS_FILE = RECIPES_DIR / "PACKAGE_STATUS.md"

CATEGORIES = ["system", "core", "extra"]
CATEGORY_DESCS = {
    "system": "Toolchain Sistem, Kernel Headers, C Library & Inisialisasi OpenRC",
    "core": "Utilitas Dasar, CLI Tools, Filesystem, Networking & Service Daemons",
    "extra": "Bahasa Pemrograman, Desktop Environment, Library Grafis, Audio & Qt6/KDE",
}

# ANSI Color Codes
CYAN = "\033[96m"
GREEN = "\033[92m"
YELLOW = "\033[93m"
RED = "\033[91m"
MAGENTA = "\033[95m"
BLUE = "\033[94m"
BOLD = "\033[1m"
GRAY = "\033[90m"
RESET = "\033[0m"

USE_COND_REGEX = re.compile(r"^\[\??([a-zA-Z0-9_\-]+)\]\s*(.+)$")


def parse_clean_dep_name(dep_token: str) -> str:
    """Normalize dependency string removing slots, version constraints, and USE conditions."""
    token = dep_token.strip()
    match = USE_COND_REGEX.match(token)
    if match:
        token = match.group(2).strip()

    token = re.split(r"[><=~]", token)[0].strip()
    if ":" in token:
        token = token.split(":")[0].strip()
    return token


class RecipeRecord:
    def __init__(self, name: str, version: str, release: int, slot: str, category: str, path: Path, data: Dict[str, Any]):
        self.name = name
        self.version = version
        self.release = release
        self.slot = slot
        self.category = category
        self.path = path
        self.data = data
        self.description = data.get("package", {}).get("description", "").strip()
        self.upstream = data.get("package", {}).get("upstream", "").strip()
        self.license = data.get("package", {}).get("license", "").strip()

        deps = data.get("dependencies", {})
        self.raw_runtime_deps = deps.get("runtime", [])
        self.raw_build_deps = deps.get("build", [])

        self.runtime_deps = [parse_clean_dep_name(d) for d in self.raw_runtime_deps if parse_clean_dep_name(d)]
        self.build_deps = [parse_clean_dep_name(d) for d in self.raw_build_deps if parse_clean_dep_name(d)]
        self.all_deps = set(self.runtime_deps + self.build_deps)

        sources = data.get("sources", {})
        self.source_urls = sources.get("urls", [])
        self.source_sha256 = sources.get("sha256", [])


class MaintainerCatalog:
    def __init__(self, recipes_dir: Path = RECIPES_DIR):
        self.recipes_dir = recipes_dir
        self.recipes: Dict[str, RecipeRecord] = {}
        self.reverse_deps: Dict[str, Set[str]] = defaultdict(set)
        self.load_all_recipes()

    def load_all_recipes(self) -> None:
        self.recipes.clear()
        self.reverse_deps.clear()

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
                    version = str(pkg.get("version", "1.0.0"))
                    release = int(pkg.get("release", 1))
                    slot = str(pkg.get("slot", "0"))
                    category = pkg.get("category", cat)

                    record = RecipeRecord(name, version, release, slot, category, recipe_file, data)
                    self.recipes[name] = record

                    for dep in record.all_deps:
                        self.reverse_deps[dep].add(name)
                except Exception as e:
                    print(f"{RED}Error loading {recipe_file}: {e}{RESET}")

    def find_matching_packages(self, query: str) -> List[str]:
        q = query.strip().lower()
        if not q:
            return []
        exact = [name for name in self.recipes if name.lower() == q]
        if exact:
            return exact
        starts = [name for name in self.recipes if name.lower().startswith(q)]
        contains = [name for name in self.recipes if q in name.lower() and name not in starts]
        return starts + contains

    def find_reverse_dependencies(self, target_pkg: str) -> List[str]:
        return sorted(list(self.reverse_deps.get(target_pkg, set())))

    def modify_package_dependency(self, pkg_name: str, dep_name: str, action: str = "add", kind: str = "runtime") -> bool:
        rec = self.recipes.get(pkg_name)
        if not rec or not rec.path.exists():
            return False

        content = rec.path.read_text(encoding="utf-8")
        deps_list = list(rec.raw_runtime_deps if kind == "runtime" else rec.raw_build_deps)

        if action == "add":
            if dep_name not in deps_list:
                deps_list.append(dep_name)
        elif action == "remove":
            deps_list = [d for d in deps_list if parse_clean_dep_name(d) != dep_name and d != dep_name]

        new_deps_str = ", ".join(f'"{d}"' for d in deps_list)
        pattern = rf'({kind}\s*=\s*\[)[^\]]*(\])'
        new_content = re.sub(pattern, rf'\1{new_deps_str}\2', content)

        rec.path.write_text(new_content, encoding="utf-8")
        self.load_all_recipes()
        return True
