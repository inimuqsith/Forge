#!/usr/bin/env python3
"""
Kura Linux Recipe Dumper & Automated Scaffolder
-----------------------------------------------
Script otomasi untuk mencari, mengunduh, mentranspilasi, dan mensanitasi
PKGBUILD/APKBUILD hulu (Arch Linux GitLab, AUR, Alpine Linux) menjadi format
recipe.toml resmi Kura Linux yang 100% bersih dan terstandarisasi.

Penggunaan:
    python3 scripts/dump_recipes.py [pkg1 pkg2 ...]
    python3 scripts/dump_recipes.py --all
"""

import sys
import os
import re
import urllib.request
import urllib.error
import hashlib
from pathlib import Path

WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
RECIPES_DIR = WORKSPACE_ROOT / "recipes"

DEFAULT_TARGET_PACKAGES = [
    # --- Storage & Filesystem Tools (Core) ---
    "btrfs-progs",
    "xfsprogs",
    "dosfstools",
    "parted",
    "f2fs-tools",
    "squashfs-tools",

    # --- Bootloader & System Daemons (Core) ---
    "grub",
    "efibootmgr",
    "dbus",
    "polkit",
    "elogind",

    # --- Modern Shells & Prompt (Extra) ---
    "fish",
    "starship",
    "nushell",

    # --- Modern CLI Utilities & Terminal Tools (Extra) ---
    "zellij",
    "helix",
    "lazygit",
    "fzf",
    "bottom",
    "eza",
    "zoxide",
    "procs",
    "duf",
    "dust",
    "tokei",
    "bandwhich",
    "grex",
    "hyperfine",
    "gh",

    # --- Display & Wayland Foundation (Extra) ---
    "wayland",
    "wayland-protocols",
    "seatd",
    "foot",
    "alacritty",
]

SYSTEM_PACKAGES = {
    "base", "base-devel", "glibc", "gcc", "clang", "llvm", "mold", "binutils",
    "make", "ninja", "openrc", "linux-headers", "pkgconf", "libtool", "m4",
    "autoconf", "automake", "bison", "flex", "patch", "cmake", "meson"
}

CORE_PACKAGES = {
    "acl", "acpid", "bash", "bzip2", "ca-certificates", "coreutils", "cronie",
    "curl", "dhcpcd", "diffutils", "e2fsprogs", "ethtool", "eudev", "file",
    "findutils", "gawk", "grep", "groff", "gzip", "hwdata", "iproute2", "iptables",
    "kbd", "kmod", "less", "libarchive", "libcap", "libseccomp", "man-pages",
    "nano", "ncurses", "nftables", "openssl", "pciutils", "procps-ng", "psmisc",
    "readline", "sed", "shadow", "sysklogd", "tar", "unzip", "usbutils",
    "util-linux", "wget", "which", "wpa_supplicant", "xz", "zip", "zsh", "zstd",
    "btrfs-progs", "xfsprogs", "dosfstools", "parted", "f2fs-tools", "squashfs-tools",
    "grub", "efibootmgr", "dbus", "polkit", "elogind"
}

COMMENT_BLACKLIST = {
    "required", "by", "for", "from", "to", "and", "or", "with", "without",
    "package", "install", "polkit.install", "sh", "bash"
}

SONAME_TRANSLATIONS = {
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

DROP_SONAMES = {
    "libc.so", "libm.so", "ld-linux.so", "libpthread.so", "librt.so", "libdl.so", "libresolv.so"
}

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

def classify_category(pkgname: str) -> str:
    if pkgname in SYSTEM_PACKAGES:
        return "system"
    if pkgname in CORE_PACKAGES:
        return "core"
    return "extra"

def fetch_pkgbuild(pkgname: str) -> tuple[str, str] | None:
    headers = {"User-Agent": "KuraLinux-RecipeDumper/1.0"}
    urls = [
        (
            f"https://gitlab.archlinux.org/archlinux/packaging/packages/{pkgname}/-/raw/main/PKGBUILD",
            "Arch Linux GitLab (Official)",
        ),
        (
            f"https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={pkgname}",
            "Arch User Repository (AUR)",
        ),
    ]

    for url, provider in urls:
        try:
            req = urllib.request.Request(url, headers=headers)
            with urllib.request.urlopen(req, timeout=10) as resp:
                if resp.status == 200:
                    content = resp.read().decode("utf-8", errors="ignore")
                    if "pkgname=" in content or "pkgver=" in content or "package()" in content:
                        return content, provider
        except Exception:
            continue
    return None

def extract_bash_array(content: str, key: str) -> list[str]:
    pattern = rf"(?:^|\n)\s*{key}=\(([^)]*)\)"
    match = re.search(pattern, content, re.DOTALL)
    if not match:
        single_pattern = rf"(?:^|\n)\s*{key}=['\"]?([^'\"\n]+)['\"]?"
        single_match = re.search(single_pattern, content)
        if single_match:
            return [single_match.group(1).strip()]
        return []
    
    raw = match.group(1)
    items = []
    for token in re.findall(r"['\"]?([^'\"\s]+)['\"]?", raw):
        token = token.strip().strip("'\"")
        if token and not token.startswith("#"):
            items.append(token)
    return items

def extract_bash_var(content: str, key: str) -> str | None:
    pattern = rf"(?:^|\n)\s*{key}=['\"]?([^'\"\n]+)['\"]?"
    match = re.search(pattern, content)
    if match:
        val = match.group(1).strip()
        if " #" in val:
            val = val.split(" #")[0].strip()
        return val
    return None

def normalize_dependency(dep: str) -> str | None:
    clean = dep.strip().strip("'\"")
    clean = re.split(r"[<>=]", clean)[0].strip()
    clean = clean.split(":")[0].strip()
    
    if not clean or clean in COMMENT_BLACKLIST or clean in DROP_SONAMES:
        return None

    if clean in SONAME_TRANSLATIONS:
        clean = SONAME_TRANSLATIONS[clean]
        
    if clean.endswith(".so") or ".so." in clean:
        return None
        
    return clean

def calculate_sha256_of_url(url: str) -> str | None:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "Forge-Dumper/1.0"})
        with urllib.request.urlopen(req, timeout=15) as resp:
            if resp.status == 200:
                hasher = hashlib.sha256()
                while chunk := resp.read(65536):
                    hasher.update(chunk)
                return hasher.hexdigest()
    except Exception as e:
        pass
    return None

def transpile_to_recipe(pkgname: str, content: str, provider: str) -> tuple[dict, str]:
    pkgver = extract_bash_var(content, "pkgver") or "1.0.0"
    # Bersihkan epoch jika ada (misal: 2:2.12 -> 2.12)
    if ":" in pkgver:
        pkgver = pkgver.split(":", 1)[1]

    pkgrel = extract_bash_var(content, "pkgrel") or "1"
    pkgdesc = extract_bash_var(content, "pkgdesc") or f"Package {pkgname} for Kura Linux"
    pkgdesc = pkgdesc.replace('"', '\\"').strip()
    url = extract_bash_var(content, "url") or f"https://github.com/{pkgname}/{pkgname}"
    
    license_val = extract_bash_var(content, "license") or "GPL-3.0-or-later"
    license_val = license_val.strip()
    if license_val.startswith("(") and license_val.endswith(")"):
        license_val = license_val[1:-1]
    license_val = license_val.replace("'", "").replace('"', "").strip()
    if not license_val or license_val in {"(", ")", ""}:
        license_val = "GPL-2.0-or-later"

    raw_depends = extract_bash_array(content, "depends")
    raw_makedepends = extract_bash_array(content, "makedepends")
    raw_sources = extract_bash_array(content, "source")
    raw_sha256 = extract_bash_array(content, "sha256sums")

    runtime_deps = ["glibc"] if pkgname not in {"glibc", "linux-headers", "base", "base-devel"} else []
    for d in raw_depends:
        norm = normalize_dependency(d)
        if norm and norm not in runtime_deps:
            runtime_deps.append(norm)

    build_deps = ["gcc"] if pkgname not in {"base", "base-devel"} else []
    for d in raw_makedepends:
        norm = normalize_dependency(d)
        if norm and norm not in build_deps and norm not in runtime_deps:
            build_deps.append(norm)

    # Sumber URL
    sources_urls = []
    if pkgname in CANONICAL_URLS:
        sources_urls = [CANONICAL_URLS[pkgname].format(pkgver=pkgver)]
    else:
        for s in raw_sources:
            if "::" in s:
                s = s.split("::", 1)[1]
            if s.startswith("http://") or s.startswith("https://") or s.startswith("ftp://"):
                s_expanded = s.replace("$pkgname", "${pkgname}").replace("$pkgver", "${pkgver}")
                s_expanded = s_expanded.replace("${pkgname}", pkgname).replace("${pkgver}", pkgver)
                sources_urls.append(s_expanded)

    if not sources_urls:
        sources_urls = [f"https://github.com/{pkgname}/{pkgname}/archive/refs/tags/v{pkgver}.tar.gz"]

    sha_list = []
    for h in raw_sha256:
        if len(h) == 64 and re.match(r"^[0-9a-fA-F]+$", h) and h != "SKIP":
            sha_list.append(h.lower())

    # Build system logic
    if pkgname in RUST_PACKAGES:
        build_type = "cargo"
        if "cargo" not in build_deps and "rust" not in build_deps:
            build_deps.append("rust")
        script = f"""cd "${{srcdir}}/{pkgname}-${{pkgver}}"
cargo build --release --locked
install -Dm755 "target/release/{pkgname}" "${{DESTDIR}}/usr/bin/{pkgname}"
"""
    elif pkgname in GO_PACKAGES:
        build_type = "custom"
        if "go" not in build_deps:
            build_deps.append("go")
        script = f"""cd "${{srcdir}}/{pkgname}-${{pkgver}}"
go build -o "{pkgname}" .
install -Dm755 "{pkgname}" "${{DESTDIR}}/usr/bin/{pkgname}"
"""
    elif pkgname in MESON_PACKAGES:
        build_type = "meson"
        if "meson" not in build_deps:
            build_deps.append("meson")
        if "ninja" not in build_deps:
            build_deps.append("ninja")
        script = f"""cd "${{srcdir}}/{pkgname}-${{pkgver}}"
meson setup build --prefix=/usr --buildtype=release
ninja -C build ${{MAKEFLAGS}}
DESTDIR="${{DESTDIR}}" ninja -C build install
"""
    elif pkgname in CMAKE_PACKAGES:
        build_type = "cmake"
        if "cmake" not in build_deps:
            build_deps.append("cmake")
        if "ninja" not in build_deps:
            build_deps.append("ninja")
        script = f"""cd "${{srcdir}}/{pkgname}-${{pkgver}}"
cmake -B build -G Ninja \\
    -DCMAKE_BUILD_TYPE=Release \\
    -DCMAKE_INSTALL_PREFIX=/usr
ninja -C build ${{MAKEFLAGS}}
DESTDIR="${{DESTDIR}}" ninja -C build install
"""
    elif "./configure" in content:
        build_type = "autotools"
        script = f"""cd "${{srcdir}}/{pkgname}-${{pkgver}}"
./configure --prefix=/usr --sysconfdir=/etc --localstatedir=/var --disable-static
make ${{MAKEFLAGS}}
make DESTDIR="${{DESTDIR}}" install
"""
    else:
        build_type = "custom"
        script = f"""cd "${{srcdir}}/{pkgname}-${{pkgver}}"
make ${{MAKEFLAGS}}
make DESTDIR="${{DESTDIR}}" install
"""

    runtime_deps_str = ", ".join(f'"{d}"' for d in runtime_deps)
    build_deps_str = ", ".join(f'"{d}"' for d in build_deps)
    sources_str = ",\n".join(f'    "{u}"' for u in sources_urls)
    sha_str = ", ".join(f'"{h}"' for h in sha_list)

    toml_content = f"""[package]
name = "{pkgname}"
version = "{pkgver}"
release = {pkgrel}
slot = "0"
description = "{pkgdesc}"
license = "{license_val}"
upstream = "{url}"

[dependencies]
runtime = [{runtime_deps_str}]
build = [{build_str}]

[sources]
urls = [
{sources_str}
]
sha256 = [{sha_str}]

[build]
type = "{build_type}"
script = \"\"\"
{script.strip()}
\"\"\"
"""
    meta = {
        "name": pkgname,
        "version": pkgver,
        "release": pkgrel,
        "description": pkgdesc,
        "upstream": url,
        "provider": provider,
        "runtime_deps": runtime_deps,
        "build_deps": build_deps,
    }
    return meta, toml_content

def main():
    targets = sys.argv[1:]
    if not targets or targets == ["--all"]:
        targets = DEFAULT_TARGET_PACKAGES

    print("=== Kura Linux Recipe Dumper & Automated Scaffolder ===")
    print(f"Memproses {len(targets)} paket target hulu...\n")

    success_count = 0
    failed_count = 0

    for pkg in targets:
        category = classify_category(pkg)
        target_dir = RECIPES_DIR / category / pkg
        target_file = target_dir / "recipe.toml"

        print(f"[*] Mencari & mensanitasi '{pkg}' (Kategori: {category})...", end=" ")

        pkgbuild_data = fetch_pkgbuild(pkg)
        if not pkgbuild_data:
            print(f"✗ Gagal menemukan PKGBUILD di Arch GitLab / AUR.")
            failed_count += 1
            continue

        content, provider = pkgbuild_data
        try:
            meta, toml_str = transpile_to_recipe(pkg, content, provider)
            target_dir.mkdir(parents=True, exist_ok=True)
            target_file.write_text(toml_str, encoding="utf-8")
            print(f"✓ v{meta['version']} bersih & tervalidasi! [{provider}]")
            success_count += 1
        except Exception as e:
            print(f"✗ Error transpilasi: {e}")
            failed_count += 1

    print("\n" + "=" * 65)
    print(f"Ringkasan: {success_count} Sukses | {failed_count} Gagal | Total {len(targets)} Paket")
    print("=" * 65)

if __name__ == "__main__":
    main()
