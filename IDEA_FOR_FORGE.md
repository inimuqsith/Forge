# IDEA_FOR_FORGE.md — Architectural Ideas, RFCs & Future Specifications for Forge

> Dokumen ini mencatat rancangan arsitektur tingkat lanjut (*Requests for Comments / RFC*), eksplorasi ide, dan spesifikasi teknis untuk pengembangan ekosistem **Forge Package Manager** dan distribusi **Kura Linux** di masa depan.

---

## Daftar Isi Ide & Spesifikasi

- [Idea #13: Hermetic Rust Vendoring & Zero-Host Cargo Builds](#idea-13-hermetic-rust-vendoring--zero-host-cargo-builds)

---

## Idea #13: Hermetic Rust Vendoring & Zero-Host Cargo Builds

### 1. Status & Metadata
- **ID:** IDEA-013 / ADR-083
- **Status:** Approved / In Progress
- **Kategori:** Toolchain, Sandbox Isolation, Rust Ecosystem, Distro Hermeticity
- **Target:** Engine `forge` (Downloader & Sandbox), Resep Resmi Rust (`recipes/`)

---

### 2. Latar Belakang & Analisis Masalah

Pada paket berbasis ekosistem Rust yang dikompilasi melalui `cargo build`, terdapat perbedaan mendasar antara model kompilasi Cargo standar dengan standar isolasi paket distribusi Linux:

1. **Host Cargo Leakage (Pencemaran Host):**
   - Secara default, Cargo mencari dan menulis cache crate di `$HOME/.cargo/registry`.
   - Pada fase bootstrap lokal, ketika sandbox Bubblewrap membuka akses baca ke root filesystem host (`--ro-bind / /`), Cargo secara tidak sengaja dapat membaca file `.crate` yang pernah diunduh host di `~/.cargo/registry/cache`.
2. **Kegagalan Fatal di Lingkungan Murni (Chroot / Baremetal Target):**
   - Ketika Forge dijalankan di dalam rootfs Kura Linux yang bersih (di mana tidak ada direktori `~/.cargo` host) atau di dalam sandbox offline murni (`--unshare-net`), perintah `cargo build` akan **gagal total** karena Cargo tidak memiliki akses internet untuk mengunduh dependensi dan tidak menemukan cache lokal.
3. **Pelanggaran Prinsip Zero Host Dependency (ADR-019):**
   - Standar arsitektur Kura Linux mewajibkan setiap paket 100% mandiri, deterministik, dan dapat dibangun tanpa mengandalkan berkas sisa atau toolchain dari host.

---

### 3. Blueprint Solusi Hermetis

Untuk menjamin kompilasi Rust yang 100% hermetis dan offline di segala lingkungan:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        FASE 1: FETCH (ONLINE / HOST)                    │
│                                                                         │
│  recipe.toml sources.urls:                                              │
│  - Tarball Kode Sumber: [repo].tar.gz                                   │
│  - Tarball Vendored Crates: [pkg]-vendor-[ver].tar.xz                  │
│                                                                         │
│  Atau Engine Auto-Fetcher (Cargo.lock Parsing):                         │
│  - forge engine mengunduh *.crate ke /var/cache/forge/distfiles/crates/ │
└────────────────────────────────────┬────────────────────────────────────┘
                                     │
                                     ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                   FASE 2: BUILD SANDBOX (OFFLINE / HERMETIS)            │
│                                                                         │
│  1. Isolasi CARGO_HOME:                                                 │
│     export CARGO_HOME="${srcdir}/.cargo_home"                           │
│                                                                         │
│  2. Konfigurasi Vendoring (.cargo/config.toml):                         │
│     [source.crates-io]                                                  │
│     replace-with = "vendored-sources"                                   │
│     [source.vendored-sources]                                           │
│     directory = "vendor"                                                │
│                                                                         │
│  3. Eksekusi Offline:                                                   │
│     cargo build --release --frozen --offline                            │
└─────────────────────────────────────────────────────────────────────────┘
```

#### A. Pola Standar Resep (Recipe-Level Vendoring)
1. **Tarball Vendored Crates:**
   - Setiap rilis paket berbasis Rust menyediakan tarball dependensi crates hasil `cargo vendor`.
   - Tarball ini dideklarasikan secara eksplisit di `sources.urls` dan diverifikasi dengan checksum SHA256.
   - Contoh `recipe.toml`:
     ```toml
     [package]
     name = "ripgrep"
     version = "14.1.1"

     [sources]
     urls = [
         "https://github.com/BurntSushi/ripgrep/archive/14.1.1.tar.gz",
         "https://pkgkura.amqs.net/distfiles/ripgrep-vendor-14.1.1.tar.xz"
     ]
     sha256 = [
         "33c616959def5f80badba61d686a6f6e0b7a840e6c51b2cc3f2d851e5e6e9e45",
         "a1b2c3d4e5f6... (sha256)"
     ]

     [build]
     type = "custom"
     script = """
     export CARGO_HOME="${srcdir}/.cargo_home"
     mkdir -p "${CARGO_HOME}"

     cd "${srcdir}/ripgrep-${pkgver}"
     mkdir -p .cargo
     cat << 'EOF' > .cargo/config.toml
     [source.crates-io]
     replace-with = "vendored-sources"

     [source.vendored-sources]
     directory = "vendor"
     EOF

     cargo build --release --frozen --offline
     install -Dm755 target/release/rg "${pkgdir}/usr/bin/rg"
     """
     ```

#### B. Pola Otomasi Engine (Future Forge Engine Crate Ingestion)
1. **Fase Ingestion & Crate Fetching:**
   - Untuk resep dengan tipe `type = "cargo"`, engine `downloader.rs` mem-parsing `Cargo.lock` saat fase fetch host.
   - Engine mengunduh seluruh crate dari `https://static.crates.io/crates/{name}/{name}-{version}.crate` secara paralel dan menyimpannya di cache `/var/cache/forge/distfiles/crates/`.
2. **Fase Sandbox Injection:**
   - Engine mengekstrak crate ke direktori `${srcdir}/vendor/` di dalam sandbox sebelum build dijalankan.
   - Flag `--offline --frozen` diinjeksi secara otomatis untuk memastikan tidak ada percobaan akses jaringan.

---

### 4. Rencana Implementasi & Milestone
- [x] **Milestone 1 (ADR-083):** Injeksi isolasi eksplisit `CARGO_HOME="${srcdir}/.cargo_home"` pada seluruh resep Rust inti (`forge`, dll.) untuk memutus ketergantungan pada `~/.cargo/` host.
- [ ] **Milestone 2:** Pembuatan script helper maintainer `scripts/maintainer/vendor_rust.py` untuk mengotomatisasi pembuatan tarball `vendor.tar.xz` dan kalkulasi SHA256 saat package bumping.
- [ ] **Milestone 3:** Integrasi parser `Cargo.lock` native di dalam `downloader.rs` untuk build hermetis otomatis tanpa perlu tarball vendor eksternal.
