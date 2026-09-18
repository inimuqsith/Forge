# MEMORY.md — Memori & Catatan Teknis Package Manager `forge`

> Dokumen memori persisten AI untuk melacak progres pengembangan package manager **`forge`**, keputusan arsitektur (ADR), status roadmap, dan log pemecahan masalah teknis.

---

## 1. Status & Roadmap Pengembangan `forge`

### Fase 1: Inisialisasi Arsitektur, Dokumentasi & Standarisasi (Selesai)
- [x] Inisialisasi repositori Git dan konfigurasi `.gitignore`.
- [x] Blueprint arsitektur Hybrid Unified & Portage-inspired di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] Panduan AI, aturan mutlak HITL & siklus verifikasi di [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- [x] Memori persisten & ADR di [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
- [x] Dokumentasi publik & panduan CLI di [`README.md`](file:///home/admin/Development/Forge/README.md).
- [x] Template konfigurasi lengkap di `config/forge.conf.example`.
- [x] Struktur direktori awal (`src/`, `recipes/`, `tests/`, `config/`, `docs/`).

### Fase 2: CPU Hardware Profiler (`forge cpu-dump`) & Config Engine
- [ ] Implementasi modul analisis CPU (`src/cpu/`): ekstraksi microarchitecture target (`znver4`, `alderlake`), deteksi ISA extensions (AVX-512, AVX2, SSE4.2), dan generasi `cpu-profile.json`.
- [ ] Parser konfigurasi `/etc/forge/forge.conf` dengan dukungan blok `[server]`, `[binhost]`, `[hybrid]`, `[cpu]`, dan `[use]`.

### Fase 3: Core CLI & Recipe Parser (Portage-Inspired)
- [ ] Implementasi CLI dispatcher (`install`, `build`, `remove`, `list`, `query`, `search`, `sync`, `cpu-dump`, `import`, `stage-export`).
- [ ] Parser resep mandiri (`Recipe.forge`): metadata deklaratif, parsing `USE_FLAGS`, evaluasi `forge_use`, `SLOT`, `depends`, dan hooks POSIX.

### Fase 4: Source Fetcher & Kriptografi Integritas
- [ ] Fetcher berkas sumber (HTTP/HTTPS/Git) ke `/var/cache/forge/distfiles/`.
- [ ] Verifikasi kriptografis SHA256 / BLAKE3 untuk sumber dan paket biner.

### Fase 5: Sandbox Build Engine & DESTDIR Staging
- [ ] Isolasi build di RAM tmpfs (`/tmp/forge/build/`).
- [ ] Injeksi otomatis compiler flags Kura Linux (`-march=native` / target CPU profil).
- [ ] Pengecualian optimasi custom untuk Glibc (ADR-002).
- [ ] Staging hasil kompilasi ke `/tmp/forge/stage/` (`DESTDIR`).

### Fase 6: Transactional Merger, Collision Detector, & Flat-File DB
- [ ] Pre-flight collision scan terhadap `/var/db/forge/installed/`.
- [ ] Penyalinan berkas atomik dari `$DESTDIR` ke rootfs (`/`).
- [ ] Pencatatan manifest berkas, symlink, permission, slot, dan metadata JSON.
- [ ] Pengelolaan daftar paket aktif (`/var/db/forge/world`).

### Fase 7: Unmerge Cleaner & Proteksi Konfigurasi
- [ ] Penghapusan presisi berdasarkan manifest dan pembersihan direktori kosong.
- [ ] Proteksi file konfigurasi yang dimodifikasi pengguna di `/etc/`.
- [ ] Analisis dan pembersihan orphan packages.

### Fase 8: Dependency Graph, Slots, & Topological DAG Resolver
- [ ] Resolusi dependensi dengan pemisahan `depends` vs `makedepends`.
- [ ] Dukungan koeksistensi multi-versi melalui Slots (misal: `python:3.12` dan `python:3.13`).
- [ ] Deteksi circular dependency dan ekspansi meta-set `@system` & `@world`.

### Fase 9: Hybrid Unified Engine (Binhost & CachyOS/Arch Fallback)
- [ ] Modul klien Forge Binhost (`src/binhost/`): pencocokan target CPU hash & USE flags.
- [ ] Modul adapter repositori biner (`src/hybrid/`): integrasi fallback CachyOS (x86-64-v4/v3) & Arch Linux.
- [ ] Mekanisme seleksi provider interaktif bagi pengguna (`--interactive`).

### Fase 10: Server & CI/CD Builder (`forge import`)
- [ ] Modul backend server (`src/server/`): sync endpoint untuk resep dan katalog biner `packages.db.zst`.
- [ ] CI/CD worker tool (`src/cicd/`): eksekusi `forge import` untuk build lock-CPU, packaging `.forge.tar.zst`, dan otomatisasi upload ke binary library.

### Fase 11: OpenRC Hook, Stage Exporter, & Base Recipes `@system`
- [ ] OpenRC service auto-discovery di `/etc/init.d/` dan post-install triggers.
- [ ] Utilitas pembuatan stage distribusi: `forge stage-export` $\rightarrow$ `kura-stage.tar.xz`.
- [ ] Penyusunan pohon resep resmi Kura Linux `@system` dan validasi pengujian penuh.

---

## 2. Keputusan Arsitektur Resmi (ADR Index)

1. **ADR-001 (Kompilasi 100% Native Silikon):** Forge mengompilasi seluruh paket langsung dari kode sumber upstream dengan menyuntikkan flag optimasi silikon target (`-march=native -O2 -pipe`) untuk performa CPU maksimal.
2. **ADR-002 (Pengecualian Optimasi Custom pada Paket Glibc):** Paket Glibc tetap di-build oleh Forge namun tanpa flag CFLAGS optimasi custom untuk menjaga kestabilan build system Glibc.
3. **ADR-003 (Format Resep Hibrida: Deklaratif + POSIX Shell):** Metadata paket ditulis deklaratif sedangkan siklus build (`prepare`, `build`, `package`) memanfaatkan fungsi POSIX shell standar.
4. **ADR-004 (Database Flat-File `/var/db/forge/` Tanpa Ketergantungan Eksternal):** Database paket terpasang menggunakan format teks manifest dan JSON di `/var/db/forge/installed/` agar tangguh saat proses bootstrap awal distro.
5. **ADR-005 (Isolasi Build RAM tmpfs & DESTDIR Staging):** Kompilasi berlangsung di `/tmp/forge/build/` (tmpfs) dan staged ke `DESTDIR` sebelum transaksi merge.
6. **ADR-006 (Pemeriksaan Tabrakan Berkas & Manifest Deterministik):** Validasi collision sebelum merge dan pencatatan seluruh berkas ke manifest untuk proses unmerge 100% bersih tanpa sisa.
7. **ADR-007 (Integrasi Layanan OpenRC):** Deteksi otomatis berkas layanan `/etc/init.d/` dan penyediaan hook pendaftaran runlevel via `rc-update`.
8. **ADR-008 (Meta-Target `@system` & `stage-export`):** Dukungan bawaan untuk kompilasi massal base distro Kura Linux dan pembuatan tarball distribusi `kura-stage.tar.xz`.
9. **ADR-009 (Protokol Mutlak HITL & Pengujian Terisolasi):** AI pengembang Forge wajib mengikuti protokol 5 langkah (*Plan $\rightarrow$ Chat $\rightarrow$ ACC $\rightarrow$ Eksekusi $\rightarrow$ Uji*) dan menguji perubahan di lingkungan terisolasi.
10. **ADR-010 (Hierarki Resolusi Hybrid Unified & Kebebasan Pengguna):** Forge menyediakan 3 tingkat resolusi paket (1: Forge Native Binhost, 2: Hybrid Fallback CachyOS/Arch, 3: Local Source Build) dengan kontrol penuh di tangan pengguna melalui konfigurasi dan flag CLI (`--binhost`, `--build-source`, `--allow-hybrid`, `--interactive`).
11. **ADR-011 (Pohon Resep Terpusat di Server & Sinkronisasi Klien):** Seluruh resep Forge di-hosting secara terpusat di Forge Server dan disinkronisasi ke klien secara efisien melalui perintah `forge sync`.
12. **ADR-012 (CI/CD Build Farm Locked to Target CPU Microarchitecture):** Server Forge menyediakan worker CI/CD builder yang mengompilasi paket dengan target arsitektur CPU pengguna (misal: `znver4`) menggunakan tool otomasi `forge import` dan mempublikasikan biner `.forge.tar.zst` ke Binary Library.
13. **ADR-013 (Introspeksi Hardware & Profil CPU `forge cpu-dump`):** Forge menyediakan perintah `forge cpu-dump` untuk mengekstrak arsitektur CPU, ekstensi ISA, dan CFLAGS optimal menjadi `cpu-profile.json` untuk disinkronkan ke server/CI/CD.
14. **ADR-014 (Sistem USE Flags & Multi-Version Slotting ala Portage):** Mengadopsi mekanisme USE flags untuk kontrol fitur granular dan Slots untuk koeksistensi beberapa versi mayor paket secara berdampingan.

---

## 3. Log Masalah & Solusi (Troubleshooting)

| Tanggal | Komponen | Masalah | Solusi |
| :--- | :--- | :--- | :--- |
| *2026-09-18* | *Inisialisasi* | *Kebutuhan arsitektur dasar Forge terpisah dari KuraLinux* | *Membangun blueprint awal Forge mengadopsi kebutuhan dari IDEA_FOR_FORGE.md* |
| *2026-09-18* | *Arsitektur* | *Kompilasi source lokal berat di mesin pengguna; butuh kecepatan binary namun tetap 100% native CPU* | *Mengembangkan ekosistem Server & CI/CD Builder Lock-CPU, perintah `cpu-dump` & `import`, serta arsitektur Hybrid Unified (Portage + Binhost + Fallback CachyOS/Arch)* |
