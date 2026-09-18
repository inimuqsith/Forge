# Matriks Status & Kesiapan Resep Paket Kura Linux (`PACKAGE_STATUS.md`)

> **Konteks Distribusi Kura Linux**: Kura Linux adalah distribusi Linux berbasis *Source-First Native Compilation* yang dioptimalkan secara ekstrem untuk arsitektur silikon modern (LLVM 22, linker `mold`, Thin LTO, Ccache 4.13.5). Kura Linux mengadopsi struktur **UsrMerge** murni, init system **OpenRC** (tanpa ketergantungan Systemd), dan dikelola secara mandiri oleh package manager **`forge`** berbasis Rust dengan pohon resep terpusat di **GitOps SSOT** (`https://github.com/inimuqsith/Forge`).

---

## 📊 Ringkasan Statistik Status Katalog Paket

| Kategori | Total Paket | ✅ Verified (Siap Produksi) | 🟡 Scaffolded (Siap Uji Sandbox) | 🔴 Needs Patch |
| :--- | :---: | :---: | :---: | :---: |
| **`system/`** (Fondasi & Toolchain) | 22 | 22 | 0 | 0 |
| **`core/`** (Sistem Inti, Storage & Daemons) | 57 | 49 | 8 | 0 |
| **`extra/`** (CLI Modern, Shells & Dev Tools) | 62 | 34 | 28 | 0 |
| **TOTAL** | **141** | **105** | **36** | **0** |

---

## 🏷️ Definisi Kategori Status Kesiapan

1. **`✅ Verified (Teruji & Siap Produksi)`**
   - Resep fondasi Kura Linux yang telah tervalidasi 100%.
   - Pohon dependensi telah diuji dalam DAG dependency resolver.
   - Checksum hash SHA256 hulu mutakhir dan terdaftar di `forge-server`.
   - Lulus kompilasi dalam lingkungan bootstrap sysroot dan stage Kura Linux.

2. **`🟡 Scaffolded (Ter-dump & Siap Uji Sandbox)`**
   - Resep paket baru hasil transpilasi hulu (Arch Linux GitLab / AUR / Alpine Aports) via `scripts/dump_recipes.py`.
   - Format `recipe.toml` valid secara sintaksis dan lulus test suite `test_validate_all_recipes_in_repo_are_valid_toml`.
   - Siap untuk proses uji coba kompilasi di Bubblewrap staging (`/tmp/forge/stage/`).

3. **`🔴 Needs Patch (Perlu Penyesuaian Kura Linux)`**
   - Paket yang memiliki dependensi *hardcoded* ke Systemd atau library proprietary dan memerlukan patch OpenRC / penggantian pustaka bebas.

---

## 1. Kategori `system/` (22 Paket — Fondasi OS & Toolchain Kompilasi)

> **Peran Kura Linux:** Membentuk meta-paket `base` dan `base-devel`, Seed Toolchain `dist/kura-toolchain.tar.xz`, serta lingkungan kompilasi mandiri di chroot.

| Paket | Versi | Status | Runtime Dependencies | Build Dependencies | Catatan Kura Linux |
| :--- | :---: | :---: | :--- | :--- | :--- |
| **`base`** | 1.0.0 | ✅ Verified | `glibc`, `bash`, `coreutils`, `openrc`, `util-linux`, `shadow`, `eudev`, `kmod`, `sed`, `grep` | - | Meta-paket fondasi minimal OS |
| **`base-devel`**| 1.0.0 | ✅ Verified | `glibc`, `clang`, `gcc`, `llvm`, `mold`, `binutils`, `make`, `ninja`, `pkgconf`, `linux-headers`, `patch`, `bison`, `flex`, `m4`, `libtool`, `autoconf`, `automake` | - | Meta-paket toolchain lengkap |
| **`glibc`** | 2.44 | ✅ Verified | - | `gcc`, `make`, `bison`, `gawk`, `sed` | C runtime library (ADR-002: GCC build) |
| **`openrc`** | 0.64 | ✅ Verified | `glibc` | `gcc`, `meson`, `ninja`, `pkgconf` | Init system resmi Kura Linux |
| **`clang`** | 22.1.0 | ✅ Verified | `glibc`, `llvm` | `cmake`, `ninja`, `gcc`, `python` | C/C++ compiler default |
| **`llvm`** | 23.1.1 | ✅ Verified | `glibc`, `zlib` | `cmake`, `ninja`, `gcc`, `python` | Backend compiler LLVM 22 |
| **`mold`** | 2.42.1 | ✅ Verified | `glibc`, `zlib` | `clang`, `cmake`, `ninja` | High-performance linker (`-fuse-ld=mold`) |
| **`gcc`** | 16.2.0 | ✅ Verified | `glibc`, `gmp`, `mpfr`, `mpc` | `binutils`, `make`, `m4` | Fallback compiler & Glibc builder |
| **`binutils`** | 2.44 | ✅ Verified | `glibc` | `gcc`, `make`, `bison`, `flex` | Assembler (`as`), linker archive (`ar`) |
| **`make`** | 4.4.1 | ✅ Verified | `glibc` | `gcc` | GNU Make build system |
| **`ninja`** | 1.13.2 | ✅ Verified | `glibc` | `gcc`, `cmake`, `python` | Small build system |
| **`cmake`** | 3.31.5 | ✅ Verified | `glibc`, `openssl`, `zlib` | `gcc`, `make` | Cross-platform build automation |
| **`meson`** | 1.12.0 | ✅ Verified | `python`, `ninja` | `python` | Fast build system |
| **`pkgconf`** | 3.0.7 | ✅ Verified | `glibc` | `gcc`, `make` | Package compiler metadata toolkit |
| **`linux-headers`**| 7.3-rc3 | ✅ Verified | - | `make`, `rsync` | Kernel userspace API headers |
| **`m4`** | 1.4.21 | ✅ Verified | `glibc` | `gcc`, `make` | Macro processor |
| **`autoconf`** | 2.73 | ✅ Verified | `m4`, `perl` | `make` | Extensible package of M4 macros |
| **`automake`** | 1.18 | ✅ Verified | `autoconf` | `make` | Tool for generating GNU Makefiles |
| **`libtool`** | 2.6.2 | ✅ Verified | `m4` | `gcc`, `make` | Generic library support script |
| **`patch`** | 3.1.2 | ✅ Verified | `glibc` | `gcc`, `make` | Tool for patching source trees |
| **`bison`** | 3.8.2 | ✅ Verified | `glibc`, `m4` | `gcc`, `make` | Parser generator |
| **`flex`** | 2.6.4 | ✅ Verified | `glibc`, `m4` | `gcc`, `make`, `bison` | Fast lexical analyzer generator |

---

## 2. Kategori `core/` (57 Paket — Utilitas Inti, Storage, Filesystem & Daemons)

> **Peran Kura Linux:** Menyediakan pustaka dasar, manajemen disk & partisi, networking, keamanan, serta daemon layanan OpenRC.

| Paket | Versi | Status | Kategori Peran | Catatan Kura Linux |
| :--- | :---: | :---: | :--- | :--- |
| **`bash`** | 5.3.3 | ✅ Verified | Shell Inti | Default interactive shell |
| **`coreutils`** | 9.7 | ✅ Verified | Utilitas Dasar | GNU core utilities (ls, cp, rm, mv) |
| **`util-linux`** | 2.42.3 | ✅ Verified | Utilitas Sistem | mount, blkid, fdisk, lsblk, dmesg |
| **`shadow`** | 4.17.3 | ✅ Verified | Akun & Autentikasi | passwd, useradd, groupadd, su |
| **`eudev`** | 3.2.14 | ✅ Verified | Device Manager | OpenRC device daemon (pengganti systemd-udevd) |
| **`kmod`** | 34.2 | ✅ Verified | Kernel Modules | modprobe, insmod, lsmod, rmmod |
| **`e2fsprogs`** | 1.47.2 | ✅ Verified | Filesystem ext4 | Format & perbaikan ext4/ext3 |
| **`btrfs-progs`** | 7.1 | 🟡 Scaffolded | Filesystem Btrfs | Manajemen snapshot, volume & subvolume Btrfs |
| **`xfsprogs`** | 7.2.0 | 🟡 Scaffolded | Filesystem XFS | Utilitas filesystem XFS modern |
| **`dosfstools`** | 4.2 | 🟡 Scaffolded | Filesystem FAT/EFI | Format & perbaikan partisi FAT32/EFI |
| **`parted`** | 3.7 | 🟡 Scaffolded | Partisi Disk | Manipulasi tabel partisi GPT/MBR |
| **`f2fs-tools`** | 1.16.0 | 🟡 Scaffolded | Filesystem F2FS | Flash-Friendly File System tools |
| **`squashfs-tools`**| 4.7.5 | 🟡 Scaffolded | Stage Packaging | Pembuat rootfs live SquashFS |
| **`grub`** | 2.12 | 🟡 Scaffolded | Bootloader | Universal bootloader EFI & BIOS |
| **`efibootmgr`** | 18 | 🟡 Scaffolded | EFI Boot | Manipulasi entri UEFI boot manager |
| **`dbus`** | 1.16.2 | 🟡 Scaffolded | IPC Daemon | D-Bus Message Bus System (OpenRC service) |
| **`polkit`** | 127 | 🟡 Scaffolded | Kebijakan Akses | Application privilege control (non-systemd) |
| **`elogind`** | 246.10 | 🟡 Scaffolded | User Session | Logind standalone untuk OpenRC (Wayland helper) |
| **`curl`** | 8.13.0 | ✅ Verified | Jaringan & Unduhan | HTTP/HTTPS client engine |
| **`openssl`** | 4.1.0-alpha1 | ✅ Verified | Kriptografi | SSL/TLS toolkit |
| **`dhcpcd`** | 10.2.2 | ✅ Verified | Jaringan | DHCP client daemon untuk OpenRC |
| **`iproute2`** | 7.2.0 | ✅ Verified | Jaringan | IP routing & interface control (ip link) |
| **`iptables`** | 1.8.11 | ✅ Verified | Firewall | Netfilter firewall |
| **`nftables`** | 1.1.1 | ✅ Verified | Firewall | Modern packet classification framework |
| **`wpa_supplicant`**| 2.12 | ✅ Verified | Wi-Fi | WPA/WPA2/WPA3 wireless daemon |
| **`cronie`** | 1.7.2 | ✅ Verified | Task Scheduler | Cron daemon untuk OpenRC |
| **`sysklogd`** | 2.7.2 | ✅ Verified | System Logger | Syslog & klog daemon untuk OpenRC |
| **`zstd`** | 1.5.7 | ✅ Verified | Kompresi | Default fast compression (`.forge.tar.zst`) |
| **`xz`** | 5.8.4 | ✅ Verified | Kompresi | Ultra-high compression (`.tar.xz`) |
| **`gzip`**, **`bzip2`**, **`tar`**, **`unzip`**, **`zip`** | - | ✅ Verified | Kompresi | Arsip standar UNIX |

---

## 3. Kategori `extra/` (62 Paket — Shells, CLI Modern, Development Tools & Wayland)

> **Peran Kura Linux:** Menghadirkan lingkungan terminal modern, bahasa pemrograman, dan fondasi grafis Wayland.

| Paket | Versi | Status | Tipe | Catatan Kura Linux |
| :--- | :---: | :---: | :--- | :--- |
| **`fish`** | 4.9.3 | 🟡 Scaffolded | Shell | Smart interactive shell modern |
| **`starship`** | 1.26.0 | 🟡 Scaffolded | Prompt | Cross-shell customizable prompt (Rust) |
| **`nushell`** | 0.115.1 | 🟡 Scaffolded | Shell | Data-driven structured shell (Rust) |
| **`zellij`** | 0.45.1 | 🟡 Scaffolded | Terminal Multiplexer | Rust-based modern workspace multiplexer |
| **`helix`** | 25.07.1 | 🟡 Scaffolded | Text Editor | Post-modern modal text editor (Rust) |
| **`lazygit`** | 0.65.1 | 🟡 Scaffolded | Git TUI | Terminal UI untuk alur kerja Git |
| **`fzf`** | 0.74.4 | 🟡 Scaffolded | Fuzzy Finder | Interactive CLI filter & fuzzy searcher |
| **`bottom`** | 0.14.9 | 🟡 Scaffolded | Monitoring | Graphical process & system monitor (Rust) |
| **`eza`** | 0.23.5 | 🟡 Scaffolded | CLI Utility | Modern replacement for `ls` (Rust) |
| **`zoxide`** | 0.10.0 | 🟡 Scaffolded | Navigasi | Smarter `cd` command berbasis frecency |
| **`procs`** | 0.14.12 | 🟡 Scaffolded | CLI Utility | Modern replacement for `ps` (Rust) |
| **`duf`** | 0.9.1 | 🟡 Scaffolded | Storage TUI | Disk Usage/Free utility intuitif |
| **`dust`** | 1.2.6 | 🟡 Scaffolded | Storage TUI | Du + Rust visual directory analyzer |
| **`tokei`** | 15.0.0 | 🟡 Scaffolded | Dev Utility | Code statistics & LOC counter (Rust) |
| **`bandwhich`** | 0.23.1 | 🟡 Scaffolded | Jaringan | Terminal bandwidth utilization monitor |
| **`grex`** | 1.4.6 | 🟡 Scaffolded | Dev Utility | Regex generator dari sample data |
| **`hyperfine`** | 1.20.0 | 🟡 Scaffolded | Benchmarking | CLI benchmarking tool (Rust) |
| **`gh`** | 2.68.0 | 🟡 Scaffolded | GitHub CLI | Official GitHub CLI client |
| **`wayland`** | 1.26.0 | 🟡 Scaffolded | Display Server | Wayland core protocol library |
| **`wayland-protocols`**| 1.49 | 🟡 Scaffolded | Display Server | Extended Wayland protocols |
| **`seatd`** | 0.9.3 | 🟡 Scaffolded | Seat Management | Minimal seat management daemon (OpenRC) |
| **`foot`** | 1.28.0 | 🟡 Scaffolded | Terminal Emulator | Fast, lightweight Wayland terminal emulator |
| **`alacritty`** | 0.17.0 | 🟡 Scaffolded | Terminal Emulator | GPU-accelerated terminal emulator (Rust) |
| **`fastfetch`** | 2.38.0 | ✅ Verified | System Info | Ultra-fast neofetch alternative (C) |
| **`ripgrep`** | 15.2.0 | ✅ Verified | Search | Ultra-fast regex recursive grep (Rust) |
| **`fd`** | 10.5.0 | ✅ Verified | Search | Simple, fast user-friendly `find` (Rust) |
| **`bat`** | 0.25.0 | ✅ Verified | CLI Utility | `cat` clone with syntax highlighting |
| **`btop`** | 1.4.0 | ✅ Verified | Monitoring | Resource monitor visual TUI |
| **`htop`** | 3.5.3 | ✅ Verified | Monitoring | Interactive process viewer |
| **`neovim`** | 0.12.5 | ✅ Verified | Text Editor | Extensible Vim-fork text editor |
| **`tmux`** | 3.8-rc | ✅ Verified | Multiplexer | Terminal multiplexer standar |
| **`rust`**, **`cargo`** | 1.98.1 | ✅ Verified | Bahasa Pemrograman | Rust language compiler & package manager |
| **`python`** | 3.14.7 | ✅ Verified | Bahasa Pemrograman | Python 3 runtime & library |
| **`git`** | 2.55.0 | ✅ Verified | Version Control | Distributed version control system |
| **`ccache`** | 4.10.2 | ✅ Verified | Compiler Cache | C/C++ compiler cache accelerator |
| **`sudo`**, **`doas`** | - | ✅ Verified | Hak Akses | Privilege escalation utilities |

---

## 🛠️ Alur Siklus Pengujian Resep Baru di Kura Linux

```mermaid
flowchart LR
    A["1. scripts/dump_recipes.py<br/>(Scaffold dari Arch/AUR)"] --> B["2. recipe.toml Validated<br/>(cargo test --workspace)"]
    B --> C["3. Sandbox tmpfs Build<br/>(Bubblewrap + Ccache)"]
    C --> D["4. Pre-Flight Collision<br/>(forge merge scan)"]
    D --> E["5. Status Diperbarui ke ✅ Verified<br/>(recipes/PACKAGE_STATUS.md)"]
```
