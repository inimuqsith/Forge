# MEMORY.md — Memori & Catatan Teknis Package Manager `forge` & `forge-server`

> Dokumen memori persisten AI untuk melacak progres pengembangan ekosistem package manager **`forge`** (klien) dan **`forge-server`** (server/CI-CD) berbasis **Rust**, keputusan arsitektur (ADR), status roadmap riil (membedakan kode yang sudah diimplementasikan vs blueprint arsitektur), dan panduan serah-terima (*handover*) antar-agen AI.

---

## 1. Status & Roadmap Pengembangan Riil (Detailed Status Tracker)

### Fase 1: Inisialisasi Arsitektur, Dokumentasi & Standarisasi
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Inisialisasi repositori Git dan konfigurasi `.gitignore`.
- [x] Blueprint arsitektur komprehensif di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] Pedoman AI mutlak Human-In-The-Loop (HITL) di [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- [x] Memori persisten & 26 ADR di [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
- [x] Dokumentasi publik & panduan CLI di [`README.md`](file:///home/admin/Development/Forge/README.md).
- [x] Template konfigurasi tunggal terpusat `config/forge.conf.example`.
- [x] Konsolidasi ke Clean 2-Crate Workspace Layout: [`crates/forge`](file:///home/admin/Development/Forge/crates/forge) dan [`crates/forge-server`](file:///home/admin/Development/Forge/crates/forge-server).

---

### Fase 2: CPU Hardware Profiler (`forge cpu-dump`) & Config Engine
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi modul CPU profiler di [`crates/forge/src/cpu.rs`](file:///home/admin/Development/Forge/crates/forge/src/cpu.rs): ekstraksi vendor, microarchitecture target (`znver4`, `alderlake`), deteksi ISA extensions (AVX-512, AVX2, SSE4.2), hierarki cache L1-L3, dan generasi `cpu-profile.json`.
- [x] Parser konfigurasi tunggal `/etc/forge/forge.conf` (`ForgeConfig`) via Serde TOML di [`crates/forge/src/lib.rs`](file:///home/admin/Development/Forge/crates/forge/src/lib.rs).
- [x] Unit test `test_cpu_detection` lulus 100%.

---

### Fase 3: Wizard Konfigurasi Package Manager (`forge setup`)
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi wizard `forge setup` untuk inisialisasi direktori dan konfigurasi package manager di [`crates/forge/src/main.rs`](file:///home/admin/Development/Forge/crates/forge/src/main.rs).
- [x] **Eliminasi `system-setup` (ADR-026):** Menghapus wizard `system-setup` dan file `system.conf` untuk menjaga prinsip *Single Responsibility* & *Single Source of Truth* di `forge.conf`.

---

### Fase 4: Core Engine & DAG Dependency Resolver
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Desain Arsitektur:** Blueprint spesifikasi graf asiklis terarah (DAG), model Node/Edge, dan algoritma *Topological Sort* & *Cycle Detection* (Kahn / Tarjan SCC) di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] **USE Flags Engine:** Implementasi `UseFlagsEngine` di [`crates/forge/src/lib.rs`](file:///home/admin/Development/Forge/crates/forge/src/lib.rs) untuk evaluasi flag global dan per-paket (`+flag`, `-flag`). Unit test `test_use_flags_engine` lulus.
- [x] **Slotting Engine:** Data model slot multi-versi (`pkg:slot`) di [`crates/forge/src/lib.rs`](file:///home/admin/Development/Forge/crates/forge/src/lib.rs).
- [x] **Resep Parser:** Deserialisasi All-in-One `recipe.toml` (`PackageMeta`, `DependenciesMeta`, `SourcesMeta`, `BuildMeta`).
- [x] **Implementasi Kode:** Engine Rust [`crates/forge/src/resolver.rs`](file:///home/admin/Development/Forge/crates/forge/src/resolver.rs):
  - `PackageId`, `PackageNode`, `DependencyKind`, `DependencyEdge`, `DependencyGraph`, `ExecutionStep`, `ResolutionPlan`.
  - Scanner resep `RecipeScanner` memindai direktori `/var/db/forge/recipes/` dan fallback `./recipes/`.
  - Evaluasi USE flags bersyarat `flag? ( dep )` dan `!flag? ( dep )`.
  - Ekspansi rekursif meta-paket (`base`, `base-devel`) sesuai filosofi *Everything is a Package*.
  - Algoritma Kahn (In-Degree Queue) untuk menyusun urutan build linier deterministik.
  - Deteksi siklus dependensi sirkular (DFS Cycle Tracer) dengan pelaporan diagnostik box-drawing visual.
  - 5 Unit tests lulus 100% (`test_dag_linear_resolution`, `test_dag_diamond_resolution`, `test_dag_cycle_detection`, `test_use_flags_conditional_filtering`, `test_meta_package_expansion`).
- [x] Integrasi CLI `forge install <target>` di [`crates/forge/src/main.rs`](file:///home/admin/Development/Forge/crates/forge/src/main.rs) untuk kalkulasi dan visualisasi Topological Resolution Plan.

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
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Desain Arsitektur:** Blueprint spesifikasi Pre-flight Collision Scanning, Atomic Merge pipeline, dan Rollback Log di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] **Engine Merger (`crates/forge/src/merger.rs`):**
  - Implementasi `MergeTransaction`, `StagedEntry`, `CollisionReport`, `JournalAction`.
  - Pemindaian tabrakan pra-instalasi (`preflight_scan`) terhadap `/var/db/forge/installed/`.
  - Penulisan berkas atomik (`tempfile` $\rightarrow$ `fsync` $\rightarrow$ `rename`), preservasi hak akses Unix (`chmod`), symlink, dan timestamps.
  - Pencatatan jurnal transaksi `txn_<id>.journal` dengan auto-rollback LIFO jika terjadi kegagalan I/O.
  - Proteksi konfigurasi `CONFIG_PROTECT`: penyimpanan `._cfg0000_<file>` untuk berkas `/etc/` yang termodifikasi.
  - Eksekusi post-merge hooks (deteksi OpenRC `/etc/init.d/`, `ldconfig`).
- [x] Unit test `test_preflight_collision_detector`, `test_atomic_merge_and_permissions`, `test_config_protect_mechanism`, `test_transactional_rollback_on_failure` lulus 100%.

---

### Fase 8: Package Manifest Database & Unmerge Cleaner
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Desain Arsitektur:** Spesifikasi format flat-file database `/var/db/forge/installed/<pkg>/manifest`, *reverse-directory pruning*, dan proteksi `CONFIG_PROTECT` di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] **Engine Database (`crates/forge/src/db.rs`):**
  - Data model `InstalledDatabase`, `PackageManifest`, `ManifestEntry` (format `obj`, `sym`, `dir` dengan hash SHA256 & mtime/size), dan `PackageMetadata`.
  - Pencatatan manifest deterministik, `metadata.json`, `USE`, `CFLAGS`, dan `CONTENTS` ke `/var/db/forge/installed/<pkg>-<ver>:<slot>/`.
  - Implementasi `unmerge_package`: penghapusan berkas & symlink, proteksi `CONFIG_PROTECT` (melindungi `/etc/` termodifikasi), dan *reverse leaf-to-root directory pruning* untuk membersihkan folder kosong tanpa merusak folder sistem bersama.
  - Integrasi CLI sub-perintah `forge remove <pkg>`, `forge list`, dan `forge query <pkg>` di [`crates/forge/src/main.rs`](file:///home/admin/Development/Forge/crates/forge/src/main.rs).
- [x] Unit test `test_manifest_entry_serialization`, `test_installed_database_record_and_get`, dan `test_unmerge_reverse_pruning` lulus 100%.

---

### Fase 9: 3-Tier Package Cascade Resolution Engine & Anti-Brick CachyOS Fallback
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi data model binhost & client matcher di [`crates/forge/src/binhost.rs`](file:///home/admin/Development/Forge/crates/forge/src/binhost.rs).
- [x] **Anti-Brick CachyOS Adapter (`crates/forge/src/cachyos.rs`):** Auto-detection mikroarsitektur CPU (`Znver4`, `X86_64_V4`, `X86_64_V3`, `Generic`) dan Core OS Blacklist Protection mutlak (`glibc`, `openrc`, `gcc`, `llvm`, `mold`, `eudev`, `kmod`, `shadow`, `util-linux`, `base`, `base-devel`, `forge`, `systemd`).
- [x] **3-Tier Cascade Resolution Engine (`crates/forge/src/cascade.rs`):** Resolusi deterministik: Tingkat 1 (Forge Native Binhost `.forge.tar.zst`) -> Tingkat 2 (CachyOS Prebuilt dengan bypass blacklist) -> Tingkat 3 (Source-First Native Compilation).
- [x] **Penyederhanaan CLI Flags (ADR-030):** Menghapus `--hybrid`, menyediakan flag `--native` (paksa kompilasi source) dan `--binhost` (aktifkan 3-tier cascade) di `crates/forge/src/main.rs`.
- [x] **Unit Tests:** `test_cachyos_tier_auto_detection`, `test_core_os_blacklist_protection`, `test_cascade_prefers_forge_binhost`, `test_cascade_fallback_to_cachyos`, `test_cascade_blocks_core_os_from_cachyos`, `test_cascade_fallback_to_source` lulus 100%.

---

### Fase 10: Server Suite & CI/CD Builder (`forge-server`) & Client Sync Engine
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi CLI suite `forge-server` di [`crates/forge-server/src/main.rs`](file:///home/admin/Development/Forge/crates/forge-server/src/main.rs).
- [x] CLI CI/CD Lock-CPU Builder: `forge-server import <pkg> --target-cpu <cpu>`.
- [x] CLI Catalog Indexer: `forge-server index --storage-path <path>`.
- [x] **HTTP REST API Daemon (`crates/forge-server/src/server.rs`):** Router `axum` & `tokio` melayani `/v1/health`, `/v1/recipes/latest.sha256`, dan streaming `/v1/recipes/latest.tar.zst`.
- [x] **Recipe Bundler Engine (`ForgeServer::bundle_recipes`):** Mengompresi direktori recipes menjadi tarball Zstandard deterministik (`recipes.tar.zst`) dan mencatat checksum SHA256 (`recipes.tar.zst.sha256`).
- [x] **Client Sync Engine (`crates/forge/src/sync.rs`):** Modul `SyncClient::sync_recipes` dengan handshake SHA256, deteksi no-op jika up-to-date, streaming download, validasi kriptografis, ekstraksi atomik Zstandard, dan pembaruan direktori resep resmi `/var/db/forge/recipes/`.
- [x] **CLI Subcommand `forge sync` & Flag `--server`:** Terintegrasi di `crates/forge/src/main.rs`.
- [x] **Unit & Integration Tests:** `test_bundle_recipes_and_hash_generation`, `test_server_health_and_endpoints`, `test_sync_recipes_client_full_cycle`, `test_sync_noop_when_up_to_date` lulus 100%.

---

### Fase 11: Seed Toolchain, Meta-Packages & Stage Exporter
**Status:** 🟡 **TOOLCHAIN & META-PACKAGES SELESAI / STAGE EXPORTER PENDING**
- [x] **Pure Source Seed Toolchain Bundler (ADR-019):** Implementasi `forge toolchain bundle` di [`crates/forge/src/toolchain.rs`](file:///home/admin/Development/Forge/crates/forge/src/toolchain.rs) yang mengemas HANYA biner/library hasil kompilasi murni dari `/tmp/forge/stage/` menjadi `dist/kura-toolchain.tar.xz` tanpa menyalin file host.
- [x] **Resep Meta-Paket Resmi Kura Linux (ADR-026):** Resep All-in-One `recipes/system/base/recipe.toml` (Base OS) dan `recipes/system/base-devel/recipe.toml` (Toolchain).
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
8. **ADR-008 (Meta-Target & `stage-export`):** Dukungan bawaan untuk kompilasi meta-paket distro Kura Linux dan pembuatan tarball distribusi `kura-stage.tar.xz`.
9. **ADR-009 (Protokol Mutlak HITL & Pengujian Terisolasi):** AI pengembang Forge wajib mengikuti protokol 5 langkah (*Plan $\rightarrow$ Chat $\rightarrow$ ACC $\rightarrow$ Eksekusi $\rightarrow$ Uji*) dan menguji perubahan di lingkungan terisolasi.
10. **ADR-010 (Hierarki Resolusi Source-First Kompilasi Native):** Forge mengutamakan kompilasi lokal langsung dari kode sumber sebagai prioritas utama (Gentoo Portage mode). Opsi *Forge Native Binhost* dan *Hybrid Fallback (CachyOS/Arch)* disediakan sebagai akselerasi opsional (*opt-in*) tanpa memaksakan biner kepada pengguna.
11. **ADR-011 (Pohon Resep Terpusat di Server & Sinkronisasi Klien):** Seluruh resep Forge di-hosting secara terpusat di server `forge-server` dan disinkronisasi ke klien secara efisien melalui perintah `forge sync`.
12. **ADR-012 (CI/CD Build Farm Locked to Target CPU Microarchitecture):** Server `forge-server` menyediakan worker CI/CD builder yang mengompilasi paket dengan target arsitektur CPU pengguna (misal: `znver4`) menggunakan tool otomasi `forge-server import` dan mempublikasikan biner `.forge.tar.zst` ke Binary Library.
13. **ADR-013 (Introspeksi Hardware & Profil CPU `forge cpu-dump`):** Forge menyediakan perintah `forge cpu-dump` untuk mengekstrak arsitektur CPU, ekstensi ISA, dan CFLAGS optimal menjadi `cpu-profile.json` untuk disinkronkan ke server/CI/CD.
14. **ADR-014 (Sistem USE Flags & Multi-Version Slotting ala Portage):** Mengadopsi mekanisme USE flags untuk kontrol fitur granular dan Slots untuk koeksistensi beberapa versi mayor paket secara berdampingan.
15. **ADR-015 (Pemisahan Binary Klien `forge` dan Server `forge-server`):** Memisahkan secara tegas antarmuka dan paket eksekusi antara aplikasi klien pengguna (`forge`) dan backend suite/CI-CD builder (`forge-server`) demi menjaga footprint klien tetap ringan dan terfokus.
16. **ADR-016 (Konfigurasi Terpusat Single Source of Truth):** Menggunakan `/etc/forge/forge.conf` sebagai satu-satunya konfigurasi package manager tanpa fragmentasi file konfigurasi OS.
17. **ADR-017 (Implementasi Bahasa Rust & Pipeline Kompilasi Ultra-Cepat):** Forge diimplementasikan murni menggunakan bahasa pemrograman Rust dalam Cargo Workspace multi-crate, ditenagai backend LLVM 22, ultra-fast linker `mold` (`-fuse-ld=mold`), optimasi Link-Time Optimization (Thin/Full LTO), `panic = "abort"`, serta dukungan PGO untuk mencapai throughput eksekusi maksimal.
18. **ADR-018 (Isolated Seed Toolchain & Sysroot Packaging):** Untuk memutus ketergantungan dari toolchain host dan mencegah polusi lingkungan build, Forge menyediakan sub-sistem `forge toolchain bundle` yang mengemas biner hasil kompilasi ke dalam arsip `kura-toolchain.tar.xz` berstruktur UsrMerge standar untuk diekstrak langsung ke dalam sysroot stage Kura Linux.
19. **ADR-019 (Penegakan Mutlak Pure Source-Built & Zero Host Harvesting):** Dilarang keras menyalin atau memanen (*harvest*) biner, library, atau compiler dari sistem host (`/usr/bin/`, `/usr/lib/llvm/22/`) untuk dimasukkan ke dalam paket distribusi atau seed toolchain. Seluruh isi tarball toolchain dan sistem Kura Linux wajib 100% murni dikompilasi dari kode sumber upstream melalui resep `recipe.toml` di direktori staging Forge (`/tmp/forge/stage/`).
20. **ADR-020 (Format Resep All-in-One `recipe.toml` & Hierarki Supremasi Compiler):** Menggabungkan metadata paket deklaratif dan embedded POSIX bash script ke dalam satu file tunggal `recipe.toml`. Engine `forge-core` mengeksekusi script dengan menginjeksi environment variable compiler wajib (`CC=clang`, `LD=mold`, `-march=native`, `-O3`, `-flto=thin`) secara mutlak sebelum script subshell dieksekusi, kecuali ada flag pengecualian (seperti Glibc).
21. **ADR-021 (Suite Toolchain Hulu Terbaru 2026):** Menstandarkan versi paket toolchain inti sistem Kura Linux ke versi rilis modern hulu: LLVM/Clang 22.1.x, mold 2.42.1, Ninja 1.13.2, Pkgconf 3.0.7, GNU Make 4.4.1, Glibc 2.44, GCC 15.3.0, Binutils 2.44, Linux-headers 6.13.x, dan OpenRC 0.56.
22. **ADR-022 (Topological DAG Dependency Resolution & Cycle Detection Engine):** Seluruh proses build dan instalasi paket wajib melewati resolusi DAG yang memisahkan `depends` (runtime) dan `makedepends` (build-time), mengevaluasi kondisi USE flags, serta mendeteksi siklus dependensi sirkular secara deterministik sebelum kompilasi dimulai.
23. **ADR-023 (Transactional Atomic Merger, Collision Detector & Flat-File Manifest Database):** Setiap paket yang berhasil dikompilasi ke staging `$DESTDIR` wajib melalui pemindaian tabrakan berkas (*pre-flight collision scan*) sebelum digabungkan secara atomik ke rootfs target `$FORGE_ROOT` dan dicatat ke `/var/db/forge/installed/<pkg>/manifest`.
24. **ADR-024 (Config-Protected Unmerge Cleaner & Reverse Directory Pruning):** Penghapusan paket dilakukan secara presisi dari leaf files ke root, menghapus folder kosong tanpa merusak direktori bersama sistem, serta melindungi berkas konfigurasi `/etc/` yang telah dimodifikasi oleh pengguna (`CONFIG_PROTECT`).
25. **ADR-025 (Integrated Compiler Acceleration with Ccache 4.13.5 & Memory tmpfs Isolation):** Engine kompilasi Forge secara otomatis mengintegrasikan akselerasi compiler cache `ccache` pada `CC` dan `CXX`, serta mengisolasi build directory di RAM `tmpfs` untuk memaksimalkan kecepatan I/O dan efisiensi siklus rebuild.
26. **ADR-026 (Paradigma Meta-Paket Murni & Eliminasi Hardcoded @system / system-setup):** Menghapus total target magis `@system` yang di-hardcode di kode biner dan menghapus wizard `system-setup`. Basis sistem Kura Linux didefinisikan murni sebagai resep meta-paket deklaratif (`base` dan `base-devel`) mengadopsi filosofi *Everything is a Package* ala Arch Linux/Alpine/Void.
27. **ADR-027 (Pipeline Bootstrap 2-Tahap & Resolusi Paradoks Ayam-Telur):** Memecahkan masalah bootstrapping OS baru (The Chicken-and-Egg Problem) melalui 2 tahap: Tahap 1 mengompilasi resep toolchain di host menjadi Seed Toolchain `dist/kura-toolchain.tar.xz`, dan Tahap 2 mengekstrak seed tersebut ke dalam chroot `/mnt/kura/` untuk kemudian menjalankan `forge install base` dan `forge install base-devel` secara self-hosted.
28. **ADR-028 (Sistem Resep Terdedikasi `/var/db/forge/recipes/`, Protokol `forge sync`, & Kemandirian Chroot Toolchain):** Menghapus ketergantungan pada direktori kerja lokal pengembang (`./recipes/`) dan menstandarkan pohon resep sistem resmi di `/var/db/forge/recipes/`. Protokol `forge sync` menyinkronkan tarball resep dari `forge-server` secara atomik. Perintah `forge toolchain bundle` otomatis menyertakan seluruh `/var/db/forge/recipes/`, `/etc/forge/forge.conf`, dan `/usr/bin/forge` ke dalam `dist/kura-toolchain.tar.xz`, memastikan lingkungan chroot `/mnt/kura/` 100% mandiri (*self-contained*) untuk langsung mengompilasi `base` dan `base-devel` tanpa ketergantungan eksternal.
29. **ADR-029 (Disiplin Git Commit Berkala & Pembaruan Kontinu Dokumentasi Markdown):** Menegakkan kewajiban bahwa setiap tahapan kerja dan pembaruan arsitektur yang telah disetujui User wajib langsung dicatat ke Git dengan Conventional Commits, serta seluruh berkas dokumentasi Markdown (`ARCHITECTURE.md`, `MEMORY.md`, `README.md`, `AGENTS.md`) wajib diperbarui secara terus-menerus (*continuous persistent sync*) agar selalu mencerminkan kondisi arsitektur riil.
30. **ADR-030 (Penyederhanaan CLI & 3-Tier Package Cascade Resolution):** Menghapus total flag kaku `--hybrid` dan menyederhanakan UX menjadi 2 opsi intuitif: `--native` (Source-First Portage Mode) dan `--binhost` (3-Tier Cascade: Forge Binhost -> CachyOS Zen4/v4/v3 -> Native Source Fallback).
31. **ADR-031 (Garansi Anti-Brick & Core OS Blacklist Protection):** Menjamin integritas sistem Kura Linux dengan melarang keras paket fondasi dan toolchain OS inti (`glibc`, `openrc`, `gcc`, `llvm`, `mold`, `eudev`, `kmod`, `shadow`, `util-linux`, `base`, `base-devel`, `forge`, `systemd`) diambil dari repo biner luar (CachyOS/Arch), dan mewajibkan fallback kompilasi dari kode sumber resmi Kura Linux.

---

## 3. Log Masalah & Solusi (Troubleshooting)

| Tanggal | Komponen | Masalah | Solusi |
| :--- | :--- | :--- | :--- |
| *2026-09-18* | *Inisialisasi* | *Kebutuhan arsitektur dasar Forge terpisah dari KuraLinux* | *Membangun blueprint awal Forge mengadopsi kebutuhan dari IDEA_FOR_FORGE.md* |
| *2026-09-18* | *Arsitektur* | *Kompilasi source lokal berat di mesin pengguna; butuh opsi biner native & server CI/CD* | *Mengembangkan ekosistem Server & CI/CD Builder Lock-CPU, perintah `cpu-dump` & `import`, serta arsitektur Hybrid Unified* |
| *2026-09-18* | *Filosofi* | *Prioritas default sempat condong ke binhost; komponen server tercampur dengan klien* | *Mengoreksi prioritas menjadi Source-First (Gentoo-style) dan memisahkan binary klien `forge` dengan server suite `forge-server`* |
| *2026-09-18* | *Isolasi Host* | *Ketergantungan terhadap compiler host saat bootstrap awal Kura Linux* | *Mengembangkan sub-perintah `forge toolchain bundle` dan menghasilkan seed toolchain `dist/kura-toolchain.tar.xz` untuk diekstrak ke sysroot stage Kura Linux* |
| *2026-09-18* | *Standarisasi Resep* | *Format terpisah bash + toml rentan fragmentasi; butuh format efisien & hierarki compiler kuat* | *Menerapkan format All-in-One `recipe.toml` berbasis Serde TOML dengan subshell environment injection mutlak* |
| *2026-09-18* | *Integritas Build* | *Haram mutlak mengambil biner dari host filesystem* | *Menegakkan aturan Pure Source-Built (ADR-019): toolchain bundler hanya mengemas biner yang sah terkompilasi dari source code oleh Forge di staging `/tmp/forge/stage/`* |
| *2026-09-18* | *Penyederhanaan Crate* | *Layout 6 crate berlebihan dan membingungkan* | *Mengkonsolidasikan workspace menjadi Clean 2-Crate Layout (`crates/forge` dan `crates/forge-server`)* |
| *2026-09-18* | *Akselerasi Rebuild* | *Kompilasi ulang source code berulang memakan waktu lama* | *Mengintegrasikan Ccache (v4.13.5) secara otomatis pada pipeline builder Forge (ADR-025)* |
| *2026-09-18* | *Penyederhanaan Desain* | *Wizard system-setup dan @system hardcoded kaku & melanggar prinsip UNIX* | *Menghapus system-setup & @system hardcoded, beralih ke paradigma meta-paket deklaratif `base` dan `base-devel` (ADR-026)* |
| *2026-09-18* | *Paradoks Bootstrap* | *Bagaimana chroot bisa build base-devel jika belum punya compiler bawaan* | *Merumuskan 2-Stage Bootstrapping Pipeline: Seed Toolchain diekstrak ke chroot, baru mengeksekusi `forge install base-devel` (ADR-027)* |
| *2026-09-18* | *Optimasi Ekstrem* | *Flag kompilasi belum memaksimalkan seluruh fitur AMD Zen 4 silikon* | *Menerapkan flag 'Mentok Ekstrem' Zen 4: AVX-512 ZMM, Thin LTO, Mold ICF `--icf=all`, Dead-strip `--gc-sections`, `-fno-math-errno`, & 32-byte function alignment* |
| *2026-09-18* | *Kemandirian Chroot* | *Chroot butuh akses ke resep tanpa mount repo git lokal host* | *Menstandarkan `/var/db/forge/recipes/`, merancang protokol `forge sync`, dan membundel seluruh resep ke `kura-toolchain.tar.xz` (ADR-028)* |
| *2026-09-18* | *Workflow & Docs* | *Perlunya disiplin Git commit berkala & pembaruan berkas MD berkelanjutan* | *Menetapkan aturan wajib Git commit dan sinkronisasi persisten berkas MD pada setiap tahapan (ADR-029)* |
| *2026-09-19* | *Cascade & Anti-Brick* | *Paket biner luar (Arch/CachyOS) berisiko menimpa glibc/init system Kura Linux* | *Menerapkan 3-Tier Cascade Resolution Engine (`cascade.rs`) & Core OS Blacklist Anti-Brick Protection (`cachyos.rs`) (ADR-030, ADR-031)* |

---

## 3. Log Masalah & Solusi (Troubleshooting)

| Tanggal | Komponen | Masalah | Solusi |
| :--- | :--- | :--- | :--- |
| *2026-09-18* | *Inisialisasi* | *Kebutuhan arsitektur dasar Forge terpisah dari KuraLinux* | *Membangun blueprint awal Forge mengadopsi kebutuhan dari IDEA_FOR_FORGE.md* |
| *2026-09-18* | *Arsitektur* | *Kompilasi source lokal berat di mesin pengguna; butuh opsi biner native & server CI/CD* | *Mengembangkan ekosistem Server & CI/CD Builder Lock-CPU, perintah `cpu-dump` & `import`, serta arsitektur Hybrid Unified* |
| *2026-09-18* | *Filosofi* | *Prioritas default sempat condong ke binhost; komponen server tercampur dengan klien* | *Mengoreksi prioritas menjadi Source-First (Gentoo-style) dan memisahkan binary klien `forge` dengan server suite `forge-server`* |
| *2026-09-18* | *Isolasi Host* | *Ketergantungan terhadap compiler host saat bootstrap awal Kura Linux* | *Mengembangkan sub-perintah `forge toolchain bundle` dan menghasilkan seed toolchain `dist/kura-toolchain.tar.xz` untuk diekstrak ke sysroot stage Kura Linux* |
| *2026-09-18* | *Standarisasi Resep* | *Format terpisah bash + toml rentan fragmentasi; butuh format efisien & hierarki compiler kuat* | *Menerapkan format All-in-One `recipe.toml` berbasis Serde TOML dengan subshell environment injection mutlak* |
| *2026-09-18* | *Integritas Build* | *Haram mutlak mengambil biner dari host filesystem* | *Menegakkan aturan Pure Source-Built (ADR-019): toolchain bundler hanya mengemas biner yang sah terkompilasi dari source code oleh Forge di staging `/tmp/forge/stage/`* |
| *2026-09-18* | *Penyederhanaan Crate* | *Layout 6 crate berlebihan dan membingungkan* | *Mengkonsolidasikan workspace menjadi Clean 2-Crate Layout (`crates/forge` dan `crates/forge-server`)* |
| *2026-09-18* | *Akselerasi Rebuild* | *Kompilasi ulang source code berulang memakan waktu lama* | *Mengintegrasikan Ccache (v4.13.5) secara otomatis pada pipeline builder Forge (ADR-025)* |
| *2026-09-18* | *Penyederhanaan Desain* | *Wizard system-setup dan @system hardcoded kaku & melanggar prinsip UNIX* | *Menghapus system-setup & @system hardcoded, beralih ke paradigma meta-paket deklaratif `base` dan `base-devel` (ADR-026)* |
| *2026-09-18* | *Paradoks Bootstrap* | *Bagaimana chroot bisa build base-devel jika belum punya compiler bawaan* | *Merumuskan 2-Stage Bootstrapping Pipeline: Seed Toolchain diekstrak ke chroot, baru mengeksekusi `forge install base-devel` (ADR-027)* |
| *2026-09-18* | *Optimasi Ekstrem* | *Flag kompilasi belum memaksimalkan seluruh fitur AMD Zen 4 silikon* | *Menerapkan flag 'Mentok Ekstrem' Zen 4: AVX-512 ZMM, Thin LTO, Mold ICF `--icf=all`, Dead-strip `--gc-sections`, `-fno-math-errno`, & 32-byte function alignment* |
| *2026-09-18* | *Kemandirian Chroot* | *Chroot butuh akses ke resep tanpa mount repo git lokal host* | *Menstandarkan `/var/db/forge/recipes/`, merancang protokol `forge sync`, dan membundel seluruh resep ke `kura-toolchain.tar.xz` (ADR-028)* |
| *2026-09-18* | *Workflow & Docs* | *Perlunya disiplin Git commit berkala & pembaruan berkas MD berkelanjutan* | *Menetapkan aturan wajib Git commit dan sinkronisasi persisten berkas MD pada setiap tahapan (ADR-029)* |

---

## 4. Panduan Serah Terima AI Agent (Incoming AI Agent Handover Guide)

> **Catatan Penting untuk AI Agent Penerus:**
> Repositori ini telah dikonsolidasi secara rapi menjadi **Clean 2-Crate Workspace Layout** dengan paradigma **Meta-Paket Murni ("Everything is a Package")**, optimasi compiler **Mentok Ekstrem (Zen 4 AVX-512 / Thin LTO / Mold ICF)**, repositori resep terstandarisasi **`/var/db/forge/recipes/` (ADR-028)**, Client Sync Engine (`sync.rs`), Forge Server HTTP Daemon (`server.rs`), DAG Dependency Resolver (`resolver.rs`), Transactional Merger (`merger.rs`), and Manifest Database Engine (`db.rs`) dengan tingkat kesiapan **~90%**. Seluruh blueprint arsitektur, diagram, aturan mutlak, dan 28 ADR telah didokumentasikan secara lengkap.

### 📌 Ringkasan Status & State Workspace:
- **Workspace:** 2 Crate murni: [`crates/forge`](file:///home/admin/Development/Forge/crates/forge) (Klien & Engine Library) dan [`crates/forge-server`](file:///home/admin/Development/Forge/crates/forge-server) (Server & CI/CD Builder).
- **Toolchain & Compiler Flags (Mentok Ekstrem):** Rust 1.97.1, LLVM/Clang 22, Linker `mold`, Ccache 4.13.5, Thin LTO, `-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2` dan LDFLAGS `-Wl,-O3 -Wl,--as-needed -Wl,--gc-sections -Wl,--icf=all -Wl,-z,relro -Wl,-z,now -fuse-ld=mold`.
- **Seed Toolchain:** Staged murni di `/tmp/forge/stage/`, membundel `/var/db/forge/recipes/`, `/etc/forge/forge.conf`, dan `/usr/bin/forge` ke `dist/kura-toolchain.tar.xz` (ADR-019, ADR-028).
- **Meta-Paket Distro:** `recipes/system/base/recipe.toml` (Base OS) dan `recipes/system/base-devel/recipe.toml` (Toolchain).

### 🛑 6 Aturan Mutlak yang Wajib Diikuti:
1. **HITL (Human-In-The-Loop):** Wajib ikuti siklus 5-langkah (*Plan $\rightarrow$ Chat $\rightarrow$ ACC $\rightarrow$ Eksekusi $\rightarrow$ Uji*). Jangan edit/buat file tanpa ACC di chat.
2. **HARAM EDIT KURALINUX:** Dilarang keras menyentuh direktori `/home/admin/Development/KuraLinux/`.
3. **HARAM AMBIL DARI HOST (ADR-019):** Biner dan toolchain wajib 100% dikompilasi dari source code upstream via resep ke `/tmp/forge/stage/`. Dilarang meng-copy biner dari `/usr/bin/` atau `/usr/lib/`.
4. **GLIBC EXEMPTION (ADR-002):** Paket Glibc di-build dengan GCC standar tanpa flag `-march` kustom demi stabilitas.
5. **CCACHE ACCELERATION (ADR-025):** Kompilasi memanfaatkan Ccache 4.13.5 pada build engine.
6. **OPENRC ONLY:** Tidak boleh ada ketergantungan pada Systemd.

### 🎯 Tugas Prioritas Pengembangan Selanjutnya (Sisa 10%):
1. **Distro Stage Exporter (`forge stage-export`):**
   - Implementasikan tarball bundler `forge stage-export --output kura-stage.tar.xz` untuk mengemas rootfs aktif menjadi stage distribusi Kura Linux.
2. **Hybrid Streaming Downloader:**
   - Live network streaming download & dekompresi zstd untuk biner binhost/CachyOS.
