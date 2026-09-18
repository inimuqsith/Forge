# AGENTS.md — Forge Package Manager Development Guidelines

> **Forge**: *High-Performance Source-First & Hybrid Package Manager* yang ditulis murni menggunakan **Rust** (ditenagai backend compiler **LLVM 22**, ultra-fast linker **mold**, optimasi **LTO (Thin/Full)**, dan dukungan **PGO**) khusus untuk distribusi **Kura Linux**. Mengadopsi filosofi inti **Gentoo Portage** (*Source-First Native Compilation*, *USE Flags*, *Slots*) dan arsitektur modular modern ala Arch/Alpine (*Meta-Packages: `base`, `base-devel`*), didukung ekosistem terpisah **`forge-server`** (Registry Resep, **Upstream Bumper & Audit**, dan **CI/CD Builder Lock-CPU**), repositori terpusat **GitOps SSOT**, kemampuan akselerasi **Forge Binhost**, integrasi repositori biner **CachyOS/Arch**, pelacakan manifest deterministik, integrasi **OpenRC**, akselerasi **Ccache 4.13.5**, isolasi build RAM (`tmpfs`), serta pembuatan stage distribusi (`forge stage-export`).

---

## 1. 🛑 ATURAN MUTLAK: HUMAN-IN-THE-LOOP (HITL) & SIKLUS VERIFIKASI

1. **HARAM MUTLAK LANGSUNG EKSEKUSI:** Dilarang keras membuat, mengedit, menghapus berkas, atau menjalankan perintah yang mengubah struktur kode tanpa persetujuan eksplisit User di chat.
2. **ALUR KERJA WAJIB (5 LANGKAH):**
   - **Langkah 1 (Plan):** Buat draf rencana perubahan di berkas Plan / Artifact.
   - **Langkah 2 (Chat):** Jelaskan ringkasan perubahan di chat (alasan, file terdampak, tujuan, dan skenario pengujian).
   - **Langkah 3 (Tunggu ACC):** Tunggu User memberikan konfirmasi persetujuan di chat.
   - **Langkah 4 (Eksekusi):** Jalankan perubahan hanya setelah disetujui.
   - **Langkah 5 (Uji & Verifikasi):** **Wajib diuji dan dites** secara menyeluruh (`cargo test`, `cargo check`, integrasi sandbox). Tugas **baru dianggap berhasil** jika seluruh pengujian lulus. Jika ditemukan error: analisis akar masalah, perbaiki, dan uji ulang sampai berfungsi sempurna.

---

## 2. Batasan & Ruang Lingkup AI

### 🎯 TUGAS & FOKUS AI DI REPOSITORI INI:

#### A. Engine Klien Pengguna (`forge` CLI - Rust Workspace):
1. **Source-First Compilation Engine (Gentoo Portage Mode):**
   - Mengutamakan kompilasi langsung dari kode sumber upstream dengan injeksi CFLAGS native CPU pengguna (`-march=native`) di RAM `tmpfs` dan akselerasi Ccache (v4.13.5).
   - Engine evaluasi *USE Flags* (`UseFlagsEngine`) dan multi-version *Slots* (`pkg:slot`).
   - Repositori resep resmi terdedikasi di `/var/db/forge/recipes/` dengan dukungan `forge sync` (ADR-028).
   - Dependency graph & DAG resolver dengan pemisahan `depends` dan `makedepends`.
   - DESTDIR staging, pre-flight collision detector, transactional merger, dan unmerge cleaner berbasis manifest.
2. **Konfigurasi Terpusat (*Single Source of Truth*):**
   - `forge setup`: Wizard konfigurasi package manager `/etc/forge/forge.conf`.
3. **Introspeksi Hardware:**
   - Perintah `forge cpu-dump`: Ekstraksi mikroarsitektur, feature ISA flags (AVX-512, AVX2, dll.), cache, dan rekomendasi compiler flags ke `cpu-profile.json`.
4. **Opsi Akselerasi & Kebebasan Pengguna:**
   - Menyediakan opsi akselerasi Binhost (`forge install --binhost <pkg>`) dan fallback 3-tier cascade dengan perlindungan Core OS Anti-Brick CachyOS.
5. **Filosofi Meta-Paket ("Everything is a Package"):**
   - Tidak ada target magis `@system` yang di-hardcode. Basis sistem dikelola murni melalui resep meta-paket standar: `forge install base` (sistem inti OS) dan `forge install base-devel` (toolchain kompilasi).
   - Perintah `forge stage-export` untuk membuat arsip stage distribusi (`kura-stage.tar.xz`).
   - Integrasi OpenRC hook `/etc/init.d/` dan `rc-update`.

#### B. Infrastruktur Server & CI/CD (`forge-server`):
1. **GitOps Recipe Registry & Webhook Sync Server:**
   - Menyajikan katalog resep terpusat dan melayani sinkronisasi klien (`forge sync`).
   - Endpoint `POST /v1/webhook/github` yang mendeteksi perubahan commit dari GitHub SSOT (`inimuqsith/Forge`), melakukan `git pull --rebase`, dan auto-rebundle `recipes.tar.zst` secara instan.
2. **Upstream Recipe Bumper & Audit Engine (Bebas Rate-Limit):**
   - Perintah `forge-server audit`: Memindai seluruh 107 resep paket secara paralel dalam hitungan detik.
   - Perintah `forge-server bump <pkg|--all>`: Mengunduh tarball baru, menghitung SHA256 baru secara atomik, dan melakukan auto-push langsung ke GitHub SSOT (`origin main`).
3. **CI/CD Build Farm (Lock-CPU):**
   - Perintah `forge-server import <cpu-profile.json>`: Menyimpan profil silikon CPU target pengguna ke `/var/db/forge/profiles/<march>.json` dan mengesetnya sebagai profil aktif (`active.json`).
   - Perintah `forge-server build <package>`: CI/CD Worker mengompilasi paket menggunakan profil aktif, membundel `.forge.tar.zst`, dan otomatis mempublikasikannya ke Binary Library (`/var/db/forge/binhost/<march>/`).
   - Perintah `forge-server index`: Regenerasi database index repositori biner `packages.db.zst`.

### ⛔ DILARANG KERAS:
- **Haram Ambil Biner Dari Host (ADR-019):** Dilarang keras menyalin biner, library, atau compiler dari sistem host (`/usr/bin/`, `/usr/lib/llvm/22/`) untuk dimasukkan ke dalam paket distribusi atau seed toolchain. Seluruh paket dan toolchain wajib murni 100% dikompilasi dari kode sumber upstream melalui resep `recipe.toml` ke staging (`/tmp/forge/stage/`).
- **Dilarang Mengedit Berkas KuraLinux:** Dilarang keras memodifikasi berkas apa pun di luar repositori ini (misal di `/home/admin/Development/KuraLinux/`).
- **Haram Ketergantungan Systemd:** Seluruh integrasi service wajib menggunakan standar OpenRC (`/etc/init.d/`, `/etc/conf.d/`, `rc-update`).
- **Dilarang Menulis ke Filesystem Host `/` Tanpa Isolasi:** Seluruh proses build dan testing wajib terisolasi dalam target root dummy (`$FORGE_ROOT` atau staging `/tmp/forge/stage/`).
- **Dilarang Bypass Verifikasi Integritas:** Pengunduhan source atau binary package wajib divalidasi dengan checksum SHA256/BLAKE3.
- **Pengecualian Khusus Glibc (ADR-002):** Flag optimasi custom (`-march=native`) diinjeksikan untuk semua paket Kura Linux, **kecuali Glibc** yang tetap di-build via `forge` dengan konfigurasi standar CFLAGS bawaan Glibc demi stabilitas build system.

---

## 3. Standar Toolchain Rust & Konfigurasi Build

Kompilasi engine `forge` dan `forge-server` dioptimalkan secara ekstrem untuk performa maksimal:
- **Compiler:** Rust 1.97+ dengan backend **LLVM 22**.
- **Linker:** **`mold`** (High-performance modern linker via `-fuse-ld=mold`).
- **Optimasi LTO:** Link-Time Optimization (`lto = "thin"` atau `"fat"`).
- **Target CPU:** Native silicon (`-C target-cpu=native` untuk lokal, `-C target-cpu=x86-64-v3` untuk server VPS).
- **Codegen Units:** `codegen-units = 1` pada release build untuk efisiensi inlining maksimal.
- **Panic Strategy:** `panic = "abort"` untuk footprint biner yang sangat kecil dan eksekusi cepat.

---

## 4. Struktur Repositori (Cargo Workspace)

```
.
├── Cargo.toml              # Cargo Workspace Configuration (LTO, mold, opt-level 3)
├── .cargo/
│   └── config.toml         # Rustflags: -C target-cpu=native & mold linker
├── AGENTS.md               # Pedoman AI, Aturan Mutlak HITL, & Siklus Verifikasi
├── ARCHITECTURE.md         # Blueprint Arsitektur Source-First, Server, & CI/CD
├── MEMORY.md               # State Engine, Roadmap 14 Fase, 39 ADR, & Log Solusi
├── README.md               # Dokumentasi Umum & Panduan Penggunaan Forge
├── .github/
│   └── workflows/
│       └── recipe-auto-updater.yml # Bot Cron 6-Jam GitHub Actions
├── .gitignore              # Konfigurasi filter berkas Git
├── crates/                 # Modul-modul crate Rust (Clean 2-Crate Layout)
│   ├── forge/              # Binary klien 'forge' & library engine (CLI, builder, CPU profiler, binhost, cachyos, cascade, toolchain, stage)
│   └── forge-server/       # Binary daemon 'forge-server' (serve, import, profiles, index, bumper, webhook, CI/CD build farm)
├── config/                 # Template konfigurasi bawaan
│   └── forge.conf.example  # Konfigurasi tunggal package manager
├── recipes/                # Pohon 107 resep paket resmi Kura Linux (di-sync dari server)
│   ├── system/             # 22 Resep sistem inti & toolchain (base, base-devel, glibc, gcc, llvm, openrc, dll.)
│   ├── core/               # 51 Resep utilitas & daemons inti sistem
│   └── extra/              # 34 Resep aplikasi dev, CLI modern & layanan tambahan
├── tests/                  # Test suite integrasi & sandbox testing
└── docs/                   # Spesifikasi teknis resep, manual GitOps & panduan API
    ├── GITOPS_AND_BUMPER_MANUAL.md # Panduan lengkap GitOps & Upstream Bumper
    └── RECIPE_SPECIFICATION.md     # Standar spesifikasi format recipe.toml
```

---

## 5. Alur Perintah Utama

### A. Klien Pengguna (`forge`):
```bash
# --- 1. Konfigurasi Package Manager ---
forge setup                     # Inisialisasi konfigurasi package manager (/etc/forge/forge.conf)

# --- 2. Manajemen Paket (Default: Source Compilation First) ---
forge install base              # Pasang sistem dasar Kura Linux (Meta-Paket)
forge install base-devel        # Pasang toolchain kompilasi Kura Linux (Meta-Paket)
forge install <pkg>             # Kompilasi paket dari source code secara native (Gentoo-style)
forge install --native <pkg>    # Paksa kompilasi 100% dari kode sumber (Portage mode)
forge install --binhost <pkg>   # 3-Tier Cascade: Forge Binhost -> CachyOS -> Native Source
forge build <pkg>               # Kompilasi dari source & kemas ke .forge.tar.zst tanpa pasang ke host
forge recipe-import <url|file>  # Transpilasi PKGBUILD Arch / APKBUILD Alpine ke recipe.toml
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync                      # Sinkronisasi resep dari Forge Server (https://pkgkura.amqs.net)
forge update @world             # Re-kompilasi / perbarui seluruh paket terpasang

# --- 3. Introspeksi Hardware ---
forge cpu-dump                  # Dump mikroarsitektur CPU & export cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini

# --- 4. Toolchain & Fitur Distro Khusus ---
forge toolchain bundle          # Kemas seed toolchain murni ke dist/kura-toolchain.tar.xz (ADR-019)
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

### B. Infrastruktur Server & CI/CD (`forge-server`):
```bash
# --- 1. Server Registry & Web Explorer ---
forge-server serve              # Jalankan service API resep, Web Explorer & Webhook receiver

# --- 2. Upstream Recipe Bumper & Audit Engine ---
forge-server audit              # Audit 107 resep paket vs rilis hulu terbaru
forge-server bump <pkg>         # Perbarui resep spesifik ke versi hulu & auto-push ke GitHub SSOT
forge-server bump --all         # Perbarui SEMUA resep yang outdated & auto-push ke GitHub SSOT

# --- 3. CI/CD Build Farm & Binary Ingestion ---
forge-server import <profile.json> [--as <name>] # Simpan profil CPU target & set sebagai aktif
forge-server list-profiles      # Tampilkan seluruh profil CPU yang tersimpan
forge-server build <package>    # CI/CD Worker: Build paket dengan profil aktif & publikasi binhost
forge-server index              # Regenerasi database index repositori biner packages.db.zst
```

---

## 6. Alur & Konvensi Git

### Format Pesan Commit:
Format commit wajib menggunakan standar Conventional Commits: `<type>: <deskripsi>`
- `feat:` Fitur baru engine Forge, modul CLI `forge`, service `forge-server`, atau resep paket.
- `fix:` Perbaikan bug pada resolver dependensi, parser, atau siklus build.
- `docs:` Pembaruan dokumentasi, blueprint arsitektur, atau catatan memori.
- `test:` Penambahan atau perbaikan unit test / integration test.
- `chore:` Konfigurasi repository, build scripts, atau refactoring internal.
- `refactor:` Restrukturisasi kode tanpa mengubah fungsionalitas publik.

### Aturan Git:
1. **Wajib Git Berkala:** Setiap tahapan kerja, pembaruan blueprint arsitektur, maupun penyesuaian yang telah disetujui User dan diverifikasi **wajib langsung dicatat ke Git**.
2. **Verifikasi Sebelum Commit:** Seluruh perubahan kode/arsitektur wajib lulus verifikasi (`cargo test` / `cargo check` jika menyangkut kode) sebelum dibuatkan commit.
3. **Jangan Commit Artefak Build & Tarball:** Target binary `target/`, file `.tar.*`, cache `distfiles/`, dan staging rootfs wajib diabaikan via `.gitignore`.
4. **Commit Terfokus & Atomik:** Setiap commit harus mencakup satu tujuan perubahan yang jelas dan terdokumentasi dengan baik sesuai standar Conventional Commits.

---

## 7. Aturan Mutlak Sinkronisasi Berkas Markdown Kontinu

1. **Pembaruan Berkas MD Terus-Menerus:** Seluruh dokumentasi arsitektur dan status ekosistem ([`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md), [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md), [`README.md`](file:///home/admin/Development/Forge/README.md), [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md)) **wajib terus diperbarui secara berkelanjutan dan persisten** setiap kali ada keputusan arsitektural, diagram baru, atau perkembangan status implementasi.
2. **Kesesuaian Dokumentasi dengan Realita Kode:** Dokumen Markdown adalah sumber kebenaran teknis (*Single Source of Truth*) bagi siapapun yang mengembangkan Forge. Jangan biarkan ada diskrepansi antara dokumentasi dan status riil di repositori.
