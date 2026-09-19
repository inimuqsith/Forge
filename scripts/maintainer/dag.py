#!/usr/bin/env python3
"""
DAG Solver - Dependency graph topological sort with Runtime vs Build modes and ADR-019 seed toolchain handling.
"""

from collections import defaultdict
from typing import Dict, List, Set, Tuple
from .catalog import MaintainerCatalog, CYAN, GREEN, YELLOW, RED, BOLD, GRAY, RESET

# Bootstrap Seed Toolchain (Tier-0 / base-devel / ADR-019)
BOOTSTRAP_TOOLCHAIN = {
    "glibc", "gcc", "binutils", "make", "linux-headers",
    "bison", "flex", "m4", "sed", "gawk", "diffutils", "patch",
    "gmp", "mpfr", "mpc", "zstd", "zlib", "xz", "pkgconf"
}


class DagSolver:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def check_all_missing_dependencies(self) -> Dict[str, List[str]]:
        missing = defaultdict(list)
        for name, rec in self.catalog.recipes.items():
            for dep in rec.all_deps:
                if dep not in self.catalog.recipes:
                    missing[name].append(dep)
        return missing

    def resolve_dag(self, target: str, mode: str = "runtime") -> Tuple[List[str], List[str], List[str]]:
        """
        Resolve DAG topological order.
        mode:
          - "runtime": Pure runtime dependencies (RDEPEND). 100% acyclic target system graph.
          - "build": Full source compilation (BDEPEND + RDEPEND) with ADR-019 bootstrap tier isolation.
        """
        target_lower = target.strip().lower()
        is_global_all = target_lower in ("all", "@world", "world", "*")

        visited: Set[str] = set()
        visiting: Set[str] = set()
        resolved_order: List[str] = []
        missing: List[str] = []
        cycles: List[str] = []

        def dfs(curr: str, path: List[str]):
            if curr in visiting:
                cycle_str = " -> ".join(path + [curr])
                if cycle_str not in cycles:
                    cycles.append(cycle_str)
                return
            if curr in visited:
                return
            if curr not in self.catalog.recipes:
                if curr not in missing:
                    missing.append(curr)
                return

            visiting.add(curr)
            rec = self.catalog.recipes[curr]

            if mode == "runtime":
                deps = list(rec.runtime_deps)
            else:
                deps = list(rec.runtime_deps)
                # In build mode for non-bootstrap packages, pull build deps
                # For bootstrap packages, isolate self-toolchain loop (ADR-019)
                if curr not in BOOTSTRAP_TOOLCHAIN:
                    deps.extend(rec.build_deps)
                else:
                    # For bootstrap packages, only depend on non-cycle toolchain primitives
                    deps.extend([d for d in rec.build_deps if d not in BOOTSTRAP_TOOLCHAIN or d == "linux-headers"])

            for d in deps:
                dfs(d, path + [curr])

            visiting.remove(curr)
            visited.add(curr)
            resolved_order.append(curr)

        if is_global_all:
            for pkg_name in sorted(self.catalog.recipes.keys()):
                dfs(pkg_name, [])
        else:
            if target not in self.catalog.recipes:
                return [], [target], []
            dfs(target, [])

        return resolved_order, missing, cycles
