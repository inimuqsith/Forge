#!/usr/bin/env python3
"""
Recipe Linter - Validates TOML syntax, mandatory metadata, and DESTDIR staging safety.
"""

from typing import List, Tuple
from .catalog import MaintainerCatalog, GREEN, RED, YELLOW, BOLD, RESET


class RecipeLinter:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def lint_all_recipes(self) -> Tuple[int, int, List[str]]:
        passed = 0
        failed = 0
        issues = []

        for name, rec in sorted(self.catalog.recipes.items()):
            pkg_issues = []
            if not rec.description:
                pkg_issues.append("Deskripsi kosong")
            if not rec.upstream:
                pkg_issues.append("URL upstream kosong")
            if not rec.license:
                pkg_issues.append("Lisensi kosong")

            b_script = rec.data.get("build", {}).get("script", "")
            if b_script:
                if "DESTDIR" not in b_script and "install" in b_script and rec.data.get("build", {}).get("type") != "meta":
                    pkg_issues.append("Script build memasang file tanpa variabel ${DESTDIR} (raw host write risk)")

            if pkg_issues:
                failed += 1
                issues.append(f"{RED}✗ {name:<22}{RESET} -> {', '.join(pkg_issues)}")
            else:
                passed += 1

        return passed, failed, issues
