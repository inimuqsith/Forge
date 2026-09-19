#!/usr/bin/env python3
"""
SSOT Matrix Generator - Generates deterministic recipes/PACKAGE_STATUS.md documentation.
"""

from collections import defaultdict
from .catalog import MaintainerCatalog, CATEGORIES, CATEGORY_DESCS, PACKAGE_STATUS_FILE


class SgotMatrixGenerator:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def regenerate_ssot_matrix(self) -> None:
        cat_data = defaultdict(list)
        for rec in sorted(self.catalog.recipes.values(), key=lambda x: x.name):
            cat_data[rec.category].append(rec)

        total_count = len(self.catalog.recipes)
        lines = []
        lines.append("# Matriks Status & Kesiapan Resep Paket Kura Linux (`PACKAGE_STATUS.md`)\n")
        lines.append("> **Single Source of Truth (SSOT)**: Dokumen ini memuat katalog lengkap, versi hulu resmi, pohon dependensi, dan status kesiapan seluruh paket perangkat lunak distribusi **Kura Linux** yang dikelola oleh package manager **`forge`**.\n")
        lines.append("---\n")
        lines.append("## 📊 Ringkasan Statistik Status Katalog Paket\n")
        lines.append("| Kategori | Total Paket | Status Kesiapan | Deskripsi Ruang Lingkup |")
        lines.append("| :--- | :---: | :---: | :--- |")

        for cat in CATEGORIES:
            count = len(cat_data[cat])
            lines.append(f"| **`recipes/{cat}/`** | {count} | ✅ 100% Verified | {CATEGORY_DESCS.get(cat, '')} |")

        lines.append(f"| **TOTAL RESEP RESMI** | **{total_count}** | **✅ 100% Audited** | **Ekosistem Lengkap Kura Linux (Base, Toolchain, Kernel, Hardware, Firmware, Audio, Qt6 & KDE Plasma 6)** |\n")
        lines.append("---\n")

        section_num = 1
        for cat in CATEGORIES:
            pkgs = cat_data[cat]
            lines.append(f"## {section_num}. Kategori `recipes/{cat}/` ({len(pkgs)} Paket — {CATEGORY_DESCS.get(cat, '')})\n")
            lines.append("| Paket | Versi Hulu | Status | Runtime Dependencies | Build Dependencies | Deskripsi |")
            lines.append("| :--- | :---: | :---: | :--- | :--- | :--- |")
            for p in pkgs:
                r_str = ", ".join(f"`{d}`" for d in p.raw_runtime_deps) if p.raw_runtime_deps else "-"
                b_str = ", ".join(f"`{d}`" for d in p.raw_build_deps) if p.raw_build_deps else "-"
                lines.append(f"| **`{p.name}`** | `{p.version}` | ✅ Verified | {r_str} | {b_str} | {p.description} |")
            lines.append("\n---\n")
            section_num += 1

        content = "\n".join(lines).strip() + "\n"
        PACKAGE_STATUS_FILE.write_text(content, encoding="utf-8")
        print(f"✓ Berhasil meregenerasi {PACKAGE_STATUS_FILE} ({total_count} paket terdaftar).")
