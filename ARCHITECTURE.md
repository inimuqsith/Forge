# ARCHITECTURE.md — Blueprint Arsitektur Package Manager `forge`

> **`forge`** adalah *Hybrid Unified & Source-based package manager* modern untuk **Kura Linux**. Forge memadukan fleksibilitas kompilasi sumber ala **Gentoo Portage** (*USE Flags*, *Slots*, *Package Sets*, kompilasi native silikon) dengan kecepatan deployment **Forge Server & CI/CD Builder Lock-CPU**, **Native Binhost**, serta kemampuan **Hybrid Fallback** terhadap repositori biner **CachyOS** (x86-64-v3/v4) dan **Arch Linux**.

---

## 1. Topologi & Lapisan Arsitektur Hybrid Forge

```
                                +--------------------------------------------+
                                |             Forge User Client              |
                                |  [ CLI Dispatcher, DAG Resolver, Profiler] |
                                +--------------------------------------------+
                                       |              |               |
               ┌───────────────────────┘              |               └───────────────────────┐
               │ (Level 1: Native Binhost)            │ (Level 2: Hybrid Fallback)            │ (Level 3: Source Build)
               v                                      v                                       v
+-----------------------------+        +-----------------------------+        +-----------------------------+
|    Forge Server Binhost     |        |    CachyOS / Arch Repos     |        |   Local RAM tmpfs Sandbox   |
| - Pre-compiled 100% Native  |        | - x86-64-v4 / v3 Binaries   |        | - Source fetch upstream     |
| - Locked to Target CPU/USE  |        | - Pacman/Zst Binary Adapter |        | - SHA256 integrity check    |
| - Zero Local Compilation    |        | - Fallback jika Forge biner |        | - Native silicon CFLAGS     |
| - Manifest-based Fast Merge |        |   belum tersedia di server  |        | - DESTDIR Staging & Merge   |
+-----------------------------+        +-----------------------------+        +-----------------------------+
               ^                                                                              ^
               │                                                                              │
               │               +---------------------------------------------+                │
               └────────────── |        Forge Server & CI/CD Builder         | ───────────────┘
                               | - Central Recipe Registry (Server-Hosted)   |   Sync Recipes
                               | - CI/CD Build Farm locked to Target CPU     |
                               | - `forge import`: Build, Package & Upload   |
                               | - `forge cpu-dump`: Profiling target CPU    |
                               +---------------------------------------------+
```

---

## 2. Tiga Tingkat Resolusi Paket (Hybrid Unified Engine)

Forge memberikan **kebebasan penuh kepada pengguna** untuk memilih strategi instalasi paket:

### 🥇 Tingkat 1: Forge Native Binhost (Rekomendasi Kecepatan & Native Maksimal)
- Klien menghubungi Forge Server Binhost.
- Server mencocokkan target **CPU Microarchitecture Profile** pengguna (misal: AMD Zen 4 `znver4` dengan AVX-512, atau `cpu-dump` hash) dan kumpulan **USE Flags** yang aktif.
- Jika biner yang cocok ditemukan: Forge langsung mengunduh paket biner `.forge.tar.zst` dan memasangnya secara deterministik via manifest (kecepatan setara binary distro, namun 100% native CPU pengguna).

### 🥈 Tingkat 2: Hybrid Fallback (CachyOS x86-64-v4/v3 & Arch Linux)
- Jika paket biner belum tersedia di Forge Server Binhost dan pengguna mengaktifkan opsi `enable_hybrid = true` di `/etc/forge/forge.conf` (atau flag `--allow-hybrid`):
- Forge menghubungi repositori biner teroptimasi **CachyOS** (target x86-64-v4 / x86-64-v3) atau **Arch Linux**.
- Binary adapter Forge mengekstrak paket `.pkg.tar.zst`, menyesuaikan layout UsrMerge & OpenRC Kura Linux, membuat manifest lokal, dan memasangnya ke sistem.

### 🥉 Tingkat 3: Local Source Compilation (Gentoo Portage Mode)
- Jika biner tidak tersedia, atau pengguna secara eksplisit meminta kompilasi lokal (`forge install --build-source <pkg>` atau preferensi `mode = "source"`):
- Forge mengunduh kode sumber upstream dari entri `sources` resep.
- Memvalidasi hash SHA256, mengekstrak ke RAM tmpfs (`/tmp/forge/build/`), menginjeksi CFLAGS native CPU pengguna, melakukan kompilasi terisolasi, memasang ke `DESTDIR`, dan melakukan transactional merge ke rootfs.

---

## 3. Ekosistem Server Forge & CI/CD Builder (Lock-CPU)

Untuk menghindari kompilasi berat berjam-jam di mesin laptop/klien lokal, ekosistem Forge menyertakan subsistem **Forge Server & CI/CD Builder**:

```
+-------------------------------------------------------------------------------+
|                       Forge Server & Build Farm                               |
+-------------------------------------------------------------------------------+
|                                                                               |
|  1. Recipe Registry Server:                                                   |
|     - Menyimpan seluruh resep resmi Kura Linux (di-hosting di server).        |
|     - Klien menyinkronkan resep via `forge sync`.                             |
|                                                                               |
|  2. Target CPU Profile Registry:                                              |
|     - Menerima `cpu-profile.json` hasil `forge cpu-dump` dari pengguna.       |
|     - Mengunci (*locks*) target compiler ke CPU pengguna (misal: `znver4`).   |
|                                                                               |
|  3. CI/CD Build Farm:                                                         |
|     - Worker node menjalankan kompilasi massal terisolasi.                    |
|     - Menggunakan tool internal: `forge import <pkg>`                         |
|     - Otomatis menghasilkan `.forge.tar.zst` + manifest + metadata.           |
|                                                                               |
|  4. Binary Library & Storage:                                                 |
|     - Menyimpan database repositori `packages.db.zst`.                        |
|     - CDN / HTTP Server melayani unduhan binary ke seluruh klien Kura Linux.  |
|                                                                               |
+-------------------------------------------------------------------------------+
```

---

## 4. Rincian Perintah Baru

### 🔬 1. `forge cpu-dump` (CPU Profiler & Hardware Introspection)
Mengekstrak identitas presisi prosesor, ekstensi instruksi (ISA), geometri cache, dan compiler flags optimal dari CPU mesin saat ini:
- **Perintah:**
  ```bash
  forge cpu-dump                    # Tampilkan info & simpan cpu-profile.json
  forge cpu-dump --export-cflags    # Tampilkan flag CFLAGS yang optimal
  forge cpu-dump --upload           # Unggah profil ke Forge Server untuk build farm
  ```
- **Contoh Struktur Output `cpu-profile.json`:**
  ```json
  {
    "architecture": "x86_64",
    "vendor": "AuthenticAMD",
    "model_name": "AMD Ryzen 7 8845HS w/ Radeon 780M Graphics",
    "family": 25,
    "model": 117,
    "target_march": "znver4",
    "isa_extensions": [
      "avx512f", "avx512vl", "avx512bw", "avx512dq", "avx512cd",
      "avx512_bf16", "avx512_vnni", "avx2", "fma", "vaes", "sha_ni",
      "bmi1", "bmi2", "aes", "sse4_2"
    ],
    "cache": {
      "l1d": "256 KiB",
      "l1i": "256 KiB",
      "l2": "8 MiB",
      "l3": "16 MiB"
    },
    "recommended_flags": {
      "cflags": "-O2 -march=znver4 -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt",
      "cxxflags": "-O2 -march=znver4 -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt",
      "ldflags": "-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now",
      "makeflags": "-j16"
    }
  }
  ```

---

### 📦 2. `forge import` / `forge impor` (Server Build, Package, & Auto-Upload)
Perintah sisi server / automated CI/CD worker untuk mengompilasi, mengemas, dan mempublikasikan paket ke Binary Library:
- **Alur Kerja `forge import`:**
  1. Membaca spesifikasi resep dari pohon resep server.
  2. Mengunci compiler ke target profil CPU pengguna (`--target-cpu=znver4` atau membaca `cpu-profile.json`).
  3. Mengompilasi paket di lingkungan sandbox chroot/container yang terisolasi.
  4. Memasang hasil ke staging `$DESTDIR`.
  5. Mengemas direktori staging menjadi arsip terkompresi `.forge.tar.zst` yang memuat:
     - `data.tar.zst` (seluruh payload berkas biner, library, konfigurasi).
     - `manifest` (daftar absolut berkas, izin hak akses, dan hash SHA256).
     - `metadata.json` (nama, versi, release, target CPU march, USE flags aktif, dependensi, slot).
  6. Mengindeks paket ke dalam database katalog repositori biner (`packages.db.zst`).
  7. Mengunggah berkas paket secara otomatis ke Forge Central Binary Library / Object Storage / CDN.
- **Perintah:**
  ```bash
  forge import <pkg>                          # Build & publish 1 paket
  forge import --target-cpu znver4 <pkg>      # Build untuk arsitektur CPU tertentu
  forge import --all-system                   # Build seluruh set @system untuk binhost
  ```

---

## 5. Fitur Filosofi Gentoo Portage di Forge

### 🎛️ 1. USE Flags (Granular Feature Control)
Memungkinkan pengguna mengaktifkan atau menonaktifkan fitur tertentu pada saat kompilasi paket:
- **Konfigurasi Global:** `/etc/forge/forge.conf` (`use = ["ssl", "openrc", "-systemd", "lto", "pgo"]`).
- **Konfigurasi Per-Paket:** `/etc/forge/package.use` (misal: `sys-apps/util-linux ncurses udev -systemd`).
- **Penerapan di Resep:**
  ```bash
  if forge_use ssl; then
    CONFIG_FLAGS+=( "--with-openssl" )
  else
    CONFIG_FLAGS+=( "--without-openssl" )
  fi
  ```

### 🏷️ 2. Package Slots (Multi-Version Coexistence)
Memungkinkan beberapa versi mayor dari satu paket terpasang secara bersamaan tanpa konflik:
- Contoh: `dev-lang/python:3.12` dan `dev-lang/python:3.13` atau `sys-devel/gcc:14` dan `sys-devel/gcc:15`.
- Manifest dan database di `/var/db/forge/installed/<pkg>-<ver>:<slot>/` melacak kepemilikan berkas secara independen.

### 📦 3. Package Sets (`@system` & `@world`)
- **`@system`**: Kumpulan paket esensial pembangun fondasi Kura Linux (Toolchain, Glibc, Kernel Linux Monolithic, OpenRC, Coreutils).
- **`@world`**: Seluruh paket yang diminta secara eksplisit oleh pengguna ditambah set `@system`.
- **Perintah:**
  ```bash
  forge install @system       # Rebuild seluruh basis sistem Kura Linux
  forge update @world         # Perbarui seluruh software yang terpasang di sistem
  ```

---

## 6. Format Resep Mandiri (`Recipe.forge`)

Seluruh resep Forge disimpan terpusat di server (`recipes/`) dan diunduh oleh klien melalui `forge sync`:

```bash
# Forge Recipe Format Standard
pkgname="openssh"
pkgver="9.8p1"
pkgrel="1"
slot="0"
pkgdesc="Premier connectivity tool for remote login with SSH protocol"
url="https://www.openssh.com/"
license="BSD-2-Clause"

# Gentoo-Style USE Flags
use_flags=("pam" "ssl" "kerberos" "ldns" "livecd")
default_use=("ssl" "pam")

depends=(
  "glibc"
  "openssl"
  "zlib"
)
makedepends=(
  "gcc"
  "make"
  "pkgconf"
)

sources=(
  "https://cdn.openbsd.org/pub/OpenBSD/OpenSSH/portable/openssh-${pkgver}.tar.gz"
)
sha256sums=(
  "dd8b5cedd4da0102d09f1665f14d8627e997f3944354b6dff618d6e3c10444a7"
)

prepare() {
  cd "${srcdir}/openssh-${pkgver}"
  # Terapkan patch distro Kura Linux jika ada
}

build() {
  cd "${srcdir}/openssh-${pkgver}"

  local conf_args=(
    --prefix=/usr
    --sysconfdir=/etc/ssh
    --with-privsep-path=/var/empty
    --with-privsep-user=sshd
    --with-ssl-dir=/usr
  )

  if forge_use pam; then
    conf_args+=( --with-pam )
  fi

  if forge_use kerberos; then
    conf_args+=( --with-kerberos5 )
  fi

  ./configure "${conf_args[@]}"
  make
}

package() {
  cd "${srcdir}/openssh-${pkgver}"
  make DESTDIR="${DESTDIR}" install

  # Pasang OpenRC Init Script bawaan Kura Linux
  install -Dm755 "${filesdir}/sshd.initd" "${DESTDIR}/etc/init.d/sshd"
  install -Dm644 "${filesdir}/sshd.confd" "${DESTDIR}/etc/conf.d/sshd"
}
```

---

## 7. Format Paket Biner Forge (`.forge.tar.zst`)

Paket biner yang dihasilkan oleh `forge import` atau diunduh dari Binhost memiliki struktur arsip terstandarisasi:

```
package-name-1.0.0-1-znver4.forge.tar.zst
├── data.tar.zst            # Payload sistem berkas (usr/bin/..., etc/...)
├── manifest                # Daftar seluruh path absolut & SHA256 file
├── metadata.json           # Info versi, target CPU (znver4), USE flags aktif, slot
└── hooks.sh                # Skrip pemicu OpenRC service, ldconfig, dll.
```

---

## 8. Standar File Konfigurasi Lengkap (`/etc/forge/forge.conf`)

```ini
[general]
root = "/"
db_path = "/var/db/forge"
cache_path = "/var/cache/forge/distfiles"
build_path = "/tmp/forge/build"
stage_path = "/tmp/forge/stage"
recipes_path = "/var/db/forge/recipes"

# Mode Resolusi Paket: "binhost" (prioritas biner), "source" (kompilasi lokal), "hybrid" (binhost -> fallback -> source), "interactive" (tanya pengguna)
mode = "hybrid"

[server]
# URL Forge Central Server untuk sinkronisasi resep & metadata
recipe_server = "https://recipes.kuralinux.org/v1"
# URL Forge Binary Library (Binhost)
binhost_url = "https://binhost.kuralinux.org/v1"
# API Token untuk otentikasi upload CI/CD (hanya di server)
server_api_token = ""

[binhost]
enable_binhost = true
auto_match_cpu = true
fallback_to_source = true

[hybrid]
# Fallback opsional ke binary repo CachyOS atau Arch Linux
enable_cachyos_fallback = true
cachyos_repo_url = "https://mirror.cachyos.org/repo/x86_64_v4/cachyos_v4"
enable_arch_fallback = false
arch_repo_url = "https://geo.mirror.pkgbuild.com/core/os/x86_64"

[cpu]
# Target arsitektur CPU (otomatis diisi dari `forge cpu-dump`)
target_march = "native"
enable_avx512 = true
enable_avx2 = true
profile_file = "/etc/forge/cpu-profile.json"

[build]
cflags = "-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"
cxxflags = "-O2 -march=native -pipe -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fno-plt"
ldflags = "-Wl,-O1 -Wl,--as-needed -Wl,-z,relro -Wl,-z,now"
makeflags = "-j$(nproc)"
jobs = "auto"
prefix = "/usr"

[use]
# USE Flags Global ala Portage
flags = "ssl openrc alsa -systemd lto pgo"

[hooks]
enable_openrc_hooks = true
enable_ldconfig_hooks = true
enable_mandoc_hooks = true
auto_prompt_services = true
```
