# AGENTS.md — Forge Package Manager Development Guidelines

> **Forge**: *Hybrid Unified & Source-based package manager* berkecepatan tinggi yang dirancang khusus untuk distribusi **Kura Linux**. Mengadopsi keunggulan **Gentoo Portage** (*USE Flags*, *Slots*, *Package Sets*, *Source-based Compilation*), didukung ekosistem **Forge Server & CI/CD Builder Lock-CPU**, kemampuan **Binhost Native**, integrasi opsional repositori biner **CachyOS/Arch**, pelacakan manifest deterministik, integrasi layanan **OpenRC**, isolasi build RAM (`tmpfs`), serta pembuatan stage distribusi (`forge stage-export`).

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
1. **Pembangunan Engine `forge` (Portage-Inspired & Hybrid Unified):**
   - Mengembangkan CLI parser, dependency graph & DAG resolver dengan dukungan *USE Flags* dan *Slots*.
   - Source fetcher & checksum verifier (SHA256/BLAKE3).
   - Sandbox compilation runner berbasis RAM (`tmpfs`) dengan injeksi flag native CPU.
   - DESTDIR staging engine, transactional manifest merger, dan unmerge cleaner.
2. **Sistem Deteksi CPU & CI/CD Builder (Lock CPU):**
   - Perintah `forge cpu-dump`: Ekstraksi mikroarsitektur, feature ISA flags (AVX-512, AVX2, dll.), cache, dan rekomendasi compiler flags ke `cpu-profile.json`.
   - Perintah `forge import` / `forge impor`: Tool server & CI/CD worker untuk kompilasi massal, packaging `.forge.tar.zst`, indexing repository, dan upload otomatis ke Forge Binary Library.
3. **Arsitektur Hybrid Unified & Kebebasan Pengguna:**
   - Menyediakan 3 tingkat resolusi paket di mana **pengguna memiliki kebebasan penuh memilih mode**:
     - *Tingkat 1:* Forge Native Binhost (unduh binary terkompilasi native dari server).
     - *Tingkat 2:* Hybrid Fallback (opsional fallback ke binary CachyOS x86-64-v3/v4 atau Arch Linux).
     - *Tingkat 3:* Local Source Compilation (kompilasi dari kode sumber upstream via resep).
   - Mendukung opsi konfigurasi dan flag CLI (`--binhost`, `--build-source`, `--allow-hybrid`, `--interactive-select`).
4. **Manajemen Resep Terpusat di Server (`recipes/`):**
   - Format resep mandiri (`Recipe.forge` / `recipe`) yang disimpan terpusat di server dan disinkronisasi ke klien via `forge sync`.
5. **Manajemen Database & Konfigurasi:**
   - Mengelola flat-file database `/var/db/forge/` (`installed/`, `manifest`, `metadata.json`, `world`, `use.mask`, `package.use`) dan parser konfigurasi `/etc/forge/forge.conf`.
6. **Integrasi OpenRC & Hook System:**
   - Deteksi otomatis `/etc/init.d/`, pendaftaran runlevel, `ldconfig`, dan pemicu pasca-instalasi.
7. **Fitur Khas Kura Linux:**
   - Meta-target `@system` untuk kompilasi ulang seluruh base system secara native.
   - Tool `forge stage-export` untuk mengemas rootfs menjadi tarball stage distribusi (`kura-stage.tar.xz`).
8. **Harness Pengujian Komprehensif:**
   - Unit tests & integration sandbox tests terisolasi tanpa menyentuh host system.

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
├── ARCHITECTURE.md         # Blueprint Arsitektur Hybrid Unified, Server, & CI/CD
├── MEMORY.md               # State Engine, Roadmap, ADR, & Log Solusi
├── README.md               # Dokumentasi Umum & Panduan Penggunaan Forge
├── .gitignore              # Konfigurasi filter berkas Git
├── src/                    # Source code engine, core modules, & CLI binary
│   ├── cli/                # Command-line interface & subcommands
│   ├── core/               # Engine inti, DAG resolver, USE flag & Slot engine
│   ├── cpu/                # CPU microarchitecture analyzer (forge cpu-dump)
│   ├── binhost/            # Forge binary host client & package installer
│   ├── hybrid/             # Provider fallback CachyOS & Arch Linux binary adapter
│   ├── server/             # Modul backend server (recipe & binary registry)
│   └── cicd/               # CI/CD builder worker & automated import (forge import)
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

```bash
# --- 1. Manajemen Paket Klien ---
forge install <pkg>             # Pasang paket (otomatis pilih Binhost / Fallback / Source sesuai config)
forge install --binhost <pkg>   # Paksa prioritaskan unduh pre-built binary native
forge install --build-source <pkg> # Paksa kompilasi lokal dari source code
forge install --interactive <pkg>  # Pilih manual provider (Forge Binhost vs CachyOS/Arch vs Source)
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync / forge update       # Sinkronisasi resep & index biner dari Forge Server

# --- 2. Analisis Hardware & Profil CPU ---
forge cpu-dump                  # Dump mikroarsitektur CPU & export cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini

# --- 3. Server & CI/CD Builder Tools ---
forge import <pkg>              # Server/CI/CD: Build, kemas ke .forge.tar.zst, & upload ke binary library
forge import --all-system       # Server/CI/CD: Kompilasi massal seluruh paket @system yang di-lock ke CPU target

# --- 4. Query & Inspeksi ---
forge list                      # Tampilkan daftar paket terpasang & versinya
forge query <pkg>               # Tampilkan metadata, USE flags aktif, dependensi, & manifest
forge search <query>            # Cari paket dalam katalog server/lokal

# --- 5. Fitur Distro Khusus ---
forge install @system           # Rebuild seluruh base system Kura Linux
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

---

## 5. Alur & Konvensi Git

### Format Pesan Commit:
Format commit wajib menggunakan standar Conventional Commits: `<type>: <deskripsi>`
- `feat:` Fitur baru engine Forge, modul CLI, server/CI/CD, atau resep paket baru.
- `fix:` Perbaikan bug pada resolver dependensi, parser, atau siklus build.
- `docs:` Pembaruan dokumentasi, blueprint arsitektur, atau catatan memori.
- `test:` Penambahan atau perbaikan unit test / integration test.
- `chore:` Konfigurasi repository, build scripts, atau refactoring internal.
- `refactor:` Restrukturisasi kode tanpa mengubah fungsionalitas publik.

### Aturan Git:
1. **Verifikasi Sebelum Commit:** Seluruh perubahan kode wajib lulus pengujian (Langkah 5 HITL) sebelum dibuatkan commit.
2. **Jangan Commit Artefak Build & Tarball:** Binari hasil kompilasi, file `.tar.*`, cache `distfiles/`, dan staging rootfs wajib diabaikan via `.gitignore`.
3. **Commit Terfokus & Atomik:** Setiap commit harus mencakup satu tujuan perubahan yang jelas dan terdokumentasi dengan baik.
