# MEMORY.md — Memori & Catatan Teknis Package Manager `forge`

> Dokumen memori persisten AI untuk melacak progres pengembangan package manager **`forge`**, keputusan arsitektur (ADR), status roadmap, dan log pemecahan masalah teknis.

---

## 1. Status & Roadmap Pengembangan `forge`

### Fase 1: Inisialisasi Arsitektur, Dokumentasi, & Standarisasi Resep (Selesai)
- [x] Inisialisasi repositori Git dan konfigurasi `.gitignore`.
- [x] Susun panduan AI & pedoman HITL di [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- [x] Susun blueprint teknis komprehensif di [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- [x] Susun catatan memori & ADR di [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
- [x] Susun dokumentasi publik di [`README.md`](file:///home/admin/Development/Forge/README.md).
- [x] Siapkan template konfigurasi bawaan `config/forge.conf.example`.
- [x] Siapkan struktur direktori awal (`src/`, `recipes/`, `tests/`, `config/`, `docs/`).

### Fase 2: Core Engine & CLI Skeleton
- [ ] Implementasi parser konfigurasi `/etc/forge/forge.conf` (dengan fallback default aman).
- [ ] Implementasi CLI dispatcher (`install`, `build`, `remove`, `list`, `query`, `search`, `update`, `clean`, `stage-export`).
- [ ] Implementasi parser spesifikasi resep (`pkgname`, `pkgver`, `depends`, `sources`, `sha256sums`, fungsi hooks).

### Fase 3: Source Fetcher & Integrity Verifier
- [ ] Modul pengunduh berkas sumber (HTTP/HTTPS/Git) ke `/var/cache/forge/distfiles/`.
- [ ] Modul verifikasi integritas kriptografis SHA256 / BLAKE3.
- [ ] Mekanisme resume download dan cache hit detection.

### Fase 4: Sandbox Build Engine & DESTDIR Staging
- [ ] Pengelolaan direktori build di RAM (`/tmp/forge/build/` berbasis tmpfs).
- [ ] Injeksi otomatis compiler flags Kura Linux (`-O2 -march=native -pipe ...`).
- [ ] Logika pengecualian khusus optimasi untuk Glibc (ADR-002).
- [ ] Eksekusi fungsi siklus hidup resep (`prepare`, `build`, `package` ke `$DESTDIR`).

### Fase 5: Transactional Merge, Collision Detector, & Manifest DB
- [ ] Pre-flight scan deteksi tabrakan berkas (*file collisions*) terhadap database `/var/db/forge/installed/`.
- [ ] Engine penyalinan berkas atomik dari `$DESTDIR` ke target root (`/` atau `$FORGE_ROOT`).
- [ ] Pencatatan manifest berkas, symlink, permission, dan metadata JSON.
- [ ] Manajemen berkas daftar paket eksplisit pengguna (`/var/db/forge/world`).

### Fase 6: Unmerge Engine & Pembersihan Bersih
- [ ] Penghapusan file secara presisi berdasarkan manifest paket.
- [ ] Pembersihan direktori kosong (*directory pruning*).
- [ ] Proteksi file konfigurasi yang telah diubah pengguna di `/etc/`.
- [ ] Analisis dan pembersihan paket orphan (*unused dependencies*).

### Fase 7: Dependency Graph & DAG Topological Sort
- [ ] Pembentukan graf dependensi dari `depends` dan `makedepends`.
- [ ] Algoritma Topological Sort untuk menentukan urutan kompilasi linier.
- [ ] Deteksi siklus dependensi melingkar (*circular dependencies*).
- [ ] Ekspansi meta-target `@system` dan `@world`.

### Fase 8: Hook System & Integrasi OpenRC Kura Linux
- [ ] Deteksi otomatis berkas daemon di `/etc/init.d/` dan rekomendasi pendaftaran runlevel (`rc-update`).
- [ ] Post-hook `ldconfig` untuk shared libraries baru di `/usr/lib`.
- [ ] Post-hook `mandoc` database index untuk manual pages di `/usr/share/man`.

### Fase 9: Generator Stage Tarball (`forge stage-export`)
- [ ] Utilitas pengemas rootfs aktif menjadi tarball distribusi `kura-stage.tar.xz`.
- [ ] Opsi filter pembersihan cache, log, dan artefak temporer sebelum pengemasan.

### Fase 10: Resep Resmi Kura Linux `@system` & Uji Integrasi
- [ ] Penyusunan pohon resep `recipes/system/` (Toolchain, Coreutils, OpenRC, Kernel Monolithic, GRUB, dll.).
- [ ] Pengujian build lengkap di mock chroot environment.

---

## 2. Keputusan Arsitektur Resmi (ADR Index)

1. **ADR-001 (Kompilasi 100% Native Silikon):** Forge mengompilasi seluruh paket langsung dari kode sumber upstream dengan menyuntikkan flag optimasi silikon target (`-march=native -O2 -pipe`) untuk memaksimalkan performa CPU pengguna Kura Linux.
2. **ADR-002 (Pengecualian Optimasi Custom pada Paket Glibc):** Mengikuti spesifikasi Kura Linux, paket Glibc tetap dikompilasi secara otomatis oleh Forge, namun tanpa menyuntikkan CFLAGS optimasi CPU custom karena keterbatasan build system internal Glibc.
3. **ADR-003 (Format Resep Hibrida: Deklaratif + POSIX Shell):** Metadata paket (nama, versi, dependensi, sumber, checksum) ditulis secara deklaratif, sedangkan tahapan kompilasi (`prepare`, `build`, `package`) memanfaatkan fungsi POSIX shell standar yang fleksibel dan mudah dipahami.
4. **ADR-004 (Database Flat-File `/var/db/forge/` Tanpa Ketergantungan Eksternal):** Database paket terpasang menggunakan format teks manifest dan file JSON sederhana di `/var/db/forge/installed/` agar tidak bergantung pada database engine biner eksternal saat proses *bootstrap* awal distro.
5. **ADR-005 (Isolasi Build RAM tmpfs & DESTDIR Staging):** Kompilasi tidak diizinkan menyentuh sistem rootfs secara langsung. Seluruh build berlangsung di `/tmp/forge/build/` dan dipasang ke staging directory sementara `/tmp/forge/stage/` sebelum transaksi merge.
6. **ADR-006 (Pemeriksaan Tabrakan Berkas & Manifest Deterministik):** Sebelum melakukan merge, Forge wajib memvalidasi tidak ada konflik file yang tidak sah dengan paket lain. Manifest mencatat seluruh berkas yang terpasang untuk menjamin proses unmerge 100% bersih tanpa sisa (*zero cruft*).
7. **ADR-007 (Integrasi Layanan OpenRC):** Forge dirancang khusus untuk lingkungan init OpenRC Kura Linux, mendeteksi skrip di `/etc/init.d/`, dan mengintegrasikan pendaftaran runlevel via `rc-update`.
8. **ADR-008 (Meta-Target `@system` & `stage-export`):** Menyediakan dukungan tingkat pertama untuk meta-set `@system` (seluruh base system Kura Linux) dan fitur `forge stage-export` untuk membuat file distribusi `kura-stage.tar.xz`.
9. **ADR-009 (Protokol Mutlak HITL & Pengujian Terisolasi):** AI pengembang Forge wajib mengikuti protokol 5 langkah (*Plan $\rightarrow$ Chat $\rightarrow$ ACC $\rightarrow$ Eksekusi $\rightarrow$ Uji*) dan dilarang memodifikasi sistem host atau repositori KuraLinux di luar batas tugas.

---

## 3. Log Masalah & Solusi (Troubleshooting)

| Tanggal | Komponen | Masalah | Solusi |
| :--- | :--- | :--- | :--- |
| *2026-09-18* | *Inisialisasi* | *Kebutuhan spesifikasi package manager Kura Linux terpisah dari repo distro* | *Mengadaptasi seluruh draf kebutuhan dari IDEA_FOR_FORGE.md ke dalam arsitektur resmi Forge di repositori mandiri* |
