# Forge — The High-Performance Source-First Package Manager

> **Forge** adalah *High-Performance Source-First & Hybrid Package Manager* yang ditulis murni menggunakan bahasa **Rust** untuk distribusi **Kura Linux**. Ditenagai compiler **LLVM**, ultra-fast linker **`mold`**, optimasi **LTO (Thin/Full)**, dan dukungan **PGO**, Forge mengusung filosofi sejati **Gentoo Portage** (*Source-First Native Compilation*, *USE Flags*, *Slots*, *Package Sets*), wizard bootstrap (`forge setup` & `forge system-setup`), serta ekosistem terpisah **`forge-server` (Lock-CPU Build Farm)**.

---

## ⚡ Fitur Utama & Filosofi Desain

- **🦀 Pure Rust & Extreme Optimization:** Ditulis murni dalam Rust, dikompilasi dengan backend LLVM 22, ultra-fast linker `mold`, optimasi Thin LTO, dan flag `-C target-cpu=native`.
- **🚀 Source-First Native Compilation (Gentoo Mode):** Secara default mengompilasi paket langsung dari kode sumber upstream dengan flag native target (`-O3 -march=native -pipe -flto=thin`) di RAM `tmpfs`.
- **🌾 Pure Source-Built Seed Toolchain:** Pengemasan `forge toolchain bundle` (`dist/kura-toolchain.tar.xz`) murni 100% dari hasil kompilasi source code di staging tanpa menyalin biner dari host.
- **🎛️ Granular USE Flags:** Mengaktifkan/menonaktifkan fitur perangkat lunak secara presisi di level global (`forge.conf`) atau per-paket (`package.use`).
- **🏷️ Multi-Version Slots:** Menjalankan beberapa versi mayor paket secara berdampingan tanpa konflik (misal: LLVM 22 vs 21, Python 3.12 & 3.13, GCC multi-versi).
- **🏗️ Wizard Setup & Bootstrap Distro:**
  - **`forge setup`:** Wizard inisialisasi dan konfigurasi package manager (`/etc/forge/forge.conf`).
  - **`forge system-setup`:** Wizard bootstrap Kura Linux untuk pemilihan arsitektur silikon, profil base, dan opsi kernel monolithic sebelum eksekusi `forge install @system`.
- **📦 Meta-Target `@system` & Fallback Cerdas:** Kompilasi ulang seluruh base OS Kura Linux sesuai konfigurasi bootstrap atau menggunakan template default standar distro.
- **⚡ Opsi Akselerasi Hybrid & Binhost (Opt-In):**
  - **Forge Native Binhost (`--binhost`):** Unduh biner terkompilasi native yang di-lock ke profil CPU pengguna dari `forge-server`.
  - **Hybrid Fallback (`--hybrid`):** Fallback biner opsional ke repositori **CachyOS** (x86-64-v4/v3) atau **Arch Linux**.
- **🔬 Introspeksi Hardware (`forge cpu-dump`):** Menganalisis CPU host, ekstensi ISA (AVX-512, AVX2, SSE4, dll.), cache, dan mengekspor profil hardware `cpu-profile.json`.
- **🏭 Suite Terpisah `forge-server`:** Daemon server resep terpusat (`serve`) dan worker CI/CD builder (`forge-server import`) yang mengompilasi paket secara massal untuk target CPU pengguna.
- **⚙️ Integrasi OpenRC Native:** Otomatis mendeteksi skrip di `/etc/init.d/` dan terintegrasi dengan `rc-update`.
- **📦 Stage Exporter (`forge stage-export`):** Utilitas pengemas rootfs menjadi tarball distribusi Kura Linux (`kura-stage.tar.xz`).

---

## 🛠️ Panduan Perintah CLI

### A. Klien Pengguna (`forge`)

```bash
# --- 1. Wizard Setup & Inisialisasi Sistem ---
forge setup                     # Wizard konfigurasi package manager (/etc/forge/forge.conf)
forge system-setup              # Wizard bootstrap distro Kura Linux (/etc/forge/system.conf)

# --- 2. Manajemen Paket (Default: Source Compilation First) ---
forge install <pkg>             # Kompilasi dari source code secara native (Default Gentoo-style)
forge install --binhost <pkg>   # Opsi Akselerasi: Unduh pre-built binary native dari Forge Server
forge install --hybrid <pkg>    # Opsi Akselerasi: Gunakan biner CachyOS/Arch jika ada
forge install --interactive <pkg> # Pilih manual provider (Source vs Binhost vs CachyOS/Arch)
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync                      # Sinkronisasi pohon resep dari Forge Server
forge update @world             # Re-kompilasi / perbarui seluruh paket terpasang

# --- 3. Introspeksi Hardware ---
forge cpu-dump                  # Dump mikroarsitektur CPU & simpan cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini
forge cpu-dump --upload         # Unggah profil CPU ke Forge Server untuk CI/CD build farm

# --- 4. Manajemen & Bundler Seed Toolchain ---
forge toolchain status          # Cek status Clang/LLVM 22, mold, GCC, Make, Ninja
forge toolchain bundle          # Kemas seed toolchain ke dist/kura-toolchain.tar.xz

# --- 5. Informasi & Query ---
forge list                      # Tampilkan daftar seluruh paket terpasang & versinya
forge query <pkg>               # Tampilkan metadata, USE flags aktif, dependensi, & manifest
forge search <query>            # Cari resep paket berdasarkan nama/deskripsi

# --- 6. Fitur Distro Khusus ---
forge install @system           # Rebuild seluruh basis sistem Kura Linux (sesuai system-setup / template)
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

### B. Infrastruktur Server & CI/CD (`forge-server`)

```bash
# --- Manajemen Server & CI/CD Builder ---
forge-server serve              # Jalankan service API resep & katalog biner
forge-server import <pkg>       # CI/CD: Build lock-CPU, kemas .forge.tar.zst, & upload ke binary library
forge-server import --all-system # CI/CD: Kompilasi massal seluruh paket @system yang di-lock ke CPU target
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
