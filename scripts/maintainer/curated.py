#!/usr/bin/env python3
"""
Curated Ecosystem Hub - Popular packages directory & 1-click recipe scaffolder.
"""

from pathlib import Path
from typing import Dict, List, Tuple
from .catalog import MaintainerCatalog, RECIPES_DIR, CYAN, GREEN, YELLOW, MAGENTA, BOLD, GRAY, RESET


CURATED_POPULAR_PACKAGES: Dict[str, List[Tuple[str, str, str]]] = {
    "🖥️  Desktop & Wayland Compositors": [
        ("hyprland", "Dynamic tiling Wayland compositor that doesn't sacrifice on looks", "extra"),
        ("sway", "i3-compatible Wayland compositor", "extra"),
        ("waybar", "Highly customizable Wayland bar for Sway and Wlroots", "extra"),
        ("foot", "Fast, lightweight and minimalistic Wayland terminal emulator", "extra"),
        ("wofi", "Launcher/menu program for wlroots based Wayland compositors", "extra"),
        ("mako", "Lightweight Wayland notification daemon", "extra"),
    ],
    "⚡ Developer Tools & Modern Runtimes": [
        ("rust", "Empowering everyone to build reliable and efficient software", "extra"),
        ("go", "Open source programming language that makes it easy to build simple, fast software", "extra"),
        ("nodejs", "JavaScript runtime built on Chrome's V8 engine", "extra"),
        ("python", "Next generation of the high-level scripting language", "core"),
        ("llvm", "Next-gen compiler infrastructure, clang, lld, and mold support", "system"),
        ("mold", "Extremely fast modern linker", "system"),
    ],
    "🎬 Multimedia, Audio & Graphics": [
        ("ffmpeg", "Complete, cross-platform solution to record, convert and stream audio and video", "extra"),
        ("pipewire", "Server and user space API to handle multimedia pipelines", "extra"),
        ("wireplumber", "Modular session / policy manager for PipeWire", "extra"),
        ("mpv", "Free, open source, and cross-platform media player", "extra"),
        ("mesa", "Open-source OpenGL, Vulkan, and Gallium drivers", "extra"),
    ],
    "🌐 Web Browsers & Networking": [
        ("firefox", "Fast, Private & Safe Web Browser by Mozilla", "extra"),
        ("chromium", "Open-source browser project that aims to build a safer, faster way to experience the web", "extra"),
        ("curl", "Command line tool and library for transferring data with URLs", "core"),
        ("openssh", "Premier connectivity tool for remote login with the SSH protocol", "core"),
        ("nginx", "High performance web server and reverse proxy", "extra"),
    ],
    "🛠️ Modern CLI & Productivity": [
        ("ripgrep", "Fast line-oriented search tool combining grep with gitignore safety", "extra"),
        ("fd", "Simple, fast and user-friendly alternative to 'find'", "extra"),
        ("bat", "A cat(1) clone with syntax highlighting and Git integration", "extra"),
        ("eza", "A modern, maintained replacement for ls", "extra"),
        ("starship", "The minimal, blazing-fast, and infinitely customizable prompt for any shell", "extra"),
        ("fzf", "General-purpose command-line fuzzy finder", "extra"),
    ],
    "🛡️ System Utilities, Kernel & Security": [
        ("linux-zen", "Zen Kernel: Result of collaborative effort of kernel hackers for everyday desktop systems", "system"),
        ("sudo", "Give certain users the ability to run some commands as root", "core"),
        ("shadow", "Password and account management tool suite with PAM support", "system"),
        ("btrfs-progs", "Userspace utilities for Btrfs filesystem", "core"),
        ("zfs", "Advanced filesystem and volume manager for Linux", "extra"),
    ],
}


class CuratedEcosystemHub:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def get_all_curated_items(self) -> List[Tuple[str, str, str, bool]]:
        items = []
        for cat_title, pkgs in CURATED_POPULAR_PACKAGES.items():
            for name, desc, cat in pkgs:
                installed = name in self.catalog.recipes
                items.append((name, desc, cat, installed))
        return items

    def auto_scaffold_package(self, pkg_name: str, category: str = "extra", desc: str = "", homepage: str = "") -> Path:
        target_dir = RECIPES_DIR / category / pkg_name
        target_dir.mkdir(parents=True, exist_ok=True)
        recipe_file = target_dir / "recipe.toml"

        template = f"""[package]
name = "{pkg_name}"
version = "1.0.0"
release = 1
slot = "0"
description = "{desc or f'Package {pkg_name} for Kura Linux'}"
license = "GPL-3.0-or-later"
upstream = "{homepage or f'https://github.com/{pkg_name}/{pkg_name}'}"

[dependencies]
runtime = ["glibc"]
build = ["make", "gcc", "pkgconf"]

[sources]
urls = [
    "https://github.com/{pkg_name}/{pkg_name}/archive/refs/tags/v1.0.0.tar.gz"
]
sha256 = [
    "0000000000000000000000000000000000000000000000000000000000000000"
]

[build]
type = "autotools"
script = \"\"\"
cd "${{srcdir}}/{pkg_name}-${{pkgver}}"
./configure --prefix=/usr
make ${{MAKEFLAGS}}
make DESTDIR="${{DESTDIR}}" install
\"\"\"
"""
        recipe_file.write_text(template, encoding="utf-8")
        self.catalog.load_all_recipes()
        print(f"{GREEN}✓ Berhasil membuat scaffold resep baru: {recipe_file.relative_to(RECIPES_DIR.parent)}{RESET}")
        return recipe_file
