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
- [x] Konfigurasi standar paten tunggal terpusat `config/forge.conf.default` (ADR-060).
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
- [x] **Sandbox Build Isolation Engine (`crates/forge/src/sandbox.rs`):** Auto-detection Bubblewrap (`bwrap`) dengan isolasi mutlak: `--ro-bind / /` (Host filesystem 100% Read-Only), `--bind <build_dir>`, `--bind <destdir>`, `--bind /tmp /tmp`, `--proc /proc`, `--dev /dev`, `--unshare-all`, `--die-with-parent`, serta graceful fallback mode untuk host tanpa `bwrap`. Terintegrasi pada `RecipeBuilder::build`.
- [x] Injeksi otomatis compiler flags Kura Linux (`-O3 -march=native -pipe -flto=thin -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt`) dan linker `mold` (`-fuse-ld=mold`).
- [x] **Akselerasi Ccache (v4.13.5):** Auto-detection binary `ccache`, resolusi direktori `CCACHE_DIR` (`/var/cache/forge/ccache` / `distfiles/.ccache`), dan injeksi `CC="ccache clang"`, `CXX="ccache clang++"`.
- [x] **Pengecualian Glibc (ADR-002):** Otomatis dialihkan ke GCC bawaan (`CC="ccache gcc"`) tanpa flag `-march` kustom demi kestabilan build system Glibc.
- [x] Staging hasil kompilasi ke direktori terisolasi `DESTDIR` (`/tmp/forge/stage/<pkg>/`).
- [x] Unit test `test_sandbox_command_construction` dan `test_sandbox_fallback_execution` lulus 100%.

---

### Fase 7: Transactional Merger & Global Concurrency Guard
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Global Concurrency Guard (`crates/forge/src/lock.rs`):** File locking RAII (`fs2` / `flock(2)`) pada `/var/lock/forge.lock` (fallback otomatis ke `$TEMP_DIR/forge.lock` untuk non-root), pencatatan PID proses aktif, penanganan error informatif ("Proses forge lain sedang berjalan..."), dan proteksi mutatif pada `install`, `remove`, `sync`, dan `update`.
- [x] **Desain Arsitektur:** Blueprint spesifikasi Pre-flight Collision Scanning, Atomic Merge pipeline, dan Rollback Log di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] **Engine Merger (`crates/forge/src/merger.rs`):**
  - Implementasi `MergeTransaction`, `StagedEntry`, `CollisionReport`, `JournalAction`.
  - Pemindaian tabrakan pra-instalasi (`preflight_scan`) terhadap `/var/db/forge/installed/`.
  - Penulisan berkas atomik (`tempfile` $\rightarrow$ `fsync` $\rightarrow$ `rename`), preservasi hak akses Unix (`chmod`), symlink, dan timestamps.
  - Pencatatan jurnal transaksi `txn_<id>.journal` dengan auto-rollback LIFO jika terjadi kegagalan I/O.
  - Proteksi konfigurasi `CONFIG_PROTECT`: penyimpanan `._cfg0000_<file>` untuk berkas `/etc/` yang termodifikasi.
  - Eksekusi post-merge hooks (deteksi OpenRC `/etc/init.d/`, `ldconfig`).
- [x] Unit test `test_acquire_and_release_lock`, `test_contended_lock_rejection`, `test_preflight_collision_detector`, `test_atomic_merge_and_permissions`, `test_config_protect_mechanism`, `test_transactional_rollback_on_failure` lulus 100%.

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

### Fase 9: 3-Tier Package Cascade Resolution & Recursive CachyOS ALPM Engine
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi data model binhost & client matcher di [`crates/forge/src/binhost.rs`](file:///home/admin/Development/Forge/crates/forge/src/binhost.rs).
- [x] **Anti-Brick CachyOS Adapter & ALPM Parser (`crates/forge/src/cachyos.rs`):**
  - Auto-detection mikroarsitektur CPU (`Znver4`, `X86_64_V4`, `X86_64_V3`, `Generic`).
  - Core OS Blacklist Protection mutlak (`glibc`, `openrc`, `gcc`, `llvm`, `mold`, `eudev`, `kmod`, `shadow`, `util-linux`, `base`, `base-devel`, `forge`, `systemd`).
  - ALPM Database Parser: `parse_alpm_desc` dan `parse_repo_db_tar_zst` untuk mengekstrak `%NAME%`, `%VERSION%`, `%DESC%`, `%DEPENDS%`, `%PROVIDES%`, `%FILENAME%`, `%CSIZE%`, `%ISIZE%`, `%SHA256SUM%`.
  - Penelusuran graf dependensi rekursif (`resolve_dependencies_recursive`) menghasilkan topological order dan mengeliminasi paket yang diblacklist dengan peringatan Anti-Brick.
- [x] **3-Tier Cascade Resolution Engine (`crates/forge/src/cascade.rs`):** Resolusi deterministik: Tingkat 1 (Forge Native Binhost `.forge.tar.zst`) -> Tingkat 2 (CachyOS Prebuilt dengan bypass blacklist) -> Tingkat 3 (Source-First Native Compilation).
- [x] **Penyederhanaan CLI Flags (ADR-030):** Menghapus `--hybrid`, menyediakan flag `--native` (paksa kompilasi source) dan `--binhost` (aktifkan 3-tier cascade) di `crates/forge/src/main.rs`.
- [x] **Unit Tests:** `test_cachyos_tier_auto_detection`, `test_core_os_blacklist_protection`, `test_parse_cachyos_desc_format`, `test_cachyos_recursive_dependency_chain`, `test_cachyos_recursive_respects_blacklist`, `test_parse_repo_db_tar_zst`, `test_cascade_prefers_forge_binhost`, `test_cascade_fallback_to_cachyos`, `test_cascade_blocks_core_os_from_cachyos`, `test_cascade_fallback_to_source` lulus 100%.

---

### Fase 10: Server Suite & CI/CD Builder (`forge-server`) & Client Sync Engine
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] Implementasi CLI suite `forge-server` di [`crates/forge-server/src/main.rs`](crates/forge-server/src/main.rs).
- [x] **Pemisahan Perintah Build & Import & Ergonomis CPU Profile Storage (ADR-036, ADR-037):**
  - **Klien `forge`:** Sub-perintah `forge build <pkg> [--output-dir <DIR>]` untuk kompilasi lokal ke `.forge.tar.zst` tanpa mengotori host rootfs `/`, `forge install <pkg>` untuk kompilasi + transactional atomic merge, `forge cpu-dump` untuk mengekstrak profil silikon CPU, dan `forge recipe-import <url|file>` untuk transpilasi resep upstream.
  - **Server `forge-server`:**
    * `forge-server import <cpu-profile.json> [--profiles-dir <PATH>] [--as <NAME>]`: Ingestion profil CPU target silikon ke `/var/db/forge/profiles/<march>.json` dan mengesetnya sebagai profil aktif (`active.json`).
    * `forge-server build <PACKAGE> [--profile <PATH>] [--recipes-path <PATH>] [--output-dir <PATH>] [--binhost-path <PATH>] [--no-publish]`: CI/CD Worker otomatis menggunakan profil CPU aktif yang tersimpan (atau custom override), mengompilasi resep di staging sandbox terisolasi, mengemas `.forge.tar.zst`, dan otomatis mempublikasikannya ke `/var/db/forge/binhost/<march>/` & memperbarui `catalog.json`.
    * `forge-server list-profiles [--profiles-dir <PATH>]`: Menampilkan daftar seluruh profil CPU yang tersimpan dan menandai profil yang aktif.
    * `forge-server index`: Generator global `packages.db.zst`.
    * `forge-server serve`: API daemon (`axum`/`tokio`) & Web Explorer.
- [x] **Server Profile Manager (`crates/forge-server/src/profiles.rs`):** Modul `ServerProfileManager` (`import_profile`, `load_active_profile`, `list_profiles`, `resolve_profiles_dir`).
- [x] **GitOps Recipe Registry & GitHub Webhook Auto-Sync (ADR-038):**
  - Endpoint `POST /v1/webhook/github`: Menerima event push/merge dari GitHub.
  - Endpoint `POST /v1/recipes/refresh`: Manual on-demand sync & rebundle trigger.
  - Modul `sync_and_rebundle_recipes`: Otomatis menjalankan `git pull --rebase` jika recipes berupa repository Git, me-regenerasi `recipes.tar.zst`, memperbarui `latest.sha256`, dan me-refresh katalog in-memory Web Explorer.
- [x] **Unit & Integration Tests:** `test_server_import_cpu_profile_saves_active`, `test_server_build_uses_imported_active_profile`, `test_server_build_with_custom_profile_override`, `test_server_list_profiles`, `test_forge_client_build_produces_tarball_without_installing`, `test_forge_server_build_and_import_separation`, `test_forge_server_import_registers_to_catalog`, `test_bundle_recipes_and_hash_generation`, `test_server_health_and_endpoints`, `test_sync_recipes_client_full_cycle`, `test_sync_noop_when_up_to_date`, `test_github_webhook_endpoint_triggers_rebundle`, `test_recipes_refresh_endpoint` lulus 100%.

---

### Fase 11: Seed Toolchain, Meta-Packages & Stage Exporter
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Pure Source Seed Toolchain Bundler (ADR-019, ADR-028):** 
  - Implementasi `forge toolchain bundle` di [`crates/forge/src/toolchain.rs`](file:///home/admin/Development/Forge/crates/forge/src/toolchain.rs) yang mengemas HANYA biner/library hasil kompilasi murni dari `/tmp/forge/stage/` menjadi `dist/kura-toolchain.tar.xz`.
  - Deteksi lengkap sistem: compiler (`clang`, `gcc`), linker (`mold`), C runtime library (`glibc` / `libc.so.6`), dynamic linker (`ld-linux-x86-64.so.2`), kernel headers (`linux-headers`), binutils (`as`, `ar`), build automation (`make`, `ninja`, `pkgconf`).
  - Hierarki root UsrMerge otomatis (`/bin`, `/sbin`, `/lib`, `/lib64 -> usr/lib`) dan kerangka direktori sistem lengkap (`/tmp` 1777, `/var/db/forge/`, `/var/cache/forge/`, dll.).
  - Injeksi konfigurasi bawaan `/etc/forge/forge.conf` (menunjuk `https://pkgkura.amqs.net`) dan `/etc/forge/toolchain.conf` untuk kemandirian chroot instan.
- [x] **Resep Meta-Paket Resmi Kura Linux (ADR-026):** Resep All-in-One `recipes/system/base/recipe.toml` (Base OS) dan `recipes/system/base-devel/recipe.toml` (Toolchain dengan runtime Glibc eksplisit).
- [x] Desain integrasi OpenRC hook `/etc/init.d/` dan `rc-update`.
- [x] **Distro Stage Exporter Engine (`crates/forge/src/stage.rs` & CLI `forge stage-export`):**
  - Data model `StageExportOptions`, `StageFormat` (Xz, Zstd), dan `StageExportResult`.
  - `StageExporter::validate_rootfs`: Validasi FHS dasar (`/usr`, `/etc`, `/var`), UsrMerge (`/bin`, `/sbin`, `/lib` symlink ke `usr/bin` / `usr/lib`), dan OpenRC tree (`/etc/init.d/`, `/etc/runlevels/`).
  - `StageExporter::sanitize_staging`: Pembersihan cache sementara (`/tmp/*`, `/var/cache/*`, `/var/log/*`, `/var/lock/*`, file transien `.tmp`/`.journal`/`.lock`) dengan proteksi mutlak direktori penting (`/var/db/forge/installed/`, `/etc/forge/`, `/etc/init.d/`).
  - `StageExporter::export`: Pembuatan tarball deterministik (`append_tree_to_tar`) dengan preservasi Unix permissions dan symlink, kompresi Zstandard level 19 dan multi-threaded XZ, serta generasi checksum kriptografis SHA256 (`.sha256`) dan BLAKE3 (`.b3sum`).
  - Integrasi CLI sub-perintah `forge stage-export` dengan flag `--root`, `--output`, `--format`, `--no-verify`, dan `--no-clean`.
  - 8 Unit tests: `test_stage_exporter_validates_usrmerge`, `test_stage_exporter_validates_openrc`, `test_stage_exporter_sanitizes_temporary_files`, `test_stage_exporter_full_export_tarball_and_checksums`, `test_stage_exporter_xz_format`, `test_toolchain_status_detection_including_glibc`, `test_bundle_seed_toolchain_usrmerge_structure_and_config`, `test_bundle_seed_toolchain_fails_when_empty_stage` lulus 100%.

---

### Fase 12: Upstream Recipe Importer & Katalog Resep Resmi Kura Linux
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Recipe Importer Engine (`crates/forge/src/importer.rs`):**
  - Parsing PKGBUILD dan APKBUILD cerdas: ekstraksi metadata (`pkgname`, `pkgver`, `pkgrel`, `pkgdesc`, `url`, `license`, `depends`, `makedepends`, `source`, `sha256sums`).
  - Normalisasi dependensi: eliminasi batasan versi (`>=`, `<=`, `=`), translasi library soname (`libssl.so` $\rightarrow$ `openssl`, `libcurl.so` $\rightarrow$ `curl`, `libz.so` $\rightarrow$ `zlib`).
  - Ekstraksi fungsi bash (`prepare`, `build`, `package`) dan transpilisasi transaksional ke format Forge `script` (`$pkgdir` / `${pkgdir}` $\rightarrow$ `"${DESTDIR}"`, `$srcdir` $\rightarrow$ `"${srcdir}"`).
  - Serialisasi deterministik ke string `recipe.toml` yang valid dan terformat rapi.
- [x] **CLI Sub-perintah `forge recipe-import`:**
  - Mendukung file lokal maupun unduhan langsung dari URL hulu (`http://`, `https://`).
  - Opsi penyimpanan `--output <PATH>`.
- [x] **Katalog Resep Resmi Kura Linux (`recipes/`):**
  - Seluruh daftar matriks, versi upstream, dependensi, dan status kesiapan paket dikonsolidasikan terpusat di [`recipes/PACKAGE_STATUS.md`](file:///home/admin/Development/Forge/recipes/PACKAGE_STATUS.md) (Single Source of Truth).
  - Terdiri dari 3 pilar: `recipes/system/` (fondasi OS & toolchain), `recipes/core/` (utilitas inti, storage, networking & daemons), dan `recipes/extra/` (dev tools, modern CLI, desktop apps, graphics/font, audio, Qt6, SDDM, KF6, dan KDE Plasma 6 Desktop).
- [x] **Unit & Integration Tests:** `test_parse_pkgbuild_metadata`, `test_transpile_build_package_steps`, `test_dependency_normalization`, `test_validate_all_recipes_in_repo_are_valid_toml`, `test_scan_actual_workspace_recipes` (100% lulus memindai seluruh resep riil).

---

### Fase 13: GitOps Recipe Registry, Real-Time Webhook & Domain Deployment (`https://pkgkura.amqs.net`)
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED & DEPLOYED)**
- [x] **GitOps Webhook Engine (`crates/forge-server/src/server.rs`):**
  - Endpoint `POST /v1/webhook/github`: Menerima event push/merge dari GitHub repo `inimuqsith/Forge`.
  - Endpoint `GET /v1/webhook/github`: Browser-friendly readiness status JSON response.
  - Endpoint `POST /v1/recipes/refresh`: Manual on-demand sync & rebundle trigger.
  - Modul `sync_and_rebundle_recipes`: Otomatis menjalankan `git pull --rebase`, me-regenerasi `recipes.tar.zst`, memperbarui `latest.sha256`, dan me-refresh katalog in-memory Web Explorer secara instan.
- [x] **Domain Publik & Deployment VPS:**
  - Layanan live di domain resmi `https://pkgkura.amqs.net` via reverse proxy Nginx + SSL.
  - Webhook GitHub ID `681556356` terverifikasi aktif (HTTP 200 delivery).
  - Target CPU server di-build dengan `RUSTFLAGS="-C target-cpu=x86-64-v3"` untuk kompatibilitas CPU AMD EPYC VPS.

---

### Fase 14: Server-Side Upstream Recipe Bumper, Zero-Quota Audit Engine & GitHub SSOT GitOps Automation
**Status:** ✅ **SELESAI & TERUJI (100% IMPLEMENTED)**
- [x] **Upstream Recipe Bumper & Audit Engine (`crates/forge-server/src/bumper.rs`):**
  - Multi-Tier Upstream Probing (Bebas Rate-Limit):
    * Tier 1: GitHub REST API dengan injeksi header `Authorization: Bearer <GITHUB_TOKEN>` jika tersedia.
    * Tier 2: GitHub Atom Feed (`https://github.com/{owner}/{repo}/releases.atom`) untuk parsing rilis terbaru bebas rate-limit (zero quota).
    * Tier 3: Anitya / Release-Monitoring.org v2 Projects API (`https://release-monitoring.org/api/v2/projects/?name={pkg}`) untuk repositori non-GitHub (GNU, kernel.org, Sourceforge, dll.).
  - `RecipeBumper::audit_all`: Pemindaian paralel asinkron (Tokio tasks) mengaudit seluruh resep paket dalam ~3 detik dengan format status tabel CLI (`[LATEST]`, `[UPDATE]`, `[UNK]`).
  - `RecipeBumper::bump_recipe_file`: Manipulasi atomik berkas `recipe.toml` (update `version`, reset `release = 1`, unduh tarball & hitung hash SHA256 baru secara otomatis).
  - `RecipeBumper::bump_package_by_name`: Targeted bumping per-paket atau `--all`.
- [x] **GitHub Single Source of Truth (SSOT) Auto-Push:**
  - Perintah `forge-server bump <pkg|--all>` otomatis melakukan `git add recipes/`, `git commit -m "chore(recipes): ..."` dan `git push origin main` ke GitHub SSOT.
  - Begitu push masuk ke GitHub, GitHub Webhook otomatis menyengat `forge-server` di VPS $\rightarrow$ auto-rebundle $\rightarrow$ `forge sync` langsung mendapatkan versi terbaru!
- [x] **GitHub Actions Auto-Updater Bot (`.github/workflows/recipe-auto-updater.yml`):**
  - Berjalan otomatis setiap 6 jam via Cron (`0 */6 * * *`) atau manual via `workflow_dispatch`.
  - Mengompilasi `forge-server`, menjalankan `audit`, menjalankan `bump --all`, dan auto-commit ke cabang `main`.
- [x] **Unit Tests:** 24/24 unit test `forge-server` (termasuk `test_extract_tag_from_atom_feed`, `test_version_tag_cleaning`, `test_version_comparison`, `test_bump_recipe_toml_manipulation`, `test_github_webhook_endpoint_triggers_rebundle`) lulus 100%.

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
32. **ADR-032 (Upstream Recipe Importer & Otomasi Katalog Resep Kura Linux):** Forge menyediakan engine `RecipeImporter` (`importer.rs`) dan CLI `forge recipe-import` untuk mengonversi spesifikasi deklaratif upstream (Arch PKGBUILD / Alpine APKBUILD) secara deterministik ke dalam format `recipe.toml` standar Kura Linux dengan normalisasi dependensi dan transposisi direktori staging `$DESTDIR`.
33. **ADR-033 (Global Concurrency Lock RAII):** Menjamin seluruh operasi mutatif package manager (`install`, `remove`, `sync`, `update`) terlindungi oleh file lock RAII eksklusif (`/var/lock/forge.lock` dengan fallback ke `$TEMP_DIR/forge.lock`) dengan pencatatan PID aktif untuk mencegah race condition dan korupsi database paket.
34. **ADR-034 (Bubblewrap Sandbox Build Isolation):** Mengisolasi siklus eksekusi script build dengan memetakan filesystem host 100% Read-Only (`--ro-bind / /`), hanya mengizinkan penulisan pada direktori build RAM dan staging `$DESTDIR`, serta unshare namespace lengkap dengan graceful fallback mode jika `bwrap` belum terpasang.
35. **ADR-035 (ALPM DB Tarball Parser & Recursive Anti-Brick Resolver):** Mem-parsing arsip database repositori `.db.tar.zst` CachyOS dan berkas `desc` secara native, serta melakukan penelusuran graf dependensi rekursif (DFS Topological Sort) yang secara otomatis menolak dan memfilter paket Core OS yang masuk dalam blacklist demi stabilitas Kura Linux.
36. **ADR-036 (Pemisahan Tanggung Jawab Command Build & Import):** Memisahkan secara ketat siklus kompilasi (`build`) dan siklus ingestion/registrasi (`import`) di klien `forge` dan server `forge-server`. `forge build` hanya mengompilasi dan mengemas ke `.forge.tar.zst` tanpa instalasi ke rootfs `/`. `forge-server build` bertindak sebagai CI/CD Worker murni, sedangkan `forge-server import` bertindak sebagai Binary Ingester yang memverifikasi metadata, memindahkan tarball ke direktori binhost resmi `/var/db/forge/binhost/<march>/`, dan memperbarui `catalog.json`.
37. **ADR-037 (Ergonomis Penyimpanan Profil CPU & CI/CD Streamlined Build Server):** Menyederhanakan alur kerja server CI/CD dengan mengizinkan `forge-server import <cpu-profile.json>` menyimpan profil silikon CPU ke `/var/db/forge/profiles/<march>.json` dan mengesetnya sebagai profil aktif (`active.json`), sehingga perintah `forge-server build <PACKAGE>` dapat langsung dijalankan berulang-ulang tanpa perlu mengetikkan path JSON berkali-kali, mengompilasi di staging sandbox terisolasi, mengemas tarball biner, dan otomatis mempublikasikannya ke `/var/db/forge/binhost/<march>/` & `catalog.json` (dengan opsi override `--profile` dan `--no-publish`).
38. **ADR-038 (GitHub Webhook & Real-Time Auto-Rebundling GitOps):** Menghubungkan endpoint `POST /v1/webhook/github` pada `forge-server` dengan GitHub Repository (`inimuqsith/Forge`), sehingga setiap commit pada resep memicu `git pull --rebase` otomatis, rebundling `recipes.tar.zst`, update `latest.sha256`, dan pembaruan katalog tanpa intervensi manual.
39. **ADR-039 (Server-Side Multi-Tier Upstream Probing & GitHub SSOT Automated Bumping):** Menyediakan sub-perintah `forge-server audit` dan `forge-server bump <pkg|--all>` berbasis Multi-Tier Probing (GitHub REST API dengan Auth Token, GitHub Atom Feed `/releases.atom` bebas kuota, dan Anitya v2 Projects API) yang memperbarui resep dan langsung mem-push perubahan ke GitHub SSOT (`origin main`) dengan integrasi GitHub Actions Cron Bot 6-jam.
40. **ADR-040 (Penegakan Wajib Sandbox Bubblewrap & Pengecualian Self-Bootstrap `bubblewrap`):** Mewajibkan seluruh proses kompilasi kode sumber dijalankan di dalam isolasi Bubblewrap (`bwrap`) dengan pemetaan filesystem host 100% Read-Only (`--ro-bind / /`). Jika biner `bwrap` tidak ditemukan di sistem, proses kompilasi paket lain akan ditolak seketika (*hard fatal exit*) demi mencegah polusi host `/`. Pengecualian satu-satunya diberikan saat mengompilasi paket `bubblewrap` itu sendiri agar proses bootstrap mandiri (*self-bootstrap*) dapat berlangsung tanpa *deadlock*.
41. **ADR-041 (Live Network Streaming Downloader & End-to-End Transactional Installation Pipeline):** Mengintegrasikan modul streaming HTTP `reqwest` dan dekompresor Zstd on-the-fly dengan kalkulasi hash SHA256 & BLAKE3 simultan pada `BinhostClient::download_and_extract_stream` serta menghubungkan eksekusi `forge install` langsung ke `MergeTransaction` dan pencatatan manifest deterministik di `/var/db/forge/installed/`.
42. **ADR-042 (Pure Rust Ed25519 Digital Package Signing & Supply-Chain Verification):** Mengimplementasikan modul kriptografi digital signing murni dalam Rust (`ed25519-dalek`) pada `crypto.rs`, sub-perintah `forge-server keygen` dan `forge-server sign <pkg.forge.tar.zst>`, serta verifikasi signature publik di `BinhostClient` sebelum merge ke rootfs tanpa ketergantungan GPG eksternal.
43. **ADR-043 (Dynamic Version Constraint Engine):** Mengintegrasikan crate `version-compare` pada `resolver.rs` (`VersionConstraint`) untuk mengevaluasi batasan versi dinamis (`>=`, `<=`, `>`, `<`, `=`, `!=`, `~`) pada resolusi dependensi dan repositori ALPM CachyOS.
44. **ADR-044 (Multi-Core Data Parallelism on RAM tmpfs):** Memanfaatkan `rayon` untuk paralelisasi CPU-bound tasks: pemindaian 188+ resep lokal (`RecipeScanner::scan_all`) dan kalkulasi hash SHA256 ribuan berkas staging secara multi-threaded.
45. **ADR-045 (Low-Level Linux Syscall Resource Governance & PID Liveness Detection):** Menggunakan `nix` untuk binding kernel aman: deteksi liveness PID lock melalui sinyal 0 (`kill(0)`) untuk mencegah *stale locks* dan penegakan limit proses build (`RLIMIT_NOFILE`, `RLIMIT_CORE` via `setrlimit`).
46. **ADR-046 (Asynchronous Non-Blocking Streaming Decompression):** Mengintegrasikan `async-compression` dan `tokio-util` pada pipeline streaming download biner untuk dekompresi *on-the-fly* non-blocking langsung dari byte stream jaringan.
47. **ADR-047 (Pre-Flight Root Privilege Enforcement & Transparent Sudo/Doas Auto-Escalation):** Menegakkan pengecekan awal hak akses root (`geteuid().is_root()`) pada seluruh perintah mutatif (`install`, `remove`, `update`, `setup`, `sync`) sebelum I/O dimulai. Jika dijalankan oleh non-root di terminal interaktif, Forge secara transparan mengeskalasi eksekusi dengan `sudo`/`doas` memunculkan prompt password secara otomatis; jika non-interaktif, Forge langsung berhenti dengan pesan instruksi yang jelas.
48. **ADR-048 (Generic Post-Merge File Triggers & Hooks Engine):** Menerapkan `HookEngine` terpusat yang memindai berkas-berkas hasil merge untuk menjalankan pemicu sistem otomatis: `ldconfig` untuk shared libraries (`/lib*`, `/usr/lib*`, `*.so*`), `update-desktop-database` untuk file `.desktop`, `gtk-update-icon-cache` untuk direktori icon tema, `glib-compile-schemas` untuk skema GSettings/GLib, `update-mime-database` untuk MIME info packages, `depmod` untuk modul kernel (`/lib/modules`), dan notifikasi OpenRC services (`/etc/init.d/`). Eksekusi hook dilengkapi deteksi keberadaan binary secara anggun (*graceful fallback* jika binary utilitas belum terpasang di root target).
49. **ADR-049 (Wavefront Parallel DAG Scheduler & Multi-Worker Build Pool):** Menerapkan engine `WavefrontScheduler` (`scheduler.rs`) yang mengurai graf dependensi (DAG) menjadi gelombang eksekusi paralel (*wavefront propagation*). Seluruh node daun atau sub-cabang independen (`in_degree == 0`) dikompilasi secara simultan menggunakan pool worker multi-thread di RAM `tmpfs` staging terisolasi (`/tmp/forge/stage/{pkg}-{ver}`). Ketika satu paket selesai, in-degree dependensi dependent otomatis didekremen, dan paket yang siap langsung didispatch ke worker. Penggabungan ke target rootfs dikoordinasikan secara transaksional (`MergeTransaction`) sesuai urutan topologis sehingga menjamin nol konflik dan integritas database manifest `/var/db/forge/`.
50. **ADR-050 (Resumable HTTP Source Downloader with Multi-Mirror Fallback & Integrity Engine):** Menerapkan `SourceDownloader` (`downloader.rs`) pure Rust HTTP streaming client yang mendukung kelanjutan unduhan (*resumption*) via HTTP header `Range: bytes={offset}-` (status 206 Partial Content), failover multi-mirror otomatis jika mirror primer offline, retry bertingkat dengan exponential backoff, penulisan atomik `.part` $\rightarrow$ final, serta verifikasi hash SHA256 & BLAKE3 otomatis.
51. **ADR-051 (TUI Menuconfig & Dynamic Recipe USE Flags Selector):** Menerapkan antarmuka terminal interaktif visual `UseFlagsTui` (`menuconfig.rs`) berbasis `dialoguer` untuk konfigurasi USE flags dengan checkbox toggle. Mendukung pemindaian flag dinamis dari resep paket (`flag? ( dep )`), persistensi global ke `/etc/forge/forge.conf`, serta override per-paket ke `/etc/forge/package.use/{pkgname}`. CLI command: `forge menuconfig [--package <pkg>]` dan opsi `forge install --interactive-use <pkg>`.
52. **ADR-052 (Seccomp BPF Syscall Filtering & Build Hardening Engine):** Menerapkan `SeccompFilterBuilder` (`sandbox.rs`) yang menyusun program Berkeley Packet Filter (BPF) biner untuk memblokir syscall berbahaya pada level kernel (`reboot`, `kexec_load`, `init_module`, `delete_module`, `ptrace`, `iopl`, `ioperm`, `clock_settime`, `keyctl`, `bpf`, dll.) dan menginjeksi `--cap-drop ALL` serta `--seccomp <FD>` ke dalam Bubblewrap sandbox dengan fallback anggun jika kernel host membatasi seccomp.
53. **ADR-053 (Automated Server Source Code Rebuild & Seamless Self-Restart on Git Webhook):** Menerapkan deteksi otomatis perubahan kode sumber biner (`crates/forge-server/`, `crates/forge/`, `Cargo.toml`, `Cargo.lock`) pada saat `git pull --rebase` dipicu oleh GitHub Webhook (`POST /v1/webhook/github`), menjalankan background `cargo build --release -p forge-server`, serta melakukan *graceful in-place process replacement* (`std::os::unix::process::CommandExt::exec`) saat kompilasi sukses sehingga pembaruan engine server live di VPS berlangsung otomatis tanpa intervensi manual SSH.
54. **ADR-054 (Pure Self-Updating Package Philosophy & Live Git VCS Head Probe Update Engine):** Mengadopsi prinsip *"Everything is a Package"* di mana pembaruan Forge sendiri ditangani secara native melalui resep resmi `recipes/system/forge/recipe.toml` tanpa perlu sub-command khusus `self-update`. Menambahkan modul deteksi Git VCS secara real-time via `git ls-remote <url> <branch>` (`RecipeBuilder::probe_git_remote_commit`) untuk mendeteksi perubahan commit HEAD di hulu, menyimpan `git_commit` di `PackageMetadata`, serta mengintegrasikan pembaruan `@world` dan target spesifik secara penuh ke dalam DAG parallel compilation & transactional merge (`WavefrontScheduler`).
55. **ADR-055 (Isolated 3-Tier Separation of Concerns & Modular Python Maintainer Suite Architecture):** Menegakkan isolasi domain tegas antara *Maintainer Suite* lokal (`scripts/maintainer/` Python interpreted), *Central Server Daemon* (`forge-server` Rust), dan *User Client Engine* (`forge` Rust). Maintainer suite dipecah secara modular (<120 baris per file: `catalog`, `dag`, `tree`, `curated`, `inspector`, `linter`, `search`, `sources`, `matrix`) dengan dukungan 2 mode DAG solver (Pure Runtime DAG 0 siklus vs Source Build DAG dengan isolasi Bootstrap Seed Tier ADR-019), visual box-card recipe inspector & live editor, linter standar 100% lulus validasi (227/227 resep), dan regenerator SSOT otomatis.
56. **ADR-056 (Toolchain Ambient Exemption & Bootstrap Loop Elimination):** Mengotomasi pengecualian implicit build tools ambient (`gcc`, `make`, `binutils`) di engine DAG resolver dan maintainer suite, serta menstandarisasi dependensi fondasi sistem sesuai standar Arch `base-devel` dan Gentoo `@system`.
57. **ADR-057 (Shared Aggregate Index Sanitization & Collision Exemption):** Menerapkan sanitasi berkas indeks transien (`/usr/share/info/dir`, `ld.so.cache`, dll.) pada tahap staging dan pengecualian berkas sistem bersama pada pre-flight collision detector untuk mencegah kegagalan instalasi paket GNU.
58. **ADR-058 (CachyOS CDN77 Query Resolver & Isolated Prebuilt Installation):** Mengintegrasikan repositori biner resmi CachyOS CDN77 (`https://cdn77.cachyos.org/repo/x86_64_v4/`), query on-the-fly database repositori `.db.tar.zst`, dan isolasi metadata ALPM (`.PKGINFO`, `.MTREE`, dll.) saat staging merger.
59. **ADR-059 (Automated Recursive Binhost Dependency Resolution):** Mengintegrasikan resolusi rantai dependensi runtime secara rekursif dan instalasi sekuensial topologis pada mode 3-tier cascade binhost.
60. **ADR-060 (Canonical Distro Default Config & Serde Layered Overrides):** Menetapkan `/usr/share/forge/forge.conf.default` sebagai konfigurasi paten resmi yang selalu diperbarui saat update, menghapus file `.example`, dan menerapkan Serde fallback layered merge.
61. **ADR-061 (Disable Go Bindings in libcap C Toolchain Build):** Menambahkan argumen `GOLANG=no` pada fase `make` dan `make install` resep `libcap` untuk mencegah deteksi otomatis compiler Go host yang memicu kegagalan kompilasi generator `good-names.go` di sandbox C murni.

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
| *2026-09-19* | *Importer & 100 Resep* | *Katalog resep kosong dan kebutuhan konversi PKGBUILD/APKBUILD upstream secara deterministik* | *Membangun `RecipeImporter` engine (`importer.rs`), CLI `forge recipe-import`, dan menyusun 105 resep paket esensial Kura Linux di `recipes/` (ADR-032)* |
| *2026-09-19* | *Concurrency & Sandbox* | *Risiko tabrakan transaksi simultan, polusi host filesystem saat build, dan dependensi biner tier 2 berantai* | *Mengimplementasikan `ForgeLockGuard` (`lock.rs`), `SandboxRunner` (`sandbox.rs`) dengan Bubblewrap / fallback, serta parser ALPM `.db.tar.zst` & resolver dependensi rekursif (`cachyos.rs`) (ADR-033, ADR-034, ADR-035)* |
| *2026-09-19* | *Build vs Import* | *Pencampuran tanggung jawab build dan import pada server/klien membingungkan alur CI/CD* | *Menerapkan pemisahan `build` (kompilasi & packaging) dan `import` (ingestion & cataloging) secara independen (ADR-036)* |
| *2026-09-19* | *Ergonomi CI/CD* | *Kebutuhan mengetik path profil CPU berulang kali saat kompilasi paket CI/CD di server* | *Menerapkan `ServerProfileManager` (`profiles.rs`), `forge-server import <cpu-profile.json>` untuk persistensi profil aktif (`active.json`), `forge-server build <PACKAGE>` otomatis menggunakan profil aktif & auto-publish ke binhost, serta `forge-server list-profiles` (ADR-037)* |
| *2026-09-19* | *Stage Exporter* | *Kebutuhan pengemasan rootfs Kura Linux menjadi stage tarball resmi (.tar.xz / .tar.zst) lengkap dengan validasi UsrMerge & OpenRC, sanitasi cache, serta hash SHA256/BLAKE3* | *Mengimplementasikan `StageExporter` (`stage.rs`) dan CLI `forge stage-export` dengan validasi FHS/UsrMerge/OpenRC, sanitasi transien, packaging preservasi symlink/permissions (`append_tree_to_tar`), dan pembuatan checksum otomatis* |
| *2026-09-19* | *Sandbox Enforcement* | *Kompilasi un-sandboxed berisiko merusak host `/`; butuh penegakan bwrap wajib namun tetap mengizinkan self-bootstrap bubblewrap* | *Menerapkan penegakan wajib Bubblewrap di `SandboxRunner` dengan pengecualian khusus untuk paket `bubblewrap`/`bwrap` (ADR-040)* |
| *2026-09-19* | *Streaming & DAG Install* | *Kebutuhan eksekusi end-to-end instalasi biner streaming dan kompilasi transaksional DAG pada `forge install`* | *Mengimplementasikan `BinhostClient::download_and_extract_stream` dan menghubungkan `MergeTransaction` penuh ke CLI `forge install` (ADR-041)* |
| *2026-09-19* | *Security & Parallelism* | *Kebutuhan tanda tangan digital biner murni Rust, evaluasi batasan versi semantik, paralelisasi I/O RAM tmpfs, dan safe low-level Linux syscalls* | *Mengintegrasikan `ed25519-dalek` (`crypto.rs`), `version-compare` (`resolver.rs`), `rayon` (`RecipeScanner` & `scan_staging`), `nix` (`lock.rs` & `sandbox.rs`), serta `async-compression` (ADR-042, ADR-043, ADR-044, ADR-045, ADR-046)* |
| *2026-09-19* | *UX & Privilege* | *User non-root menjalankan `forge install` baru gagal di tengah jalan saat merge; butuh pre-flight root check dan auto-escalation prompt* | *Mengimplementasikan `PrivilegeManager` (`privilege.rs`) dengan deteksi UID 0 awal via `nix::unistd::geteuid()`, auto-escalation transparan via `sudo`/`doas` di TTY interaktif, dan pesan error jelas (ADR-047)* |
| *2026-09-19* | *Hooks & Triggers* | *Paket baru terpasang (shared lib, icon, schema, desktop entry, service) membutuhkan update cache/database sistem otomatis* | *Menerapkan `HookEngine` (`hooks.rs`) generic post-merge triggers untuk `ldconfig`, `update-desktop-database`, `gtk-update-icon-cache`, `glib-compile-schemas`, `update-mime-database`, `depmod`, dan OpenRC (ADR-048)* |
| *2026-09-19* | *Parallel Build* | *Kompilasi source DAG linear membuang potensi multi-core; leaf nodes independen perlu dibangun simultan* | *Menerapkan `WavefrontScheduler` (`scheduler.rs`) multi-worker thread pool dengan dynamic in-degree DAG propagation dan coordinated transactional merge (ADR-049)* |
| *2026-09-19* | *Resumable Downloader* | *Koneksi putus saat unduh tarball besar mengulang dari 0; butuh range resumption dan mirror failover murni Rust* | *Menerapkan `SourceDownloader` (`downloader.rs`) dengan HTTP Range header 206 Partial Content, mirror failover, dan verifikasi hash otomatis (ADR-050)* |
| *2026-09-19* | *TUI Menuconfig* | *Pengguna butuh antarmuka visual terminal untuk mengelola USE flags global dan per-paket* | *Menerapkan `UseFlagsTui` (`menuconfig.rs`) multi-select checkbox TUI, dynamic recipe inspector, persistensi `/etc/forge/forge.conf` dan `/etc/forge/package.use` (ADR-051)* |
| *2026-09-19* | *Build Hardening & Seccomp* | *Kompilasi upstream berpotensi mengeksekusi syscall berbahaya (reboot, ptrace, kernel module)* | *Menerapkan `SeccompFilterBuilder` (`sandbox.rs`) BPF bytecode generator, `--cap-drop ALL`, dan `--seccomp <FD>` bubblewrap hardening (ADR-052)* |
| *2026-09-20* | *Server Self-Rebuild* | *Perubahan source code Rust `forge-server` di Git membutuhkan kompilasi & restart manual via SSH di VPS* | *Menerapkan auto-rebuild background task & graceful in-place process replacement (`exec`) pada webhook receiver saat terdeteksi perubahan `.rs`/`Cargo.toml` (ADR-053)* |
| *2026-09-20* | *Pure Self-Updating & Git VCS Probe* | *Kebutuhan pembaruan `forge` sendiri secara terpadu tanpa self-update sub-command dan deteksi pembaruan Git commit live branch* | *Menyediakan resep resmi `recipes/system/forge/recipe.toml`, menambahkan `probe_git_remote_commit` via `git ls-remote`, penyimpanan `git_commit` di `PackageMetadata`, serta integrasi `forge update` ke Wavefront parallel DAG compilation (ADR-054)* |
| *2026-09-20* | *DAG Completeness & Missing Recipes* | *Kegagalan resolusi DAG pada `bash`/`base` karena dependensi hilang (`perl`, `bc`, `elfutils`, dll.) dan polusi tool generator dokumen* | *Menambahkan 31 resep paket esensial baru (total 227 resep), membersihkan dependensi generator dokumen non-esensial, dan mengimplementasikan maintainer suite (ADR-055)* |
| *2026-09-20* | *Modular Maintainer & Bootstrap Resolution* | *Skrip Python monolitik (>1100 baris) sulit dipelihara, siklus bootstrap glibc<->gcc loop pada traversal build, dan resep belum terstandarisasi* | *Memecah maintainer tool menjadi paket Python modular (`scripts/maintainer/`), memisahkan mode Pure Runtime DAG vs Source Build DAG dengan isolasi seed toolchain, menstandarkan 227 resep (100% lint pass), dan membersihkan runtime glibc loop (ADR-055)* |
| *2026-09-20* | *Toolchain Ambient Exemption & Bootstrap Loop Elimination* | *Pencantuman build tools (gcc, make, binutils) pada makedepends resep dasar memicu circular dependency deadlock (make ↔ gcc ↔ gmp)* | *Menambahkan filter implicit ambient build tools di engine `resolver.rs` & `dag.py`, menstandarisasi dependensi fondasi (`make`, `gmp`, `mpfr`, `mpc`, `zlib`, `binutils`, `gcc`, `base`, `base-devel`) sesuai standar Arch `base-devel` & Gentoo `@system` (ADR-056)* |
| *2026-09-20* | *Shared Aggregate Index Sanitization & Collision Exemption* | *Berkas katalog/indeks bersama (`/usr/share/info/dir`, `ld.so.cache`, `gschemas.compiled`) dibuat oleh banyak paket GNU dan memicu tabrakan pra-instalasi* | *Menerapkan `sanitize_staging_dir` di `builder.rs` untuk menghapus berkas transien saat staging & `is_shared_system_file` di `merger.rs` untuk mengecualikan berkas sistem bersama dari deteksi tabrakan (ADR-057)* |
| *2026-09-20* | *CachyOS CDN77 Query & Isolated Prebuilt Installation* | *Kegagalan parsing `[use]` pada konfigurasi kustom sysroot dan URL mirror CachyOS upstream usang* | *Menambahkan `#[serde(alias = "use")]` di `ForgeConfig`, mengintegrasikan mirror CDN77 resmi CachyOS (`cdn77.cachyos.org`), pencarian otomatis paket via query database repositori `.db.tar.zst`, dan isolasi metadata ALPM saat staging merge (ADR-058)* |
| *2026-09-20* | *Automated Recursive Binhost Dependency Resolution* | *Mode binhost hanya mengunduh paket target tunggal tanpa menyelesaikan rantai dependensi runtime hulu secara otomatis* | *Menerapkan `fetch_package_db_pool` & `resolve_dependency_chain` di `cachyos.rs`, mengintegrasikan auto-download/merge topological sequence untuk seluruh dependensi belum terpasang di `main.rs` (ADR-059)* |
| *2026-09-20* | *Canonical Distro Default Config & Layered Serde Overrides* | *Format `.example` redundan, konfigurasi user rentan usang/rusak saat ada opsi baru dari upstream* | *Menetapkan `/usr/share/forge/forge.conf.default` sebagai konfigurasi paten resmi yang selalu ter-update, mengeliminasi `.example`, dan menerapkan `#[serde(default)]` layered parsing di `ForgeConfig` (ADR-060)* |
| *2026-09-20* | *libcap Go Bindings Build Deadlock* | *libcap otomatis mendeteksi Go di host dan memicu kegagalan build `good-names.go` di sandbox C murni saat `forge update forge`* | *Menambahkan `GOLANG=no` pada target `make` dan `make install` di `recipes/core/libcap/recipe.toml` (ADR-061)* |

---

## 4. Panduan Serah Terima AI Agent (Incoming AI Agent Handover Guide)

> **Catatan Penting untuk AI Agent Penerus:**
> Repositori ini telah dikonsolidasikan secara rapi menjadi **Clean 2-Crate Workspace Layout** dengan paradigma **Meta-Paket Murni ("Everything is a Package")**, optimasi compiler **Mentok Ekstrem (Zen 4 AVX-512 / Thin LTO / Mold ICF)**, repositori resep terstandarisasi **`/var/db/forge/recipes/` (ADR-028)**, 227 resep paket resmi tervalidasi 100% bebas broken dependency dan 100% lulus linter, Client Sync Engine (`sync.rs`), Forge Server HTTP Daemon & Auto-Rebuilder (`server.rs`), DAG Dependency Resolver (`resolver.rs`), Transactional Merger (`merger.rs`), Manifest Database Engine (`db.rs`), Strict Sandbox & Seccomp BPF Hardening (`sandbox.rs`), Live Streaming Downloader (`binhost.rs`), Distro Stage Exporter (`stage.rs`), Digital Signing Engine (`crypto.rs`), Root Privilege Manager (`privilege.rs`), Generic Post-Merge Hooks Engine (`hooks.rs`), Wavefront Parallel DAG Scheduler (`scheduler.rs`), Resumable HTTP Source Downloader (`downloader.rs`), TUI Menuconfig (`menuconfig.rs`), all-in-one modular maintainer suite ([`scripts/maintainer/`](file:///home/admin/Development/Forge/scripts/maintainer/)), dan 5 Crate High-Performance/Security dengan tingkat kesiapan **100%**. Seluruh blueprint arsitektur, diagram, aturan mutlak, dan 61 ADR telah didokumentasikan secara lengkap.

### 📌 Ringkasan Status & State Workspace:
- **Workspace:** 2 Crate murni: [`crates/forge`](file:///home/admin/Development/Forge/crates/forge) (Klien & Engine Library) dan [`crates/forge-server`](file:///home/admin/Development/Forge/crates/forge-server) (Server & CI/CD Builder).
- **Katalog Resep Resmi:** 227 paket terverifikasi (`recipes/system/`: 27, `recipes/core/`: 87, `recipes/extra/`: 113) dengan 0 missing dependencies di seluruh pohon DAG (termasuk `bash`, `base`, `base-devel`) dan 100% lulus audit linter.
- **Toolchain & Compiler Flags (Mentok Ekstrem):** Rust 1.97.1, LLVM/Clang 22, Linker `mold`, Ccache 4.13.5, Thin LTO, `-O3 -march=native -pipe -flto=thin -fno-plt -fno-math-errno -fno-trapping-math -ffunction-sections -fdata-sections -falign-functions=32 -fstack-protector-strong -D_FORTIFY_SOURCE=2` dan LDFLAGS `-Wl,-O3 -Wl,--as-needed -Wl,--gc-sections -Wl,--icf=all -Wl,-z,relro -Wl,-z,now -fuse-ld=mold`.
- **Seed Toolchain:** Staged murni di `/tmp/forge/stage/`, membundel `/var/db/forge/recipes/`, `/etc/forge/forge.conf`, dan `/usr/bin/forge` ke `dist/kura-toolchain.tar.xz` (ADR-019, ADR-028).
- **Distro Stage Exporter:** Modul `crates/forge/src/stage.rs` dan CLI `forge stage-export` mengemas staging/rootfs Kura Linux menjadi `dist/kura-stage.tar.xz` / `dist/kura-stage.tar.zst` lengkap dengan validasi FHS/UsrMerge/OpenRC, sanitasi cache, dan hash SHA256 (`.sha256`) & BLAKE3 (`.b3sum`).
- **Maintainer Suite (`scripts/maintainer/`):** Paket Python modular terisolasi dengan 10 modul independen (catalog, dag, tree, curated, inspector, linter, search, sources, matrix) dan entrypoint ramping `scripts/maintainer.py`.
- **Test Suite:** 98 unit & integration tests lulus 100% (`cargo test --workspace`).

### 🛑 6 Aturan Mutlak yang Wajib Diikuti:
1. **HITL (Human-In-The-Loop):** Wajib ikuti siklus 5-langkah (*Plan $\rightarrow$ Chat $\rightarrow$ ACC $\rightarrow$ Eksekusi $\rightarrow$ Uji*). Jangan edit/buat file tanpa ACC di chat.
2. **HARAM EDIT KURALINUX:** Dilarang keras menyentuh direktori `/home/admin/Development/KuraLinux/`.
3. **HARAM AMBIL DARI HOST (ADR-019):** Biner dan toolchain wajib 100% dikompilasi dari source code upstream via resep ke `/tmp/forge/stage/`. Dilarang meng-copy biner dari `/usr/bin/` atau `/usr/lib/`.
4. **GLIBC EXEMPTION (ADR-002):** Paket Glibc di-build dengan GCC standar tanpa flag `-march` kustom demi kestabilan.
5. **CCACHE ACCELERATION (ADR-025):** Kompilasi memanfaatkan Ccache 4.13.5 pada build engine.
6. **OPENRC ONLY:** Tidak boleh ada ketergantungan pada Systemd.
