# ARCHITECTURE.md — Blueprint Arsitektur Package Manager `forge` & `forge-server`

> **`forge`** adalah *High-Performance Source-First Hybrid Package Manager* yang dibangun murni menggunakan bahasa **Rust** khusus untuk distribusi **Kura Linux**. Mengadopsi fondasi performa tinggi dari compiler **LLVM**, ultra-fast linker **`mold`**, Link-Time Optimization (**LTO Thin/Full**), dan dukungan **PGO (Profile-Guided Optimization)**, Forge menggabungkan filosofi kompilasi **Gentoo Portage** (*Source-First*, *USE Flags*, *Slots*), paradigma meta-paket modular modern (*`base`*, *`base-devel`*), akselerasi **Ccache (v4.13.5)**, DAG Dependency Resolver, Transactional Merger, Manifest Database, ekosistem **`forge-server` (Lock-CPU Build Farm)**, konfigurasi terpusat (`/etc/forge/forge.conf`), serta akselerasi Binhost / CachyOS.

---

## 1. Topologi Cargo Workspace & Struktur Crate Rust (Clean 2-Crate Layout)

Forge dirancang menggunakan arsitektur modular yang terkonsolidasi secara rapi dalam 2 crate utama di dalam Cargo Workspace:

```
                                      [ PENGGUNA KURA LINUX ]
                                                 │
                                                 ▼
+=================================================================================================+
|                                  CRATE 1: `crates/forge`                                        |
|                          (Binary CLI Klien & Core Engine Library)                               |
+=================================================================================================+
|  1. CLI Dispatcher (`src/main.rs`)                                                              |
|     - `forge setup`, `forge install`, `forge remove`, `forge cpu-dump`                          |
|     - `forge toolchain bundle`, `forge stage-export`, `forge list`, `forge query`              |
|                                                                                                 |
|  2. Core Engine Library (`src/lib.rs` & sub-modul internal):                                    |
|     ├── `src/builder.rs`   : Compilation Engine, tmpfs sandbox, Ccache 4.13.5, DESTDIR staging   |
|     ├── `src/resolver.rs`  : DAG Dependency Graph, Topological Sort, Circular Cycle Detector   |
|     ├── `src/merger.rs`    : Pre-flight Collision Check, Atomic Transactional Rootfs Merger    |
|     ├── `src/db.rs`        : Flat-File Manifest Database, Unmerge Cleaner, CONFIG_PROTECT       |
|     ├── `src/cpu.rs`       : Hardware Introspection (AVX-512/AVX2/Cache), `forge cpu-dump`      |
|     ├── `src/toolchain.rs` : Pure Source Seed Toolchain Bundler (ADR-019, Zero Host Harvesting) |
|     ├── `src/binhost.rs`   : Forge Binhost Client & Zstd/BLAKE3 streaming verification          |
|     └── `src/hybrid.rs`    : CachyOS (v4/v3) & Arch Linux Fallback Provider                     |
+=================================================================================================+
                                                 ▲
                                                 │ Sinkronisasi Resep & Unduhan Biner
                                                 ▼
+=================================================================================================+
|                                CRATE 2: `crates/forge-server`                                   |
|                            (Daemon Server & CI/CD Build Farm Suite)                             |
+=================================================================================================+
|  1. `forge-server serve`  : Recipe Registry HTTP API & Binary Catalog Server                    |
|  2. `forge-server import` : CI/CD Worker Builder yang di-lock ke profil CPU target pengguna     |
|  3. `forge-server index`  : Generator database index repositori biner `packages.db.zst`         |
+=================================================================================================+
```

---

## 2. Paradigma Meta-Paket ("Everything is a Package")

Untuk menjaga agar engine Forge tetap ramping, bersih, dan universal (seperti Arch `pacman` atau Alpine `apk`), Forge mengadopsi filosofi **"Everything is a Package"**:

1. **Tidak Ada Target Magis yang Di-Hardcode:**
   - Tidak ada target `@system` khusus di dalam biner Rust.
   - Tidak ada wizard `system-setup` yang mencampuri urutan instalasi.
2. **Sistem Operasi Didefinisikan Sebagai Resep Meta-Paket Deklaratif:**
   - **`base` (`recipes/system/base/recipe.toml`):** Mendefinisikan fondasi OS minimal (Glibc, Bash, Coreutils, Sed, Grep, OpenRC, Util-linux, Shadow, Eudev, Kmod).
   - **`base-devel` (`recipes/system/base-devel/recipe.toml`):** Mendefinisikan toolchain kompilasi sistem lengkap (LLVM/Clang, Mold, Make, Ninja, GCC, Binutils, Pkgconf, Linux-Headers).
3. **Konfigurasi Tunggal Terpusat (`/etc/forge/forge.conf`):**
   - Menjadi satu-satunya sumber kebenaran (*Single Source of Truth*) untuk variabel compiler (`cflags`, `makeflags`, `ldflags`), target microarchitecture silikon (`target_march`), dan `use_flags` global.

---

## 3. Siklus Hidup & Alur Kerja Ekosistem (The 3 Epochs)

```mermaid
flowchart TD
    subgraph EPOCH_1["Fase 1: Developer Host (Bootstrap Seed Toolchain)"]
        A["Resep Lokal\n(./recipes/system/)"] --> B["Forge Builder\n+ Ccache 4.13.5"]
        B --> C["Staging DESTDIR\n(/tmp/forge/stage/)"]
        C --> D["forge toolchain bundle\n-> dist/kura-toolchain.tar.xz"]
    end

    subgraph EPOCH_2["Fase 2: Chroot Environment (Rebuild Base OS Kura Linux)"]
        D --> E["Ekstrak Seed Toolchain\nke Sysroot /mnt/kura/"]
        E --> F["Masuk chroot /mnt/kura"]
        F --> G["forge install base\n(Memasang Basis OS Minimal)"]
        F --> H["forge install base-devel\n(Memasang Toolchain Lengkap)"]
        G & H --> I["forge stage-export\n-> kura-stage.tar.xz (OS Siap Pakai)"]
    end

    subgraph EPOCH_3["Fase 3: Lingkungan Produksi (Klien & Server Publik)"]
        J["forge-server\n(Central Recipe Git & Binhost)"]
        K["User Laptop / PC"]
        K -- "1. forge sync" --> J
        J -- "Resep Terkini" --> K
        K -- "2. forge install <pkg>" --> L["Local Native Compilation"]
        K -- "2b. forge install --binhost" --> M["Download Pre-built Binary\n(Lock-CPU)"]
    end

    EPOCH_1 --> EPOCH_2
    EPOCH_2 --> EPOCH_3
```

---

## 4. Blueprint Mendalam: DAG Dependency Resolver (`src/resolver.rs`)

### 🎯 Tujuan & Filosofi
Memetakan seluruh pohon ketergantungan paket dari resep `recipe.toml`, memvalidasi ketiadaan siklus (*cycle detection*), dan menghasilkan urutan eksekusi kompilasi topologis yang deterministik (*Topological Sort*).

```
                 [ Target: base / base-devel / mold / nginx ]
                                      │
                                      ▼
                        +---------------------------+
                        |  1. Recipe Loader & Scan  |
                        | (recipes/system,core,...) |
                        +---------------------------+
                                      │
                                      ▼
                        +---------------------------+
                        |  2. USE Flags Evaluator   |
                        |   (Filter conditional deps|
                        +---------------------------+
                                      │
                                      ▼
                        +---------------------------+
                        |  3. Construct Directed    |
                        |     Acyclic Graph (DAG)   |
                        +---------------------------+
                                      │
                                      ▼
                        +---------------------------+
                        | 4. Cycle Detection Check  |
                        | (Tarjan / Kahn Algorithm) |
                        +---------------------------+
                                      │
                                      ▼
                        +---------------------------+
                        | 5. Topological Execution  |
                        |   Order Resolution Queue  |
                        +---------------------------+
```

### 📐 Spesifikasi Data & Algoritma:
1. **Model Graf Ketergantungan:**
   - **Node:** Merepresentasikan paket unik dengan atribut `(Name, Version, Slot, ActiveUSE)`.
   - **Edge:** Merepresentasikan jenis ketergantungan:
     - `DependencyType::Build` (`makedepends`): Ketergantungan yang wajib terpasang di host/sysroot sebelum kompilasi dimulai (misal: `ninja`, `cmake`, `linux-headers`).
     - `DependencyType::Runtime` (`depends`): Ketergantungan yang wajib tersedia agar biner dapat dieksekusi (misal: `glibc`, `openssl`, `zlib`).
2. **USE Flag Dependency Conditionals:**
   - Engine mengevaluasi conditional dependency format: `ssl? ( >=dev-libs/openssl-3.0 )`. Jika flag `ssl` tidak aktif pada konfigurasi, dependensi tidak akan dimasukkan ke dalam graf.
3. **Penyelesaian Siklus & Urutan Topologis:**
   - Menggunakan algoritma **Kahn** atau **Tarjan Strongly Connected Components (SCC)**.
   - Jika terdeteksi siklus tertutup (circular dependency), resolver menghasilkan laporan diagnostik detail beserta path siklusnya dan membatalkan build dengan pesan error yang jelas.

---

## 5. Blueprint Mendalam: Transactional Merger & Collision Detector (`src/merger.rs`)

### 🎯 Tujuan & Filosofi
Memindahkan berkas hasil kompilasi dari direktori staging `$DESTDIR` (`/tmp/forge/stage/<pkg>`) ke rootfs target `$FORGE_ROOT` (default `/`) secara atomik, dengan jaminan integritas, tanpa risiko merusak file sistem host jika terjadi error.

```
  +--------------------------+
  |  Staging DESTDIR         |
  | (/tmp/forge/stage/<pkg>) |
  +--------------------------+
               │
               ▼
  +--------------------------+
  | Pre-flight Collision     |
  | Scanner vs Installed DB  |
  +--------------------------+
     │                    │
[Ada Konflik]         [Aman / Bersih]
     │                    │
     ▼                    ▼
[Abort / Error]  +--------------------------+
                 | Atomic Transaction Merge |
                 | (Copy, Symlink, Chmod)   |
                 +--------------------------+
                              │
                              ▼
                 +--------------------------+
                 | Write Package Manifest   |
                 | & Metadata to /var/db/   |
                 +--------------------------+
                              │
                              ▼
                 +--------------------------+
                 | Trigger Post-Hooks       |
                 | (OpenRC, ldconfig, etc.) |
                 +--------------------------+
```

---

## 6. Blueprint Mendalam: Flat-File Manifest Database & Unmerge Cleaner (`src/db.rs`)

```
/var/db/forge/
├── world                        # Daftar paket eksplisit yang diminta user
├── installed/
│   ├── sys-devel/
│   │   └── mold-2.42.1:0/       # <kategori>/<nama>-<versi>:<slot>
│   │       ├── manifest         # Daftar seluruh berkas, ukuran, hash SHA256
│   │       ├── metadata.json    # Info build, target march, compiler flags
│   │       ├── USE              # USE flags yang aktif saat kompilasi
│   │       ├── CFLAGS           # CFLAGS yang digunakan
│   │       └── CONTENTS         # Struktur tree file terpasang
│   └── sys-libs/
│       └── glibc-2.44:0/
│           ├── manifest
│           └── metadata.json
```

### 🧹 Spesifikasi Unmerge Cleaner (`forge remove <pkg>`):
1. **Pembacaan Manifest:** Mengambil daftar berkas dan symlink milik paket dari `/var/db/forge/installed/<pkg>/manifest`.
2. **Proteksi Konfigurasi (`CONFIG_PROTECT`):** Berkas di bawah `/etc/` yang mengalami modifikasi hash tidak akan dihapus sembarangan.
3. **Reverse Directory Pruning:** Menghapus berkas dari level terdalam ke luar dan hanya menghapus direktori jika sudah kosong.
4. **Post-Unmerge Hook Trigger:** Menjalankan `ldconfig` dan `rc-update` jika paket menyediakan service OpenRC.

---

## 7. Blueprint Akselerasi Ccache & Hierarki Supremasi Compiler

```
                      [ Resep: recipe.toml ]
                                │
                                ▼
               +----------------------------------+
               | Injeksi Ccache & Compiler Flags  |
               +----------------------------------+
                                │
          ┌─────────────────────┴─────────────────────┐
          ▼                                           ▼
[ Paket Glibc / Override GCC ]            [ Paket Sistem Standar ]
(ADR-002: Pengecualian Khusus)            (Supremasi Native LLVM/Mold)
- CC="ccache gcc"                         - CC="ccache clang"
- CXX="ccache g++"                        - CXX="ccache clang++"
- LD="ld"                                 - LD="mold"
- CFLAGS="-O2 -pipe ..."                  - CFLAGS="-O3 -march=native -flto=thin ..."
- LDFLAGS="-Wl,-O1 ..."                   - LDFLAGS="-Wl,-O1 ... -fuse-ld=mold"
```

---

## 8. Blueprint Pure Source Seed Toolchain (`dist/kura-toolchain.tar.xz`)

```
+-------------------------------------------------------------------------------------------------+
|               PEMBUATAN SEED TOOLCHAIN PURE SOURCE-BUILT: `forge toolchain bundle`              |
+-------------------------------------------------------------------------------------------------+
|  1. ATURAN MUTLAK: HARAM MENGAMBIL BINER/LIBRARY DARI HOST (/usr/bin, /usr/lib).               |
|  2. Mengemas HANYA biner & library yang 100% dikompilasi dari source code oleh Forge ke staging |
|     `/tmp/forge/stage/<pkg>/` (LLVM 22, Mold 2.42, Ninja 1.13, Pkgconf 3.0.7, Make 4.4.1).      |
|  3. Menyusun layout UsrMerge standar:                                                           |
|     - `usr/bin/` (clang, cc, clang++, c++, mold, ld, lld, llvm-ar, ar, make, ninja, pkgconf)    |
|     - `usr/lib/` (library pendukung & header compiler Clang/LLVM)                               |
|     - `etc/forge/toolchain.conf` (CC=clang, LD=mold, CFLAGS="-O3 -march=native -flto=thin")     |
|  4. Mengompresi ke `dist/kura-toolchain.tar.xz` + generasi hash `kura-toolchain.tar.xz.sha256`. |
+-------------------------------------------------------------------------------------------------+
```

---

## 9. Format All-in-One `recipe.toml`

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
