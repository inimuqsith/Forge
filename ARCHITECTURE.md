# ARCHITECTURE.md — Blueprint Arsitektur Package Manager `forge` & `forge-server`

> **`forge`** adalah *Source-First Hybrid Package Manager* modern untuk **Kura Linux**. Berakar kuat pada filosofi **Gentoo Portage** (*Source-First Native Compilation*, *Granular USE Flags*, *Multi-Version Slots*, *Package Sets*), Forge mengutamakan kompilasi kode sumber 100% native silikon di mesin pengguna, didukung suite terpisah **`forge-server`** untuk penyediaan repositori resep terpusat dan **CI/CD Build Farm Lock-CPU** untuk akselerasi biner opsional.

---

## 1. Topologi Sistem: Pemisahan `forge` (Klien) & `forge-server` (Server)

```
                                      [ PENGGUNA KURA LINUX ]
                                                 │
                                                 ▼
+-------------------------------------------------------------------------------------------------+
|                                 Forge Client CLI (`forge`)                                      |
+-------------------------------------------------------------------------------------------------+
|  - CLI Dispatcher & Interactive Prompt Layer                                                    |
|  - Dependency Graph & Topological DAG Resolver (depends & makedepends)                         |
|  - USE Flags Engine (`forge_use`) & Multi-Version Slotting (`pkg:slot`)                         |
|  - Hardware Profiler (`forge cpu-dump`) -> Ekspor cpu-profile.json                             |
|  - Manifest Tracker & Transactional Merger (/var/db/forge/)                                    |
|  - OpenRC Service Discovery & System Hooks Trigger                                              |
+-------------------------------------------------------------------------------------------------+
          │                                      │                                      │
          │ 🥇 PRIORITAS UTAMA (DEFAULT)         │ 🥈 OPSI AKSELERASI (OPT-IN)          │ 🥉 OPSI FALLBACK (OPT-IN)
          ▼                                      ▼                                      ▼
+-------------------------+            +-------------------------+            +-------------------------+
| Local RAM tmpfs Builder |            |  Forge Native Binhost   |            |  CachyOS / Arch Adapter |
| - Fetch source upstream |            | - Unduh .forge.tar.zst  |            | - Unduh .pkg.tar.zst    |
| - Verify SHA256/BLAKE3  |            | - Dicocokkan ke profil  |            | - Adaptasi layout root  |
| - Inject native CFLAGS  |            |   CPU user (znver4) &   |            |   & buat manifest local |
| - RAM tmpfs compilation |            |   USE flags aktif       |            |                         |
| - DESTDIR Staging       |            |                         |            |                         |
+-------------------------+            +-------------------------+            +-------------------------+
          ▲                                      ▲
          │ Sync Recipes                         │ Unduh Pre-compiled Binaries
          └──────────────────┐ ┌─────────────────┘
                             │ │
+-------------------------------------------------------------------------------------------------+
|                               Forge Server Suite (`forge-server`)                               |
+-------------------------------------------------------------------------------------------------+
|  1. `forge-server serve` : Recipe Registry HTTP/API & Binary Catalog Server (`packages.db.zst`)|
|  2. `forge-server import`: CI/CD Worker Builder yang di-lock ke profil CPU pengguna           |
|  3. `forge-server index` : Generator database index katalog resep & paket biner                 |
|  4. Binary Storage       : Object Storage / CDN penampung arsip `.forge.tar.zst`                |
+-------------------------------------------------------------------------------------------------+
```

---

## 2. Hierarki Resolusi Paket: Source-First (Gentoo Portage Philosophy)

Forge menganut filosofi **Source-First Compilation**: software paling optimal, aman, dan berdaya guna tinggi adalah software yang dikompilasi langsung dari kode sumber untuk mikroarsitektur silikon pengguna.

### 🥇 Tingkat 1: Local Source Compilation (Prioritas Bawaan / Default)
- **Alur Utama:** Ketika pengguna menjalankan `forge install <pkg>`:
  1. Forge membaca resep mandiri (`Recipe.forge`) dari pohon resep lokal (hasil `forge sync`).
  2. Memeriksa ketersediaan dependensi runtime (`depends`) dan build-time (`makedepends`).
  3. Mengunduh source code upstream ke cache `/var/cache/forge/distfiles/`.
  4. Memvalidasi hash kriptografis SHA256/BLAKE3.
  5. Mengekstrak berkas ke area RAM `tmpfs` berkecepatan tinggi (`/tmp/forge/build/<pkg>`).
  6. Menginjeksi compiler flags native silikon (`-O2 -march=native -pipe`) dan mengaktifkan cabang kode sesuai *USE Flags* pengguna.
  7. Memasang ke staging directory (`DESTDIR`) `/tmp/forge/stage/<pkg>`.
  8. Menjalankan pre-flight collision check, melakukan transactional merge ke rootfs (`/`), mencatat manifest di `/var/db/forge/installed/`, dan memicu hook OpenRC.

---

### 🥈 Tingkat 2: Forge Native Binhost (Opsi Akselerasi Terpusat)
- Pengguna yang memiliki koneksi internet cepat atau ingin melewati kompilasi lokal yang berat (misal: Chromium, LLVM) dapat mengaktifkan opsi Binhost melalui flag `--binhost` atau konfigurasi `mode = "binhost"` / `prefer_binhost = true`.
- Forge Klien mengirimkan hash profil CPU (dari `cpu-profile.json`) dan daftar USE flags aktif ke `forge-server`.
- Jika `forge-server` memiliki biner yang telah di-build secara identik oleh worker CI/CD, paket `.forge.tar.zst` langsung diunduh dan dipasang secara instan.

---

### 🥉 Tingkat 3: Hybrid Fallback (CachyOS x86-64-v4/v3 & Arch Linux)
- Opsi fallback opsional bagi pengguna yang membutuhkan software yang belum memiliki resep Forge resmi atau belum di-build di binhost.
- Diaktifkan melalui flag `--hybrid` / `--allow-hybrid` atau konfigurasi `enable_cachyos_fallback = true`.
- Adapter `src/hybrid/` mengekstrak arsip `.pkg.tar.zst`, menyesuaikan skema UsrMerge & OpenRC, serta menyusun manifest Forge lokal agar unmerge tetap 100% bersih.

---

## 3. Komponen Pengguna: `forge` CLI Client

Klien baris perintah yang dijalankan oleh pengguna di workstation Kura Linux:

| Perintah | Deskripsi |
| :--- | :--- |
| `forge install <pkg>` | **(Default)** Mengompilasi dan memasang paket langsung dari source code secara native |
| `forge install --binhost <pkg>` | Mengutamakan unduhan biner pre-compiled dari Forge Server |
| `forge install --hybrid <pkg>` | Mengizinkan fallback ke repositori CachyOS / Arch Linux |
| `forge install --interactive <pkg>` | Menampilkan menu interaktif bagi pengguna untuk memilih provider instalasi |
| `forge build <pkg>` | Mengompilasi paket dari source code hanya sampai tahap staging `DESTDIR` |
| `forge remove <pkg>` | Menghapus paket secara bersih berdasarkan manifest di `/var/db/forge/` |
| `forge sync` | Menyinkronisasikan pohon resep terbaru dari `forge-server` |
| `forge update @world` | Memperbarui dan mengompilasi ulang seluruh software terpasang di sistem |
| `forge cpu-dump` | Menganalisis CPU host, ekstensi ISA, cache, dan mengekspor `cpu-profile.json` |
| `forge cpu-dump --export-cflags` | Menampilkan CFLAGS yang paling optimal untuk silikon saat ini |
| `forge stage-export` | Mengemas rootfs aktif menjadi arsip stage distribusi (`kura-stage.tar.xz`) |
| `forge list` | Menampilkan seluruh paket terpasang beserta versi dan slot |
| `forge query <pkg>` | Menampilkan metadata rinci, USE flags aktif, dan manifest file |
| `forge search <query>` | Mencari resep paket dalam katalog lokal |

---

## 4. Komponen Server & CI/CD: `forge-server` Suite

Suite aplikasi backend terpisah untuk pengelolaan repositori terpusat dan otomasi build farm:

### 🌐 1. `forge-server serve` (Registry & Catalog Server)
- Menyajikan endpoint HTTP REST / WebSocket untuk:
  - Distribusi tarball pohon resep ke klien (`/v1/recipes/sync`).
  - Katalog biner terkompilasi dan metadata dependensi (`/v1/binhost/packages.db.zst`).
  - Endpoint pendaftaran profil CPU (`/v1/cpu-profiles/register`).

### 🏭 2. `forge-server import` (CI/CD Worker Lock-CPU)
- Perintah otomasi di sisi worker CI/CD untuk:
  1. Membaca target profil CPU pengguna (misal: `znver4` dengan AVX-512 dari `cpu-profile.json`).
  2. Mengunci compiler ke flag silikon target tersebut.
  3. Mengompilasi resep paket di lingkungan container/chroot sandbox terisolasi.
  4. Memasang ke staging `DESTDIR`.
  5. Mengemas menjadi arsip biner `.forge.tar.zst` yang memuat `data.tar.zst`, `manifest`, `metadata.json`, dan `hooks.sh`.
  6. Mengindeks paket ke dalam database katalog repositori `packages.db.zst`.
  7. Mengunggah biner secara otomatis ke Binary Library Storage / CDN.
- **Sintaks:**
  ```bash
  forge-server import <pkg>                     # Build & upload 1 paket
  forge-server import --target-cpu znver4 <pkg> # Kunci arsitektur target ke znver4
  forge-server import --all-system              # Rebuild seluruh set @system di CI/CD
  ```

### 🗂️ 3. `forge-server index` (Catalog Indexer)
- Memindai seluruh berkas `.forge.tar.zst` dalam direktori storage dan membangun ulang database index terkompresi `packages.db.zst`.

---

## 5. Fitur Filosofi Gentoo Portage di Forge

### 🎛️ 1. Granular USE Flags
Memungkinkan pengguna menyesuaikan fitur software sebelum kompilasi:
- Global: `/etc/forge/forge.conf` (`flags = "ssl openrc alsa -systemd lto pgo"`).
- Per-Paket: `/etc/forge/package.use` (`sys-apps/util-linux ncurses udev -systemd`).
- Pengujian di Resep:
  ```bash
  if forge_use ssl; then
    conf_args+=( --with-openssl )
  fi
  ```

### 🏷️ 2. Multi-Version Slots
Mendukung koeksistensi beberapa versi mayor tanpa tabrakan berkas:
- Contoh: `dev-lang/python:3.12` dan `dev-lang/python:3.13`.
- Database `/var/db/forge/installed/<pkg>-<ver>:<slot>/` mencatat manifest masing-masing versi.

### 📦 3. Package Sets (`@system` & `@world`)
- **`@system`**: Fondasi distro Kura Linux (Toolchain, Glibc, Linux Kernel Monolithic, OpenRC, Coreutils).
- **`@world`**: Seluruh aplikasi yang terpasang secara eksplisit oleh pengguna + `@system`.
- Kompilasi ulang seluruh basis OS secara native: `forge install @system`.

---

## 6. Format Resep Mandiri (`Recipe.forge`)

```bash
# Forge Package Recipe Standard
pkgname="openssh"
pkgver="9.8p1"
pkgrel="1"
slot="0"
pkgdesc="Premier connectivity tool for remote login with SSH protocol"
url="https://www.openssh.com/"
license="BSD-2-Clause"

# Gentoo-Style USE Flags
use_flags=("pam" "ssl" "kerberos" "ldns")
default_use=("ssl" "pam")

depends=("glibc" "openssl" "zlib")
makedepends=("gcc" "make" "pkgconf")

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

  # Pasang skrip layanan OpenRC Kura Linux
  install -Dm755 "${filesdir}/sshd.initd" "${DESTDIR}/etc/init.d/sshd"
  install -Dm644 "${filesdir}/sshd.confd" "${DESTDIR}/etc/conf.d/sshd"
}
```

---

## 7. Format Paket Biner Forge (`.forge.tar.zst`)

Paket biner yang dihasilkan oleh `forge-server import` memiliki struktur arsip terstandarisasi:

```
package-name-1.0.0-1-znver4.forge.tar.zst
├── data.tar.zst            # Payload sistem berkas (usr/bin/..., etc/...)
├── manifest                # Daftar seluruh path absolut & SHA256 file
├── metadata.json           # Info versi, target CPU (znver4), USE flags aktif, slot
└── hooks.sh                # Skrip pemicu OpenRC service, ldconfig, dll.
```

---

## 8. Konfigurasi Klien Default (`/etc/forge/forge.conf`)

```ini
[general]
root = "/"
db_path = "/var/db/forge"
cache_path = "/var/cache/forge/distfiles"
build_path = "/tmp/forge/build"
stage_path = "/tmp/forge/stage"
recipes_path = "/var/db/forge/recipes"

# Mode Bawaan: "source" (Source-First Kompilasi Lokal ala Gentoo)
# Opsi Lain: "binhost", "hybrid", "interactive"
mode = "source"

[server]
# URL Forge Central Server untuk sinkronisasi resep
recipe_server = "https://recipes.kuralinux.org/v1"
# URL Forge Central Binary Library (Binhost)
binhost_url = "https://binhost.kuralinux.org/v1"

[binhost]
# Opsi akselerasi unduh binary native dari Forge Server
enable_binhost = false
auto_match_cpu = true
fallback_to_source = true

[hybrid]
# Opsi akselerasi fallback ke binary CachyOS / Arch Linux
enable_cachyos_fallback = false
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
# USE Flags Global Kura Linux
flags = "ssl openrc alsa -systemd lto pgo"

[hooks]
enable_openrc_hooks = true
enable_ldconfig_hooks = true
enable_mandoc_hooks = true
auto_prompt_services = true
```
