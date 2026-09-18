# Forge — The High-Performance Source-First Package Manager

> **Forge** adalah *High-Performance Source-First & Hybrid Package Manager* yang ditulis murni menggunakan bahasa **Rust** khusus untuk distribusi **Kura Linux**. Ditenagai compiler **LLVM**, ultra-fast linker **`mold`**, Link-Time Optimization (**LTO Thin/Full**), dan dukungan **PGO**, Forge mengusung filosofi kompilasi **Gentoo Portage** (*Source-First Native Compilation*, *USE Flags*, *Slots*), paradigma meta-paket modular modern (*`base`*, *`base-devel`*), akselerasi **Ccache (v4.13.5)**, DAG Dependency Resolver, Transactional Merger, konfigurasi terpusat (`/etc/forge/forge.conf`), serta ekosistem terpisah **`forge-server` (Lock-CPU Build Farm)**.

---

## ⚡ Fitur Utama & Filosofi Desain

- **🦀 Pure Rust & Extreme Optimization:** Ditulis murni dalam Rust, dikompilasi dengan backend LLVM 22, ultra-fast linker `mold`, optimasi Thin LTO, dan flag `-C target-cpu=native`.
- **🚀 Source-First Native Compilation (Gentoo Mode):** Secara default mengompilasi paket langsung dari kode sumber upstream dengan flag native target mentok ekstrem (`-O3 -march=native -pipe -flto=thin -fno-math-errno -falign-functions=32`) di RAM `tmpfs`.
- **⚡ Ccache 4.13.5 Acceleration:** Integrasi otomatis compiler cache untuk memangkas waktu kompilasi ulang hingga 80-90%.
- **🌾 Pure Source-Built Seed Toolchain (ADR-019, ADR-028):** Pengemasan `forge toolchain bundle` (`dist/kura-toolchain.tar.xz`) murni 100% dari hasil kompilasi source code di staging tanpa menyalin biner host, menyertakan seluruh `/var/db/forge/recipes/` sehingga lingkungan chroot mandiri seketika.
- **📦 Meta-Paket Murni ("Everything is a Package", ADR-026):** Basis OS dikelola murni melalui resep meta-paket deklaratif (`forge install base` dan `forge install base-devel`) tanpa hardcode logika OS di dalam biner package manager.
- **🗃️ Sistem Resep Terdedikasi & `forge sync` (ADR-028):** Repositori resep resmi berlokasi di `/var/db/forge/recipes/`, disinkronkan secara atomik dari `forge-server` melalui perintah `forge sync`.
- **🌳 DAG Dependency Graph & Cycle Detection:** Resolver dependensi asiklis terarah dengan pemisahan dependensi runtime (`depends`) dan build-time (`makedepends`), evaluasi USE flags, dan pengurutan topologis.
- **🔒 Transactional Merger & Collision Detector:** Pre-flight scanning untuk mencegah tabrakan berkas dan penggabungan atomik dari staging `$DESTDIR` ke target `$FORGE_ROOT`.
- **📁 Flat-File Manifest Database:** Pencatatan deterministik berkas, checksum SHA256, dan metadata build di `/var/db/forge/installed/` tanpa ketergantungan DB eksternal yang rapuh.
- **🎛️ Granular USE Flags:** Mengaktifkan/menonaktifkan fitur perangkat lunak secara presisi di level global (`forge.conf`) atau per-paket (`package.use`).
- **🏷️ Multi-Version Slots:** Menjalankan beberapa versi mayor paket secara berdampingan tanpa konflik (misal: LLVM 22 vs 21, Python 3.12 & 3.13, GCC multi-versi).
- **⚙️ Konfigurasi Terpusat (*Single Source of Truth*):** Pengaturan build terpusat di `/etc/forge/forge.conf` (`cflags`, `march`, `use_flags`, `ccache`, `makeflags`).
- **⚡ Opsi Akselerasi 3-Tier Package Cascade Resolution (Opt-In):**
  - **3-Tier Cascade (`--binhost`):** Menjalankan resolusi 3 tingkat: (1) Forge Native Binhost (`.forge.tar.zst`), (2) CachyOS Prebuilt (Zen4/v4/v3) dengan Anti-Brick Core OS protection, (3) Source Code fallback.
  - **Native Compilation (`--native` / Default):** Kompilasi 100% dari kode sumber secara native (Portage mode) dengan optimasi silikon host.
- **🔬 Introspeksi Hardware (`forge cpu-dump`):** Menganalisis CPU host, ekstensi ISA (AVX-512, AVX2, SSE4, dll.), cache, dan mengekspor profil hardware `cpu-profile.json`.
- **🏭 Suite Terpisah `forge-server`:** Daemon server resep terpusat (`serve`) dan worker CI/CD builder (`forge-server import`) yang mengompilasi paket secara massal untuk target CPU pengguna.
- **⚙️ Integrasi OpenRC Native:** Otomatis mendeteksi skrip di `/etc/init.d/` dan terintegrasi dengan `rc-update`.
- **📦 Stage Exporter (`forge stage-export`):** Utilitas pengemas rootfs menjadi tarball distribusi Kura Linux (`kura-stage.tar.xz`).

---

## 🛠️ Panduan Perintah CLI

### A. Klien Pengguna (`forge`)

```bash
# --- 1. Inisialisasi Konfigurasi Package Manager ---
forge setup                     # Inisialisasi konfigurasi package manager (/etc/forge/forge.conf)

# --- 2. Manajemen Paket (Default: Source Compilation First) ---
forge install base              # Pasang sistem dasar Kura Linux (Meta-Paket)
forge install base-devel        # Pasang toolchain kompilasi Kura Linux (Meta-Paket)
forge install <pkg>             # Kompilasi dari source code secara native (Default Gentoo-style)
forge install --native <pkg>    # Paksa kompilasi 100% dari kode sumber (Portage mode)
forge install --binhost <pkg>   # 3-Tier Cascade: Forge Binhost -> Fallback CachyOS Zen4/v4/v3 -> Source
forge install --interactive <pkg> # Pilih manual provider (Source vs Binhost vs CachyOS)
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync                      # Sinkronisasi pohon resep dari Forge Server
forge update @world             # Re-kompilasi / perbarui seluruh paket terpasang

# --- 3. Introspeksi Hardware ---
forge cpu-dump                  # Dump mikroarsitektur CPU & simpan cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini
forge cpu-dump --upload         # Unggah profil CPU ke Forge Server untuk CI/CD build farm

# --- 4. Manajemen & Bundler Seed Toolchain ---
forge toolchain status          # Cek status Clang/LLVM 22, mold, GCC, Make, Ninja
forge toolchain bundle          # Kemas seed toolchain murni ke dist/kura-toolchain.tar.xz (ADR-019)

# --- 5. Informasi & Query ---
forge list                      # Tampilkan daftar seluruh paket terpasang & versinya
forge query <pkg>               # Tampilkan metadata, USE flags aktif, dependensi, & manifest
forge search <query>            # Cari resep paket berdasarkan nama/deskripsi

# --- 6. Fitur Distro Khusus ---
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

### B. Infrastruktur Server & CI/CD (`forge-server`)

```bash
# --- Manajemen Server & CI/CD Builder ---
forge-server serve              # Jalankan service API resep & katalog biner
forge-server import <pkg>       # CI/CD: Build lock-CPU, kemas .forge.tar.zst, & upload ke binary library
forge-server index              # Regenerasi database index repositori packages.db.zst
```

---

## 🔨 Membangun Forge dari Kode Sumber (Rust)

```bash
# Kompilasi rilis dengan optimasi native silikon & linker mold
cargo build --release

# Menjalankan test suite
cargo test
```

Biner hasil kompilasi:
- `target/release/forge` (CLI Klien Pengguna)
- `target/release/forge-server` (Server & CI/CD Suite)

---

## 📜 Standar Format Resep All-in-One (`recipe.toml`)

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

---

## 🧭 Dokumentasi Pengembangan & Panduan AI

- **Pedoman AI & Protokol Mutlak HITL:** Lihat [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- **Blueprint Arsitektur & Desain Sistem:** Lihat [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- **Memori Persisten, Roadmap & ADR:** Lihat [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
