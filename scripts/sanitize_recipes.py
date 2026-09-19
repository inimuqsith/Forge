#!/usr/bin/env python3
"""
Kura Linux Recipe Sanitizer & Validator
---------------------------------------
Script untuk melakukan audit dan sanitasi otomatis pada seluruh resep di recipes/:
1. Membersihkan sonames pustaka biner (*.so).
2. Menghapus / mengalihkan ketergantungan systemd (systemd, systemd-libs -> eudev).
3. Membersihkan artefak kata komentar dan karakter lisensi yang rusak.
4. Menyesuaikan build type dan build script (Rust -> cargo, Meson -> meson, dll.).
5. Menyelaraskan URL upstream resmi dan mengisi hash SHA256 valid.
"""

import sys
import os
import re
import urllib.request
import hashlib
from pathlib import Path
import tomllib

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
RECIPES_DIR = WORKSPACE_ROOT / "recipes"

# Blacklist kata sampah / artefak parsing
COMMENT_BLACKLIST = {
    "required", "by", "for", "from", "to", "and", "or", "with", "without",
    "package", "install", "polkit.install", "sh", "bash"
}

# Soname translation & normalization
SONAME_REPLACEMENTS = {
    "libssl.so": "openssl",
    "libcrypto.so": "openssl",
    "libcurl.so": "curl",
    "libz.so": "zlib",
    "libzstd.so": "zstd",
    "liblzma.so": "xz",
    "libbz2.so": "bzip2",
    "libexpat.so": "expat",
    "libpcre2-8.so": "pcre2",
    "libpcre2-16.so": "pcre2",
    "libpcre2-32.so": "pcre2",
    "libsqlite3.so": "sqlite",
    "libffi.so": "libffi",
    "libxml2.so": "libxml2",
    "libxslt.so": "libxslt",
    "libarchive.so": "libarchive",
    "libseccomp.so": "libseccomp",
    "libcap.so": "libcap",
    "libudev.so": "eudev",
    "libkmod.so": "kmod",
    "libncursesw.so": "ncurses",
    "libreadline.so": "readline",
    "libacl.so": "acl",
    "libgcc_s.so": "gcc",
    "libgcc": "gcc",
    "libgcrypt.so": "libgcrypt",
    "libgcrypt": "libgcrypt",
    "liblz4.so": "lz4",
    "liblzo2.so": "lzo",
    "libfcft.so": "fcft",
    "libgit2.so": "libgit2",
    "systemd-libs": "eudev",
    "systemd": "eudev",
    "elogind-systemd": "elogind",
    "util-linux-libs": "util-linux",
}

# Soname yang aman dibuang (karena bagian dari glibc / compiler standard)
DROP_SONAMES = {
    "libc.so", "libm.so", "ld-linux.so", "libpthread.so", "librt.so", "libdl.so", "libresolv.so"
}

# Upstream canonical URLs for packages
CANONICAL_URLS = {
    "btrfs-progs": "https://www.kernel.org/pub/linux/kernel/people/kdave/btrfs-progs/btrfs-progs-v{pkgver}.tar.xz",
    "xfsprogs": "https://www.kernel.org/pub/linux/utils/fs/xfs/xfsprogs/xfsprogs-{pkgver}.tar.xz",
    "dosfstools": "https://github.com/dosfstools/dosfstools/releases/download/v{pkgver}/dosfstools-{pkgver}.tar.gz",
    "parted": "https://ftp.gnu.org/gnu/parted/parted-{pkgver}.tar.xz",
    "f2fs-tools": "https://git.kernel.org/pub/scm/linux/kernel/git/jaegeuk/f2fs-tools.git/snapshot/f2fs-tools-{pkgver}.tar.gz",
    "squashfs-tools": "https://github.com/plougher/squashfs-tools/archive/refs/tags/{pkgver}.tar.gz",
    "grub": "https://ftp.gnu.org/gnu/grub/grub-{pkgver}.tar.xz",
    "efibootmgr": "https://github.com/rhboot/efibootmgr/archive/refs/tags/{pkgver}.tar.gz",
    "dbus": "https://dbus.freedesktop.org/releases/dbus/dbus-{pkgver}.tar.xz",
    "polkit": "https://github.com/polkit-org/polkit/archive/refs/tags/{pkgver}.tar.gz",
    "elogind": "https://github.com/elogind/elogind/archive/refs/tags/v{pkgver}.tar.gz",
    "fish": "https://github.com/fish-shell/fish-shell/releases/download/{pkgver}/fish-{pkgver}.tar.xz",
    "starship": "https://github.com/starship/starship/archive/refs/tags/v{pkgver}.tar.gz",
    "nushell": "https://github.com/nushell/nushell/archive/refs/tags/{pkgver}.tar.gz",
    "zellij": "https://github.com/zellij-org/zellij/archive/refs/tags/v{pkgver}.tar.gz",
    "helix": "https://github.com/helix-editor/helix/archive/refs/tags/{pkgver}.tar.gz",
    "lazygit": "https://github.com/jesseduffield/lazygit/archive/refs/tags/v{pkgver}.tar.gz",
    "fzf": "https://github.com/junegunn/fzf/archive/refs/tags/v{pkgver}.tar.gz",
    "bottom": "https://github.com/ClementTsang/bottom/archive/refs/tags/{pkgver}.tar.gz",
    "eza": "https://github.com/eza-community/eza/archive/refs/tags/v{pkgver}.tar.gz",
    "zoxide": "https://github.com/ajeetdsouza/zoxide/archive/refs/tags/v{pkgver}.tar.gz",
    "procs": "https://github.com/dalance/procs/archive/refs/tags/v{pkgver}.tar.gz",
    "duf": "https://github.com/muesli/duf/archive/refs/tags/v{pkgver}.tar.gz",
    "dust": "https://github.com/bootandy/dust/archive/refs/tags/v{pkgver}.tar.gz",
    "tokei": "https://github.com/XAMPPRocky/tokei/archive/refs/tags/v{pkgver}.tar.gz",
    "bandwhich": "https://github.com/imsnif/bandwhich/archive/refs/tags/v{pkgver}.tar.gz",
    "grex": "https://github.com/pemistahl/grex/archive/refs/tags/v{pkgver}.tar.gz",
    "hyperfine": "https://github.com/sharkdp/hyperfine/archive/refs/tags/v{pkgver}.tar.gz",
    "gh": "https://github.com/cli/cli/archive/refs/tags/v{pkgver}.tar.gz",
    "wayland": "https://gitlab.freedesktop.org/wayland/wayland/-/releases/{pkgver}/downloads/wayland-{pkgver}.tar.xz",
    "wayland-protocols": "https://gitlab.freedesktop.org/wayland/wayland-protocols/-/releases/{pkgver}/downloads/wayland-protocols-{pkgver}.tar.xz",
    "seatd": "https://git.sr.ht/~kennylevinsen/seatd/archive/{pkgver}.tar.gz",
    "foot": "https://codeberg.org/dnkl/foot/archive/{pkgver}.tar.gz",
    "alacritty": "https://github.com/alacritty/alacritty/archive/refs/tags/v{pkgver}.tar.gz",
}

RUST_PACKAGES = {
    "starship", "zellij", "helix", "bottom", "eza", "zoxide", "procs", "dust",
    "tokei", "bandwhich", "grex", "hyperfine", "alacritty"
}

GO_PACKAGES = {
    "lazygit", "duf", "gh"
}

MESON_PACKAGES = {
    "dbus", "polkit", "elogind", "wayland", "wayland-protocols", "seatd", "foot"
}

CMAKE_PACKAGES = {
    "fish"
}

def clean_dependency_list(deps: list[str]) -> list[str]:
    cleaned = []
    for d in deps:
        d = d.strip().strip("'\"")
        # Hilangkan batasan versi: foo>=1.2 -> foo
        d = re.split(r"[<>=]", d)[0].strip()
        # Hilangkan :arch
        d = d.split(":")[0].strip()
        
        if not d or d in COMMENT_BLACKLIST or d in DROP_SONAMES:
            continue
        
        if d in SONAME_REPLACEMENTS:
            d = SONAME_REPLACEMENTS[d]
            
        if d.endswith(".so") or ".so." in d:
            continue
            
        if d not in cleaned:
            cleaned.append(d)
    return cleaned

def clean_license(lic: str) -> str:
    lic = lic.strip()
    if lic.startswith("(") and lic.endswith(")"):
        lic = lic[1:-1]
    lic = lic.replace("'", "").replace('"', "").strip()
    if not lic or lic in {"(", ")", ""}:
        return "GPL-2.0-or-later"
    return lic

def calculate_sha256_of_url(url: str) -> str | None:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Forge-Sanitizer/1.0"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            if resp.status == 200:
                hasher = hashlib.sha256()
                while chunk := resp.read(65536):
                    hasher.update(chunk)
                return hasher.hexdigest()
    except Exception as e:
        print(f"    [!] Gagal mengunduh {url}: {e}")
    return None

def sanitize_recipe(path: Path) -> bool:
    with open(path, "rb") as f:
        data = tomllib.load(f)

    pkg = data.get("package", {})
    name = pkg.get("name", path.parent.name)
    version = str(pkg.get("version", "1.0.0"))
    release = pkg.get("release", 1)
    slot = str(pkg.get("slot", "0"))
    description = pkg.get("description", f"Package {name} for Kura Linux").replace('"', '\\"')
    license_str = clean_license(pkg.get("license", "GPL-2.0-or-later"))
    upstream = pkg.get("upstream", f"https://github.com/{name}/{name}")

    deps = data.get("dependencies", {})
    runtime = clean_dependency_list(deps.get("runtime", ["glibc"]))
    build_deps = clean_dependency_list(deps.get("build", ["gcc"]))
    
    # Pastikan glibc ada di runtime jika bukan glibc/linux-headers/meta
    if name not in {"glibc", "linux-headers", "base", "base-devel"} and "glibc" not in runtime:
        runtime.insert(0, "glibc")

    sources = data.get("sources", {})
    urls = sources.get("urls", [])
    sha256_list = sources.get("sha256", [])

    # Update canonical URL jika ada
    if name in CANONICAL_URLS:
        canonical = CANONICAL_URLS[name].format(pkgver=version)
        urls = [canonical]

    # Build section determination
    if name in RUST_PACKAGES:
        build_type = "cargo"
        if "cargo" not in build_deps and "rust" not in build_deps:
            build_deps.append("rust")
        script = f"""cd "${{srcdir}}/{name}-${{pkgver}}"
cargo build --release --locked
install -Dm755 "target/release/{name}" "${{DESTDIR}}/usr/bin/{name}"
"""
    elif name in GO_PACKAGES:
        build_type = "custom"
        if "go" not in build_deps:
            build_deps.append("go")
        script = f"""cd "${{srcdir}}/{name}-${{pkgver}}"
go build -o "{name}" .
install -Dm755 "{name}" "${{DESTDIR}}/usr/bin/{name}"
"""
    elif name in MESON_PACKAGES:
        build_type = "meson"
        if "meson" not in build_deps:
            build_deps.append("meson")
        if "ninja" not in build_deps:
            build_deps.append("ninja")
        script = f"""cd "${{srcdir}}/{name}-${{pkgver}}"
meson setup build --prefix=/usr --buildtype=release
ninja -C build ${{MAKEFLAGS}}
DESTDIR="${{DESTDIR}}" ninja -C build install
"""
    elif name in CMAKE_PACKAGES:
        build_type = "cmake"
        if "cmake" not in build_deps:
            build_deps.append("cmake")
        if "ninja" not in build_deps:
            build_deps.append("ninja")
        script = f"""cd "${{srcdir}}/{name}-${{pkgver}}"
cmake -B build -G Ninja \\
    -DCMAKE_BUILD_TYPE=Release \\
    -DCMAKE_INSTALL_PREFIX=/usr
ninja -C build ${{MAKEFLAGS}}
DESTDIR="${{DESTDIR}}" ninja -C build install
"""
    else:
        build_info = data.get("build", {})
        build_type = build_info.get("type", "autotools")
        script = build_info.get("script", f"""cd "${{srcdir}}/{name}-${{pkgver}}"
./configure --prefix=/usr --sysconfdir=/etc --localstatedir=/var --disable-static
make ${{MAKEFLAGS}}
make DESTDIR="${{DESTDIR}}" install
""").strip() + "\n"

    # Download sha256 jika kosong dan ada url
    if urls and (not sha256_list or any(len(s) < 32 for s in sha256_list)):
        print(f"  [*] Mengunduh & menghitung SHA256 untuk '{name}' (v{version})...")
        computed = calculate_sha256_of_url(urls[0])
        if computed:
            sha256_list = [computed]
            print(f"      ✓ SHA256: {computed}")

    runtime_str = ", ".join(f'"{d}"' for d in runtime)
    build_str = ", ".join(f'"{d}"' for d in build_deps)
    urls_str = ",\n".join(f'    "{u}"' for u in urls)
    sha_str = ", ".join(f'"{s}"' for s in sha256_list)

    toml_output = f"""[package]
name = "{name}"
version = "{version}"
release = {release}
slot = "{slot}"
description = "{description}"
license = "{license_str}"
upstream = "{upstream}"

[dependencies]
runtime = [{runtime_str}]
build = [{build_str}]

[sources]
urls = [
{urls_str}
]
sha256 = [{sha_str}]

[build]
type = "{build_type}"
script = \"\"\"
{script.strip()}
\"\"\"
"""
    path.write_text(toml_output, encoding="utf-8")
    return True

def main():
    print("=== Kura Linux Recipe Sanitizer & Validator ===")
    all_recipes = list(RECIPES_DIR.glob("*/*/recipe.toml"))
    print(f"Memproses {len(all_recipes)} resep di {RECIPES_DIR}...\n")
    
    for r in sorted(all_recipes):
        sanitize_recipe(r)

    print("\n✓ Seluruh resep berhasil disanitasi & dinormalisasi!")

if __name__ == "__main__":
    main()
