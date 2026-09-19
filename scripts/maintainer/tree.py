#!/usr/bin/env python3
"""
Dependency Tree Visualizer - ASCII tree visualizer with depth controls and root discovery.
"""

from typing import Optional, Set, Tuple, List
from .catalog import MaintainerCatalog, CYAN, RED, BOLD, GRAY, RESET


class DependencyTreeVisualizer:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def get_root_and_leaf_packages(self) -> Tuple[List[str], List[str]]:
        roots = []
        leaves = []
        for name, rec in self.catalog.recipes.items():
            if not self.catalog.reverse_deps.get(name):
                roots.append(name)
            if not rec.all_deps:
                leaves.append(name)
        return sorted(roots), sorted(leaves)

    def print_ascii_tree(self, target: str, prefix: str = "", is_last: bool = True, depth: int = 0, max_depth: int = 4, visited: Optional[Set[str]] = None) -> None:
        target_lower = target.strip().lower()
        if target_lower in ("all", "@world", "world", "*"):
            roots, _ = self.get_root_and_leaf_packages()
            print(f"{BOLD}{CYAN}Pohon Dependensi Global (@world — {len(roots)} Root Packages):{RESET}")
            for idx, r in enumerate(roots):
                self.print_ascii_tree(r, "", idx == len(roots) - 1, 0, max_depth)
            return

        if visited is None:
            visited = set()

        rec = self.catalog.recipes.get(target)
        tag = f"{CYAN}{target}{RESET}" if rec else f"{RED}{target} (MISSING){RESET}"
        ver_tag = f" {GRAY}v{rec.version} [{rec.category}]{RESET}" if rec else ""

        connector = "└── " if is_last else "├── "
        if depth == 0:
            print(f"{BOLD}{target}{RESET}{ver_tag}")
        else:
            print(f"{prefix}{connector}{tag}{ver_tag}")

        if depth >= max_depth or target in visited or not rec:
            return

        visited.add(target)
        children = sorted(list(rec.all_deps))
        new_prefix = prefix + ("    " if is_last else "│   ")
        for i, child in enumerate(children):
            self.print_ascii_tree(child, new_prefix, i == len(children) - 1, depth + 1, max_depth, set(visited))
