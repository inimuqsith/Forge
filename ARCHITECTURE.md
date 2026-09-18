# ARCHITECTURE.md — Blueprint & Desain Arsitektur Package Manager `forge`

> **`forge`** adalah *source-based package manager* modern, cepat, deterministik, dan berbobot ringan yang dirancang khusus untuk menjadi fondasi ekosistem distribusi **Kura Linux**. Forge bertanggung jawab mengelola siklus hidup kompilasi native silikon (`-march=native`), pelacakan manifest berkas, resolusi dependensi DAG, integrasi daemon **OpenRC**, serta pembuatan stage arsip distribusi (`kura-stage.tar.xz`).

---

## 1. Lapisan Arsitektur Engine `forge`

```
+-------------------------------------------------------------------------+
|                        Forge CLI Interface Layer                        |
|  [ forge install | build | remove | update | list | query | stage-export] |
+-------------------------------------------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                        Core Engine & Orchestrator                       |
|  - Config Parser (/etc/forge/forge.conf & CLI flags)                    |
|  - Dependency Graph & Topological DAG Resolver                          |
|  - Package Set Resolver (@system, @world)                               |
+-------------------------------------------------------------------------+
                                     |
         +---------------------------+---------------------------+
         |                                                       |
         v                                                       v
+---------------------------------+     +---------------------------------+
|   Source Fetcher & Verifier     |     |   Build Sandbox & Compiler      |
|  - Tarball Fetcher (HTTP/HTTPS) |     |  - RAM tmpfs (/tmp/forge/build) |
|  - SHA256 Integrity Verifier    |     |  - Native Silicon Flags Injector|
|  - Distfiles Cache Storage      |     |  - POSIX Recipe Hooks Execution |
|    (/var/cache/forge/distfiles) |     |  - DESTDIR Staging Engine       |
+---------------------------------+     +---------------------------------+
                                                                 |
                                                                 v
+---------------------------------+     +---------------------------------+
|   System Hook Triggers          |     |  Transactional Merger & Tracker |
|  - OpenRC Service Discovery     | <-- |  - Collision Pre-flight Check   |
|  - Dynamic Linker (ldconfig)    |     |  - Atomic Copy to Rootfs (/)    |
|  - Manual Page Index (mandoc)   |     |  - Manifest Generator & Hash DB |
+---------------------------------+     +---------------------------------+
                                                         |
                                                         v
+-------------------------------------------------------------------------+
|                  Database & State Layer (/var/db/forge/)                |
|  ├── installed/<pkg>-<ver>/[manifest, metadata.json, dependencies]       |
|  └── world (Daftar paket eksplisit pengguna)                            |
+-------------------------------------------------------------------------+
```

---

## 2. Rincian 10 Sub-Sistem Utama `forge`

### 1️⃣ CLI Interface & Command Dispatcher
Menyediakan antarmuka baris perintah yang ergonomis, cepat, dan jelas:
- `forge build <pkg>`: Mengunduh, memverifikasi, dan mengompilasi paket hingga tahap staging (`DESTDIR`) tanpa menyentuh root filesystem.
- `forge install <pkg>`: Siklus lengkap (Fetch $\rightarrow$ Verify $\rightarrow$ Build $\rightarrow$ Stage $\rightarrow$ Merge $\rightarrow$ Register DB $\rightarrow$ Run Hooks).
- `forge remove <pkg>`: Menghapus seluruh berkas milik paket secara presisi berdasarkan manifest dan membersihkan direktori kosong.
- `forge update`: Memperbarui pohon resep lokal (`recipes/`) dan memeriksa pembaruan versi paket.
- `forge list`: Menampilkan seluruh paket terpasang beserta versi dan arsitektur silikon target.
- `forge query <pkg>`: Menampilkan metadata rinci, dependensi langsung/terbalik, dan daftar berkas paket.
- `forge search <query>`: Melakukan pencarian cepat berdasarkan nama atau deskripsi paket.
- `forge clean`: Membersihkan sisa file cache sementara di `/tmp/forge/` dan distfiles lama.
- `forge stage-export`: Mengemas rootfs aktif menjadi stage tarball (`kura-stage.tar.xz`).

---

### 2️⃣ Spesifikasi & Eksekusi Resep (*Recipe Engine*)
Resep paket Forge (`recipe` atau `Recipe.forge`) dirancang modular, deklaratif untuk metadata, dan memanfaatkan fungsi POSIX Shell untuk tahap eksekusi build.

#### Metadata Deklaratif Wajib:
- `pkgname`: Nama unik paket (misal: `bash`).
- `pkgver`: Versi upstream rilis (misal: `5.3`).
- `pkgrel`: Revisi rilis resep distro Kura Linux (misal: `1`).
- `pkgdesc`: Deskripsi singkat fungsi paket.
- `url`: Situs resmi atau repositori upstream.
- `license`: Lisensi perangkat lunak (misal: `GPL-3.0-or-later`).
- `depends`: Array dependensi runtime yang wajib terpasang.
- `makedepends`: Array dependensi yang hanya diperlukan saat proses kompilasi.
- `sources`: Array URL sumber tarball / patch.
- `sha256sums`: Array checksum SHA256 untuk memvalidasi setiap entri `sources`.

#### Hook Fungsi Eksekusi Build:
1. `prepare()`: Ekstraksi patch lokal dan penyesuaian awal codebase.
2. `build()`: Konfigurasi sistem build (`./configure`, `cmake`, `meson`, `make`) dengan injeksi CFLAGS/CXXFLAGS distro.
3. `package()`: Pemasangan hasil build ke staging directory `$DESTDIR` (misal: `make DESTDIR="$DESTDIR" install`).

---

### 3️⃣ Dependency Graph & DAG Resolver
- Menggunakan struktur data **Directed Acyclic Graph (DAG)** dan algoritma **Topological Sort** untuk menentukan urutan kompilasi yang benar.
- Mendeteksi *circular dependency* sebelum proses download atau kompilasi dimulai.
- Mendukung pemisahan antara `makedepends` (dibutuhkan hanya saat build) dan `depends` (dibutuhkan saat runtime).
- Resolusi Meta-Set: Mampu mengekspansi meta-target `@system` dan `@world` menjadi urutan build linier yang terurut.

---

### 4️⃣ Source Fetcher & Integrity Verifier
- Mengunduh tarball sumber secara aman melalui protokol `HTTP`, `HTTPS`, atau `Git`.
- Menyimpan cache berkas sumber di `/var/cache/forge/distfiles/` untuk menghemat bandwidth pada kompilasi berulang.
- **Validasi Kriptografis Mutlak:** Setiap berkas yang diunduh wajib lolos verifikasi hash SHA256 sebelum diizinkan diekstrak. Jika hash tidak cocok, proses langsung dihentikan demi keamanan.

---

### 5️⃣ Sandbox Build Space & Injektor Flag Native Silikon
- **Area Build Berkecepatan Tinggi (RAM tmpfs):** Seluruh proses ekstraksi dan kompilasi berlangsung di `/tmp/forge/build/<pkg>-<ver>/` yang dialokasikan di RAM (`tmpfs`).
- **Injeksi Flag Kompilasi Kura Linux:** Forge secara otomatis mengekspor flag native yang terkonfigurasi di `/etc/forge/forge.conf`:
  ```bash
  export CFLAGS="-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"
  export CXXFLAGS="${CFLAGS}"
  export LDFLAGS="-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now"
  export MAKEFLAGS="-j$(nproc)"
  export PREFIX="/usr"
  ```
- **Pengecualian Khusus Glibc:** Sesuai standar arsitektur Kura Linux (ADR-003 & ADR-012), paket Glibc dikompilasi oleh Forge tanpa flag `-march=native` custom untuk menjamin stabilitas build system Glibc.
- **Isolasi Staging (`DESTDIR`):** Kompilasi tidak pernah memasang file langsung ke sistem hidup, melainkan ke staging directory sementara `/tmp/forge/stage/<pkg>/`.

---

### 6️⃣ Transactional File Merger & Collision Detector
- **Pre-flight Collision Check:** Sebelum melakukan merge ke sistem root (`/`), Forge memindai seluruh file di direktori staging dan mencocokkannya dengan database paket lain di `/var/db/forge/installed/`.
- Jika terdapat konflik file yang dimiliki oleh paket lain, transaksi dibatalkan sebelum merusak sistem, kecuali ada flag overwrite eksplisit.
- **Atomic Merge:** Salin file dari `$DESTDIR` ke target rootfs (`/`) dengan mempertahankan permission, ownership, symlink UsrMerge, dan timestamp.
- **Generasi Manifest Otomatis:** Mencatat seluruh path berkas, symlink, direktori, dan hash file yang berhasil dipasang ke sistem.

---

### 7️⃣ Database Flat-File & State Engine (`/var/db/forge/`)
Forge menggunakan database berbasis *flat-file text/JSON* yang tidak bergantung pada daemon eksternal (seperti SQLite/PostgreSQL) sehingga sangat tangguh saat sistem dalam status bootstrap atau pemulihan darurat.

```
/var/db/forge/
├── installed/
│   └── <pkgname>-<pkgver>-<pkgrel>/
│       ├── manifest        # Daftar absolut berkas yang dipasang (/usr/bin/bash, dll.)
│       ├── metadata.json   # Versi, lisensi, timestamp build, CPU flags
│       └── dependencies    # Daftar dependensi yang terpasang
└── world                   # Daftar nama paket eksplisit yang diminta pengguna
```

---

### 8️⃣ Removal / Unmerge & Orphan Cleaner Engine
- Menghapus paket secara aman dengan membaca entri file dari berkas `/var/db/forge/installed/<pkg>/manifest`.
- **Proteksi File Konfigurasi (`/etc/`):** File konfigurasi yang telah dimodifikasi oleh pengguna tidak dihapus secara paksa, melainkan diberi peringatan atau disimpan sebagai backup `.forge-backup`.
- **Pruning Direktori Kosong:** Direktori induk yang menjadi kosong setelah file di dalamnya dihapus akan dibersihkan secara otomatis.
- **Deteksi Orphan:** Mendeteksi paket dependensi runtime yang tidak lagi dibutuhkan oleh paket apa pun di `world`.

---

### 9️⃣ OpenRC Hook & System Triggers
Forge secara otomatis memindai hasil instalasi pada tahap post-merge dan memicu hook sistem:
1. **OpenRC Service Hook:** Jika paket menyertakan skrip di `/etc/init.d/<service>`, Forge mendeteksi service tersebut dan memberikan opsi otomatisasi pendaftaran runlevel (`rc-update add <service> default`).
2. **Dynamic Linker Hook (`ldconfig`):** Otomatis menjalankan `ldconfig` jika ada file shared library (`.so`) yang dipasang ke `/usr/lib` atau `/usr/lib64`.
3. **Mandoc Database Hook:** Otomatis memperbarui index pencarian manual page (`makewhatis` / `mandoc -Tlint`) jika ada berkas manpage baru di `/usr/share/man/`.
4. **Desktop / MIME Hook:** Memperbarui cache icon dan desktop database jika paket menyertakan file `.desktop` di `/usr/share/applications/`.

---

### 🔟 Fitur Spesial Distro Kura Linux

#### A. Meta-Target `@system`
Forge memiliki pemahaman bawaan terhadap kumpulan paket inti sistem Kura Linux (`@system`):
- Memungkinkan pembangunan ulang seluruh sistem operasi dari nol dengan 1 perintah:
  ```bash
  forge install @system
  ```
- Kumpulan `@system` mencakup:
  - Toolchain: `glibc`, `gcc`, `binutils`, `linux-headers`.
  - Core Utils: `coreutils`, `bash`, `sed`, `grep`, `gawk`, `make`, `patch`, `tar`, `xz`, `zstd`, `findutils`, `diffutils`, `file`, `which`.
  - Kernel & Boot: `linux` (Monolithic Kernel), `grub`.
  - Init & System: `openrc`, `eudev`, `acpid`, `kmod`, `util-linux`, `shadow`, `opendoas`.
  - Session & Network: `elogind`, `dbus`, `dhcpcd`, `iwd`, `chrony`.
  - Crypto & Base: `openssl`, `ca-certificates`, `curl`, `e2fsprogs`, `dosfstools`, `pkgconf`.
  - Daemons & Docs: `metalog`, `cronie`, `earlyoom`, `nftables`, `mandoc`, `nano`, `less`.

#### B. Generator Stage Tarball (`forge stage-export`)
Perintah khusus untuk mengemas rootfs Kura Linux menjadi tarball distribusi minimal:
```bash
forge stage-export --output /dist/kura-stage.tar.xz --exclude-logs --clean-cache
```
Arsip ini yang kemudian didistribusikan kepada pengguna akhir untuk instalasi *Gentoo-style*.

---

## 3. Diagram Alur Siklus Hidup Resep (*Build Lifecycle Flow*)

```
[ 1. FETCH ] --------> [ 2. VERIFY ] --------> [ 3. EXTRACT ]
Unduh tarball sumber   Validasi Hash SHA256    Ekstrak ke /tmp/forge/build
ke distfiles cache     dengan entri resep      berbasis RAM tmpfs
                                                       |
                                                       v
[ 6. STAGE (DESTDIR) ] <--- [ 5. BUILD ] <------- [ 4. PATCH ]
Pasang hasil kompilasi      Jalankan configure &  Terapkan patch khusus
ke /tmp/forge/stage/        make dengan CFLAGS    distro Kura Linux
         |
         v
[ 7. PRE-CHECK ] ----> [ 8. MERGE ] ---------> [ 9. REGISTER & HOOKS ]
Pindai collision       Salin file ke target /  Tulis /var/db/forge/ manifest,
dengan paket lain      (atau mock $FORGE_ROOT) picu OpenRC/ldconfig hooks
```

---

## 4. Contoh Standar Berkas Resep (`recipes/core/bash/recipe`)

```bash
# Forge Package Recipe Standard
pkgname="bash"
pkgver="5.3"
pkgrel="1"
pkgdesc="The GNU Bourne Again shell"
url="https://www.gnu.org/software/bash/"
license="GPL-3.0-or-later"
depends=("glibc" "ncurses" "readline")
makedepends=("gcc" "make" "bison")
sources=(
  "https://ftp.gnu.org/gnu/bash/bash-${pkgver}.tar.gz"
)
sha256sums=(
  "e7be46976ca8018e698ef65e90ab0192e10697962dbf37c35272a85e83ec90eb"
)

prepare() {
  cd "${srcdir}/bash-${pkgver}"
  # Terapkan patch upstream atau Kura Linux jika ada
}

build() {
  cd "${srcdir}/bash-${pkgver}"
  ./configure \
    --prefix=/usr \
    --without-bash-malloc \
    --with-installed-readline
  make
}

package() {
  cd "${srcdir}/bash-${pkgver}"
  make DESTDIR="${DESTDIR}" install
  ln -sf bash "${DESTDIR}/usr/bin/sh"
}
```

---

## 5. Standar File Konfigurasi (`/etc/forge/forge.conf`)

```ini
[general]
root = "/"
db_path = "/var/db/forge"
cache_path = "/var/cache/forge/distfiles"
build_path = "/tmp/forge/build"
stage_path = "/tmp/forge/stage"
recipes_path = "/var/db/forge/recipes"

[build]
cflags = "-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"
cxxflags = "-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"
ldflags = "-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now"
makeflags = "-j$(nproc)"
jobs = "auto"
prefix = "/usr"

[hooks]
enable_openrc_hooks = true
enable_ldconfig_hooks = true
enable_mandoc_hooks = true
auto_prompt_services = true
```
