# MEMORY.md — Memori & Catatan Teknis Package Manager `forge` & `forge-server`

> Dokumen memori persisten AI untuk melacak progres pengembangan ekosistem package manager **`forge`** (klien) dan **`forge-server`** (server/CI-CD), keputusan arsitektur (ADR), status roadmap, dan log pemecahan masalah teknis.

---

## 1. Status & Roadmap Pengembangan

### Fase 1: Inisialisasi Arsitektur, Dokumentasi & Standarisasi (Selesai)
- [x] Inisialisasi repositori Git dan konfigurasi `.gitignore`.
- [x] Blueprint arsitektur Source-First & pemisahan `forge` vs `forge-server` di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] Panduan AI, aturan mutlak HITL & siklus verifikasi di [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- [x] Memori persisten & ADR di [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
- [x] Dokumentasi publik & panduan CLI di [`README.md`](file:///home/admin/Development/Forge/README.md).
- [x] Template konfigurasi bawaan `config/forge.conf.example` dengan default `mode = "source"`.
- [x] Struktur modular awal (`src/cli/`, `src/core/`, `src/cpu/`, `src/binhost/`, `src/hybrid/`, `src/server/`, `src/cicd/`).

### Fase 2: CPU Hardware Profiler (`forge cpu-dump`) & Config Engine
- [ ] Implementasi modul analisis CPU (`src/cpu/`): ekstraksi microarchitecture target (`znver4`, `alderlake`), deteksi ISA extensions (AVX-512, AVX2, SSE4.2), dan generasi `cpu-profile.json`.
- [ ] Parser konfigurasi `/etc/forge/forge.conf` dengan dukungan blok `[server]`, `[binhost]`, `[hybrid]`, `[cpu]`, dan `[use]`.

### Fase 3: Core CLI Klien `forge` & Recipe Parser (Portage-Inspired)
- [ ] Implementasi CLI dispatcher klien `forge` (`install`, `build`, `remove`, `list`, `query`, `search`, `sync`, `cpu-dump`, `stage-export`).
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

### Fase 10: Server Suite & CI/CD Builder (`forge-server`)
- [ ] Modul backend server (`src/server/`): `forge-server serve` untuk resep dan katalog biner `packages.db.zst`.
- [ ] CI/CD worker tool (`src/cicd/`): eksekusi `forge-server import` untuk build lock-CPU, packaging `.forge.tar.zst`, dan otomatisasi upload ke binary library.
- [ ] Catalog indexer tool (`forge-server index`).

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
10. **ADR-010 (Hierarki Resolusi Source-First Kompilasi Native):** Forge mengutamakan kompilasi lokal langsung dari kode sumber sebagai prioritas utama (Gentoo Portage mode). Opsi *Forge Native Binhost* dan *Hybrid Fallback (CachyOS/Arch)* disediakan sebagai akselerasi opsional (*opt-in*) tanpa memaksakan biner kepada pengguna.
11. **ADR-011 (Pohon Resep Terpusat di Server & Sinkronisasi Klien):** Seluruh resep Forge di-hosting secara terpusat di server `forge-server` dan disinkronisasi ke klien secara efisien melalui perintah `forge sync`.
12. **ADR-012 (CI/CD Build Farm Locked to Target CPU Microarchitecture):** Server `forge-server` menyediakan worker CI/CD builder yang mengompilasi paket dengan target arsitektur CPU pengguna (misal: `znver4`) menggunakan tool otomasi `forge-server import` dan mempublikasikan biner `.forge.tar.zst` ke Binary Library.
13. **ADR-013 (Introspeksi Hardware & Profil CPU `forge cpu-dump`):** Forge menyediakan perintah `forge cpu-dump` untuk mengekstrak arsitektur CPU, ekstensi ISA, dan CFLAGS optimal menjadi `cpu-profile.json` untuk disinkronkan ke server/CI/CD.
14. **ADR-014 (Sistem USE Flags & Multi-Version Slotting ala Portage):** Mengadopsi mekanisme USE flags untuk kontrol fitur granular dan Slots untuk koeksistensi beberapa versi mayor paket secara berdampingan.
15. **ADR-015 (Pemisahan Binary Klien `forge` dan Server `forge-server`):** Memisahkan secara tegas antarmuka dan paket eksekusi antara aplikasi klien pengguna (`forge`) dan backend suite/CI-CD builder (`forge-server`) demi menjaga footprint klien tetap ringan dan terfokus.

---

## 3. Log Masalah & Solusi (Troubleshooting)

| Tanggal | Komponen | Masalah | Solusi |
| :--- | :--- | :--- | :--- |
| *2026-09-18* | *Inisialisasi* | *Kebutuhan arsitektur dasar Forge terpisah dari KuraLinux* | *Membangun blueprint awal Forge mengadopsi kebutuhan dari IDEA_FOR_FORGE.md* |
| *2026-09-18* | *Arsitektur* | *Kompilasi source lokal berat di mesin pengguna; butuh opsi biner native & server CI/CD* | *Mengembangkan ekosistem Server & CI/CD Builder Lock-CPU, perintah `cpu-dump` & `import`, serta arsitektur Hybrid Unified* |
| *2026-09-18* | *Filosofi* | *Prioritas default sempat condong ke binhost; komponen server tercampur dengan klien* | *Mengoreksi prioritas menjadi Source-First (Gentoo-style) dan memisahkan binary klien `forge` dengan server suite `forge-server`* |
