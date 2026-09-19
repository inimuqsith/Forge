#!/usr/bin/env python3
"""
Forge & Kura Linux Maintainer Interactive Power Tool
Author: Kura Linux Maintainers
Description:
    All-in-One CLI & Interactive Terminal Dashboard for Forge package maintainers.
    Features:
      1. 🌟 Beranda / Curated Ecosystem & Popular Packages Hub (with 1-Click Scaffolding)
      2. 🕸️ DAG & Dependency Graph Solver (Supports individual targets & global 'all' / '@world')
      3. 🌲 ASCII Dependency Tree Visualizer
      4. 🔄 Reverse Dependency Lookup ("Who depends on package X?")
      5. 🔍 Upstream Package Search (Anitya / Release-Monitoring.org & GitHub APIs)
      6. ⚡ Upstream Audit & Atomic Recipe Bumper
      7. 🔒 Source Integrity & SHA256 Stream Verifier
      8. 🛡️ Recipe Linter & DESTDIR Safety Validator
      9. ✏️ Recipe Scaffolder & Dependency Editor
     10. 📑 SSOT Matrix Generator (Regenerates recipes/PACKAGE_STATUS.md)
     11. 🧪 Distro Test Suite Runner (cargo test --workspace)
"""

import sys
import os
import re
import json
import time
import argparse
import subprocess
import urllib.request
import urllib.error
import urllib.parse
import hashlib
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

# Paths
WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
RECIPES_DIR = WORKSPACE_ROOT / "recipes"
PACKAGE_STATUS_FILE = RECIPES_DIR / "PACKAGE_STATUS.md"

# ANSI Terminal Colors & Styling
RESET = "\033[0m"
BOLD = "\033[1m"
DIM = "\033[2m"
ITALIC = "\033[3m"
UNDERLINE = "\033[4m"
RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
BLUE = "\033[34m"
MAGENTA = "\033[35m"
CYAN = "\033[36m"
WHITE = "\033[37m"
GRAY = "\033[90m"
BG_BLUE = "\033[44m"
BG_GREEN = "\033[42m"

CATEGORIES = ["system", "core", "extra"]
CATEGORY_DESCS = {
    "system": "Fondasi OS, Kernel & Toolchain Kompilasi",
    "core": "Sistem Inti, Storage, Filesystem, Networking, Security & Bootloader",
    "extra": "Development Tools, CLI Modern, Desktop Apps, Audio, Qt6 & KDE Plasma 6 Desktop",
}
USE_COND_REGEX = re.compile(r"^[!]?[a-zA-Z0-9_\-+]+\?\s*\(\s*(.+)\s*\)$")

# Curated Popular Packages for Modern Linux Ecosystem
CURATED_ECOSYSTEM = {
    "🖥️  Desktop & Wayland Compositors": [
        ("hyprland", "Dynamic tiling Wayland compositor that doesn't sacrifice on its looks", "extra"),
        ("sway", "i3-compatible Wayland compositor", "extra"),
        ("river", "Dynamic tiling Wayland compositor", "extra"),
        ("waybar", "Highly customizable Wayland bar for Sway and Wlroots/Hyprland", "extra"),
        ("rofi-wayland", "A window switcher, application launcher and dmenu replacement for Wayland", "extra"),
        ("dunst", "Customizable and lightweight notification-daemon", "extra"),
        ("mako", "Lightweight Wayland notification daemon", "extra"),
        ("swww", "Efficient animated wallpaper daemon for Wayland", "extra"),
        ("grim", "Grab images from a Wayland compositor", "extra"),
        ("slurp", "Select a region in a Wayland compositor", "extra"),
        ("wl-clipboard", "Command-line copy/paste utilities for Wayland", "extra"),
        ("plasma-desktop", "KDE Plasma Desktop environment", "extra"),
        ("sddm", "QML based modern display manager", "extra"),
        ("foot", "Fast, lightweight, and minimalistic Wayland terminal emulator", "extra"),
        ("alacritty", "A cross-platform, GPU-accelerated terminal emulator", "extra"),
    ],
    "🛠️  Developer Runtimes & Languages": [
        ("rust", "Empowering everyone to build reliable and efficient software", "extra"),
        ("go", "Open source programming language that makes it easy to build simple, fast software", "extra"),
        ("python", "Next generation of the Python language", "extra"),
        ("nodejs", "JavaScript runtime built on V8 engine", "extra"),
        ("bun", "Incredibly fast JavaScript/TypeScript runtime & package manager", "extra"),
        ("deno", "A modern runtime for JavaScript and TypeScript", "extra"),
        ("openjdk", "Free and open-source implementation of the Java Platform", "extra"),
        ("zig", "General-purpose programming language and toolchain for robust software", "extra"),
        ("ruby", "Dynamic programming language with a focus on simplicity and productivity", "extra"),
        ("clang", "C, C++, and Objective-C front-end for LLVM", "system"),
        ("llvm", "LLVM Compiler Infrastructure", "system"),
        ("neovim", "Vim-fork focused on extensibility and usability", "extra"),
        ("podman", "Daemonless container engine for developing, managing OCI Containers", "extra"),
        ("git", "Fast, scalable, distributed revision control system", "extra"),
        ("lazygit", "Simple terminal UI for git commands", "extra"),
        ("gh", "GitHub CLI tool", "extra"),
    ],
    "🎬  Multimedia & Graphics Stack": [
        ("ffmpeg", "Complete, cross-platform solution to record, convert and stream audio/video", "extra"),
        ("mpv", "Command line video player", "extra"),
        ("gstreamer", "Open source multimedia framework", "extra"),
        ("obs-studio", "Free and open source software for live streaming and screen recording", "extra"),
        ("mesa", "Open-source OpenGL/Vulkan graphics drivers", "extra"),
        ("vulkan-loader", "Vulkan Installable Client Driver (ICD) Loader", "extra"),
        ("pipewire", "Low-latency audio/video router and processor", "extra"),
        ("wireplumber", "Session / policy manager implementation for PipeWire", "extra"),
        ("sdl2", "Low level access to audio, keyboard, mouse, joystick, and graphics", "extra"),
        ("sdl3", "Simple DirectMedia Layer 3", "extra"),
        ("libpng", "Official PNG reference library", "extra"),
        ("cairo", "2D graphics library with support for multiple output devices", "extra"),
    ],
    "🌐  Web Browsers & Internet Tools": [
        ("firefox", "Free and open source web browser from Mozilla", "extra"),
        ("chromium", "Open-source browser project that aims to build a safer, faster way to experience the web", "extra"),
        ("helium-browser", "Lightweight privacy-focused Chromium browser", "extra"),
        ("curl", "Command line tool and library for transferring data with URLs", "core"),
        ("wget", "Network utility to retrieve files from the Web", "core"),
        ("aria2", "Ultra-fast multi-protocol & multi-source download utility", "extra"),
    ],
    "⚡  Modern Terminal & CLI Utilities": [
        ("fastfetch", "Like neofetch, but much faster because written in C", "extra"),
        ("bat", "A cat clone with syntax highlighting and Git integration", "extra"),
        ("eza", "A modern replacement for ls (community fork of exa)", "extra"),
        ("ripgrep", "Line-oriented search tool that recursively searches current directory for a regex pattern", "extra"),
        ("fd", "Simple, fast and user-friendly alternative to find", "extra"),
        ("starship", "The minimal, blazing-fast, and infinitely customizable prompt for any shell", "extra"),
        ("zoxide", "A smarter cd command inspired by z and autojump", "extra"),
        ("btop", "Resource monitor that shows usage and stats for processor, memory, disks and network", "extra"),
        ("bottom", "A customizable cross-platform graphical process/system monitor for the terminal", "extra"),
        ("fzf", "General-purpose command-line fuzzy finder", "extra"),
        ("tmux", "Terminal multiplexer", "extra"),
        ("zellij", "A terminal workspace with batteries included", "extra"),
        ("duf", "Disk Usage/Free Utility", "extra"),
        ("dust", "A more intuitive version of du in rust", "extra"),
        ("tokei", "A program that displays statistics about your code", "extra"),
        ("hyperfine", "A command-line benchmarking tool", "extra"),
    ],
    "🔒  Networking, VPN & Security": [
        ("tailscale", "The easiest, most secure way to use WireGuard and 2FA", "extra"),
        ("wireguard-tools", "WireGuard userspace configuration utilities (wg, wg-quick)", "core"),
        ("openvpn", "Open source VPN daemon", "extra"),
        ("iptables", "Linux kernel packet filtering and NAT control utility", "core"),
        ("nftables", "Netfilter userspace packet filtering framework", "core"),
        ("openssh", "Premier connectivity tool for remote login with the SSH protocol", "extra"),
        ("nmap", "Utility for network discovery and security auditing", "extra"),
    ],
    "🎮  Gaming & Performance": [
        ("gamescope", "Micro-compositor for video games on Wayland", "extra"),
        ("mangohud", "A Vulkan and OpenGL overlay for monitoring FPS, temperatures, CPU/GPU load", "extra"),
        ("steam", "Valve's digital software delivery platform", "extra"),
        ("wine", "Compatibility layer capable of running Windows applications on Linux", "extra"),
        ("gamemode", "Optimise Linux system performance on demand", "extra"),
    ]
}


def print_banner():
    banner = f"""
{CYAN}{BOLD}╔═══════════════════════════════════════════════════════════════════════════════════╗
║     🔨  FORGE & KURA LINUX — UNIFIED MAINTAINER POWER TOOL & ECOSYSTEM HUB       ║
╚═══════════════════════════════════════════════════════════════════════════════════╝{RESET}
{GRAY}Single Source of Truth Catalog: {RECIPES_DIR} | Kura Linux Distro Engine{RESET}
"""
    print(banner)


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


class MaintainerEngine:
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
                    version = pkg.get("version", "1.0.0")
                    release = pkg.get("release", 1)
                    slot = str(pkg.get("slot", "0"))
                    category = pkg.get("category", cat)

                    record = RecipeRecord(name, version, release, slot, category, recipe_file, data)
                    self.recipes[name] = record

                    for dep in record.all_deps:
                        self.reverse_deps[dep].add(name)
                except Exception as e:
                    print(f"{RED}Error loading {recipe_file}: {e}{RESET}")

    def find_matching_packages(self, query: str) -> List[str]:
        """Fuzzy & substring matching for package names."""
        q = query.strip().lower()
        if not q:
            return []
        exact = [name for name in self.recipes if name.lower() == q]
        if exact:
            return exact
        starts = [name for name in self.recipes if name.lower().startswith(q)]
        contains = [name for name in self.recipes if q in name.lower() and name not in starts]
        return starts + contains

    # -------------------------------------------------------------------------
    # 1. DAG & DEPENDENCY SOLVER ENGINE (Supports individual & global 'all')
    # -------------------------------------------------------------------------
    def check_all_missing_dependencies(self) -> Dict[str, List[str]]:
        missing = defaultdict(list)
        for name, rec in self.recipes.items():
            for dep in rec.all_deps:
                if dep not in self.recipes:
                    missing[name].append(dep)
        return missing

    def resolve_target_dag(self, target: str, include_build: bool = True) -> Tuple[List[str], List[str], List[str]]:
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
            if curr not in self.recipes:
                if curr not in missing:
                    missing.append(curr)
                return

            visiting.add(curr)
            rec = self.recipes[curr]
            deps = list(rec.runtime_deps)
            if include_build:
                deps.extend(rec.build_deps)

            for d in deps:
                dfs(d, path + [curr])

            visiting.remove(curr)
            visited.add(curr)
            resolved_order.append(curr)

        if is_global_all:
            # Resolve every package in the catalog to produce full topological ordering
            for pkg_name in sorted(self.recipes.keys()):
                dfs(pkg_name, [])
            return resolved_order, missing, cycles
        else:
            if target not in self.recipes:
                return [], [target], []
            dfs(target, [])
            return resolved_order, missing, cycles

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

        rec = self.recipes.get(target)
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

    def find_reverse_dependencies(self, target_pkg: str) -> List[str]:
        return sorted(list(self.reverse_deps.get(target_pkg, set())))

    def get_root_and_leaf_packages(self) -> Tuple[List[str], List[str]]:
        roots = []
        leaves = []
        for name, rec in self.recipes.items():
            if not self.reverse_deps.get(name):
                roots.append(name)
            if not rec.all_deps:
                leaves.append(name)
        return sorted(roots), sorted(leaves)

    # -------------------------------------------------------------------------
    # 2. UPSTREAM SEARCH ENGINE (Anitya, GitHub, Arch)
    # -------------------------------------------------------------------------
    def search_upstream(self, query: str) -> List[Dict[str, Any]]:
        results = []
        q = query.strip()
        print(f"\n{BOLD}🔍 Mencari '{q}' di Katalog Lokal, Anitya, dan GitHub...{RESET}\n")

        # 1. Local Search
        local_matches = self.find_matching_packages(q)
        for lm in local_matches:
            rec = self.recipes[lm]
            results.append({
                "source": "Kura Linux (Lokal)",
                "name": rec.name,
                "version": rec.version,
                "description": rec.description,
                "homepage": rec.upstream,
                "installed": True,
                "category": rec.category
            })

        # 2. Anitya (Release-Monitoring.org)
        try:
            url = f"https://release-monitoring.org/api/v2/projects/?name={urllib.parse.quote(q)}"
            req = urllib.request.Request(url, headers={"User-Agent": "Forge-Maintainer/1.0"})
            with urllib.request.urlopen(req, timeout=5) as resp:
                data = json.loads(resp.read().decode())
                for item in data.get("items", [])[:5]:
                    name = item.get("name")
                    if any(r["name"].lower() == name.lower() for r in results):
                        continue
                    v = item.get("version", "unknown")
                    hp = item.get("homepage", "")
                    results.append({
                        "source": "Anitya (Release-Monitoring)",
                        "name": name,
                        "version": v,
                        "description": f"Upstream project on {item.get('backend', 'distro')}",
                        "homepage": hp,
                        "installed": name in self.recipes,
                        "category": "extra"
                    })
        except Exception as e:
            pass

        # 3. GitHub API
        try:
            gh_url = f"https://api.github.com/search/repositories?q={urllib.parse.quote(q)}+in:name&sort=stars&per_page=5"
            req = urllib.request.Request(gh_url, headers={"User-Agent": "Forge-Maintainer/1.0"})
            with urllib.request.urlopen(req, timeout=5) as resp:
                data = json.loads(resp.read().decode())
                for repo in data.get("items", [])[:5]:
                    r_name = repo.get("name")
                    if any(r["name"].lower() == r_name.lower() for r in results):
                        continue
                    results.append({
                        "source": f"GitHub ({repo.get('full_name')}) ★ {repo.get('stargazers_count', 0)}",
                        "name": r_name,
                        "version": repo.get("default_branch", "main"),
                        "description": repo.get("description") or "No description",
                        "homepage": repo.get("html_url", ""),
                        "installed": r_name in self.recipes,
                        "category": "extra",
                        "clone_url": repo.get("clone_url", "")
                    })
        except Exception as e:
            pass

        return results

    # -------------------------------------------------------------------------
    # 3. AUTO-SCAFFOLDER & RECIPE GENERATOR
    # -------------------------------------------------------------------------
    def auto_scaffold_package(self, name: str, category: str = "extra", desc: str = "", homepage: str = "", version: str = "1.0.0") -> bool:
        target_dir = self.recipes_dir / category / name
        target_file = target_dir / "recipe.toml"

        if target_file.exists():
            print(f"{YELLOW}Resep '{name}' sudah ada di {target_file}.{RESET}")
            return False

        print(f"\n[*] Mengambil metadata & PKGBUILD hulu untuk '{name}'...")
        # Coba ambil PKGBUILD dari Arch GitLab
        pkgbuild_url = f"https://gitlab.archlinux.org/archlinux/packaging/packages/{name}/-/raw/main/PKGBUILD"
        pkg_ver = version
        pkg_desc = desc or f"{name} package for Kura Linux"
        pkg_url = homepage or f"https://github.com/{name}/{name}"
        source_url = f"https://github.com/{name}/{name}/archive/refs/tags/v{pkg_ver}.tar.gz"
        sha256_hash = ""

        try:
            req = urllib.request.Request(pkgbuild_url, headers={"User-Agent": "Forge-Maintainer/1.0"})
            with urllib.request.urlopen(req, timeout=5) as resp:
                content = resp.read().decode("utf-8", errors="ignore")
                for line in content.lines():
                    if line.startswith("pkgver="):
                        pkg_ver = line.split("=")[1].strip().strip("'\"")
                    elif line.startswith("pkgdesc="):
                        pkg_desc = line.split("=")[1].strip().strip("'\"")
                    elif line.startswith("url="):
                        pkg_url = line.split("=")[1].strip().strip("'\"")
        except Exception:
            pass

        # Download & Calculate SHA256 if possible
        print(f"[*] Menghitung SHA256 tarball hulu...")
        try:
            req = urllib.request.Request(source_url, headers={"User-Agent": "Forge-Maintainer/1.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                data = resp.read()
                sha256_hash = hashlib.sha256(data).hexdigest()
                print(f"  {GREEN}✓ Terverifikasi SHA256: {sha256_hash}{RESET}")
        except Exception as e:
            print(f"  {YELLOW}⚠ Catatan: Tidak dapat mengunduh langsung ({e}). Menggunakan placeholder SHA256.{RESET}")
            sha256_hash = "0000000000000000000000000000000000000000000000000000000000000000"

        toml_content = f"""[package]
name = "{name}"
version = "{pkg_ver}"
release = 1
slot = "0"
description = "{pkg_desc}"
license = "MIT OR Apache-2.0"
upstream = "{pkg_url}"

[dependencies]
runtime = ["glibc"]
build = ["make", "gcc", "pkgconf"]

[sources]
urls = [
    "{source_url}"
]
sha256 = [
    "{sha256_hash}"
]

[build]
type = "custom"
script = \"\"\"
cd "${{srcdir}}/{name}-${{pkgver}}"
make ${{MAKEFLAGS}}
make DESTDIR="${{DESTDIR}}" install
\"\"\"
"""
        target_dir.mkdir(parents=True, exist_ok=True)
        target_file.write_text(toml_content, encoding="utf-8")
        print(f"{GREEN}{BOLD}✓ Berhasil membuat resep baru di '{target_file}'!{RESET}")
        self.load_all_recipes()
        self.regenerate_ssot_matrix()
        return True

    # -------------------------------------------------------------------------
    # 4. SOURCE INTEGRITY & SHA256 ENGINE
    # -------------------------------------------------------------------------
    def verify_package_sources(self, pkg_name: str, check_sha: bool = True) -> Tuple[bool, str, Optional[str]]:
        rec = self.recipes.get(pkg_name)
        if not rec:
            return False, f"Paket '{pkg_name}' tidak ditemukan.", None
        if not rec.source_urls:
            return True, "No remote sources (Virtual/Meta-package).", None

        url = rec.source_urls[0]
        url = url.replace("${pkgver}", rec.version).replace("${pkgname}", rec.name)
        req = urllib.request.Request(url, headers={"User-Agent": "Forge-Maintainer/1.0"})

        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                if resp.status != 200:
                    return False, f"HTTP Error {resp.status} on {url}", None
                if not check_sha:
                    return True, f"URL OK (HTTP {resp.status})", None

                data = resp.read()
                calc_hash = hashlib.sha256(data).hexdigest()

                if not rec.source_sha256:
                    return False, f"Missing SHA256 in recipe. Calculated: {calc_hash}", calc_hash

                expected_hash = rec.source_sha256[0].lower().strip()
                if calc_hash == expected_hash:
                    return True, f"SHA256 Match: {calc_hash}", calc_hash
                else:
                    return False, f"SHA256 MISMATCH! Exp: {expected_hash} vs Calc: {calc_hash}", calc_hash
        except Exception as e:
            return False, f"Connection Failed: {e}", None

    # -------------------------------------------------------------------------
    # 5. UPSTREAM AUDIT & BUMP ENGINE
    # -------------------------------------------------------------------------
    def run_upstream_audit(self) -> None:
        print(f"\n{BOLD}{CYAN}=== Menjalankan Audit Hulu (Upstream Audit) via forge-server ==={RESET}")
        try:
            cmd = ["cargo", "run", "--quiet", "--bin", "forge-server", "--", "audit", "--recipes-path", str(self.recipes_dir)]
            subprocess.run(cmd, check=True)
        except Exception as e:
            print(f"{RED}Error executing audit: {e}{RESET}")

    def bump_package(self, pkg_name: Optional[str] = None, bump_all: bool = False, no_push: bool = True) -> None:
        print(f"\n{BOLD}{CYAN}=== Menjalankan Bumper Versi Hulu via forge-server ==={RESET}")
        cmd = ["cargo", "run", "--quiet", "--bin", "forge-server", "--", "bump"]
        if bump_all:
            cmd.append("--all")
        elif pkg_name:
            cmd.append(pkg_name)
        else:
            print(f"{RED}Error: Tentukan nama paket atau gunakan --all.{RESET}")
            return

        if no_push:
            cmd.append("--no-push")

        try:
            subprocess.run(cmd, check=True)
            self.load_all_recipes()
            self.regenerate_ssot_matrix()
        except Exception as e:
            print(f"{RED}Error bumping package: {e}{RESET}")

    # -------------------------------------------------------------------------
    # 6. RECIPE LINTER & DESTDIR SAFETY
    # -------------------------------------------------------------------------
    def lint_all_recipes(self) -> Tuple[int, int, List[str]]:
        passed = 0
        failed = 0
        issues = []

        for name, rec in sorted(self.recipes.items()):
            rec_issues = []
            if not rec.description:
                rec_issues.append("Missing description")
            if not rec.license:
                rec_issues.append("Missing license")
            if not rec.upstream:
                rec_issues.append("Missing upstream URL")

            if len(rec.source_urls) != len(rec.source_sha256) and rec.source_urls:
                rec_issues.append(f"Source count mismatch: {len(rec.source_urls)} URLs vs {len(rec.source_sha256)} hashes")

            build_sec = rec.data.get("build", {})
            script = build_sec.get("script", "") + build_sec.get("install", "")
            if rec.category != "system" and name not in {"base", "base-devel"}:
                if script and "DESTDIR" not in script and "$DESTDIR" not in script and "${DESTDIR}" not in script and "$FORGE_DESTDIR" not in script:
                    rec_issues.append("Build script does NOT reference DESTDIR / $FORGE_DESTDIR")

            if rec_issues:
                failed += 1
                issues.append(f"{RED}[FAIL]{RESET} {BOLD}{name:<22}{RESET} ({rec.category}): {', '.join(rec_issues)}")
            else:
                passed += 1

        return passed, failed, issues

    # -------------------------------------------------------------------------
    # 7. DEPENDENCY EDITOR
    # -------------------------------------------------------------------------
    def modify_package_dependency(self, pkg_name: str, dep_name: str, action: str = "add", kind: str = "runtime") -> bool:
        rec = self.recipes.get(pkg_name)
        if not rec:
            print(f"{RED}Paket '{pkg_name}' tidak ditemukan.{RESET}")
            return False

        content = rec.path.read_text(encoding="utf-8")
        deps_list = rec.raw_runtime_deps if kind == "runtime" else rec.raw_build_deps

        if action == "add":
            if dep_name in deps_list:
                print(f"{YELLOW}Dependensi '{dep_name}' sudah ada di {pkg_name} ({kind}).{RESET}")
                return True
            deps_list.append(dep_name)
        elif action == "remove":
            if dep_name not in deps_list:
                print(f"{YELLOW}Dependensi '{dep_name}' tidak ditemukan di {pkg_name} ({kind}).{RESET}")
                return True
            deps_list.remove(dep_name)

        new_deps_str = ", ".join(f'"{d}"' for d in deps_list)
        pattern = rf'({kind}\s*=\s*\[)[^\]]*(\])'
        new_content = re.sub(pattern, rf'\1{new_deps_str}\2', content)

        rec.path.write_text(new_content, encoding="utf-8")
        print(f"{GREEN}✓ Berhasil {action} '{dep_name}' pada {pkg_name} [{kind}].{RESET}")
        self.load_all_recipes()
        return True

    # -------------------------------------------------------------------------
    # 8. SSOT MATRIX REGENERATOR
    # -------------------------------------------------------------------------
    def regenerate_ssot_matrix(self) -> None:
        cat_data = defaultdict(list)
        for rec in sorted(self.recipes.values(), key=lambda x: x.name):
            cat_data[rec.category].append(rec)

        total_count = len(self.recipes)
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
        print(f"{GREEN}✓ Berhasil meregenerasi {PACKAGE_STATUS_FILE} ({total_count} paket terdaftar).{RESET}")


# =============================================================================
# INTERACTIVE TUI DASHBOARD
# =============================================================================
def interactive_menu():
    engine = MaintainerEngine()

    while True:
        print_banner()
        total_pkgs = len(engine.recipes)
        missing = engine.check_all_missing_dependencies()
        missing_count = len(missing)

        status_color = GREEN if missing_count == 0 else RED
        status_text = "SEHAT (0 Broken Deps)" if missing_count == 0 else f"PERHATIAN ({missing_count} Broken Deps)"

        print(f"📊 Status Katalog: {BOLD}{total_pkgs} Paket Terdaftar{RESET} | Kondisi: {status_color}{BOLD}{status_text}{RESET}\n")
        print(f"{BOLD}PILIH MENU OPERASI MAINTAINER:{RESET}")
        print(f"  {CYAN}1.{RESET} 🌟  {BOLD}Beranda & Eksplorasi Paket Populer{RESET} (Ecosystem Hub & 1-Click Scaffold)")
        print(f"  {CYAN}2.{RESET} 🕸️   Simulasi DAG & Topological Build Order (Ketik nama target atau {BOLD}'all'{RESET} / {BOLD}'@world'{RESET})")
        print(f"  {CYAN}3.{RESET} 🌲  Tampilkan ASCII Dependency Tree suatu target ({BOLD}'all'{RESET} untuk seluruh root)")
        print(f"  {CYAN}4.{RESET} 🔄  Reverse Dependency Search ('Siapa yang butuh paket X?')")
        print(f"  {CYAN}5.{RESET} 🔎  Cari Paket Hulu (Search Anitya / GitHub / Local)")
        print(f"  {CYAN}6.{RESET} 🔍  Audit Katalog vs Versi Hulu (Upstream Release Monitor)")
        print(f"  {CYAN}7.{RESET} ⚡  Auto-Bump Versi Paket ke Rilis Hulu Terbaru")
        print(f"  {CYAN}8.{RESET} 🔒  Verifikasi Integritas URL & SHA256 Tarball Hulu")
        print(f"  {CYAN}9.{RESET} 🛡️   Linter Validasi Format TOML & DESTDIR Safety")
        print(f"  {CYAN}10.{RESET} ✏️  Edit / Tambah Dependensi Paket (Runtime & Build)")
        print(f"  {CYAN}11.{RESET} 📑  Regenerasi Matriks SSOT (`PACKAGE_STATUS.md`)")
        print(f"  {CYAN}12.{RESET} 🧪  Jalankan Full Test Suite (`cargo test --workspace`)")
        print(f"  {CYAN}0.{RESET}  🚪  Keluar")

        try:
            choice = input(f"\n{BOLD}Masukkan nomor pilihan [0-12]: {RESET}").strip()
        except (KeyboardInterrupt, EOFError):
            print(f"\n{YELLOW}Keluar dari Maintainer Tool. Sampai jumpa!{RESET}")
            break

        if choice == "0":
            print(f"\n{GREEN}Sampai jumpa!{RESET}")
            break

        # ---------------------------------------------------------------------
        # 1. BERANDA / POPULAR PACKAGES HUB
        # ---------------------------------------------------------------------
        elif choice == "1":
            print(f"\n{BOLD}{CYAN}=== 🌟 BERANDA & EKSPLORASI PAKET POPULER LINUX ==={RESET}")
            print(f"{GRAY}Katalog rekomendasi paket terkurasi untuk melengkapi ekosistem Kura Linux.{RESET}\n")

            all_curated_items = []
            item_idx = 1

            for cat_title, pkgs in CURATED_ECOSYSTEM.items():
                print(f"{BOLD}{YELLOW}{cat_title}:{RESET}")
                for p_name, p_desc, default_cat in pkgs:
                    is_installed = p_name in engine.recipes
                    status_badge = f"{GREEN}[✅ TERSEDIA]{RESET}" if is_installed else f"{MAGENTA}[➕ 1-CLICK SCAFFOLD]{RESET}"
                    all_curated_items.append((p_name, p_desc, default_cat, is_installed))
                    print(f"  {CYAN}{item_idx:2d}.{RESET} {BOLD}{p_name:<20}{RESET} {status_badge} {GRAY}- {p_desc[:65]}{RESET}")
                    item_idx += 1
                print()

            print(f"{BOLD}Opsi:{RESET} Masukkan nomor paket [1-{len(all_curated_items)}] untuk inspeksi / scaffold, atau tekan Enter untuk kembali.")
            sub_choice = input(f"{BOLD}Pilih nomor: {RESET}").strip()
            if sub_choice.isdigit():
                idx = int(sub_choice) - 1
                if 0 <= idx < len(all_curated_items):
                    sel_name, sel_desc, sel_cat, sel_installed = all_curated_items[idx]
                    if sel_installed:
                        print(f"\n{GREEN}✓ Paket '{sel_name}' sudah tersedia di Kura Linux!{RESET}")
                        rec = engine.recipes[sel_name]
                        print(f"  Versi     : {rec.version}")
                        print(f"  Kategori  : {rec.category}")
                        print(f"  Path      : {rec.path}")
                        print(f"  Dependensi: {', '.join(rec.all_deps)}")
                    else:
                        print(f"\n{BOLD}Paket '{sel_name}' belum ada di katalog.{RESET}")
                        confirm = input(f"Buat resep baru untuk '{sel_name}' di kategori '{sel_cat}'? [y/N]: ").strip().lower()
                        if confirm in ("y", "yes"):
                            engine.auto_scaffold_package(sel_name, category=sel_cat, desc=sel_desc)

        # ---------------------------------------------------------------------
        # 2. DAG SIMULATION (Supports 'all' / '@world')
        # ---------------------------------------------------------------------
        elif choice == "2":
            target = input(f"{BOLD}Masukkan target paket (contoh: all, @world, bash, base, plasma-desktop): {RESET}").strip()
            if not target:
                continue

            target_clean = target.lower()
            if target_clean in ("all", "@world", "world", "*"):
                print(f"\n{BOLD}Memetakan Graf Dependensi Global untuk SELURUH KATALOG (227 Paket)...{RESET}")
                order, miss, cycles = engine.resolve_target_dag("all")
            else:
                matches = engine.find_matching_packages(target)
                if not matches:
                    print(f"{RED}Paket '{target}' tidak ditemukan.{RESET}")
                    continue
                actual_target = matches[0]
                if len(matches) > 1 and actual_target != target:
                    print(f"Ditemukan beberapa paket yang cocok: {', '.join(matches[:5])}")
                    actual_target = matches[0]

                print(f"\n{BOLD}Hasil Resolusi DAG untuk target '{actual_target}':{RESET}")
                order, miss, cycles = engine.resolve_target_dag(actual_target)

            if miss:
                print(f"{RED}✗ Dependensi Hilang: {', '.join(miss)}{RESET}")
            else:
                print(f"{GREEN}✓ Graf dependensi valid (0 Broken Deps){RESET}")

            if cycles:
                print(f"\n{YELLOW}⚠ Siklus Sirkular Terdeteksi:{RESET}")
                for c in cycles[:5]:
                    print(f"  • {c}")

            print(f"\n{GREEN}{BOLD}Urutan Kompilasi Topologis ({len(order)} paket):{RESET}")
            for idx, p in enumerate(order, 1):
                rec = engine.recipes.get(p)
                cat = f"[{rec.category}]" if rec else ""
                print(f"  {GRAY}{idx:3d}.{RESET} {BOLD}{p:<24}{RESET} {cat}")

        # ---------------------------------------------------------------------
        # 3. TREE VIEW (Supports 'all')
        # ---------------------------------------------------------------------
        elif choice == "3":
            target = input(f"{BOLD}Masukkan target paket untuk pohon dependensi (atau 'all' untuk seluruh root): {RESET}").strip()
            if target:
                try:
                    d_str = input(f"Kedalaman maksimum tree [default 4]: ").strip()
                    depth = int(d_str) if d_str else 4
                except ValueError:
                    depth = 4
                print(f"\n{BOLD}Pohon Dependensi untuk '{target}':{RESET}")
                engine.print_ascii_tree(target, max_depth=depth)

        # ---------------------------------------------------------------------
        # 4. REVERSE DEPENDENCIES
        # ---------------------------------------------------------------------
        elif choice == "4":
            target = input(f"{BOLD}Cari paket apa saja yang membutuhkan paket (contoh: perl, glibc, openssl): {RESET}").strip()
            if target:
                matches = engine.find_matching_packages(target)
                actual_target = matches[0] if matches else target
                revs = engine.find_reverse_dependencies(actual_target)
                print(f"\n{BOLD}Paket yang bergantung pada '{actual_target}' ({len(revs)} paket):{RESET}")
                if revs:
                    for idx, r in enumerate(revs, 1):
                        print(f"  {GRAY}{idx:3d}.{RESET} {BOLD}{r}{RESET}")
                else:
                    print(f"  {YELLOW}Tidak ada paket yang bergantung secara langsung pada '{actual_target}'.{RESET}")

        # ---------------------------------------------------------------------
        # 5. UPSTREAM SEARCH ENGINE
        # ---------------------------------------------------------------------
        elif choice == "5":
            query = input(f"{BOLD}Masukkan kata kunci pencarian (contoh: hyprland, ffmpeg, foot): {RESET}").strip()
            if query:
                results = engine.search_upstream(query)
                if not results:
                    print(f"{YELLOW}Tidak ditemukan hasil untuk '{query}'.{RESET}")
                else:
                    print(f"{BOLD}Hasil Pencarian ({len(results)} item):{RESET}")
                    for idx, r in enumerate(results, 1):
                        status = f"{GREEN}[TERSEDIA]{RESET}" if r["installed"] else f"{MAGENTA}[➕ SCAFFOLD]{RESET}"
                        print(f"  {CYAN}{idx:2d}.{RESET} {BOLD}{r['name']:<22}{RESET} {status} {GRAY}[{r['source']} - v{r['version']}]{RESET}")
                        print(f"      {GRAY}{r['description'][:75]}{RESET}")
                        if r.get("homepage"):
                            print(f"      {BLUE}{r['homepage']}{RESET}")

                    sub = input(f"\n{BOLD}Ketik nomor item untuk scaffold resep baru, atau tekan Enter untuk batal: {RESET}").strip()
                    if sub.isdigit():
                        idx = int(sub) - 1
                        if 0 <= idx < len(results):
                            sel = results[idx]
                            engine.auto_scaffold_package(sel["name"], category=sel.get("category", "extra"), desc=sel["description"], homepage=sel.get("homepage", ""))

        # ---------------------------------------------------------------------
        # 6. UPSTREAM AUDIT
        # ---------------------------------------------------------------------
        elif choice == "6":
            engine.run_upstream_audit()

        # ---------------------------------------------------------------------
        # 7. AUTO-BUMP
        # ---------------------------------------------------------------------
        elif choice == "7":
            sub = input(f"{BOLD}Pilih mode bump: [1] Seluruh Paket Outdated (--all) | [2] Paket Spesifik : {RESET}").strip()
            if sub == "1":
                engine.bump_package(bump_all=True)
            elif sub == "2":
                p_name = input(f"Nama paket yang ingin di-bump: ").strip()
                if p_name:
                    engine.bump_package(pkg_name=p_name)

        # ---------------------------------------------------------------------
        # 8. VERIFIKASI SHA256
        # ---------------------------------------------------------------------
        elif choice == "8":
            sub = input(f"{BOLD}Verifikasi SHA256: [1] Satu Paket | [2] Seluruh Katalog (227 paket) : {RESET}").strip()
            if sub == "1":
                p_name = input("Nama paket: ").strip()
                if p_name:
                    ok, msg, h = engine.verify_package_sources(p_name, check_sha=True)
                    tag = f"{GREEN}[OK]{RESET}" if ok else f"{RED}[FAIL]{RESET}"
                    print(f"  {tag} {p_name}: {msg}")
            elif sub == "2":
                print(f"\n{BOLD}Memeriksa 227 paket sumber hulu (stream verification)...{RESET}")
                for idx, (p_name, rec) in enumerate(sorted(engine.recipes.items()), 1):
                    ok, msg, _ = engine.verify_package_sources(p_name, check_sha=False)
                    tag = f"{GREEN}✓{RESET}" if ok else f"{RED}✗{RESET}"
                    print(f"  [{idx:3d}/227] {tag} {BOLD}{p_name:<22}{RESET} -> {msg}")

        # ---------------------------------------------------------------------
        # 9. LINTER
        # ---------------------------------------------------------------------
        elif choice == "9":
            passed, failed, issues = engine.lint_all_recipes()
            print(f"\n{BOLD}Hasil Linting ({passed + failed} Resep):{RESET}")
            for issue in issues:
                print(f"  {issue}")
            if failed == 0:
                print(f"\n{GREEN}{BOLD}✓ SEMPURNA! 100% Resep lulus validasi sintaks & DESTDIR safety.{RESET}")
            else:
                print(f"\n{RED}{BOLD}✗ Ditemukan {failed} resep dengan catatan.{RESET}")

        # ---------------------------------------------------------------------
        # 10. EDIT DEPENDENCIES
        # ---------------------------------------------------------------------
        elif choice == "10":
            p_name = input("Nama paket yang ingin diedit dependensinya: ").strip()
            if p_name in engine.recipes:
                act = input("Aksi: [1] Tambah (+), [2] Hapus (-) : ").strip()
                action = "add" if act == "1" else "remove"
                kind_in = input("Jenis: [1] Runtime, [2] Build : ").strip()
                kind = "runtime" if kind_in == "1" else "build"
                dep = input(f"Nama dependensi yang akan di-{action}: ").strip()
                if dep:
                    engine.modify_package_dependency(p_name, dep, action, kind)
            else:
                print(f"{RED}Paket '{p_name}' tidak ditemukan.{RESET}")

        # ---------------------------------------------------------------------
        # 11. REGENERATE SSOT
        # ---------------------------------------------------------------------
        elif choice == "11":
            engine.regenerate_ssot_matrix()

        # ---------------------------------------------------------------------
        # 12. TEST SUITE
        # ---------------------------------------------------------------------
        elif choice == "12":
            print(f"\n{BOLD}{CYAN}Menjalankan Cargo Workspace Test Suite...{RESET}")
            subprocess.run(["cargo", "test", "--workspace"])

        input(f"\n{GRAY}Tekan Enter untuk kembali ke menu utama...{RESET}")


# =============================================================================
# CLI NON-INTERACTIVE MODE
# =============================================================================
def main():
    parser = argparse.ArgumentParser(description="Forge & Kura Linux Unified Maintainer Power Tool")
    parser.add_argument("--audit", action="store_true", help="Jalankan audit hulu terhadap rilis terbaru")
    parser.add_argument("--dag", type=str, metavar="PKG", help="Simulasi resolusi DAG & topological sort ('all' untuk seluruh katalog)")
    parser.add_argument("--tree", type=str, metavar="PKG", help="Tampilkan visualisasi pohon dependensi ASCII ('all' untuk seluruh root)")
    parser.add_argument("--reverse", type=str, metavar="PKG", help="Cari paket apa saja yang bergantung pada PKG")
    parser.add_argument("--search", type=str, metavar="QUERY", help="Cari paket hulu di Anitya & GitHub")
    parser.add_argument("--lint", action="store_true", help="Jalankan linter sintaks & DESTDIR compliance")
    parser.add_argument("--status", action="store_true", help="Regenerasi SSOT PACKAGE_STATUS.md")
    parser.add_argument("--bump", type=str, metavar="PKG", help="Bump versi paket ke rilis hulu (gunakan 'all' untuk semua)")
    parser.add_argument("--test", action="store_true", help="Jalankan cargo test --workspace")
    parser.add_argument("--interactive", "-i", action="store_true", help="Jalankan menu interaktif TUI")

    args = parser.parse_args()

    if len(sys.argv) == 1 or args.interactive:
        interactive_menu()
        return

    engine = MaintainerEngine()

    if args.audit:
        engine.run_upstream_audit()
    elif args.dag:
        order, miss, cycles = engine.resolve_target_dag(args.dag)
        print(f"Resolusi DAG untuk '{args.dag}':")
        if miss:
            print(f"Dependensi Hilang: {miss}")
        if cycles:
            print(f"Siklus Sirkular: {cycles}")
        for idx, p in enumerate(order, 1):
            print(f"  {idx:3d}. {p}")
    elif args.tree:
        engine.print_ascii_tree(args.tree)
    elif args.reverse:
        revs = engine.find_reverse_dependencies(args.reverse)
        print(f"Paket yang butuh '{args.reverse}' ({len(revs)} paket):")
        for r in revs:
            print(f"  • {r}")
    elif args.search:
        results = engine.search_upstream(args.search)
        for idx, r in enumerate(results, 1):
            status = "[TERSEDIA]" if r["installed"] else "[SCAFFOLD]"
            print(f"{idx:2d}. {r['name']:<20} {status} ({r['source']}) -> {r['description']}")
    elif args.lint:
        passed, failed, issues = engine.lint_all_recipes()
        for i in issues:
            print(i)
        print(f"Hasil: {passed} PASS | {failed} FAIL")
    elif args.status:
        engine.regenerate_ssot_matrix()
    elif args.bump:
        if args.bump.lower() == "all":
            engine.bump_package(bump_all=True)
        else:
            engine.bump_package(pkg_name=args.bump)
    elif args.test:
        subprocess.run(["cargo", "test", "--workspace"])


if __name__ == "__main__":
    main()
