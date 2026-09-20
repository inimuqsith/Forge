# Matriks Status & Kesiapan Resep Paket Kura Linux (`PACKAGE_STATUS.md`)

> **Single Source of Truth (SSOT)**: Dokumen ini memuat katalog lengkap, versi hulu resmi, pohon dependensi, dan status kesiapan seluruh paket perangkat lunak distribusi **Kura Linux** yang dikelola oleh package manager **`forge`**.

---

## 📊 Ringkasan Statistik Status Katalog Paket

| Kategori | Total Paket | Status Kesiapan | Deskripsi Ruang Lingkup |
| :--- | :---: | :---: | :--- |
| **`recipes/system/`** | 27 | 🟡 Belum Diuji | Toolchain Sistem, Kernel Headers, C Library & Inisialisasi OpenRC |
| **`recipes/core/`** | 87 | 🟡 Belum Diuji | Utilitas Dasar, CLI Tools, Filesystem, Networking & Service Daemons |
| **`recipes/extra/`** | 113 | 🟡 Belum Diuji | Bahasa Pemrograman, Desktop Environment, Library Grafis, Audio & Qt6/KDE |
| **TOTAL RESEP RESMI** | **227** | **🟡 227 Resep Terdaftar (Belum Diuji)** | **Ekosistem Lengkap Kura Linux (Base, Toolchain, Kernel, Hardware, Firmware, Audio, Qt6 & KDE Plasma 6)** |

---

## 1. Kategori `recipes/system/` (27 Paket — Toolchain Sistem, Kernel Headers, C Library & Inisialisasi OpenRC)

| Paket | Versi Hulu | Status | Runtime Dependencies | Build Dependencies | Deskripsi |
| :--- | :---: | :---: | :--- | :--- | :--- |
| **`autoconf`** | `2.73` | 🟡 Belum Diuji | `m4`, `glibc` | `make`, `m4`, `gcc` | Extensible package of M4 macros to produce configuration scripts |
| **`automake`** | `1.19` | 🟡 Belum Diuji | `autoconf`, `glibc` | `make`, `autoconf`, `gcc` | Tool for automatically generating Makefile.in files |
| **`base`** | `1.0.0` | 🟡 Belum Diuji | `glibc`, `coreutils`, `sed`, `grep`, `gawk`, `tar`, `xz`, `zstd`, `findutils`, `diffutils`, `file`, `which`, `util-linux`, `shadow`, `openrc`, `eudev`, `kmod`, `ca-certificates` | - | Kura Linux Minimal Base System (Meta-Package) |
| **`base-devel`** | `1.0.0` | 🟡 Belum Diuji | `glibc`, `llvm`, `mold`, `make`, `ninja`, `gcc`, `binutils`, `pkgconf`, `linux-headers`, `patch`, `bubblewrap` | - | Kura Linux Base Development Toolchain (Meta-Package) |
| **`binutils`** | `2.47` | 🟡 Belum Diuji | `glibc`, `zlib`, `zstd` | `bison`, `flex` | GNU binary utilities (as, ld, readelf, objdump, strip, ar) |
| **`bison`** | `3.8.2` | 🟡 Belum Diuji | `glibc`, `m4` | `make`, `m4`, `gcc` | General-purpose parser generator |
| **`clang`** | `23.1.1` | 🟡 Belum Diuji | `llvm`, `glibc` | `cmake`, `ninja`, `pkgconf`, `make`, `python` | C, C++, and Objective-C front-end for LLVM (v22) |
| **`cmake`** | `4.3.5` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib`, `libarchive` | `make`, `gcc`, `binutils`, `pkgconf` | Cross-platform open-source build system generator |
| **`flex`** | `2.6.4` | 🟡 Belum Diuji | `glibc`, `m4` | `make`, `m4`, `bison`, `gcc` | Fast lexical analyzer generator |
| **`forge`** | `git` | 🟡 Belum Diuji | `glibc`, `zlib`, `zstd`, `bubblewrap`, `mold`, `clang`, `llvm`, `git`, `ca-certificates`, `tar`, `xz` | `rust`, `mold`, `clang`, `llvm`, `pkgconf`, `git` | High-Performance Source-First & Hybrid Package Manager for Kura Linux |
| **`gcc`** | `16.2.0` | 🟡 Belum Diuji | `glibc`, `gmp`, `mpfr`, `mpc`, `zstd` | `bison`, `flex` | GNU Compiler Collection (C and C++ Compilers - Latest 15.3) |
| **`gettext`** | `1.0` | 🟡 Belum Diuji | `glibc`, `acl`, `ncurses` | `make`, `gcc` | GNU internationalization and localization utilities |
| **`glibc`** | `2.44` | 🟡 Belum Diuji | - | `linux-headers` | GNU C Library (Standard Core System C Library - Latest 2.44) |
| **`gperf`** | `3.3` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Perfect hash function generator |
| **`libtool`** | `2.6.2` | 🟡 Belum Diuji | `glibc`, `m4` | `make`, `m4`, `autoconf`, `automake`, `gcc` | Generic library support script |
| **`linux-cachyos-bore`** | `git` | 🟡 Belum Diuji | `kmod`, `eudev` | `clang`, `llvm`, `mold`, `make`, `bc`, `bison`, `flex`, `elfutils`, `openssl`, `rsync`, `kmod`, `zstd`, `diffutils` | Linux CachyOS Kernel bleeding-edge Git with BORE scheduler, sched-ext, and LLVM 22 LTO |
| **`linux-headers`** | `7.2` | 🟡 Belum Diuji | - | - | Linux kernel API headers for userspace |
| **`llvm`** | `23.1.1` | 🟡 Belum Diuji | `glibc`, `zlib`, `zstd`, `libxml2` | `cmake`, `ninja`, `pkgconf`, `make`, `python` | LLVM Compiler Infrastructure with Clang, LLD, and Compiler-RT (v22) |
| **`m4`** | `1.4.21` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | GNU Macro Processor |
| **`make`** | `4.4.1` | 🟡 Belum Diuji | `glibc` | - | GNU Make utility to maintain groups of programs |
| **`meson`** | `1.12.0` | 🟡 Belum Diuji | `glibc`, `python`, `ninja` | `python` | Fast and user friendly build system |
| **`mold`** | `2.42.1` | 🟡 Belum Diuji | `glibc`, `zlib`, `openssl` | `gcc`, `cmake`, `make` | High-performance modern linker (Latest 2.42.1) |
| **`ninja`** | `1.13.2` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `python` | Small build system with a focus on speed (Latest 1.13.2) |
| **`openrc`** | `0.64` | 🟡 Belum Diuji | `glibc`, `ncurses` | `meson`, `ninja`, `pkgconf`, `make` | Service and init manager for Kura Linux |
| **`patch`** | `2.8` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Utility to apply diffs to files |
| **`perl`** | `5.44.0` | 🟡 Belum Diuji | `glibc`, `zlib`, `bzip2` | `make`, `gcc` | Highly capable, feature-rich programming language |
| **`pkgconf`** | `3.0.7` | 🟡 Belum Diuji | `glibc` | `gcc`, `make` | Package compiler and linker metadata toolkit (Latest 3.0.7) |

---

## 2. Kategori `recipes/core/` (87 Paket — Utilitas Dasar, CLI Tools, Filesystem, Networking & Service Daemons)

| Paket | Versi Hulu | Status | Runtime Dependencies | Build Dependencies | Deskripsi |
| :--- | :---: | :---: | :--- | :--- | :--- |
| **`acl`** | `2.4.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Access control list utilities and library |
| **`acpid`** | `2.0.34` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Flexible and extensible ACPI event daemon |
| **`amd-ucode`** | `20260916` | 🟡 Belum Diuji | - | - | Microcode update image and firmware for AMD CPUs |
| **`bash`** | `5.3` | 🟡 Belum Diuji | `glibc`, `readline`, `ncurses` | `make`, `gcc`, `bison`, `pkgconf` | GNU Bourne Again SHell |
| **`bc`** | `1.08.2` | 🟡 Belum Diuji | `glibc`, `readline` | `make`, `gcc`, `flex`, `bison` | Arbitrary precision numeric processing language |
| **`bluez`** | `5.87` | 🟡 Belum Diuji | `glibc`, `dbus`, `glib2`, `eudev`, `readline` | `gcc`, `make`, `pkgconf`, `libtool` | Official Linux Bluetooth protocol stack with OpenRC service |
| **`btrfs-progs`** | `7.1` | 🟡 Belum Diuji | `glibc`, `libgcrypt`, `lzo`, `eudev`, `util-linux`, `zlib`, `zstd` | `gcc`, `e2fsprogs`, `pkgconf`, `make` | Btrfs filesystem utilities |
| **`bubblewrap`** | `0.12.0` | 🟡 Belum Diuji | `glibc`, `libcap` | `meson`, `ninja`, `pkgconf`, `gcc` | Unprivileged sandboxing tool based on Linux user namespaces |
| **`bzip2`** | `1.0.8` | 🟡 Belum Diuji | `glibc` | - | A high-quality data compression program |
| **`ca-certificates`** | `20260909` | 🟡 Belum Diuji | `glibc`, `openssl` | `python` | Common CA root certificates bundle from Mozilla |
| **`coreutils`** | `9.12` | 🟡 Belum Diuji | `glibc`, `libcap`, `acl` | `make`, `gcc`, `pkgconf` | The basic file, shell and text manipulation utilities of the GNU operating system |
| **`cronie`** | `1.7.2` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Standard cron daemon and crontab scheduler |
| **`curl`** | `8.22.0` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib`, `zstd` | `make`, `gcc`, `pkgconf` | Command line tool and library for transferring data with URLs |
| **`dbus`** | `1.16.2` | 🟡 Belum Diuji | `glibc`, `expat`, `libcap-ng`, `eudev` | `gcc`, `meson`, `ninja`, `pkgconf`, `glib2`, `python` | Freedesktop.org message bus system |
| **`dhcpcd`** | `10.5.2` | 🟡 Belum Diuji | `glibc`, `openssl` | `make`, `gcc`, `pkgconf` | RFC2131 and RFC3315 compliant DHCP/DHCPv6 and IPv4LL dual-stack client |
| **`diffutils`** | `3.12` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | GNU diff, cmp, diff3 and sdiff programs |
| **`dosfstools`** | `4.2` | 🟡 Belum Diuji | `glibc` | `gcc` | DOS filesystem utilities |
| **`e2fsprogs`** | `1.47.4` | 🟡 Belum Diuji | `glibc`, `util-linux` | `make`, `gcc`, `pkgconf` | Ext2/3/4 filesystem management utilities (mke2fs, fsck.ext4) |
| **`efibootmgr`** | `18` | 🟡 Belum Diuji | `glibc`, `efivar`, `popt` | `gcc`, `make`, `pkgconf`, `linux-headers` | Linux user-space application to modify the EFI Boot Manager |
| **`efivar`** | `39` | 🟡 Belum Diuji | `glibc`, `popt` | `gcc`, `make`, `pkgconf`, `linux-headers` | Tools and library to manipulate EFI variables |
| **`elfutils`** | `0.196` | 🟡 Belum Diuji | `glibc`, `zlib`, `bzip2`, `xz`, `zstd` | `make`, `gcc`, `pkgconf`, `m4`, `flex`, `bison` | Libraries and tools for handling ELF files and DWARF data |
| **`elogind`** | `257.16` | 🟡 Belum Diuji | `glibc`, `pam`, `acl`, `libcap` | `gcc`, `intltool`, `libtool`, `gperf`, `libcap`, `meson`, `ninja`, `pkgconf` | The systemd project |
| **`ethtool`** | `7.1` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf` | Utility for examining and tuning network interfaces and drivers |
| **`eudev`** | `3.2.14` | 🟡 Belum Diuji | `glibc`, `kmod`, `hwdata` | `make`, `gcc`, `pkgconf`, `gperf` | Standalone device manager fork of systemd-udev for OpenRC |
| **`f2fs-tools`** | `1.16.0` | 🟡 Belum Diuji | `glibc`, `util-linux` | `gcc`, `git` | Tools for Flash-Friendly File System (F2FS) |
| **`file`** | `5.48` | 🟡 Belum Diuji | `glibc`, `zlib`, `bzip2`, `xz`, `zstd` | `make`, `gcc`, `pkgconf` | File type identification utility and libmagic |
| **`findutils`** | `4.11.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | GNU utilities for finding files (find, xargs, locate) |
| **`gawk`** | `5.4.1` | 🟡 Belum Diuji | `glibc`, `readline`, `gmp`, `mpfr` | `make`, `gcc` | GNU awk pattern scanning and processing language |
| **`grep`** | `3.12` | 🟡 Belum Diuji | `glibc`, `pcre2` | `make`, `gcc`, `pkgconf` | GNU grep, egrep and fgrep |
| **`groff`** | `1.24.1` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `bison`, `pkgconf` | GNU troff text-formatting system |
| **`grub`** | `2.14` | 🟡 Belum Diuji | `glibc`, `xz` | `gcc`, `make`, `pkgconf`, `flex`, `bison`, `python` | GNU GRand Unified Bootloader (2) |
| **`gzip`** | `1.14` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Popular data compression program |
| **`hwdata`** | `0.411` | 🟡 Belum Diuji | `glibc` | `make` | Hardware identification databases (pci.ids, usb.ids, oui.txt) |
| **`intel-ucode`** | `20260812` | 🟡 Belum Diuji | - | - | Microcode update files and early-initramfs image for Intel CPUs |
| **`intltool`** | `0.51.0` | 🟡 Belum Diuji | `perl` | `make`, `gcc`, `perl` | Internationalization tool collection |
| **`iproute2`** | `7.2.0` | 🟡 Belum Diuji | `glibc`, `libcap`, `libseccomp` | `make`, `gcc`, `bison`, `flex`, `pkgconf` | IP routing and network device configuration suite (ip, ss, tc) |
| **`iptables`** | `1.8.13` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf` | Linux kernel packet filtering and NAT control utility |
| **`kbd`** | `2.10.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `bison`, `flex`, `pkgconf` | Keytable files and keyboard utilities (loadkeys, setfont) |
| **`kmod`** | `34.2` | 🟡 Belum Diuji | `glibc`, `zlib`, `xz`, `zstd`, `openssl` | `gcc`, `make`, `pkgconf` | Linux kernel module management tools and library (lsmod, modprobe, insmod) |
| **`less`** | `710` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc` | A terminal based program for viewing text files |
| **`libarchive`** | `3.8.9` | 🟡 Belum Diuji | `glibc`, `zlib`, `bzip2`, `xz`, `zstd`, `openssl`, `expat` | `make`, `gcc`, `pkgconf` | Multi-format archive and compression library (bsdtar, bsdcpio) |
| **`libcap`** | `2.78` | 🟡 Belum Diuji | `glibc` | - | POSIX 1003.1e capabilities library and tools (setcap, getcap) |
| **`libcap-ng`** | `0.9.6` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `autoconf`, `automake`, `libtool` | Alternate POSIX capabilities library |
| **`libedit`** | `20260512-3.1` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc` | NetBSD Editline library (BSD-licensed alternative to GNU readline) |
| **`libevdev`** | `1.13.7` | 🟡 Belum Diuji | `glibc` | `meson`, `ninja`, `gcc`, `pkgconf`, `python` | Wrapper library for evdev devices |
| **`libgcrypt`** | `1.12.4` | 🟡 Belum Diuji | `glibc`, `libgpg-error` | `make`, `gcc` | General purpose cryptographic library based on the code from GnuPG |
| **`libgpg-error`** | `1.61` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Small library with error codes and strings based on libgcrypt |
| **`libinih`** | `62` | 🟡 Belum Diuji | `glibc` | `meson`, `ninja`, `gcc`, `pkgconf` | Simple INI file parser written in C |
| **`libpciaccess`** | `0.19` | 🟡 Belum Diuji | `glibc`, `zlib` | `meson`, `ninja`, `gcc`, `pkgconf` | Generic PCI access library |
| **`libseccomp`** | `2.6.1` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Enhanced Seccomp library and kernel syscall filtering interface |
| **`liburcu`** | `0.15.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Userspace RCU (read-copy-update) library |
| **`linux-firmware`** | `20260916` | 🟡 Belum Diuji | - | `make` | Firmware files for Linux kernel drivers (Wi-Fi, GPU, Bluetooth, Audio SOF) |
| **`lz4`** | `1.10.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf` | Extremely fast compression algorithm |
| **`lzo`** | `2.10` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Portable lossless data compression library |
| **`man-pages`** | `6.19` | 🟡 Belum Diuji | `glibc` | `make` | Linux system documentation manual pages |
| **`mkinitcpio`** | `42` | 🟡 Belum Diuji | `bash`, `kmod`, `coreutils`, `util-linux`, `zstd`, `findutils` | `make` | Modular and fast initramfs creation utility for Linux |
| **`mtdev`** | `1.1.7` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Multitouch Protocol Translation Library |
| **`nano`** | `9.2` | 🟡 Belum Diuji | `glibc`, `ncurses`, `file` | `make`, `gcc`, `pkgconf` | Pico editor clone with enhancements |
| **`ncurses`** | `6.5` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf` | System V Release 4.0 curses emulation library |
| **`nftables`** | `1.1.7` | 🟡 Belum Diuji | `glibc`, `gmp`, `readline` | `make`, `gcc`, `bison`, `flex`, `pkgconf` | Netfilter userspace packet filtering framework |
| **`openssl`** | `4.1.0-alpha1` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `perl` | Robust, commercial-grade TLS/SSL cryptography toolkit |
| **`pam`** | `1.7.2` | 🟡 Belum Diuji | `glibc` | `gcc`, `make`, `flex`, `bison`, `linux-headers`, `pkgconf` | Pluggable Authentication Modules for Linux |
| **`parted`** | `3.7` | 🟡 Belum Diuji | `glibc`, `util-linux` | `make`, `gcc`, `pkgconf` | A program for creating, destroying, resizing, checking and copying partitions |
| **`pciutils`** | `3.15.0` | 🟡 Belum Diuji | `glibc`, `hwdata`, `kmod` | `pkgconf` | PCI bus configuration and diagnostic tools (lspci, setpci) |
| **`polkit`** | `127` | 🟡 Belum Diuji | `glibc`, `duktape`, `expat`, `glib2`, `pam`, `eudev` | `gcc`, `dbus`, `glib2`, `gobject-introspection`, `meson`, `ninja`, `pkgconf` | Application development toolkit for controlling system-wide privileges |
| **`popt`** | `1.19` | 🟡 Belum Diuji | `glibc` | `gcc`, `make`, `autoconf`, `automake`, `libtool`, `pkgconf` | Command line option parsing library |
| **`procps-ng`** | `4.0.7` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc`, `pkgconf` | Utilities for monitoring your system and its processes (ps, top, free) |
| **`psmisc`** | `23.7` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc` | Miscellaneous proc-based tools (killall, fuser, pstree) |
| **`readline`** | `8.3` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc` | GNU Readline command line editing library |
| **`sed`** | `4.10` | 🟡 Belum Diuji | `glibc`, `acl` | `make`, `gcc` | GNU stream editor |
| **`shadow`** | `4.20.2` | 🟡 Belum Diuji | `glibc`, `acl`, `libcap`, `libseccomp` | `make`, `gcc`, `pkgconf` | Password and account management utilities |
| **`squashfs-tools`** | `4.7.5` | 🟡 Belum Diuji | `glibc`, `gcc`, `lz4`, `lzo`, `xz`, `zlib`, `zstd` | `gcc`, `make`, `pkgconf` | Tools for squashfs, a highly compressed read-only filesystem for Linux |
| **`sysklogd`** | `2.7.2` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf` | Standard Linux system and kernel logging daemons (syslogd, klogd) |
| **`tar`** | `1.35` | 🟡 Belum Diuji | `glibc`, `acl` | `make`, `gcc` | Utility used to store, backup, and transport files |
| **`unzip`** | `6.0` | 🟡 Belum Diuji | `glibc`, `bzip2` | `make`, `gcc` | Extraction utility for archives compressed in .zip format |
| **`usbutils`** | `019` | 🟡 Belum Diuji | `glibc`, `hwdata` | `make`, `gcc`, `pkgconf` | USB device listing and inspection utilities (lsusb) |
| **`util-linux`** | `2.42.3` | 🟡 Belum Diuji | `glibc`, `ncurses`, `zlib`, `libcap`, `libseccomp` | `make`, `gcc`, `pkgconf` | Miscellaneous system utilities for Linux |
| **`wget`** | `1.25.0` | 🟡 Belum Diuji | `glibc`, `openssl`, `pcre2`, `zlib` | `make`, `gcc`, `pkgconf` | Network utility to retrieve files from the Web using HTTP, HTTPS and FTP |
| **`which`** | `2.25` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Displays where a particular program is found in the path |
| **`wireless-regdb`** | `2026.09.03` | 🟡 Belum Diuji | - | - | Wireless regulatory database for Linux kernel and CRDA |
| **`wpa_supplicant`** | `2.12` | 🟡 Belum Diuji | `glibc`, `openssl`, `readline` | `make`, `gcc`, `pkgconf` | WPA/WPA2/WPA3 and IEEE 802.1X wireless client daemon |
| **`xfsprogs`** | `7.2.0` | 🟡 Belum Diuji | `glibc`, `libedit`, `libinih`, `liburcu`, `util-linux` | `gcc`, `git`, `icu` | XFS filesystem utilities |
| **`xkeyboard-config`** | `2.48` | 🟡 Belum Diuji | - | `meson`, `ninja`, `gcc`, `pkgconf`, `python` | X Keyboard Extension configuration data |
| **`xz`** | `5.8.4` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Free general-purpose data compression software with high compression ratio (LZMA2) |
| **`zip`** | `8.6.0` | 🟡 Belum Diuji | `glibc`, `bzip2` | `make`, `gcc` | Compressor utility for zipfile archives |
| **`zsh`** | `5.9.2` | 🟡 Belum Diuji | `glibc`, `ncurses`, `pcre2` | `make`, `gcc`, `pkgconf` | Advanced programmable command interpreter |
| **`zstd`** | `1.5.7-kernel` | 🟡 Belum Diuji | `glibc`, `zlib`, `xz` | `make`, `gcc`, `pkgconf` | Zstandard - Fast real-time compression algorithm |

---

## 3. Kategori `recipes/extra/` (113 Paket — Bahasa Pemrograman, Desktop Environment, Library Grafis, Audio & Qt6/KDE)

| Paket | Versi Hulu | Status | Runtime Dependencies | Build Dependencies | Deskripsi |
| :--- | :---: | :---: | :--- | :--- | :--- |
| **`alacritty`** | `0.17.0` | 🟡 Belum Diuji | `glibc`, `freetype`, `fontconfig`, `libxi`, `libxcursor`, `libxkbcommon`, `libxrandr`, `libxcb` | `gcc`, `rust`, `mold`, `clang`, `llvm`, `cmake`, `ncurses`, `pkgconf`, `libxcb` | A cross-platform, GPU-accelerated terminal emulator |
| **`alsa-lib`** | `1.2.16.1` | 🟡 Belum Diuji | `glibc` | `autoconf`, `automake`, `libtool`, `make`, `pkgconf` | Advanced Linux Sound Architecture (ALSA) core runtime library |
| **`alsa-utils`** | `1.2.16` | 🟡 Belum Diuji | `glibc`, `alsa-lib`, `ncurses` | `autoconf`, `automake`, `libtool`, `make`, `pkgconf`, `gettext` | Advanced Linux Sound Architecture (ALSA) utilities (alsamixer, amixer, aplay) |
| **`antigravity-cli`** | `1.2.7` | 🟡 Belum Diuji | `glibc`, `ca-certificates` | `tar`, `gzip` | Official Google DeepMind Antigravity CLI (agy) for agentic AI pair programming |
| **`bandwhich`** | `0.23.1` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `cargo` | Terminal bandwidth utilization tool |
| **`bat`** | `0.26.1` | 🟡 Belum Diuji | `glibc`, `zlib` | `cargo`, `rust` | A cat clone with syntax highlighting and Git integration |
| **`bottom`** | `0.14.9` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `git`, `rust` | A graphical process/system monitor |
| **`breeze`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `kcoreaddons`, `kconfig`, `kwidgetsaddons`, `kwindowsystem` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Artwork, styles and assets for the Breeze visual theme |
| **`btop`** | `1.4.7` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Modern and beautiful resource monitor that shows usage and stats for processor, memory, disks, network and processes |
| **`cairo`** | `1.18.4` | 🟡 Belum Diuji | `glibc`, `pixman`, `freetype`, `fontconfig`, `zlib`, `libpng` | `meson`, `ninja`, `pkgconf` | 2D graphics library with support for multiple output devices |
| **`cargo`** | `1.85.0` | 🟡 Belum Diuji | `glibc`, `rust`, `openssl`, `curl`, `zlib` | `rust` | Rust Package Manager and Build Tool |
| **`ccache`** | `4.14` | 🟡 Belum Diuji | `glibc`, `zstd` | `cmake`, `ninja`, `gcc`, `pkgconf` | Fast compiler cache for C/C++ |
| **`cowsay`** | `3.8.4` | 🟡 Belum Diuji | `glibc`, `perl` | `make` | Configurable talking cow (Latest 3.8.4) |
| **`doas`** | `6.8.2` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `bison`, `flex` | Simple OpenBSD privilege escalation tool ported to Linux |
| **`duf`** | `0.9.1` | 🟡 Belum Diuji | `glibc` | `gcc`, `git`, `go` | Disk Usage/Free Utility |
| **`duktape`** | `2.7.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Embeddable Javascript engine with a focus on portability and compact footprint |
| **`dust`** | `1.2.6` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `git`, `rust` | A more intuitive version of du in rust |
| **`expat`** | `2.8.4` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | XML parser library written in C |
| **`extra-cmake-modules`** | `6.30.0` | 🟡 Belum Diuji | - | `cmake` | Extra modules and scripts for CMake used by KDE Frameworks |
| **`eza`** | `0.23.5` | 🟡 Belum Diuji | `glibc`, `gcc`, `libgit2` | `rust`, `mold`, `clang`, `llvm`, `pkgconf` | A modern replacement for ls (community fork of exa) |
| **`fastfetch`** | `2.68.1` | 🟡 Belum Diuji | `glibc`, `zlib` | `cmake`, `ninja`, `pkgconf`, `gcc` | Like neofetch, but much faster because written in C (Latest 2.38.0) |
| **`fcft`** | `3.3.3` | 🟡 Belum Diuji | `glibc`, `fontconfig`, `freetype`, `pixman`, `libutf8proc` | `meson`, `ninja`, `gcc`, `pkgconf`, `tllist` | Simple library for font loading and glyph rasterization |
| **`fd`** | `10.5.0` | 🟡 Belum Diuji | `glibc` | `cargo`, `rust` | Simple, fast and user-friendly alternative to find |
| **`fish`** | `4.9.3` | 🟡 Belum Diuji | `glibc`, `gcc`, `pcre2` | `gcc`, `cmake`, `ninja`, `rust`, `pkgconf` | Smart and user friendly shell intended mostly for interactive use |
| **`fontconfig`** | `2.18.3` | 🟡 Belum Diuji | `glibc`, `freetype`, `expat` | `meson`, `ninja`, `pkgconf`, `gperf` | Library for configuring and customizing font access |
| **`foot`** | `1.28.0` | 🟡 Belum Diuji | `glibc`, `fcft`, `fontconfig`, `libutf8proc`, `libxkbcommon`, `ncurses`, `pixman`, `wayland` | `gcc`, `meson`, `ninja`, `pkgconf`, `tllist`, `wayland-protocols` | Fast, lightweight, and minimalistic Wayland terminal emulator |
| **`freetype`** | `2.14.3` | 🟡 Belum Diuji | `glibc`, `zlib`, `bzip2`, `libpng` | `meson`, `ninja`, `pkgconf` | Freely available software library to render fonts |
| **`fzf`** | `0.74.4` | 🟡 Belum Diuji | `glibc` | `gcc`, `git`, `go` | Command-line fuzzy finder |
| **`gh`** | `2.101.0` | 🟡 Belum Diuji | `glibc`, `git` | `gcc`, `make`, `go` | The GitHub CLI |
| **`git`** | `2.55.0` | 🟡 Belum Diuji | `glibc`, `curl`, `openssl`, `zlib`, `expat`, `pcre2` | `make`, `gcc`, `pkgconf` | Fast, scalable, distributed revision control system |
| **`glib2`** | `2.90.0` | 🟡 Belum Diuji | `glibc`, `libffi`, `pcre2`, `zlib` | `meson`, `ninja`, `pkgconf`, `gcc` | Core low-level data structure and utility library from GNOME |
| **`gmp`** | `6.3.0` | 🟡 Belum Diuji | `glibc` | `m4` | GNU Multiple Precision Arithmetic Library |
| **`go`** | `1.27.1` | 🟡 Belum Diuji | `glibc`, `ca-certificates` | `make`, `gcc`, `bash` | Open source programming language that makes it easy to build simple, fast, and reliable software |
| **`gobject-introspection`** | `1.86.0` | 🟡 Belum Diuji | `glibc`, `glib2`, `libffi` | `meson`, `ninja`, `gcc`, `pkgconf`, `python`, `bison`, `flex` | Middleware layer for creating language bindings for C libraries |
| **`grex`** | `1.4.6` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `cargo` | A command-line tool for generating regular expressions from user-provided input strings |
| **`harfbuzz`** | `14.4.0` | 🟡 Belum Diuji | `glibc`, `freetype`, `glib2`, `icu` | `meson`, `ninja`, `pkgconf` | OpenType text shaping engine |
| **`helium-browser`** | `0.17.2.1` | 🟡 Belum Diuji | `glibc`, `dbus`, `ncurses` | `tar`, `xz` | Lightweight, privacy-focused, bloat-free Chromium-based web browser |
| **`helix`** | `25.07.1` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `rust`, `mold`, `clang`, `llvm`, `pkgconf`, `git` | A post-modern modal text editor |
| **`htop`** | `3.5.3` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc`, `pkgconf` | Interactive process viewer for Unix systems |
| **`hyperfine`** | `1.20.0` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `cargo` | A command-line benchmarking tool |
| **`icu`** | `78.3` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf` | International Components for Unicode library |
| **`intellij-idea`** | `2025.2.5` | 🟡 Belum Diuji | `glibc`, `ca-certificates`, `tar` | `tar`, `gzip` | Capable and Ergonomic IDE for JVM, Java, Kotlin, and Polyglot Development by JetBrains |
| **`jq`** | `1.8.2` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `bison`, `pkgconf` | Command-line JSON processor |
| **`kauth`** | `6.30.0` | 🟡 Belum Diuji | `kcoreaddons`, `polkit` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Execute actions as privileged user through authentication backends |
| **`kconfig`** | `6.30.0` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Configuration system for KDE applications and frameworks |
| **`kcoreaddons`** | `6.30.0` | 🟡 Belum Diuji | `qt6-base` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Addons to QtCore offering various utilities and classes |
| **`kcrash`** | `6.30.0` | 🟡 Belum Diuji | `kcoreaddons`, `kwindowsystem` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Graceful handling of application crashes and signal handlers |
| **`kdbusaddons`** | `6.30.0` | 🟡 Belum Diuji | `qt6-base`, `dbus` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Addons to QtDBus offering DBus application management |
| **`kglobalaccel`** | `6.30.0` | 🟡 Belum Diuji | `kconfig`, `kcoreaddons`, `kcrash`, `kdbusaddons`, `kwindowsystem` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Global desktop keyboard shortcuts management engine |
| **`ki18n`** | `6.30.0` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `python` | Advanced internationalization framework for KDE applications |
| **`kpipewire`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `qt6-wayland`, `kcoreaddons`, `ki18n`, `pipewire`, `libdrm`, `mesa` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `wayland-protocols` | Components for Flatpak and PipeWire integration in KDE Plasma |
| **`kservice`** | `6.30.0` | 🟡 Belum Diuji | `kcoreaddons`, `kconfig`, `ki18n` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `flex`, `bison` | Plugin framework for desktop services and applications |
| **`kwidgetsaddons`** | `6.30.0` | 🟡 Belum Diuji | `qt6-base` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Addons to QtWidgets providing diverse GUI widgets |
| **`kwin`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `qt6-wayland`, `kcoreaddons`, `kconfig`, `kwindowsystem`, `kcrash`, `kdbusaddons`, `kglobalaccel`, `kpipewire`, `layer-shell-qt`, `breeze`, `libdrm`, `libinput`, `libxkbcommon`, `mesa`, `wayland` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `wayland-protocols`, `vulkan-headers` | Flexible, high-performance, and feature-rich Wayland window manager |
| **`kwindowsystem`** | `6.30.0` | 🟡 Belum Diuji | `qt6-base`, `qt6-wayland`, `libxkbcommon`, `wayland` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `wayland-protocols` | Access to the windowing system for KDE frameworks (Wayland and X11) |
| **`layer-shell-qt`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `qt6-wayland`, `wayland` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `wayland-protocols` | Qt component for Wayland wl-layer-shell protocol integration |
| **`lazygit`** | `0.65.1` | 🟡 Belum Diuji | `glibc`, `git` | `gcc`, `go` | Simple terminal UI for git commands |
| **`libdrm`** | `2.4.134` | 🟡 Belum Diuji | `glibc`, `libpciaccess` | `meson`, `ninja`, `pkgconf` | Userspace interface to kernel DRM services |
| **`libffi`** | `3.8.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Portable foreign function interface library |
| **`libgit2`** | `1.9.7` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib`, `pcre2` | `cmake`, `ninja`, `gcc`, `pkgconf`, `python` | Highly portable, pure C implementation of the Git core methods |
| **`libinput`** | `1.32.0` | 🟡 Belum Diuji | `glibc`, `eudev`, `libevdev`, `mtdev` | `meson`, `ninja`, `pkgconf` | Input device management and event handling library |
| **`libksysguard`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `kcoreaddons`, `kconfig`, `ki18n`, `kauth`, `zlib` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | Task management and system monitoring library for KDE Plasma |
| **`libpng`** | `1.6.58` | 🟡 Belum Diuji | `glibc`, `zlib` | `make`, `gcc` | Official PNG reference library |
| **`libssh2`** | `1.11.1` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib` | `make`, `gcc`, `pkgconf` | Client-side C library implementing the SSH2 protocol |
| **`libutf8proc`** | `2.11.3` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Clean C library for processing UTF-8 Unicode data |
| **`libxcb`** | `1.17.0` | 🟡 Belum Diuji | `glibc` | `make`, `gcc`, `pkgconf`, `python` | X11 C Bindings library |
| **`libxcursor`** | `1.2.3` | 🟡 Belum Diuji | `glibc`, `libxcb` | `make`, `gcc`, `pkgconf` | Cursor management library for X |
| **`libxi`** | `1.8.3` | 🟡 Belum Diuji | `glibc`, `libxcb` | `make`, `gcc`, `pkgconf` | X11 Input extension library |
| **`libxkbcommon`** | `1.13.2` | 🟡 Belum Diuji | `glibc`, `xkeyboard-config` | `meson`, `ninja`, `pkgconf`, `bison`, `wayland`, `wayland-protocols` | Keyboard description compilation and handling library |
| **`libxml2`** | `2.15.4` | 🟡 Belum Diuji | `glibc`, `zlib`, `xz`, `icu` | `make`, `gcc`, `pkgconf` | XML parsing library and utility toolkit |
| **`libxrandr`** | `1.5.5` | 🟡 Belum Diuji | `glibc`, `libxcb` | `make`, `gcc`, `pkgconf` | X11 RandR extension library |
| **`libxslt`** | `1.1.45` | 🟡 Belum Diuji | `glibc`, `libxml2` | `make`, `gcc`, `pkgconf` | XML stylesheet transformation library (XSLT) |
| **`lsof`** | `4.99.7` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Lists information about files opened by processes |
| **`mesa`** | `26.2.3` | 🟡 Belum Diuji | `glibc`, `libdrm`, `expat`, `zstd`, `zlib`, `libxkbcommon`, `wayland` | `meson`, `ninja`, `pkgconf`, `python`, `bison`, `flex`, `llvm`, `clang`, `wayland-protocols`, `vulkan-headers` | Open-source OpenGL and Vulkan 3D graphics drivers |
| **`mpc`** | `1.4.1` | 🟡 Belum Diuji | `glibc`, `gmp`, `mpfr` | - | Library for the arithmetic of complex numbers with arbitrarily high precision |
| **`mpfr`** | `4.2.2` | 🟡 Belum Diuji | `glibc`, `gmp` | - | Multiple-precision floating-point arithmetic library |
| **`neovim`** | `0.12.5` | 🟡 Belum Diuji | `glibc` | `cmake`, `ninja`, `gcc`, `pkgconf` | Vim-fork focused on extensibility and usability |
| **`nushell`** | `0.115.1` | 🟡 Belum Diuji | `glibc`, `curl`, `gcc`, `libgit2`, `libssh2`, `openssl`, `sqlite`, `zstd` | `gcc`, `rust`, `mold`, `clang`, `llvm`, `pkgconf`, `git` | A new type of shell |
| **`openssh`** | `10.5p1` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib`, `libcap` | `make`, `gcc`, `pkgconf` | Premier connectivity tool for remote login with the SSH protocol |
| **`pcre2`** | `10.48` | 🟡 Belum Diuji | `glibc`, `zlib`, `bzip2`, `readline` | `make`, `gcc`, `pkgconf` | Perl Compatible Regular Expressions 2 (PCRE2) |
| **`pipewire`** | `1.6.9` | 🟡 Belum Diuji | `glibc`, `alsa-lib`, `dbus`, `elogind` | `meson`, `ninja`, `pkgconf` | Low-latency audio/video routing daemon and multimedia processing graph |
| **`pixman`** | `0.46.4` | 🟡 Belum Diuji | `glibc` | `meson`, `ninja`, `pkgconf` | Low-level pixel manipulation and rasterization library |
| **`plasma-desktop`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `qt6-wayland`, `qt6-svg`, `kcoreaddons`, `kconfig`, `kwindowsystem`, `ki18n`, `kauth`, `kwidgetsaddons`, `kservice`, `kcrash`, `kdbusaddons`, `kglobalaccel`, `plasma-workspace`, `libksysguard`, `breeze` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `wayland-protocols` | KDE Plasma Desktop user interface, panels, widgets and settings |
| **`plasma-workspace`** | `6.7.5` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `qt6-wayland`, `qt6-svg`, `kcoreaddons`, `kconfig`, `kwindowsystem`, `ki18n`, `kauth`, `kservice`, `kcrash`, `kdbusaddons`, `kglobalaccel`, `kpipewire`, `layer-shell-qt`, `libksysguard`, `breeze`, `kwin`, `pam`, `shadow` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf`, `wayland-protocols` | KDE Plasma Workspace components and session management |
| **`procs`** | `0.14.12` | 🟡 Belum Diuji | `glibc`, `gcc` | `rust`, `mold`, `clang`, `llvm`, `pkgconf` | A modern replacement for ps written in Rust |
| **`python`** | `3.14.7` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib`, `bzip2`, `xz`, `sqlite`, `libffi`, `expat`, `ncurses`, `readline` | `make`, `gcc`, `pkgconf` | Next generation of the high-level scripting language Python |
| **`qt6-base`** | `6.8.2` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib`, `zstd`, `dbus`, `libxkbcommon`, `fontconfig`, `freetype`, `harfbuzz`, `mesa`, `libdrm` | `cmake`, `ninja`, `pkgconf`, `vulkan-headers` | Cross-platform application and UI framework (Core, Gui, Widgets, Network, DBus) |
| **`qt6-declarative`** | `6.8.2` | 🟡 Belum Diuji | `qt6-base` | `cmake`, `ninja`, `pkgconf`, `python` | Classes for QML and JavaScript languages for Qt6 |
| **`qt6-svg`** | `6.8.2` | 🟡 Belum Diuji | `qt6-base`, `zlib` | `cmake`, `ninja`, `pkgconf` | Classes for displaying the contents of SVG files in Qt6 |
| **`qt6-wayland`** | `6.8.2` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `libxkbcommon`, `libdrm`, `wayland` | `cmake`, `ninja`, `pkgconf`, `wayland-protocols`, `vulkan-headers` | Provides APIs for Wayland client and compositor support in Qt6 |
| **`ripgrep`** | `15.2.0` | 🟡 Belum Diuji | `glibc`, `pcre2` | `cargo`, `rust`, `pkgconf` | Ultra-fast line-oriented search tool combining grep with find |
| **`rsync`** | `3.5.0` | 🟡 Belum Diuji | `glibc`, `zstd`, `openssl`, `acl` | `make`, `gcc`, `pkgconf` | Fast and versatile remote file copying tool |
| **`rust`** | `1.98.1` | 🟡 Belum Diuji | `glibc`, `llvm`, `zlib`, `openssl`, `curl` | `python`, `cmake`, `ninja`, `gcc`, `make` | Empowering everyone to build reliable and efficient systems programming language |
| **`sddm`** | `0.21.0` | 🟡 Belum Diuji | `qt6-base`, `qt6-declarative`, `libxkbcommon`, `pam`, `shadow` | `cmake`, `ninja`, `extra-cmake-modules`, `pkgconf` | QML and Wayland based modern display manager |
| **`seatd`** | `0.9.3` | 🟡 Belum Diuji | `glibc`, `eudev` | `meson`, `ninja`, `pkgconf` | A minimal seat management daemon, and a universal seat management library |
| **`sqlite`** | `3.53.4` | 🟡 Belum Diuji | `glibc`, `readline`, `zlib` | `make`, `gcc`, `pkgconf` | Self-contained serverless SQL database engine |
| **`starship`** | `1.26.0` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `cmake`, `git`, `rust` | The cross-shell prompt for astronauts |
| **`strace`** | `7.2` | 🟡 Belum Diuji | `glibc` | `make`, `gcc` | Diagnostic, debugging and instructional userspace utility for Linux syscall tracing |
| **`sudo`** | `1.9.17p2` | 🟡 Belum Diuji | `glibc`, `openssl`, `zlib` | `make`, `gcc`, `pkgconf` | Authority delegation tool allowing users to execute commands as root |
| **`tailscale`** | `1.102.4` | 🟡 Belum Diuji | `glibc`, `ca-certificates`, `iptables` | `tar`, `gzip` | Zero config VPN daemon and CLI for secure mesh networks |
| **`tllist`** | `1.1.0` | 🟡 Belum Diuji | - | `meson`, `ninja`, `pkgconf` | C header-only implementation of a typed linked list |
| **`tmux`** | `3.8-rc` | 🟡 Belum Diuji | `glibc`, `ncurses` | `make`, `gcc`, `pkgconf` | Terminal multiplexer workspace utility |
| **`tokei`** | `15.0.0` | 🟡 Belum Diuji | `glibc`, `gcc` | `gcc`, `rust`, `cargo` | A blazingly fast CLOC (Count Lines Of Code) program |
| **`tree`** | `2.3.2` | 🟡 Belum Diuji | `glibc` | - | Recursive directory indentation and tree-format listing program |
| **`vscode-oss`** | `1.135.06055` | 🟡 Belum Diuji | `glibc`, `ca-certificates`, `dbus`, `ncurses` | `tar`, `gzip` | Free/Libre Open Source Software Binaries of Visual Studio Code (VSCodium) |
| **`vulkan-headers`** | `1.4.363` | 🟡 Belum Diuji | - | `cmake`, `ninja` | Vulkan Header files and API registry |
| **`vulkan-loader`** | `1.4.363` | 🟡 Belum Diuji | `glibc`, `vulkan-headers`, `wayland` | `cmake`, `ninja`, `pkgconf`, `python` | Vulkan Installable Client Driver (ICD) Loader |
| **`wayland`** | `1.26.0` | 🟡 Belum Diuji | `glibc`, `libffi`, `expat`, `libxml2` | `gcc`, `meson`, `ninja`, `pkgconf`, `libxslt` | A computer display server protocol |
| **`wayland-protocols`** | `1.49` | 🟡 Belum Diuji | `glibc` | `gcc`, `wayland`, `meson`, `ninja` | Specifications of extended Wayland protocols |
| **`wireplumber`** | `0.5.17` | 🟡 Belum Diuji | `glibc`, `pipewire`, `glib2` | `meson`, `ninja`, `pkgconf`, `glib2` | Modular session manager daemon and policy router for PipeWire |
| **`zellij`** | `0.45.1` | 🟡 Belum Diuji | `glibc`, `curl`, `gcc`, `zlib` | `rust`, `mold`, `clang`, `llvm`, `pkgconf` | A terminal multiplexer |
| **`zlib`** | `1.3.2` | 🟡 Belum Diuji | `glibc` | - | Standard compression library implementing DEFLATE algorithm |
| **`zoxide`** | `0.10.0` | 🟡 Belum Diuji | `glibc` | `gcc`, `git`, `rust` | A smarter cd command for your terminal |

---
