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
            upstream = rec.upstream or rec.data.get("package", {}).get("homepage", "").strip()
            if not upstream:
                pkg_issues.append("URL upstream/homepage kosong")
            if not rec.license:
                pkg_issues.append("Lisensi kosong")

            b_script = rec.data.get("build", {}).get("script", "") or rec.data.get("build", {}).get("install", "")
            if b_script:
                is_meta = rec.data.get("build", {}).get("type") == "meta"
                has_dest = "DESTDIR" in b_script or "pkgdir" in b_script
                if "install" in b_script and not has_dest and not is_meta:
                    pkg_issues.append("Script build memasang file tanpa variabel ${DESTDIR} atau ${pkgdir} (raw host write risk)")

            if pkg_issues:
                failed += 1
                issues.append(f"{RED}✗ {name:<22}{RESET} -> {', '.join(pkg_issues)}")
            else:
                passed += 1

        return passed, failed, issues
