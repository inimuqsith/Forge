# MEMORY.md — Memori & Catatan Teknis Package Manager `forge` & `forge-server`

> Dokumen memori persisten AI untuk melacak progres pengembangan ekosistem package manager **`forge`** (klien) dan **`forge-server`** (server/CI-CD) berbasis **Rust**, keputusan arsitektur (ADR), status roadmap riil (membedakan kode yang sudah diimplementasikan vs blueprint arsitektur), dan panduan serah-terima (*handover*) antar-agen AI.

---

## 1. Status & Roadmap Pengembangan Riil (Detailed Status Tracker)

### Fase 1: Inisialisasi Arsitektur, Dokumentasi & Standarisasi
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Inisialisasi repositori Git dan konfigurasi `.gitignore`.
- [x] Blueprint arsitektur komprehensif di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] Pedoman AI mutlak Human-In-The-Loop (HITL) di [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- [x] Memori persisten & 25 ADR di [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
- [x] Dokumentasi publik & panduan CLI di [`README.md`](file:///home/admin/Development/Forge/README.md).
- [x] Template konfigurasi bawaan `config/forge.conf.example` & `config/system.conf.example`.
- [x] Konsolidasi ke Clean 2-Crate Workspace Layout: [`crates/forge`](file:///home/admin/Development/Forge/crates/forge) dan [`crates/forge-server`](file:///home/admin/Development/Forge/crates/forge-server).

---

### Fase 2: CPU Hardware Profiler (`forge cpu-dump`) & Config Engine
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi modul CPU profiler di [`crates/forge/src/cpu.rs`](file:///home/admin/Development/Forge/crates/forge/src/cpu.rs): ekstraksi vendor, microarchitecture target (`znver4`, `alderlake`), deteksi ISA extensions (AVX-512, AVX2, SSE4.2), hierarki cache L1-L3, dan generasi `cpu-profile.json`.
- [x] Parser konfigurasi `/etc/forge/forge.conf` (`ForgeConfig`) dan `/etc/forge/system.conf` (`SystemSetupConfig`) via Serde TOML di [`crates/forge/src/lib.rs`](file:///home/admin/Development/Forge/crates/forge/src/lib.rs).
- [x] Unit test `test_cpu_detection` lulus 100%.

---

### Fase 3: Wizard `forge setup` & `forge system-setup` (Bootstrap Distro)
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi wizard `forge setup` untuk inisialisasi direktori dan konfigurasi package manager di [`crates/forge/src/main.rs`](file:///home/admin/Development/Forge/crates/forge/src/main.rs).
- [x] Implementasi wizard `forge system-setup` untuk bootstrap Kura Linux (pemilihan arsitektur, base profile, driver kernel monolithic, OpenRC defaults).
- [x] Integrasi prompt panduan pasca-setup untuk memicu kompilasi base OS `forge install @system`.
- [x] Mekanisme fallback template bawaan jika `forge install @system` dijalankan sebelum wizard.

---

### Fase 4: Core Engine (Portage-Inspired) & DAG Dependency Resolver
**Status:** ⚠️ **BLUEPRINT SELESAI / IMPLEMENTASI KODE PENDING**
- [x] **Desain Arsitektur:** Blueprint spesifikasi graf asiklis terarah (DAG), model Node/Edge, dan algoritma *Topological Sort* & *Cycle Detection* (Kahn / Tarjan SCC) di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] **USE Flags Engine:** Implementasi `UseFlagsEngine` di [`crates/forge/src/lib.rs`](file:///home/admin/Development/Forge/crates/forge/src/lib.rs) untuk evaluasi flag global dan per-paket (`+flag`, `-flag`). Unit test `test_use_flags_engine` lulus.
- [x] **Slotting Engine:** Data model slot multi-versi (`pkg:slot`) di [`crates/forge/src/lib.rs`](file:///home/admin/Development/Forge/crates/forge/src/lib.rs).
- [x] **Resep Parser:** Deserialisasi All-in-One `recipe.toml` (`PackageMeta`, `DependenciesMeta`, `SourcesMeta`, `BuildMeta`).
- [ ] **Pending Implementasi Kode:** Engine Rust `crates/forge/src/resolver.rs` untuk:
  - Membaca dan membangun graph dari seluruh pohon resep (`recipes/system/`, `core/`, `extra/`).
  - Menyusun urutan eksekusi kompilasi topologis otomatis untuk single package dan `@system`.
  - Filter conditional dependencies berbasis USE flags (`flag? ( dep )`).
  - Laporan diagnostik jika terjadi siklus dependensi sirkular (*circular dependency error*).

---

### Fase 5: Source Fetcher & Kriptografi Integritas
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Fetcher berkas sumber (HTTP/HTTPS via curl) ke direktori cache (`/var/cache/forge/distfiles/` atau `./distfiles/`) di [`crates/forge/src/builder.rs`](file:///home/admin/Development/Forge/crates/forge/src/builder.rs).
- [x] Verifikasi kriptografis hash SHA256 otomatis sebelum ekstraksi tarball.

---

### Fase 6: Sandbox Build Engine & DESTDIR Staging
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Isolasi direktori build di RAM tmpfs (`/tmp/forge/build/<pkg>-<ver>/`).
- [x] Injeksi otomatis compiler flags Kura Linux (`-O3 -march=native -pipe -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt`) dan linker `mold` (`-fuse-ld=mold`).
- [x] **Akselerasi Ccache (v4.13.5):** Auto-detection binary `ccache`, resolusi direktori `CCACHE_DIR` (`/var/cache/forge/ccache` / `distfiles/.ccache`), dan injeksi `CC="ccache clang"`, `CXX="ccache clang++"`.
- [x] **Pengecualian Glibc (ADR-002):** Otomatis dialihkan ke GCC bawaan (`CC="ccache gcc"`) tanpa flag `-march` kustom demi kestabilan build system Glibc.
- [x] Staging hasil kompilasi ke direktori terisolasi `DESTDIR` (`/tmp/forge/stage/<pkg>/`).

---

### Fase 7: Transactional Merger & Collision Detector
**Status:** ⚠️ **BLUEPRINT SELESAI / IMPLEMENTASI KODE PENDING**
- [x] **Desain Arsitektur:** Blueprint spesifikasi Pre-flight Collision Scanning, Atomic Merge pipeline, dan Rollback Log di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [ ] **Pending Implementasi Kode:** Engine Rust `crates/forge/src/merger.rs` untuk:
  - Memindai tabrakan berkas staging terhadap database paket terpasang (`/var/db/forge/installed/`).
  - Menyalin file dari `$DESTDIR` ke `$FORGE_ROOT` (`/`) dengan preservasi symlink, permissions Unix, dan timestamps.
  - Pencatatan log transaksi sementara (`/tmp/forge/txn_<id>.log`) untuk auto-rollback jika terjadi kegagalan I/O.

---

### Fase 8: Package Manifest Database & Unmerge Cleaner
**Status:** ⚠️ **BLUEPRINT SELESAI / IMPLEMENTASI KODE PENDING**
- [x] **Desain Arsitektur:** Spesifikasi format flat-file database `/var/db/forge/installed/<pkg>/manifest`, *reverse-directory pruning*, dan proteksi `CONFIG_PROTECT` di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [ ] **Pending Implementasi Kode:** Engine Rust `crates/forge/src/db.rs` untuk:
  - Menulis manifest berkas, ukuran, hash SHA256, dan metadata build ke database flat-file.
  - Eksekusi `forge remove <pkg>`: membaca manifest, menghapus file paket, dan membersihkan direktori kosong secara rekursif (*leaf-to-root pruning*).
  - Melindungi file konfigurasi di `/etc/` dari modifikasi pengguna (`CONFIG_PROTECT`).
  - Eksekusi post-unmerge hooks (`ldconfig`, `rc-update`).
  - Implementasi CLI `forge list` dan `forge query <pkg>`.

---

### Fase 9: Hybrid Unified Engine (Binhost & CachyOS/Arch Fallback)
**Status:** 🟡 **LOGIKA MODEL SELESAI / INTEGRASI NETWORK API PENDING**
- [x] Implementasi data model binhost & client matcher di [`crates/forge/src/binhost.rs`](file:///home/admin/Development/Forge/crates/forge/src/binhost.rs).
- [x] Implementasi adapter fallback CachyOS (v4/v3) & Arch Linux di [`crates/forge/src/hybrid.rs`](file:///home/admin/Development/Forge/crates/forge/src/hybrid.rs).
- [x] CLI flag dispatcher (`--binhost`, `--hybrid`, `--interactive`) di [`crates/forge/src/main.rs`](file:///home/admin/Development/Forge/crates/forge/src/main.rs).
- [ ] **Pending:** Live network streaming download & decompresi zstd untuk biner binhost/CachyOS.

---

### Fase 10: Server Suite & CI/CD Builder (`forge-server`)
**Status:** 🟡 **CLI WORKER SELESAI / REAL HTTP DAEMON PENDING**
- [x] Implementasi CLI suite `forge-server` di [`crates/forge-server/src/main.rs`](file:///home/admin/Development/Forge/crates/forge-server/src/main.rs).
- [x] CLI CI/CD Lock-CPU Builder: `forge-server import <pkg> --target-cpu <cpu>` dan `forge-server import --all-system`.
- [x] CLI Catalog Indexer: `forge-server index --storage-path <path>`.
- [ ] **Pending:** Implementasi HTTP REST API daemon riil (`forge-server serve`) menggunakan `tokio`/`axum` untuk sinkronisasi pohon resep dan serving katalog `packages.db.zst`.

---

### Fase 11: Seed Toolchain Pure Source, OpenRC Hook & Stage Exporter
**Status:** 🟡 **SEED TOOLCHAIN SELESAI / STAGE EXPORTER PENDING**
- [x] **Pure Source Seed Toolchain Bundler (ADR-019):** Implementasi `forge toolchain bundle` di [`crates/forge/src/toolchain.rs`](file:///home/admin/Development/Forge/crates/forge/src/toolchain.rs) yang mengemas HANYA biner/library hasil kompilasi murni dari `/tmp/forge/stage/` menjadi `dist/kura-toolchain.tar.xz` tanpa menyalin file host.
- [x] **Resep Hulu Resmi Kura Linux `@system`:** Resep All-in-One di `recipes/system/` (glibc, gcc, llvm, mold, make, ninja, linux-headers, openrc, pkgconf).
- [x] Desain integrasi OpenRC hook `/etc/init.d/` dan `rc-update`.
- [ ] **Pending:** Implementasi riil `forge stage-export` untuk mengemas rootfs target menjadi `kura-stage.tar.xz`.

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
22. **ADR-022 (Topological DAG Dependency Resolution & Cycle Detection Engine):** Seluruh proses build dan instalasi paket wajib melewati resolusi DAG yang memisahkan `depends` (runtime) dan `makedepends` (build-time), mengevaluasi kondisi USE flags, serta mendeteksi siklus dependensi sirkular secara deterministik sebelum kompilasi dimulai.
23. **ADR-023 (Transactional Atomic Merger, Collision Detector & Flat-File Manifest Database):** Setiap paket yang berhasil dikompilasi ke staging `$DESTDIR` wajib melalui pemindaian tabrakan berkas (*pre-flight collision scan*) sebelum digabungkan secara atomik ke rootfs target `$FORGE_ROOT` dan dicatat ke `/var/db/forge/installed/<pkg>/manifest`.
24. **ADR-024 (Config-Protected Unmerge Cleaner & Reverse Directory Pruning):** Penghapusan paket dilakukan secara presisi dari leaf files ke root, menghapus folder kosong tanpa merusak direktori bersama sistem, serta melindungi berkas konfigurasi `/etc/` yang telah dimodifikasi oleh pengguna (`CONFIG_PROTECT`).
25. **ADR-025 (Integrated Compiler Acceleration with Ccache 4.13.5 & Memory tmpfs Isolation):** Engine kompilasi Forge secara otomatis mengintegrasikan akselerasi compiler cache `ccache` pada `CC` dan `CXX`, serta mengisolasi build directory di RAM `tmpfs` untuk memaksimalkan kecepatan I/O dan efisiensi siklus rebuild.

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
| *2026-09-18* | *Penyederhanaan Crate* | *Layout 6 crate berlebihan dan membingungkan* | *Mengkonsolidasikan workspace menjadi Clean 2-Crate Layout (`crates/forge` dan `crates/forge-server`)* |
| *2026-09-18* | *Akselerasi Rebuild* | *Kompilasi ulang source code berulang memakan waktu lama* | *Mengintegrasikan Ccache (v4.13.5) secara otomatis pada pipeline builder Forge (ADR-025)* |
| *2026-09-18* | *Fokus Arsitektur & Transparansi* | *Kebutuhan blueprint mendalam dan transparansi status kode vs arsitektur* | *Mendokumentasikan blueprint lengkap DAG, Merger, Manifest DB, memperinci status tiap fase di MEMORY.md, dan menambah ADR-022 s/d ADR-025* |

---

## 4. Panduan Serah Terima AI Agent (Incoming AI Agent Handover Guide)

> **Catatan Penting untuk AI Agent Penerus:**
> Repositori ini telah dikonsolidasi secara rapi menjadi **Clean 2-Crate Workspace Layout** dengan tingkat kesiapan **~65%**. Seluruh blueprint arsitektur, diagram, aturan mutlak, dan ADR telah didokumentasikan secara lengkap.

### 📌 Ringkasan Status & State Workspace:
- **Workspace:** 2 Crate murni: [`crates/forge`](file:///home/admin/Development/Forge/crates/forge) (Klien & Engine Library) dan [`crates/forge-server`](file:///home/admin/Development/Forge/crates/forge-server) (Server & CI/CD Builder).
- **Toolchain:** Rust 1.97.1, LLVM/Clang 22, Linker `mold`, Ccache 4.13.5, Thin LTO, `-O3 -march=native`.
- **Seed Toolchain:** Staged murni di `/tmp/forge/stage/` dan dikemas ke `dist/kura-toolchain.tar.xz` (ADR-019).

### 🛑 6 Aturan Mutlak yang Wajib Diikuti:
1. **HITL (Human-In-The-Loop):** Wajib ikuti siklus 5-langkah (*Plan $\rightarrow$ Chat $\rightarrow$ ACC $\rightarrow$ Eksekusi $\rightarrow$ Uji*). Jangan edit/buat file tanpa ACC di chat.
2. **HARAM EDIT KURALINUX:** Dilarang keras menyentuh direktori `/home/admin/Development/KuraLinux/`.
3. **HARAM AMBIL DARI HOST (ADR-019):** Biner dan toolchain wajib 100% dikompilasi dari source code upstream via resep ke `/tmp/forge/stage/`. Dilarang meng-copy biner dari `/usr/bin/` atau `/usr/lib/`.
4. **GLIBC EXEMPTION (ADR-002):** Paket Glibc di-build dengan GCC standar tanpa flag `-march` kustom demi stabilitas.
5. **CCACHE ACCELERATION (ADR-025):** Kompilasi memanfaatkan Ccache 4.13.5 pada build engine.
6. **OPENRC ONLY:** Tidak boleh ada ketergantungan pada Systemd.

### 🎯 4 Tugas Prioritas Pengembangan Selanjutnya (Sisa 35%):
1. **Task 1: DAG Dependency Resolver (`crates/forge/src/resolver.rs`):**
   - Implementasikan algoritma *Topological Sort* (Kahn / Tarjan SCC) dan deteksi circular dependency dari resep `recipe.toml` (`[dependencies.runtime]` vs `[dependencies.build]`).
2. **Task 2: Transactional Merger & Collision Detector (`crates/forge/src/merger.rs`):**
   - Implementasikan pre-flight collision scanner terhadap `/var/db/forge/installed/`, atomic copy ke `$FORGE_ROOT`, preservasi symlink/permissions, dan rollback handler.
3. **Task 3: Manifest Database & Unmerge Cleaner (`crates/forge/src/db.rs`):**
   - Implementasikan pencatatan manifest di `/var/db/forge/installed/<pkg>/manifest`, reverse-directory pruning untuk `forge remove <pkg>`, proteksi `CONFIG_PROTECT` di `/etc/`, dan query/list CLI.
4. **Task 4: Distro Stage Exporter & Real Server Daemon:**
   - Implementasikan tarball bundler `forge stage-export` $\rightarrow$ `kura-stage.tar.xz`, dan HTTP REST API daemon pada `crates/forge-server` (`serve`).
