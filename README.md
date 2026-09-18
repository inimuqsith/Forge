# Forge — The High-Performance Source-First Package Manager

> **Forge** adalah *Source-First & Hybrid Package Manager* modern, deterministik, dan berkecepatan tinggi yang dirancang khusus untuk distribusi **Kura Linux**. Mengusung filosofi sejati **Gentoo Portage** (*Source-First Native Compilation*, *USE Flags*, *Slots*, *Package Sets*), Forge mengompilasi perangkat lunak secara langsung dari kode sumber 100% native silikon, dengan dukungan ekosistem terpisah **`forge-server`** untuk akselerasi *Binhost Lock-CPU*.

---

## ⚡ Fitur Utama & Filosofi Desain

- **🚀 Source-First Native Compilation (Gentoo Mode):** Secara default mengompilasi paket langsung dari kode sumber upstream dengan flag arsitektur native target (`-O2 -march=native -pipe`) dan isolasi RAM `tmpfs`.
- **🎛️ Granular USE Flags:** Mengaktifkan/menonaktifkan fitur perangkat lunak secara presisi di level global (`forge.conf`) atau per-paket (`package.use`).
- **🏷️ Multi-Version Slots:** Menjalankan beberapa versi mayor paket secara berdampingan tanpa konflik (misal: Python 3.12 & 3.13, GCC multi-versi).
- **📦 Meta-Target `@system` & `@world`:** Kompilasi ulang seluruh basis sistem Kura Linux dalam satu perintah (`forge install @system`).
- **⚡ Opsi Akselerasi Hybrid & Binhost (Opt-In):**
  - **Forge Native Binhost (`--binhost`):** Akselerasi unduhan biner terkompilasi native yang di-lock ke profil CPU pengguna.
  - **Hybrid Fallback (`--hybrid`):** Fallback biner opsional ke repositori **CachyOS** (x86-64-v4/v3) atau **Arch Linux**.
- **🔬 Introspeksi Hardware (`forge cpu-dump`):** Menganalisis CPU host, ekstensi ISA (AVX-512, AVX2, SSE4, dll.), cache, dan mengekspor profil hardware `cpu-profile.json`.
- **🏭 Suite Terpisah `forge-server`:** Daemon server resep terpusat dan worker CI/CD builder (`forge-server import`) yang mengompilasi paket secara massal untuk target CPU pengguna.
- **🛡️ Manifest Deterministik & Zero-Cruft:** Pelacakan berkas absolut untuk instalasi aman dan penghapusan bersih tanpa sisa.
- **⚙️ Integrasi OpenRC Native:** Otomatis mendeteksi skrip di `/etc/init.d/` dan terintegrasi dengan `rc-update`.
- **🏗️ Stage Exporter (`forge stage-export`):** Utilitas pengemas rootfs menjadi tarball distribusi Kura Linux (`kura-stage.tar.xz`).

---

## 🛠️ Panduan Perintah CLI

### A. Klien Pengguna (`forge`)

```bash
# --- 1. Manajemen Paket (Default: Source Compilation First) ---
forge install <pkg>             # Kompilasi dari source code secara native (Default Gentoo-style)
forge install --binhost <pkg>   # Opsi Akselerasi: Unduh pre-built binary native dari Forge Server
forge install --hybrid <pkg>    # Opsi Akselerasi: Gunakan biner CachyOS/Arch jika ada
forge install --interactive <pkg> # Pilih manual provider (Source vs Binhost vs CachyOS/Arch)
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync                      # Sinkronisasi pohon resep dari Forge Server
forge update @world             # Re-kompilasi / perbarui seluruh paket terpasang

# --- 2. Introspeksi Hardware ---
forge cpu-dump                  # Dump mikroarsitektur CPU & simpan cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini
forge cpu-dump --upload         # Unggah profil CPU ke Forge Server untuk CI/CD build farm

# --- 3. Informasi & Query ---
forge list                      # Tampilkan daftar seluruh paket terpasang & versinya
forge query <pkg>               # Tampilkan metadata, USE flags aktif, dependensi, & manifest
forge search <query>            # Cari resep paket berdasarkan nama/deskripsi

# --- 4. Fitur Distro Khusus ---
forge install @system           # Rebuild seluruh basis sistem Kura Linux 100% native
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

### B. Infrastruktur Server & CI/CD (`forge-server`)

```bash
# --- Manajemen Server & CI/CD Builder ---
forge-server serve              # Jalankan service API resep & katalog biner
forge-server import <pkg>       # CI/CD: Build lock-CPU, kemas .forge.tar.zst, & upload ke binary library
forge-server import --all-system # CI/CD: Kompilasi massal seluruh paket @system yang di-lock ke CPU target
forge-server index              # Regenerasi database index repositori packages.db.zst
```

---

## 📜 Standar Format Resep (`Recipe.forge`)

```bash
pkgname="openssh"
pkgver="9.8p1"
pkgrel="1"
slot="0"
pkgdesc="Premier connectivity tool for remote login with SSH protocol"
url="https://www.openssh.com/"
license="BSD-2-Clause"

# Portage-style USE Flags
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

build() {
  cd "${srcdir}/openssh-${pkgver}"
  local conf_args=(
    --prefix=/usr
    --sysconfdir=/etc/ssh
    --with-ssl-dir=/usr
  )

  if forge_use pam; then
    conf_args+=( --with-pam )
  fi

  ./configure "${conf_args[@]}"
  make
}

package() {
  cd "${srcdir}/openssh-${pkgver}"
  make DESTDIR="${DESTDIR}" install
}
```

---

## 🧭 Dokumentasi Pengembangan & Panduan AI

- **Pedoman AI & Protokol Mutlak HITL:** Lihat [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- **Blueprint Arsitektur & Desain Sistem:** Lihat [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- **Memori Persisten, Roadmap & ADR:** Lihat [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
