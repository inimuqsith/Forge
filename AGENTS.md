# AGENTS.md — Forge Package Manager Development Guidelines

> **Forge**: *Source-based package manager* berkecepatan tinggi yang dirancang khusus untuk distribusi **Kura Linux**. Berfokus pada kompilasi 100% native silikon (`-march=native`), pelacakan manifest file deterministik, integrasi layanan **OpenRC**, isolasi build berbasis RAM (`tmpfs`), meta-target `@system`, dan kemampuan pembuatan stage distribusi (`forge stage-export`).

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
1. **Pembangunan Engine `forge`:** Mengembangkan CLI parser, dependency graph & DAG resolver, source fetcher & checksum verifier, sandbox compilation runner, DESTDIR staging engine, manifest-based transactional merger, dan unmerge cleaner.
2. **Manajemen Database & Konfigurasi:** Mengelola format flat-file database `/var/db/forge/` (`installed/`, `manifest`, `metadata.json`, `world`) dan parser konfigurasi `/etc/forge/forge.conf`.
3. **Integrasi OpenRC & Hook System:** Menyediakan mekanisme deteksi skrip `/etc/init.d/`, pendaftaran runlevel otomatis, `ldconfig`, dan hook pasca-instalasi.
4. **Fitur Khas Kura Linux:**
   - Meta-target `@system` untuk kompilasi ulang seluruh base system secara native.
   - Tool `forge stage-export` untuk mengemas rootfs menjadi tarball stage distribusi (`kura-stage.tar.xz`).
5. **Pohon Resep Paket (`recipes/`):** Menyusun resep build paket resmi (`core/`, `system/`, `extra/`) sesuai standar POSIX/bash lifecycle resep Forge.
6. **Harness Pengujian Komprehensif:** Menyusun unit tests dan integration tests dengan mock rootfs/chroot sandbox agar aman diuji tanpa menyentuh host system.

### ⛔ DILARANG KERAS:
- **Dilarang Mengedit Berkas KuraLinux:** Dilarang keras memodifikasi berkas apa pun di luar repositori ini (misal di `/home/admin/Development/KuraLinux/`).
- **Haram Ketergantungan Systemd:** Seluruh integrasi service wajib menggunakan standar OpenRC (`/etc/init.d/`, `/etc/conf.d/`, `rc-update`).
- **Dilarang Menulis ke Filesystem Host `/` Tanpa Isolasi:** Seluruh proses build dan testing wajib terisolasi dalam target root dummy (`$FORGE_ROOT` atau `/tmp/forge/stage/`). Jangan pernah memasang file uji coba ke sistem host nyata.
- **Dilarang Bypass Verifikasi Integritas:** Pengunduhan source wajib divalidasi dengan hash SHA256. Resep tanpa checksum yang valid dilarang diproses untuk instalasi.
- **Pengecualian Khusus Glibc:** Flag optimasi custom (`-march=native`) wajib diinjeksikan untuk semua paket Kura Linux, **kecuali Glibc** yang harus tetap di-build via `forge` dengan konfigurasi standar CFLAGS bawaan Glibc demi stabilitas build system Glibc.

---

## 3. Struktur Repositori

```
.
├── AGENTS.md               # Pedoman AI, Aturan Mutlak HITL, & Siklus Verifikasi
├── ARCHITECTURE.md         # Blueprint & Desain Arsitektur Engine Forge
├── MEMORY.md               # State Engine, Roadmap, ADR, & Log Solusi
├── README.md               # Dokumentasi Umum & Panduan Penggunaan Forge
├── .gitignore              # Konfigurasi filter berkas Git
├── src/                    # Source code engine, core modules, & CLI binary
├── config/                 # Template konfigurasi bawaan (forge.conf.example)
├── recipes/                # Pohon resep paket resmi Kura Linux
│   ├── system/             # Resep set @system (Glibc, GCC, Kernel, OpenRC, dll.)
│   ├── core/               # Resep utilitas inti sistem
│   └── extra/              # Resep aplikasi & layanan tambahan
├── tests/                  # Test suite (unit tests, integration sandbox tests)
├── docs/                   # Spesifikasi teknis resep, hook API, & manual
└── scripts/                # Helper scripts pengembangan & CI/CD
```

---

## 4. Alur Perintah Utama

```bash
# --- 1. Manajemen Paket ---
forge build <pkg>           # Kompilasi paket ke staging tanpa instalasi ke root
forge install <pkg>         # Fetch -> Verify -> Build -> Stage -> Merge ke rootfs
forge remove <pkg>          # Hapus paket secara bersih berdasarkan manifest
forge update                # Sinkronisasi & perbarui pohon resep lokal

# --- 2. Query & Inspeksi ---
forge list                  # Tampilkan daftar paket terpasang & versinya
forge query <pkg>           # Tampilkan metadata, dependensi, & daftar file paket
forge search <query>        # Cari paket dalam pohon resep lokal

# --- 3. Fitur Distro Khusus ---
forge install @system       # Kompilasi seluruh base system Kura Linux 100% native
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

---

## 5. Alur & Konvensi Git

### Format Pesan Commit:
Format commit wajib menggunakan standar Conventional Commits: `<type>: <deskripsi>`
- `feat:` Fitur baru engine Forge, modul CLI, atau resep paket baru.
- `fix:` Perbaikan bug pada resolver dependensi, parser, atau siklus build.
- `docs:` Pembaruan dokumentasi, blueprint arsitektur, atau catatan memori.
- `test:` Penambahan atau perbaikan unit test / integration test.
- `chore:` Konfigurasi repository, build scripts, atau refactoring internal.
- `refactor:` Restrukturisasi kode tanpa mengubah fungsionalitas publik.

### Aturan Git:
1. **Verifikasi Sebelum Commit:** Seluruh perubahan kode wajib lulus pengujian (Langkah 5 HITL) sebelum dibuatkan commit.
2. **Jangan Commit Artefak Build & Tarball:** Binari hasil kompilasi, file `.tar.*`, cache `distfiles/`, dan staging rootfs wajib diabaikan via `.gitignore`.
3. **Commit Terfokus & Atomik:** Setiap commit harus mencakup satu tujuan perubahan yang jelas dan terdokumentasi dengan baik.
