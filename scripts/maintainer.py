#!/usr/bin/env python3
"""
🔨 Forge & Kura Linux — Unified Maintainer Power Tool
Lean entrypoint & dispatcher for the modular maintainer suite.
"""

import argparse
import subprocess
import sys
from pathlib import Path

# Add scripts directory to module path
sys.path.insert(0, str(Path(__file__).resolve().parent))

from maintainer import (
    MaintainerCatalog, DagSolver, DependencyTreeVisualizer,
    CuratedEcosystemHub, RecipeInspector, RecipeLinter,
    UpstreamSearchEngine, SourceVerifier, SgotMatrixGenerator,
    BOOTSTRAP_TOOLCHAIN
)
from maintainer.catalog import (
    RECIPES_DIR, WORKSPACE_ROOT, CYAN, GREEN, YELLOW, RED, MAGENTA, BLUE, BOLD, GRAY, RESET
)


def print_banner(catalog: MaintainerCatalog, missing_count: int):
    status_str = f"{GREEN}SEHAT (0 Broken Deps){RESET}" if missing_count == 0 else f"{RED}PERINGATAN ({missing_count} Broken Deps){RESET}"
    banner = f"""
{CYAN}{BOLD}╔═══════════════════════════════════════════════════════════════════════════════════╗
║     🔨  FORGE & KURA LINUX — UNIFIED MAINTAINER POWER TOOL & ECOSYSTEM HUB       ║
╚═══════════════════════════════════════════════════════════════════════════════════╝{RESET}
{GRAY}Single Source of Truth Catalog: {RECIPES_DIR} | Kura Linux Distro Engine{RESET}

📊 Status Katalog: {BOLD}{len(catalog.recipes)} Paket Terdaftar{RESET} | Kondisi: {status_str}

PILIH MENU OPERASI MAINTAINER:
  {CYAN}1.{RESET} 🌟  Beranda & Eksplorasi Paket Populer (Ecosystem Hub & 1-Click Scaffold)
  {CYAN}2.{RESET} 🕸️   Simulasi DAG & Topological Build Order (Target 'all' / '@world' / paket)
  {CYAN}3.{RESET} 🌲  Tampilkan ASCII Dependency Tree suatu target
  {CYAN}4.{RESET} 🔄  Reverse Dependency Search ('Siapa yang butuh paket X?')
  {CYAN}5.{RESET} 🔍  Upstream Search Engine (Cari di Katalog Lokal, Anitya, & GitHub)
  {CYAN}6.{RESET} 📋  Audit Versi Hulu Seluruh Resep (`forge-server audit`)
  {CYAN}7.{RESET} 🚀  Auto-Bump Versi Resep ke Rilis Hulu Terbaru (`forge-server bump`)
  {CYAN}8.{RESET} 🔒  Verifikasi Integritas Source URL & SHA256 Tarball
  {CYAN}9.{RESET} 🛡️   Linter Resep, Sintaks TOML & Keamanan DESTDIR
  {CYAN}10.{RESET} 📦 Visual Recipe Inspector & Live Dependency Editor
  {CYAN}11.{RESET} 📝 Regenerasi Matriks Dokumentasi SSOT (`recipes/PACKAGE_STATUS.md`)
  {CYAN}12.{RESET} 🧪 Jalankan Test Suite Workspace (`cargo test --workspace`)
  {CYAN}0.{RESET}  🚪 Keluar
"""
    print(banner)


def interactive_menu():
    catalog = MaintainerCatalog()
    dag_solver = DagSolver(catalog)
    tree_vis = DependencyTreeVisualizer(catalog)
    curated_hub = CuratedEcosystemHub(catalog)
    inspector = RecipeInspector(catalog)
    linter = RecipeLinter(catalog)
    searcher = UpstreamSearchEngine(catalog)
    verifier = SourceVerifier(catalog)
    matrix_gen = SgotMatrixGenerator(catalog)

    while True:
        missing = dag_solver.check_all_missing_dependencies()
        print_banner(catalog, len(missing))
        choice = input(f"{BOLD}PILIH NOMOR OPERASI [0-12] atau ketik 'exit': {RESET}").strip()

        if choice in ("0", "exit", "quit", "q"):
            print(f"{GREEN}Keluar dari Maintainer Tool. Selamat berkarya! 🚀{RESET}")
            break

        # 1. ECOSYSTEM HUB
        elif choice == "1":
            all_items = curated_hub.get_all_curated_items()
            print(f"\n{BOLD}{CYAN}🌟 FORGE POPULAR PACKAGES & ECOSYSTEM HUB{RESET}\n")
            for idx, (name, desc, cat, installed) in enumerate(all_items, 1):
                badge = f"{GREEN}[TERSEDIA]{RESET}" if installed else f"{YELLOW}[BELUM ADA]{RESET}"
                print(f"  {CYAN}{idx:2d}.{RESET} {BOLD}{name:<16}{RESET} {badge} {GRAY}[{cat}]{RESET} - {desc[:50]}")
            sub = input(f"\n{BOLD}Pilih nomor untuk inspeksi/scaffold, atau tekan Enter untuk batal: {RESET}").strip()
            if sub.isdigit() and 1 <= int(sub) <= len(all_items):
                sel_name, sel_desc, sel_cat, sel_installed = all_items[int(sub) - 1]
                if sel_installed:
                    inspector.run(sel_name)
                else:
                    conf = input(f"Scaffold resep baru '{sel_name}' di '{sel_cat}'? [y/N]: ").strip().lower()
                    if conf in ("y", "yes"):
                        curated_hub.auto_scaffold_package(sel_name, category=sel_cat, desc=sel_desc)

        # 2. DAG SIMULATION
        elif choice == "2":
            target = input(f"{BOLD}Masukkan target paket (contoh: all, @world, bash, base): {RESET}").strip()
            if not target:
                continue
            mode_choice = input(f"Pilih mode: [1] Pure Runtime DAG (RDEPEND - 0 Siklus) | [2] Full Source Build DAG (BDEPEND + ADR-019) [default 1]: ").strip()
            mode = "build" if mode_choice == "2" else "runtime"

            mode_name = "Pure Runtime DAG (RDEPEND)" if mode == "runtime" else "Full Source Build DAG (BDEPEND + ADR-019)"
            print(f"\n{BOLD}Menghitung {mode_name} untuk '{target}'...{RESET}")
            order, miss, cycles = dag_solver.resolve_dag(target, mode=mode)

            if miss:
                print(f"{RED}✗ Dependensi Hilang: {', '.join(miss)}{RESET}")
            else:
                print(f"{GREEN}✓ Graf dependensi valid (0 Broken Deps){RESET}")

            if cycles:
                print(f"\n{YELLOW}⚠ Siklus Sirkular Terdeteksi:{RESET}")
                for c in cycles[:5]:
                    print(f"  • {c}")
            else:
                print(f"{GREEN}✓ 0 Siklus Sirkular (Acyclic DAG sempurna){RESET}")

            print(f"\n{GREEN}{BOLD}Urutan Topologis ({len(order)} paket):{RESET}")
            for idx, p in enumerate(order, 1):
                rec = catalog.recipes.get(p)
                cat = f"[{rec.category}]" if rec else ""
                print(f"  {GRAY}{idx:3d}.{RESET} {BOLD}{p:<24}{RESET} {cat}")

        # 3. TREE VIEW
        elif choice == "3":
            target = input(f"{BOLD}Masukkan target paket untuk pohon dependensi (atau 'all'): {RESET}").strip()
            if target:
                d_str = input(f"Kedalaman maksimum tree [default 4]: ").strip()
                depth = int(d_str) if d_str.isdigit() else 4
                tree_vis.print_ascii_tree(target, max_depth=depth)

        # 4. REVERSE DEPENDENCIES
        elif choice == "4":
            target = input(f"{BOLD}Cari paket apa saja yang membutuhkan paket (contoh: glibc, openssl): {RESET}").strip()
            if target:
                matches = catalog.find_matching_packages(target)
                actual_target = matches[0] if matches else target
                revs = catalog.find_reverse_dependencies(actual_target)
                print(f"\n{BOLD}Paket yang bergantung pada '{actual_target}' ({len(revs)} paket):{RESET}")
                for idx, r in enumerate(revs, 1):
                    print(f"  {GRAY}{idx:3d}.{RESET} {BOLD}{r}{RESET}")

        # 5. SEARCH ENGINE
        elif choice == "5":
            query = input(f"{BOLD}Masukkan kata kunci pencarian: {RESET}").strip()
            if query:
                results = searcher.search_upstream(query)
                for idx, r in enumerate(results, 1):
                    status = f"{GREEN}[TERSEDIA]{RESET}" if r["installed"] else f"{MAGENTA}[➕ SCAFFOLD]{RESET}"
                    print(f"  {CYAN}{idx:2d}.{RESET} {BOLD}{r['name']:<22}{RESET} {status} {GRAY}[{r['source']} - v{r['version']}]{RESET}")
                    print(f"      {GRAY}{r['description'][:75]}{RESET}")
                sub = input(f"\n{BOLD}Ketik nomor item untuk scaffold resep, atau tekan Enter untuk batal: {RESET}").strip()
                if sub.isdigit() and 1 <= int(sub) <= len(results):
                    sel = results[int(sub) - 1]
                    curated_hub.auto_scaffold_package(sel["name"], category=sel.get("category", "extra"), desc=sel["description"], homepage=sel.get("homepage", ""))

        # 6. AUDIT
        elif choice == "6":
            subprocess.run(["cargo", "run", "-p", "forge-server", "--", "audit"])

        # 7. BUMP
        elif choice == "7":
            sub = input(f"{BOLD}Pilih mode bump: [1] Seluruh Paket Outdated (--all) | [2] Paket Spesifik : {RESET}").strip()
            if sub == "1":
                subprocess.run(["cargo", "run", "-p", "forge-server", "--", "bump", "--all", "--no-push"])
            elif sub == "2":
                p_name = input("Nama paket: ").strip()
                if p_name:
                    subprocess.run(["cargo", "run", "-p", "forge-server", "--", "bump", p_name, "--no-push"])

        # 8. VERIFIKASI SHA256
        elif choice == "8":
            p_name = input("Nama paket (atau tekan Enter untuk seluruh 227 paket): ").strip()
            if p_name:
                ok, msg, _ = verifier.verify_package_sources(p_name, check_sha=True)
                tag = f"{GREEN}[OK]{RESET}" if ok else f"{RED}[FAIL]{RESET}"
                print(f"  {tag} {p_name}: {msg}")
            else:
                for idx, (p_n, _) in enumerate(sorted(catalog.recipes.items()), 1):
                    ok, msg, _ = verifier.verify_package_sources(p_n, check_sha=False)
                    tag = f"{GREEN}✓{RESET}" if ok else f"{RED}✗{RESET}"
                    print(f"  [{idx:3d}/227] {tag} {BOLD}{p_n:<22}{RESET} -> {msg}")

        # 9. LINTER
        elif choice == "9":
            passed, failed, issues = linter.lint_all_recipes()
            print(f"\n{BOLD}Hasil Linting ({passed + failed} Resep):{RESET}")
            for iss in issues:
                print(f"  {iss}")
            if failed == 0:
                print(f"\n{GREEN}{BOLD}✓ SEMPURNA! 100% Resep lulus validasi sintaks & DESTDIR safety.{RESET}")

        # 10. VISUAL INSPECTOR
        elif choice == "10":
            p_in = input(f"{BOLD}Masukkan nama paket yang ingin diinspeksi / diedit: {RESET}").strip()
            if p_in:
                matches = catalog.find_matching_packages(p_in)
                actual_pkg = matches[0] if matches else p_in
                inspector.run(actual_pkg)

        # 11. MATRIX SSOT
        elif choice == "11":
            matrix_gen.regenerate_ssot_matrix()

        # 12. CARGO TEST
        elif choice == "12":
            subprocess.run(["cargo", "test", "--workspace"])

        input(f"\n{GRAY}Tekan Enter untuk kembali ke menu utama...{RESET}")


def main():
    parser = argparse.ArgumentParser(description="Forge & Kura Linux Maintainer Suite")
    parser.add_argument("--dag", help="Resolve DAG topological order for a package or 'all'")
    parser.add_argument("--runtime", action="store_true", help="Use pure runtime dependencies in DAG simulation")
    parser.add_argument("--build", action="store_true", help="Use full source build dependencies in DAG simulation")
    parser.add_argument("--tree", help="Display ASCII dependency tree for a package or 'all'")
    parser.add_argument("--depth", type=int, default=4, help="Max depth for ASCII dependency tree")
    parser.add_argument("--inspect", help="Open visual box-card inspector for a package")
    parser.add_argument("--search", help="Search upstream packages")
    parser.add_argument("--lint", action="store_true", help="Lint all recipes")
    parser.add_argument("--matrix", action="store_true", help="Regenerate PACKAGE_STATUS.md SSOT matrix")
    args = parser.parse_args()

    catalog = MaintainerCatalog()

    if args.dag:
        mode = "build" if args.build else "runtime"
        solver = DagSolver(catalog)
        order, miss, cycles = solver.resolve_dag(args.dag, mode=mode)
        print(f"DAG Resolution for '{args.dag}' (Mode: {mode}):")
        if miss:
            print(f"{RED}Missing: {', '.join(miss)}{RESET}")
        if cycles:
            print(f"{YELLOW}Cycles detected:{RESET}")
            for c in cycles:
                print(f"  • {c}")
        else:
            print(f"{GREEN}✓ Acyclic DAG (0 cycles, 0 missing){RESET}")
        print(f"Topological Order ({len(order)} packages):")
        for idx, p in enumerate(order, 1):
            print(f"  {idx:3d}. {p}")
        return

    if args.tree:
        tree_vis = DependencyTreeVisualizer(catalog)
        tree_vis.print_ascii_tree(args.tree, max_depth=args.depth)
        return

    if args.inspect:
        inspector = RecipeInspector(catalog)
        inspector.run(args.inspect)
        return

    if args.search:
        searcher = UpstreamSearchEngine(catalog)
        results = searcher.search_upstream(args.search)
        for r in results:
            print(f"- {r['name']} ({r['source']}): {r['description']}")
        return

    if args.lint:
        linter = RecipeLinter(catalog)
        passed, failed, issues = linter.lint_all_recipes()
        print(f"Linting: {passed} passed, {failed} failed")
        for i in issues:
            print(i)
        return

    if args.matrix:
        matrix_gen = SgotMatrixGenerator(catalog)
        matrix_gen.regenerate_ssot_matrix()
        return

    # Default to interactive menu
    interactive_menu()


if __name__ == "__main__":
    main()
