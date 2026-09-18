# MEMORY.md — Memori & Catatan Teknis Package Manager `forge` & `forge-server`

> Dokumen memori persisten AI untuk melacak progres pengembangan ekosistem package manager **`forge`** (klien) dan **`forge-server`** (server/CI-CD) berbasis **Rust**, keputusan arsitektur (ADR), status roadmap, dan log pemecahan masalah teknis.

---

## 1. Status & Roadmap Pengembangan

### Fase 1: Inisialisasi Arsitektur, Dokumentasi & Standarisasi (Selesai)
- [x] Inisialisasi repositori Git dan konfigurasi `.gitignore`.
- [x] Blueprint arsitektur Rust, Source-First, wizard `forge setup` & `forge system-setup` di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] Panduan AI, aturan mutlak HITL & siklus verifikasi di [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- [x] Memori persisten & ADR di [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
- [x] Dokumentasi publik & panduan CLI di [`README.md`](file:///home/admin/Development/Forge/README.md).
- [x] Template konfigurasi bawaan `config/forge.conf.example` & `config/system.conf.example`.
- [x] Inisialisasi Cargo Workspace (`crates/forge-core`, `crates/forge-cpu`, `crates/forge-binhost`, `crates/forge-hybrid`, `crates/forge-cli`, `crates/forge-server`).

### Fase 2: CPU Hardware Profiler (`forge cpu-dump`) & Config Engine
- [x] Implementasi crate `forge-cpu`: ekstraksi microarchitecture target (`znver4`, `alderlake`), deteksi ISA extensions (AVX-512, AVX2, SSE4.2), dan generasi `cpu-profile.json`.
- [x] Parser konfigurasi `/etc/forge/forge.conf` dan `/etc/forge/system.conf`.

### Fase 3: Wizard `forge setup` & `forge system-setup` (Bootstrap Distro)
- [x] Implementasi wizard interaktif `forge setup` untuk inisialisasi direktori dan konfigurasi package manager.
- [x] Implementasi wizard `forge system-setup` untuk bootstrap Kura Linux (pemilihan arsitektur, base profile, driver kernel monolithic, OpenRC defaults).
- [x] Integrasi prompt panduan pasca-setup untuk menjalankan `forge install @system`.
- [x] Mekanisme fallback template bawaan jika `forge install @system` dijalankan tanpa wizard.

### Fase 4: Core Engine (Portage-Inspired) & DAG Resolver
- [x] Implementasi crate `forge-core`: DAG resolver berbasis *Petgraph* untuk dependensi (`depends`, `makedepends`).
- [x] Engine USE flags & evaluasi status fitur paket (`forge_use`).
- [x] Engine multi-version slots (`pkg:slot`).
- [x] Parser resep All-in-One `recipe.toml` dengan embedded script dan compiler supremacy hierarchy.

### Fase 5: Source Fetcher & Kriptografi Integritas
- [x] Fetcher berkas sumber (HTTP/HTTPS via curl) ke cache path (`/var/cache/forge/distfiles/` atau `./distfiles/`).
- [x] Verifikasi kriptografis SHA256 untuk sumber dan paket biner.

### Fase 6: Sandbox Build Engine & DESTDIR Staging
- [x] Isolasi build di RAM tmpfs (`/tmp/forge/build/`).
- [x] Injeksi otomatis compiler flags Kura Linux (`-march=native` / target CPU profil).
- [x] Pengecualian optimasi custom untuk Glibc (ADR-002).
- [x] Staging hasil kompilasi ke `/tmp/forge/stage/` (`DESTDIR`).

### Fase 7: Transactional Merger, Collision Detector, & Flat-File DB
- [x] Pre-flight collision scan terhadap `/var/db/forge/installed/`.
- [x] Penyalinan berkas atomik dari `$DESTDIR` ke rootfs (`/`).
- [x] Pencatatan manifest berkas, symlink, permission, slot, dan metadata JSON.
- [x] Pengelolaan daftar paket aktif (`/var/db/forge/world`).

### Fase 8: Unmerge Cleaner & Proteksi Konfigurasi
- [x] Penghapusan presisi berdasarkan manifest dan pembersihan direktori kosong.
- [x] Proteksi file konfigurasi yang dimodifikasi pengguna di `/etc/`.
- [x] Analisis dan pembersihan orphan packages.

### Fase 9: Hybrid Unified Engine (Binhost & CachyOS/Arch Fallback)
- [x] Implementasi crate `forge-binhost`: pencocokan target CPU hash & USE flags.
- [x] Implementasi crate `forge-hybrid`: adapter repositori biner CachyOS (x86-64-v4/v3) & Arch Linux.
- [x] Mekanisme seleksi provider interaktif bagi pengguna (`--interactive`).

### Fase 10: Server Suite & CI/CD Builder (`forge-server`)
- [x] Implementasi crate `forge-server`: `forge-server serve` untuk resep dan katalog biner `packages.db.zst`.
- [x] CI/CD worker tool: eksekusi `forge-server import` untuk build lock-CPU, packaging `.forge.tar.zst`, dan otomatisasi upload ke binary library.
- [x] Catalog indexer tool (`forge-server index`).

### Fase 11: OpenRC Hook, Stage Exporter, & Base Recipes `@system`
- [x] OpenRC service auto-discovery di `/etc/init.d/` dan post-install triggers.
- [x] Utilitas pembuatan stage distribusi: `forge stage-export` $\rightarrow$ `kura-stage.tar.xz`.
- [x] Penyusunan pohon resep resmi Kura Linux `@system` (LLVM 22, Mold, Ninja, Pkgconf, Make, Glibc, GCC, Binutils, OpenRC, Linux-Headers).

---

## 2. Keputusan Arsitektur Resmi (ADR Index)

1. **ADR-001 (Kompilasi 100% Native Silikon):** Forge mengompilasi seluruh paket langsung dari kode sumber upstream dengan menyuntikkan flag optimasi silikon target (`-march=native -O3 -pipe -flto=thin`) untuk performa CPU maksimal.
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
16. **ADR-016 (Setup Wizards & System Bootstrap Configuration):** Menyediakan perintah `forge setup` untuk inisialisasi package manager dan `forge system-setup` untuk bootstrap distro Kura Linux (pemilihan arsitektur CPU, profil base, opsi kernel monolithic) sebelum memicu eksekusi `forge install @system`, dengan fallback template bawaan jika wizard dilewati.
17. **ADR-017 (Implementasi Bahasa Rust & Pipeline Kompilasi Ultra-Cepat):** Forge diimplementasikan murni menggunakan bahasa pemrograman Rust dalam Cargo Workspace multi-crate, ditenagai backend LLVM 22, ultra-fast linker `mold` (`-fuse-ld=mold`), optimasi Link-Time Optimization (Thin/Full LTO), `panic = "abort"`, serta dukungan PGO untuk mencapai throughput eksekusi maksimal.
18. **ADR-018 (Isolated Seed Toolchain & Sysroot Packaging):** Untuk memutus ketergantungan dari toolchain host dan mencegah polusi lingkungan build, Forge menyediakan sub-sistem `forge toolchain bundle` yang mengemas biner hasil kompilasi ke dalam arsip `kura-toolchain.tar.xz` berstruktur UsrMerge standar untuk diekstrak langsung ke dalam sysroot stage Kura Linux.
19. **ADR-019 (Penegakan Mutlak Pure Source-Built & Zero Host Harvesting):** Dilarang keras menyalin atau memanen (*harvest*) biner, library, atau compiler dari sistem host (`/usr/bin/`, `/usr/lib/llvm/22/`) untuk dimasukkan ke dalam paket distribusi atau seed toolchain. Seluruh isi tarball toolchain dan sistem Kura Linux wajib 100% murni dikompilasi dari kode sumber upstream melalui resep `recipe.toml` di direktori staging Forge (`/tmp/forge/stage/`).
20. **ADR-020 (Format Resep All-in-One `recipe.toml` & Hierarki Supremasi Compiler):** Menggabungkan metadata paket deklaratif dan embedded POSIX bash script ke dalam satu file tunggal `recipe.toml`. Engine `forge-core` mengeksekusi script dengan menginjeksi environment variable compiler wajib (`CC=clang`, `LD=mold`, `-march=native`, `-O3`, `-flto=thin`) secara mutlak sebelum script subshell dieksekusi, kecuali ada flag pengecualian (seperti Glibc).
21. **ADR-021 (Suite Toolchain Hulu Terbaru 2026):** Menstandarkan versi paket toolchain inti sistem Kura Linux ke versi rilis modern hulu: LLVM/Clang 22.1.x, mold 2.42.1, Ninja 1.13.2, Pkgconf 3.0.7, GNU Make 4.4.1, Glibc 2.44, GCC 15.3.0, Binutils 2.44, Linux-headers 6.13.x, dan OpenRC 0.56.

---

## 3. Log Masalah & Solusi (Troubleshooting)

| Tanggal | Komponen | Masalah | Solusi |
| :--- | :--- | :--- | :--- |
| *2026-09-18* | *Inisialisasi* | *Kebutuhan arsitektur dasar Forge terpisah dari KuraLinux* | *Membangun blueprint awal Forge mengadopsi kebutuhan dari IDEA_FOR_FORGE.md* |
| *2026-09-18* | *Arsitektur* | *Kompilasi source lokal berat di mesin pengguna; butuh opsi biner native & server CI/CD* | *Mengembangkan ekosistem Server & CI/CD Builder Lock-CPU, perintah `cpu-dump` & `import`, serta arsitektur Hybrid Unified* |
| *2026-09-18* | *Filosofi* | *Prioritas default sempat condong ke binhost; komponen server tercampur dengan klien* | *Mengoreksi prioritas menjadi Source-First (Gentoo-style) dan memisahkan binary klien `forge` dengan server suite `forge-server`* |
| *2026-09-18* | *Toolchain & UX* | *Dibutuhkan engine performa tinggi, wizard setup interaktif, dan bootstrap Kura Linux* | *Memilih bahasa Rust dengan linker mold + Thin LTO, serta mendesain wizard `forge setup` dan `forge system-setup` sebelum `forge install @system`* |
| *2026-09-18* | *Isolasi Host* | *Ketergantungan terhadap compiler host saat bootstrap awal Kura Linux* | *Mengembangkan sub-perintah `forge toolchain bundle` dan menghasilkan seed toolchain `dist/kura-toolchain.tar.xz` untuk diekstrak ke sysroot stage Kura Linux* |
| *2026-09-18* | *Standarisasi Resep* | *Format terpisah bash + toml rentan fragmentasi; butuh format efisien & hierarki compiler kuat* | *Menerapkan format All-in-One `recipe.toml` berbasis Serde TOML dengan subshell environment injection mutlak* |
| *2026-09-18* | *Integritas Build* | *Haram mutlak mengambil biner dari host filesystem* | *Menegakkan aturan Pure Source-Built (ADR-019): toolchain bundler hanya mengemas biner yang sah terkompilasi dari source code oleh Forge di staging `/tmp/forge/stage/`* |
