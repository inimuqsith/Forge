# Forge — The High-Performance Hybrid & Source Package Manager

> **Forge** adalah *Hybrid Unified & Source-based package manager* modern, deterministik, dan berkecepatan tinggi yang dirancang khusus untuk distribusi **Kura Linux**.

---

## ⚡ Fitur Utama & Filosofi Desain

- **🎛️ Filosofi Gentoo Portage:** Mendukung penuh *USE Flags* granular, *Slots* multi-versioning, *Package Sets* (`@system`, `@world`), dan kompilasi sumber terisolasi.
- **⚡ 3 Tingkat Resolusi Hybrid (Kebebasan Penuh Pengguna):**
  - **Tingkat 1 (Forge Native Binhost):** Unduh paket biner siap pakai yang 100% cocok dengan profil CPU & USE flags dari Forge Server.
  - **Tingkat 2 (Hybrid Fallback):** Fallback opsional ke repositori biner teroptimasi **CachyOS** (x86-64-v4 / v3) atau **Arch Linux**.
  - **Tingkat 3 (Local Source Compilation):** Kompilasi lokal dari source code dengan RAM tmpfs dan flag native silikon.
- **🔒 Server & CI/CD Builder (Lock CPU):** Server sentral dan build farm yang dikunci (*locked*) khusus ke arsitektur CPU pengguna (misal: AMD Zen 4 `znver4`, AVX-512) untuk mengompilasi paket secara terpusat.
- **🔬 Introspeksi Hardware (`forge cpu-dump`):** Menganalisis CPU host, instruksi ISA (AVX-512, AVX2, SSE4, dll.), cache, dan mengekspor `cpu-profile.json` untuk CI/CD.
- **📦 Server Build & Auto-Upload (`forge import`):** Tool server/CI/CD untuk kompilasi massal, packaging `.forge.tar.zst`, dan otomatisasi publikasi ke Binary Library.
- **🌐 Resep Terpusat di Server:** Seluruh resep resmi disimpan di server dan disinkronisasi ke klien via `forge sync`.
- **🛡️ Manifest Deterministik & Zero-Cruft:** Pelacakan berkas absolut untuk instalasi aman dan penghapusan bersih tanpa sisa.
- **⚙️ Integrasi OpenRC Native:** Otomatis mendeteksi skrip di `/etc/init.d/` dan terintegrasi dengan `rc-update`.
- **🏗️ Stage Exporter (`forge stage-export`):** Utilitas pengemas rootfs menjadi tarball distribusi Kura Linux (`kura-stage.tar.xz`).

---

## 🛠️ Ringkasan Perintah CLI

```bash
# --- 1. Manajemen Paket (Klien) ---
forge install <pkg>             # Pasang paket (otomatis pilih Binhost / Fallback / Source)
forge install --binhost <pkg>   # Paksa prioritaskan unduh pre-built binary native
forge install --build-source <pkg> # Paksa kompilasi lokal dari source code
forge install --interactive <pkg>  # Pilih manual provider (Forge Binhost vs CachyOS/Arch vs Source)
forge remove <pkg>              # Hapus paket secara bersih berdasarkan manifest
forge sync                      # Sinkronisasi pohon resep & index biner dari Forge Server
forge update @world             # Perbarui seluruh paket terpasang di sistem

# --- 2. Analisis Hardware & Profil CPU ---
forge cpu-dump                  # Dump mikroarsitektur CPU & simpan cpu-profile.json
forge cpu-dump --export-cflags  # Tampilkan rekomendasi CFLAGS untuk CPU saat ini
forge cpu-dump --upload         # Unggah profil CPU ke Forge Server untuk build farm

# --- 3. Server & CI/CD Builder Tools ---
forge import <pkg>              # Server/CI/CD: Build, kemas ke .forge.tar.zst, & upload ke binary library
forge import --all-system       # Server/CI/CD: Kompilasi massal seluruh paket @system yang di-lock ke CPU target

# --- 4. Informasi & Query ---
forge list                      # Tampilkan daftar seluruh paket terpasang & versinya
forge query <pkg>               # Tampilkan metadata, USE flags aktif, dependensi, & manifest
forge search <query>            # Cari resep paket berdasarkan nama/deskripsi

# --- 5. Fitur Distro Khusus ---
forge install @system           # Rebuild seluruh basis sistem Kura Linux 100% native
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
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
