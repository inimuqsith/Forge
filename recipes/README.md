# Pohon Resep Paket Resmi Kura Linux (`recipes/`)

> Direktori ini memuat seluruh **107 resep paket resmi Kura Linux** yang dikelola menggunakan standar format deklaratif **`recipe.toml`**. Resep-resep ini disinkronkan ke klien pengguna (`/var/db/forge/recipes/`) melalui perintah `forge sync` dan diaudit secara berkala oleh **Upstream Bumper Bot**.

---

## 📂 Struktur Kategori Resep

```
recipes/
├── system/   # (22 Paket) Fondasi OS, Meta-Paket, Compiler Toolchain & Init System
├── core/     # (51 Paket) Utilitas Sistem Inti, Filesystem, Networking & Libraries
└── extra/    # (34 Paket) Aplikasi Pengembangan, Text Editor, Shells & Tools Modern
```

---

## 📦 Ringkasan 107 Resep Paket Terdaftar:

### 1. Kategori `system/` (22 Paket Fondasi & Toolchain)
- **Meta-Paket:** `base` (Fondasi OS), `base-devel` (Toolchain Kompilasi Lengkap).
- **Core Runtime & Init:** `glibc`, `openrc`.
- **Compilers & Linkers:** `clang`, `gcc`, `llvm`, `mold`.
- **Build Tools:** `autoconf`, `automake`, `binutils`, `bison`, `cmake`, `flex`, `libtool`, `m4`, `make`, `meson`, `ninja`, `patch`, `pkgconf`.
- **Kernel API:** `linux-headers`.

### 2. Kategori `core/` (51 Paket Utilitas Inti & Daemons)
- `acl`, `acpid`, `bash`, `bzip2`, `ca-certificates`, `coreutils`, `cronie`, `curl`, `dhcpcd`, `diffutils`, `e2fsprogs`, `ethtool`, `eudev`, `file`, `findutils`, `gawk`, `grep`, `groff`, `gzip`, `hwdata`, `iproute2`, `iptables`, `kbd`, `kmod`, `less`, `libarchive`, `libcap`, `libseccomp`, `man-pages`, `nano`, `ncurses`, `nftables`, `openssl`, `pciutils`, `procps-ng`, `psmisc`, `readline`, `sed`, `shadow`, `sysklogd`, `tar`, `unzip`, `usbutils`, `util-linux`, `wget`, `which`, `wpa_supplicant`, `xz`, `zip`, `zsh`, `zstd`.

### 3. Kategori `extra/` (34 Paket Dev Tools & CLI Modern)
- `bat`, `btop`, `cargo`, `ccache`, `cowsay`, `doas`, `expat`, `fastfetch`, `fd`, `glib2`, `gmp`, `htop`, `icu`, `jq`, `libffi`, `libxml2`, `libxslt`, `lsof`, `mpc`, `mpfr`, `neovim`, `openssh`, `pcre2`, `python`, `ripgrep`, `rsync`, `rust`, `sqlite`, `strace`, `sudo`, `tmux`, `tree`, `zlib`, dll.

---

## 📜 Spesifikasi Teknis Format Resep

Untuk panduan lengkap sintaks TOML, USE flags bersyarat, slotting multi-versi, dan variabel builder, silakan baca:
👉 [`docs/RECIPE_SPECIFICATION.md`](file:///home/admin/Development/Forge/docs/RECIPE_SPECIFICATION.md)
