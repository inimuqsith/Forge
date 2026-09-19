#!/usr/bin/env python3
"""
Recipe Inspector - Visual Box-Card Inspector & Live Interactive Dependency Editor.
"""

import time
from .catalog import (
    MaintainerCatalog, parse_clean_dep_name, WORKSPACE_ROOT,
    CYAN, GREEN, YELLOW, RED, BOLD, GRAY, RESET
)
from .sources import SourceVerifier
from .tree import DependencyTreeVisualizer


class RecipeInspector:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog
        self.tree_vis = DependencyTreeVisualizer(catalog)
        self.src_ver = SourceVerifier(catalog)

    def run(self, pkg_name: str) -> None:
        while True:
            rec = self.catalog.recipes.get(pkg_name)
            if not rec:
                print(f"{RED}Paket '{pkg_name}' tidak ditemukan.{RESET}")
                break

            rev_deps = self.catalog.find_reverse_dependencies(pkg_name)
            rev_str = f"{len(rev_deps)} paket ({', '.join(rev_deps[:4])}{'...' if len(rev_deps) > 4 else ''})" if rev_deps else "Tidak ada paket lain yang bergantung langsung"

            # Render Visual Inspector Card
            print(f"\n{CYAN}┌{'─' * 77}┐{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}📦 INSPEKTUR RESEP: {rec.name:<18} (v{rec.version}) [Kategori: {rec.category:<6}] [Slot: {rec.slot}]{RESET}{' ' * 9}{CYAN}│{RESET}")
            print(f"{CYAN}├{'─' * 77}┤{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}📝 Deskripsi  :{RESET} {rec.description[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}📜 Lisensi    :{RESET} {rec.license[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}🌐 Upstream   :{RESET} {rec.upstream[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}📁 Berkas     :{RESET} {str(rec.path.relative_to(WORKSPACE_ROOT))[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}🔄 Digunakan  :{RESET} {rev_str[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}├{'─' * 77}┤{RESET}")

            # Runtime Dependencies
            r_count = len(rec.raw_runtime_deps)
            print(f"{CYAN}│{RESET} {GREEN}{BOLD}🏃 RUNTIME DEPENDENCIES ({r_count} paket):{RESET}{' ' * (50 - len(str(r_count)))}{CYAN}│{RESET}")
            if rec.raw_runtime_deps:
                for idx, r in enumerate(rec.raw_runtime_deps, 1):
                    clean_r = parse_clean_dep_name(r)
                    exists_tag = f"{GREEN}✓{RESET}" if clean_r in self.catalog.recipes else f"{RED}✗ MISSING{RESET}"
                    print(f"{CYAN}│{RESET}   {CYAN}[R{idx}]{RESET} {BOLD}{clean_r:<22}{RESET} {exists_tag:<18}{' ' * 28}{CYAN}│{RESET}")
            else:
                print(f"{CYAN}│{RESET}   {GRAY}(Kosong / Zero Runtime Dependencies — Library Dasar/Root){RESET}{' ' * 19}{CYAN}│{RESET}")

            print(f"{CYAN}│{' ' * 77}│{RESET}")

            # Build Dependencies
            b_count = len(rec.raw_build_deps)
            print(f"{CYAN}│{RESET} {YELLOW}{BOLD}🔨 BUILD DEPENDENCIES ({b_count} paket):{RESET}{' ' * (52 - len(str(b_count)))}{CYAN}│{RESET}")
            if rec.raw_build_deps:
                for idx, b in enumerate(rec.raw_build_deps, 1):
                    clean_b = parse_clean_dep_name(b)
                    exists_tag = f"{GREEN}✓{RESET}" if clean_b in self.catalog.recipes else f"{RED}✗ MISSING{RESET}"
                    print(f"{CYAN}│{RESET}   {YELLOW}[B{idx}]{RESET} {BOLD}{clean_b:<22}{RESET} {exists_tag:<18}{' ' * 28}{CYAN}│{RESET}")
            else:
                print(f"{CYAN}│{RESET}   {GRAY}(Kosong / Tidak ada build dependencies khusus){RESET}{' ' * 30}{CYAN}│{RESET}")

            print(f"{CYAN}├{'─' * 77}┤{RESET}")

            # Sources
            s_url = rec.source_urls[0] if rec.source_urls else "None"
            s_sha = rec.source_sha256[0] if rec.source_sha256 else "None"
            print(f"{CYAN}│{RESET} {BOLD}🔗 SOURCE URL :{RESET} {s_url[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}│{RESET} {BOLD}🔒 SHA256     :{RESET} {s_sha[:60]:<60} {CYAN}│{RESET}")
            print(f"{CYAN}└{'─' * 77}┘{RESET}")

            # Actions Menu
            print(f"\n{BOLD}PILIH AKSI PADA '{pkg_name}':{RESET}")
            print(f"  {CYAN}[1]{RESET} ➕ Tambah Runtime Dependency")
            print(f"  {CYAN}[2]{RESET} ➖ Hapus Runtime Dependency (Pilih nomor R1, R2, dst.)")
            print(f"  {CYAN}[3]{RESET} ➕ Tambah Build Dependency")
            print(f"  {CYAN}[4]{RESET} ➖ Hapus Build Dependency (Pilih nomor B1, B2, dst.)")
            print(f"  {CYAN}[5]{RESET} 🌲 Tampilkan Pohon Dependensi Paket Ini")
            print(f"  {CYAN}[6]{RESET} 🔒 Uji & Verifikasi SHA256 Tarball Hulu")
            print(f"  {CYAN}[0]{RESET} ↩️  Kembali ke Menu Utama")

            act = input(f"\n{BOLD}Pilihan aksi [0-6]: {RESET}").strip()

            if act == "0":
                break
            elif act == "1":
                dep_in = input(f"Ketik nama dependensi runtime yang ingin ditambahkan: ").strip()
                if dep_in:
                    matches = self.catalog.find_matching_packages(dep_in)
                    actual_dep = matches[0] if matches else dep_in
                    self.catalog.modify_package_dependency(pkg_name, actual_dep, action="add", kind="runtime")
                    print(f"{GREEN}✓ Berhasil menambahkan runtime dependency '{actual_dep}'.{RESET}")
            elif act == "2":
                if not rec.raw_runtime_deps:
                    print(f"{YELLOW}Paket ini tidak memiliki runtime dependency.{RESET}")
                    time.sleep(1)
                    continue
                print(f"\n{BOLD}Pilih nomor Runtime Dependency yang ingin dihapus:{RESET}")
                for idx, r in enumerate(rec.raw_runtime_deps, 1):
                    print(f"  {CYAN}[{idx}]{RESET} {r}")
                del_choice = input(f"Masukkan nomor [1-{len(rec.raw_runtime_deps)}] atau 0 untuk batal: ").strip()
                if del_choice.isdigit():
                    d_idx = int(del_choice) - 1
                    if 0 <= d_idx < len(rec.raw_runtime_deps):
                        target_dep = rec.raw_runtime_deps[d_idx]
                        self.catalog.modify_package_dependency(pkg_name, target_dep, action="remove", kind="runtime")
                        print(f"{GREEN}✓ Berhasil menghapus runtime dependency '{target_dep}'.{RESET}")
            elif act == "3":
                dep_in = input(f"Ketik nama dependensi build yang ingin ditambahkan: ").strip()
                if dep_in:
                    matches = self.catalog.find_matching_packages(dep_in)
                    actual_dep = matches[0] if matches else dep_in
                    self.catalog.modify_package_dependency(pkg_name, actual_dep, action="add", kind="build")
                    print(f"{GREEN}✓ Berhasil menambahkan build dependency '{actual_dep}'.{RESET}")
            elif act == "4":
                if not rec.raw_build_deps:
                    print(f"{YELLOW}Paket ini tidak memiliki build dependency.{RESET}")
                    time.sleep(1)
                    continue
                print(f"\n{BOLD}Pilih nomor Build Dependency yang ingin dihapus:{RESET}")
                for idx, b in enumerate(rec.raw_build_deps, 1):
                    print(f"  {YELLOW}[{idx}]{RESET} {b}")
                del_choice = input(f"Masukkan nomor [1-{len(rec.raw_build_deps)}] atau 0 untuk batal: ").strip()
                if del_choice.isdigit():
                    d_idx = int(del_choice) - 1
                    if 0 <= d_idx < len(rec.raw_build_deps):
                        target_dep = rec.raw_build_deps[d_idx]
                        self.catalog.modify_package_dependency(pkg_name, target_dep, action="remove", kind="build")
                        print(f"{GREEN}✓ Berhasil menghapus build dependency '{target_dep}'.{RESET}")
            elif act == "5":
                print(f"\n{BOLD}Pohon Dependensi untuk '{pkg_name}':{RESET}")
                self.tree_vis.print_ascii_tree(pkg_name, max_depth=4)
                input(f"\n{GRAY}Tekan Enter untuk kembali ke inspektur resep...{RESET}")
            elif act == "6":
                ok, msg, _ = self.src_ver.verify_package_sources(pkg_name, check_sha=True)
                tag = f"{GREEN}[OK]{RESET}" if ok else f"{RED}[FAIL]{RESET}"
                print(f"\n{tag} {pkg_name}: {msg}")
                input(f"\n{GRAY}Tekan Enter untuk kembali ke inspektur resep...{RESET}")
