# AGENTS.md — Forge Package Manager Development Guidelines

> **Forge**: *Source-First & Hybrid Package Manager* berkecepatan tinggi yang dirancang khusus untuk distribusi **Kura Linux**. Mengadopsi filosofi inti **Gentoo Portage** (*Source-First Native Compilation*, *USE Flags*, *Slots*, *Package Sets*), didukung ekosistem terpisah **`forge-server`** (Registry Resep & **CI/CD Builder Lock-CPU**), kemampuan akselerasi **Forge Binhost**, integrasi opsional repositori biner **CachyOS/Arch**, pelacakan manifest deterministik, integrasi layanan **OpenRC**, isolasi build RAM (`tmpfs`), serta pembuatan stage distribusi (`forge stage-export`).

---

## 1. 🛑 ATURAN MUTLAK: HUMAN-IN-THE-LOOP (HITL) & SIKLUS VERIFIKASI

1. **HARAM MUTLAK LANGSUNG EKSEKUSI:** Dilarang keras membuat, mengedit, menghapus berkas, atau menjalankan perintah yang mengubah struktur kode tanpa persetujuan eksplisit User di chat.
2. **ALUR KERJA WAJIB (5 LANGKAH):**
   - **Langkah 1 (Plan):** Buat draf rencana perubahan di berkas Plan / Artifact.
   - **Langkah 2 (Chat):** Jelaskan ringkasan perubahan di chat (alasan, file terdampak, tujuan, dan skenario pengujian).
   - **Langkah 3 (Tunggu ACC):** Tunggu User memberikan konfirmasi persetujuan di chat.
   - **Langkah 4 (Eksekusi):** Jalankan perubahan hanya setelah disetujui.
   - **Langkah 5 (Uji & Verifikasi):** **Wajib diuji dan dites** secara menyeluruh (unit test, integrasi build sandbox, verifikasi hash/manifest). Tugas **baru dianggap berhasil** jika seluruh pengujian lulus. Jika ditemukan error: analisis akar masalah, perbaiki, dan uji ulang sampai berfungsi sempurna.

---

## 2. Batasan & Ruang Lingkup AI

### 🎯 TUGAS & FOKUS AI DI REPOSITORI INI:

#### A. Engine Klien Pengguna (`forge` CLI):
1. **Source-First Compilation Engine (Gentoo Portage Mode):**
   - Mengutamakan kompilasi langsung dari kode sumber upstream dengan injeksi CFLAGS native CPU pengguna (`-march=native`) di RAM `tmpfs`.
   - Engine evaluasi *USE Flags* (`forge_use`) dan multi-version *Slots* (`pkg:slot`).
   - Dependency graph & DAG resolver dengan pemisahan `depends` dan `makedepends`.
   - DESTDIR staging, pre-flight collision detector, transactional merger, dan unmerge cleaner berbasis manifest.
2. **Introspeksi Hardware:**
   - Perintah `forge cpu-dump`: Ekstraksi mikroarsitektur, feature ISA flags (AVX-512, AVX2, dll.), cache, dan rekomendasi compiler flags ke `cpu-profile.json`.
3. **Opsi Akselerasi & Kebebasan Pengguna:**
   - Menyediakan opsi akselerasi Binhost (`forge install --binhost <pkg>`) dan fallback hybrid CachyOS/Arch (`--hybrid`) dengan kebebasan penuh di tangan pengguna.
4. **Fitur Khusus Distro Kura Linux:**
   - Meta-target `@system` untuk rebuild seluruh basis sistem Kura Linux.
   - Perintah `forge stage-export` untuk membuat arsip stage distribusi (`kura-stage.tar.xz`).
   - Integrasi OpenRC hook `/etc/init.d/` dan `rc-update`.

#### B. Infrastruktur Server & CI/CD (`forge-server`):
1. **Recipe Registry & Sync Server:**
   - Menyajikan katalog resep terpusat dan melayani sinkronisasi klien (`forge sync`).
2. **CI/CD Build Farm (Lock-CPU):**
   - Menerima `cpu-profile.json` pengguna dan mengunci (*lock*) worker builder ke CPU target pengguna.
   - Perintah `forge-server import <pkg>`: Tool otomasi server untuk kompilasi massal, packaging `.forge.tar.zst`, indexing `packages.db.zst`, dan publikasi ke Binary Library.

### ⛔ DILARANG KERAS:
- **Dilarang Mengedit Berkas KuraLinux:** Dilarang keras memodifikasi berkas apa pun di luar repositori ini (misal di `/home/admin/Development/KuraLinux/`).
- **Haram Ketergantungan Systemd:** Seluruh integrasi service wajib menggunakan standar OpenRC (`/etc/init.d/`, `/etc/conf.d/`, `rc-update`).
- **Dilarang Menulis ke Filesystem Host `/` Tanpa Isolasi:** Seluruh proses build dan testing wajib terisolasi dalam target root dummy (`$FORGE_ROOT` atau staging `/tmp/forge/stage/`).
- **Dilarang Bypass Verifikasi Integritas:** Pengunduhan source atau binary package wajib divalidasi dengan checksum SHA256/BLAKE3.
- **Pengecualian Khusus Glibc:** Flag optimasi custom (`-march=native`) diinjeksikan untuk semua paket Kura Linux, **kecuali Glibc** yang tetap di-build via `forge` dengan konfigurasi standar CFLAGS bawaan Glibc demi stabilitas build system.

---

## 3. Struktur Repositori

```
.
├── AGENTS.md               # Pedoman AI, Aturan Mutlak HITL, & Siklus Verifikasi
├── ARCHITECTURE.md         # Blueprint Arsitektur Source-First, Server, & CI/CD
├── MEMORY.md               # State Engine, Roadmap, ADR, & Log Solusi
├── README.md               # Dokumentasi Umum & Panduan Penggunaan Forge
├── .gitignore              # Konfigurasi filter berkas Git
├── src/                    # Source code engine, core modules, & CLI binaries
│   ├── cli/                # Command-line interface klien 'forge'
│   ├── core/               # Engine inti, DAG resolver, USE flag & Slot engine
│   ├── cpu/                # CPU microarchitecture analyzer (forge cpu-dump)
│   ├── binhost/            # Forge binary host client & package installer
│   ├── hybrid/             # Provider fallback CachyOS & Arch Linux binary adapter
│   ├── server/             # Daemon 'forge-server' (recipe & binary registry)
│   └── cicd/               # CI/CD builder worker & importer (forge-server import)
├── config/                 # Template konfigurasi bawaan (forge.conf.example)
├── recipes/                # Pohon resep paket resmi Kura Linux (di-sync dari server)
│   ├── system/             # Resep set @system (Glibc, GCC, Kernel, OpenRC, dll.)
│   ├── core/               # Resep utilitas inti sistem
│   └── extra/              # Resep aplikasi & layanan tambahan
├── tests/                  # Test suite (unit tests, integration sandbox tests)
├── docs/                   # Spesifikasi teknis resep, hook API, & manual
└── scripts/                # Helper scripts pengembangan, server setup, & CI/CD
```

---

## 4. Alur Perintah Utama

### A. Klien Pengguna (`forge`):
```bash
# --- 1. Manajemen Paket (Default: Source Compilation First) ---
forge install <pkg>             # Kompilasi dari source code secara native (Default Gentoo-style)
forge install --binhost <pkg>   # Opsi Akselerasi: Unduh pre-built binary native dari Forge Server
forge install --hybrid <pkg>    # Opsi Akselerasi: Gunakan biner CachyOS/Arch jika ada
forge install --interactive <pkg> # Pilih manual provider (Source vs Binhost vs CachyOS/Arch)
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync                      # Sinkronisasi resep dari Forge Server
forge update @world             # Re-kompilasi / perbarui seluruh paket terpasang

# --- 2. Introspeksi Hardware ---
forge cpu-dump                  # Dump mikroarsitektur CPU & export cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini

# --- 3. Fitur Distro Khusus ---
forge install @system           # Rebuild seluruh base system Kura Linux 100% native
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

### B. Infrastruktur Server & CI/CD (`forge-server`):
```bash
# --- Server Registry & Build Farm ---
forge-server serve              # Jalankan service API resep & katalog biner
forge-server import <pkg>       # CI/CD: Build lock-CPU, kemas .forge.tar.zst, & upload ke binary library
forge-server import --all-system # CI/CD: Kompilasi massal seluruh paket @system yang di-lock ke CPU target
forge-server index              # Regenerasi database index repositori packages.db.zst
```

---

## 5. Alur & Konvensi Git

### Format Pesan Commit:
Format commit wajib menggunakan standar Conventional Commits: `<type>: <deskripsi>`
- `feat:` Fitur baru engine Forge, modul CLI `forge`, service `forge-server`, atau resep paket.
- `fix:` Perbaikan bug pada resolver dependensi, parser, atau siklus build.
- `docs:` Pembaruan dokumentasi, blueprint arsitektur, atau catatan memori.
- `test:` Penambahan atau perbaikan unit test / integration test.
- `chore:` Konfigurasi repository, build scripts, atau refactoring internal.
- `refactor:` Restrukturisasi kode tanpa mengubah fungsionalitas publik.

### Aturan Git:
1. **Verifikasi Sebelum Commit:** Seluruh perubahan kode wajib lulus pengujian (Langkah 5 HITL) sebelum dibuatkan commit.
2. **Jangan Commit Artefak Build & Tarball:** Binari hasil kompilasi, file `.tar.*`, cache `distfiles/`, dan staging rootfs wajib diabaikan via `.gitignore`.
3. **Commit Terfokus & Atomik:** Setiap commit harus mencakup satu tujuan perubahan yang jelas dan terdokumentasi dengan baik.
