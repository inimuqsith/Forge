# ARCHITECTURE.md — Blueprint Arsitektur Package Manager `forge` & `forge-server`

> **`forge`** adalah *High-Performance Source-First Hybrid Package Manager* yang dibangun menggunakan bahasa pemrograman **Rust** untuk distribusi **Kura Linux**. Mengadopsi fondasi performa tinggi dari compiler **LLVM**, ultra-fast linker **`mold`**, Link-Time Optimization (**LTO Thin/Full**), dan dukungan **PGO (Profile-Guided Optimization)**, Forge menggabungkan filosofi kompilasi **Gentoo Portage** (*Source-First*, *USE Flags*, *Slots*, *Package Sets*), ekosistem **`forge-server` (Lock-CPU Build Farm)**, wizard bootstrap (`forge setup` & `forge system-setup`), serta akselerasi Binhost / CachyOS.

---

## 1. Topologi Cargo Workspace & Struktur Crate Rust

Forge dirancang menggunakan arsitektur modular multi-crate dalam satu Cargo Workspace terpadu:

```
                                      [ PENGGUNA KURA LINUX ]
                                                 │
                                                 ▼
+-------------------------------------------------------------------------------------------------+
|                                 Binary Klien: `forge` (crates/forge-cli)                        |
+-------------------------------------------------------------------------------------------------+
|  - CLI Dispatcher (clap v4)                                                                     |
|  - Wizard `forge setup` (Konfigurasi /etc/forge/forge.conf)                                     |
|  - Wizard `forge system-setup` (Bootstrap Kura Linux & target arsitektur)                       |
|  - `forge install @system` (Rebuild OS sesuai system-setup atau default template)               |
|  - `forge stage-export` (Generator kura-stage.tar.xz)                                           |
+-------------------------------------------------------------------------------------------------+
          │                                      │                                      │
          ▼                                      ▼                                      ▼
+-------------------------+            +-------------------------+            +-------------------------+
|    crates/forge-core    |            |   crates/forge-binhost  |            |   crates/forge-hybrid   |
| - DAG Resolver (Petgraph|            | - Forge Binhost Client  |            | - CachyOS/Arch Adapter  |
| - USE Flags Engine      |            | - Zstd Streaming Decomp |            | - UsrMerge & OpenRC     |
| - Multi-Version Slots   |            | - Hash Verify (BLAKE3)  |            |   Layout Normalizer     |
| - Staging & Merge Engine|            +-------------------------+            +-------------------------+
| - Manifest Flat-File DB |                         ▲                                      ▲
| - OpenRC Hook Triggers  |                         │                                      │
+-------------------------+                         │                                      │
          ▲                                         │                                      │
          │                                         │                                      │
+-------------------------+                         │                                      │
|    crates/forge-cpu     |                         │                                      │
| - CPUID & ISA Inspector |                         │                                      │
| - `forge cpu-dump`      |                         │                                      │
| - CFLAGS Recommender    |                         │                                      │
+-------------------------+                         │                                      │
          │                                         │                                      │
          └──────────────────┐ ┌────────────────────┘                                      │
                             │ │                                                           │
+-------------------------------------------------------------------------------------------------+
|                            Binary Server: `forge-server` (crates/forge-server)                  |
+-------------------------------------------------------------------------------------------------+
|  1. `forge-server serve` : Recipe Registry HTTP/API & Binary Catalog Server                     |
|  2. `forge-server import`: CI/CD Worker Builder yang di-lock ke profil CPU target pengguna      |
|  3. `forge-server index` : Generator database index katalog `packages.db.zst`                   |
+-------------------------------------------------------------------------------------------------+
```

---

## 2. Rincian Crate dalam Workspace (Clean 2-Crate Layout)

### 1️⃣ `crates/forge` (Biner Klien & Library Engine `forge`)
Menggabungkan seluruh fungsionalitas klien ke dalam satu crate yang terorganisir rapi:
- **`src/main.rs` (CLI Binary):** Antarmuka pengguna (`forge setup`, `forge system-setup`, `forge install`, `forge build`, `forge cpu-dump`, `forge toolchain`, `forge stage-export`).
- **`src/builder.rs` (Compilation Engine):** Eksekusi resep `recipe.toml`, akselerasi `ccache`, isolasi `tmpfs`, injeksi flag native silikon (`-O3 -march=native -pipe -flto=thin`), dan staging `DESTDIR`.
- **`src/toolchain.rs` (Pure Source Toolchain Bundler):** Mengemas biner hasil kompilasi source di staging `/tmp/forge/stage/` menjadi `dist/kura-toolchain.tar.xz`.
- **`src/cpu.rs` (Hardware Introspection):** Deteksi ISA flags (AVX-512, AVX2), cache L1-L3, rekomendasi CFLAGS, dan ekspor `cpu-profile.json`.
- **`src/binhost.rs` & `src/hybrid.rs`:** Modul akselerasi binhost & fallback CachyOS/Arch.
- **`src/lib.rs`:** DAG dependency graph, USE flag engine, slots, dan konfigurasi.

---

### 2️⃣ `crates/forge-server` (Biner Server & CI/CD Suite `forge-server`)
- Mengompilasi binary server `/usr/bin/forge-server`.
- Menyajikan service HTTP REST / sync endpoint untuk distribusi resep (`serve`).
- Menjalankan CI/CD worker builder yang mengunci (*lock*) kompilasi ke target arsitektur CPU pengguna (`import`).
- Mengindeks repositori biner server `packages.db.zst` (`index`).

---

## 3. Spesifikasi Wizard Baru: `forge setup` & `forge system-setup`

### 🛠️ A. Wizard Package Manager: `forge setup`
Bertujuan untuk menginisialisasi atau mengubah konfigurasi Forge klien:
- **Alur Interaktif:**
  1. Menanyakan preferensi mode resolusi default (`source` / `binhost` / `hybrid` / `interactive`).
  2. Menentukan URL Server Resep & URL Binhost.
  3. Mengatur alokasi jumlah thread kompilasi paralel (`makeflags = -j<N>`).
  4. Menetapkan USE flags global (`ssl openrc alsa -systemd lto pgo`).
  5. Menulis konfigurasi ke `/etc/forge/forge.conf` dan membuat struktur direktori `/var/db/forge/`, `/var/cache/forge/distfiles/`, `/tmp/forge/`.
- **Mode Non-Interaktif:** `forge setup --defaults` langsung menerapkan template default distro.

---

### 🏗️ B. Wizard Bootstrap Distro: `forge system-setup`
Wizard khusus persiapan bootstrap sistem operasi Kura Linux (biasanya dijalankan pertama kali saat instalasi distro baru atau di lingkungan chroot):
- **Alur Langkah demi Langkah:**
  1. **Deteksi / Pemilihan Arsitektur CPU:**
     - Menjalankan introspeksi CPU (`forge cpu-dump`).
     - Menawarkan pilihan: *[1] Auto-Detect Native (Rekomendasi: AMD Zen 4 znver4)*, *[2] Generic x86-64-v3*, *[3] Generic x86-64-v4*, *[4] Input Manual*.
  2. **Pemilihan Profil Base Distro Kura Linux:**
     - *Standard Base* (Default: Toolchain + Monolithic Kernel + OpenRC + Core utils).
     - *Minimal Headless* (Embedded / Server footprint minimal).
     - *Desktop / Wayland Ready* (Disertai stack elogind & D-Bus untuk KDE/Hyprland).
  3. **Konfigurasi Kernel Linux Monolithic:**
     - Memilih driver storage built-in (`=y`): Ext4, NVMe, SATA AHCI, VirtIO.
  4. **Konfigurasi Init System & Regional:**
     - Timezone, default locale (`en_US.UTF-8`), hostname, keymap keyboard (`us`).
  5. **Simpan Konfigurasi:**
     - Menulis konfigurasi ke `/etc/forge/system.conf`.
  6. **Prompt Instruksi Lanjutan:**
     - Wizard menampilkan notifikasi:
       ```
       ===============================================================
       ✓ Konfigurasi Bootstrap Kura Linux Berhasil Disimpan!
       Silakan jalankan perintah berikut untuk memulai kompilasi base OS:
         # forge install @system
       ===============================================================
       ```

---

### 📦 C. Mekanisme Fallback Template pada `forge install @system`
- Ketika pengguna mengeksekusi `forge install @system`:
  - **Skenario 1 (Telah menjalankan `forge system-setup`):** Forge membaca `/etc/forge/system.conf` dan mengompilasi set paket `@system` sesuai arsitektur silikon, profil base, dan opsi kernel yang dipilih oleh pengguna.
  - **Skenario 2 (Belum menjalankan `forge system-setup`):** Forge secara cerdas menggunakan **Template Default Standar Kura Linux** (`/etc/forge/system.conf.example`):
    - Arsitektur CPU: Auto-detect `-march=native`.
    - Profil Base: Standard Kura Linux Base.
    - Kernel: Monolithic Kernel standar dengan driver Ext4, NVMe, VirtIO built-in.
    - Menjamin sistem tetap dapat di-bootstrap secara instan tanpa kegagalan.

---

## 4. Konfigurasi Toolchain Rust & Optimasi Kompilasi

Konfigurasi Cargo workspace diatur untuk menghasilkan performa biner tingkat tertinggi:

```toml
# Cargo.toml (Release Profile)
[profile.release]
opt-level = 3
lto = "thin"           # Link-Time Optimization antar-crate
codegen-units = 1      # Maksimalisasi optimasi cross-module
panic = "abort"        # Eliminasi overhead unwinding
strip = "symbols"      # Ukuran biner sangat ringkas
debug = false
```

```toml
# .cargo/config.toml (Linker & Target Flags)
[target.x86_64-unknown-linux-gnu]
rustflags = [
    "-C", "target-cpu=native",
    "-C", "link-arg=-fuse-ld=mold",
]
```

---

## 5. Standar Format File Konfigurasi Bootstrap (`/etc/forge/system.conf`)

```ini
# /etc/forge/system.conf
# Konfigurasi Bootstrap Sistem Kura Linux (Dihasilkan oleh `forge system-setup`)

[system]
profile = "standard"        # "standard", "minimal", "desktop-ready"
hostname = "kuralinux"
locale = "en_US.UTF-8"
keymap = "us"
timezone = "UTC"

[target]
architecture = "x86_64"
march = "znver4"
enable_avx512 = true
enable_avx2 = true
cflags = "-O2 -march=znver4 -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"

[kernel]
type = "monolithic"
drivers_builtin = ["ext4", "nvme", "sata_ahci", "virtio", "virtio_pci", "virtio_blk", "virtio_net"]

[init]
manager = "openrc"
default_services = ["metalog", "chronyd", "eudev", "dhcpcd", "acpid"]
```

---

## 6. Arsitektur Seed Toolchain Pure Source-Built (`kura-toolchain.tar.xz`)

Untuk mencegah polusi dari compiler host dan menjamin Kura Linux dapat melakukan bootstrap secara mandiri (*self-contained*), Forge menyediakan sub-sistem **Seed Toolchain Bundler**:

```
+-------------------------------------------------------------------------------------------------+
|               PEMBUATAN SEED TOOLCHAIN PURE SOURCE-BUILT: `forge toolchain bundle`              |
+-------------------------------------------------------------------------------------------------+
|  1. ATURAN MUTLAK: HARAM MENGAMBIL BINER/LIBRARY DARI HOST (/usr/bin, /usr/lib).               |
|  2. Mengemas HANYA biner & library yang 100% dikompilasi dari source code oleh Forge ke staging |
|     `/tmp/forge/stage/<pkg>/` (LLVM 22, Mold 2.42, Ninja 1.13, Pkgconf 3.0.7, Make 4.4.1).      |
|  3. Menyusun layout UsrMerge standar:                                                           |
|     - `usr/bin/` (clang, cc, clang++, c++, mold, ld, lld, llvm-ar, ar, make, ninja, pkgconf)    |
|     - `usr/lib/` (library pendukung & header compiler Clang)                                    |
|     - `etc/forge/toolchain.conf` (CC=clang, LD=mold, CFLAGS="-O3 -march=native -flto=thin")     |
|  4. Mengompresi ke `dist/kura-toolchain.tar.xz` + generasi hash `kura-toolchain.tar.xz.sha256`. |
+-------------------------------------------------------------------------------------------------+
                                                 │
                                                 ▼
+-------------------------------------------------------------------------------------------------+
|                         INTEGRASI SYSROOT: `kura-stage.tar.xz`                                  |
+-------------------------------------------------------------------------------------------------+
|  1. Developer distro Kura Linux mengekstrak seed toolchain langsung ke rootfs staging:          |
|     # tar -xpJf kura-toolchain.tar.xz -C $KURA_ROOTFS/ --numeric-owner                          |
|  2. Saat pengguna masuk chroot, seluruh toolchain sudah tersedia di `/usr/bin/`.                 |
|  3. Eksekusi `forge install @system` menggunakan seed toolchain ini untuk mengompilasi ulang    |
|     seluruh sistem operasi secara 100% native untuk silikon pengguna tanpa polusi host.         |
+-------------------------------------------------------------------------------------------------+
```

---

## 7. Standar Format All-in-One `recipe.toml` & Hierarki Konfigurasi Compiler

Forge mengadopsi format resep **All-in-One `recipe.toml`** yang menyatukan metadata deklaratif dan script build dalam satu berkas tunggal:

```toml
[package]
name = "pkgconf"
version = "3.0.7"
release = 1
slot = "0"
description = "Package compiler and linker metadata toolkit (Latest 3.0.7)"
license = "ISC"
upstream = "http://pkgconf.org/"

[dependencies]
runtime = ["glibc"]
build = ["gcc", "make"]

[sources]
urls = ["https://distfiles.ariadne.space/pkgconf/pkgconf-3.0.7.tar.xz"]
sha256 = ["c926ff491cbd9a331a589160811bd97ab1749b4d5198a519338f2cdfabe6940a"]

[build]
type = "autotools"
script = """
cd "${srcdir}/pkgconf-${pkgver}"
./configure \
    --prefix=/usr \
    --sysconfdir=/etc \
    --localstatedir=/var \
    --disable-static
make ${MAKEFLAGS}
make DESTDIR="${DESTDIR}" install
ln -sf pkgconf "${DESTDIR}/usr/bin/pkg-config"
"""
```

### 👑 Hierarki Konfigurasi Compiler (*Forge Config Supremacy*):
Meskipun resep memuat script bash kustom, **Aturan Konfigurasi Forge Tetap Berada pada Hierarki Tertinggi**:
1. **Injeksi Lingkungan Otomatis:** Sebelum mengeksekusi script resep, Forge secara otomatis mengekspor:
   - `CC="clang"`
   - `CXX="clang++"`
   - `LD="mold"`
   - `LDFLAGS="-Wl,-O1 -Wl,--as-needed -fuse-ld=mold"`
   - `CFLAGS="-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"`
   - `CXXFLAGS="${CFLAGS}"`
   - `MAKEFLAGS="-j$(nproc)"`
2. **Pengecualian Khusus Glibc (ADR-002):**
   Jika paket adalah `glibc` atau memiliki tag `compiler_override = "gcc"`:
   - Forge secara otomatis mengalihkan toolchain ke compiler `gcc` standar (`CC=gcc`, `CXX=g++`, `CFLAGS=-O2 -pipe`) tanpa menginjeksikan flag `-march` kustom untuk menjamin kestabilan build system Glibc.
