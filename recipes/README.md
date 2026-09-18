# Pohon Resep Paket Resmi Kura Linux (`recipes/`)

> Direktori ini memuat seluruh **141 resep paket resmi Kura Linux** yang dikelola menggunakan standar format deklaratif **`recipe.toml`**. Resep-resep ini disinkronkan ke klien pengguna (`/var/db/forge/recipes/`) melalui perintah `forge sync` dan diaudit secara berkala oleh **Upstream Bumper Bot**.

---

## 📊 Matriks Status Kesiapan Paket Kura Linux

Untuk melihat status kesiapan terperinci setiap paket (`✅ Verified`, `🟡 Scaffolded`, `🔴 Needs Patch`), silakan lihat:
👉 **[`recipes/PACKAGE_STATUS.md`](file:///home/admin/Development/Forge/recipes/PACKAGE_STATUS.md)**

---

## 📂 Struktur Kategori Resep

```
recipes/
├── system/   # (22 Paket) Fondasi OS, Meta-Paket, Compiler Toolchain & Init System
├── core/     # (57 Paket) Utilitas Sistem Inti, Storage/Filesystem, Networking & Daemons
└── extra/    # (62 Paket) Shells Modern, CLI Utilities, Text Editors, Wayland & Dev Tools
```

---

## 📦 Ringkasan 141 Resep Paket Terdaftar:

### 1. Kategori `system/` (22 Paket Fondasi & Toolchain)
- **Meta-Paket:** `base` (Fondasi OS), `base-devel` (Toolchain Kompilasi Lengkap).
- **Core Runtime & Init:** `glibc`, `openrc`.
- **Compilers & Linkers:** `clang`, `gcc`, `llvm`, `mold`.
- **Build Tools:** `autoconf`, `automake`, `binutils`, `bison`, `cmake`, `flex`, `libtool`, `m4`, `make`, `meson`, `ninja`, `patch`, `pkgconf`.
- **Kernel API:** `linux-headers`.

### 2. Kategori `core/` (57 Paket Utilitas Inti, Storage & Daemons)
- **Storage & Filesystems:** `btrfs-progs`, `xfsprogs`, `dosfstools`, `e2fsprogs`, `parted`, `f2fs-tools`, `squashfs-tools`.
- **Bootloader & IPC:** `grub`, `efibootmgr`, `dbus`, `polkit`, `elogind`.
- **System Daemons & Core:** `acl`, `acpid`, `bash`, `bzip2`, `ca-certificates`, `coreutils`, `cronie`, `curl`, `dhcpcd`, `diffutils`, `ethtool`, `eudev`, `file`, `findutils`, `gawk`, `grep`, `groff`, `gzip`, `hwdata`, `iproute2`, `iptables`, `kbd`, `kmod`, `less`, `libarchive`, `libcap`, `libseccomp`, `man-pages`, `nano`, `ncurses`, `nftables`, `openssl`, `pciutils`, `procps-ng`, `psmisc`, `readline`, `sed`, `shadow`, `sysklogd`, `tar`, `unzip`, `usbutils`, `util-linux`, `wget`, `which`, `wpa_supplicant`, `xz`, `zip`, `zsh`, `zstd`.

### 3. Kategori `extra/` (62 Paket Modern CLI, Shells, Wayland & Dev Tools)
- **Modern Shells & Prompt:** `fish`, `starship`, `nushell`.
- **Modern CLI Utilities:** `zellij`, `helix`, `lazygit`, `fzf`, `bottom`, `eza`, `zoxide`, `procs`, `duf`, `dust`, `tokei`, `bandwhich`, `grex`, `hyperfine`, `gh`, `fastfetch`, `bat`, `ripgrep`, `fd`, `btop`, `htop`, `tmux`, `tree`.
- **Wayland & GUI Foundations:** `wayland`, `wayland-protocols`, `seatd`, `foot`, `alacritty`.
- **Development Runtimes & Libraries:** `rust`, `cargo`, `python`, `git`, `ccache`, `neovim`, `jq`, `sqlite`, `strace`, `sudo`, `doas`, `glib2`, `gmp`, `icu`, `libffi`, `libxml2`, `libxslt`, `lsof`, `mpc`, `mpfr`, `openssh`, `pcre2`, `zlib`, `cowsay`, `expat`.

---

## 🛠️ Script Scaffolder & Otomasi Resep

Untuk mengunduh dan mentranspilasi paket baru secara massal dari Arch Linux / AUR / Alpine ke pohon resep Kura Linux:
```bash
python3 scripts/dump_recipes.py <package_name>
```

---

## 📜 Spesifikasi Teknis Format Resep

Untuk panduan lengkap sintaks TOML, USE flags bersyarat, slotting multi-versi, dan variabel builder:
👉 [`docs/RECIPE_SPECIFICATION.md`](file:///home/admin/Development/Forge/docs/RECIPE_SPECIFICATION.md)
