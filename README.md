# Forge — The High-Performance Source Package Manager

> **Forge** adalah *source-based package manager* modern, cepat, deterministik, dan berbobot ringan yang dirancang khusus sebagai penggerak ekosistem distribusi **Kura Linux**.

---

## ⚡ Fitur Utama

- **🚀 100% Native Silicon Compilation:** Mengompilasi seluruh software langsung dari sumber dengan menyuntikkan flag arsitektur native target (`-O2 -march=native -pipe`).
- **🛡️ Manifest-Driven Precision:** Pelacakan berkas absolut saat instalasi dan penghapusan paket untuk menjamin sistem bebas sampah (*zero cruft*).
- **⚡ Fast RAM tmpfs Builds:** Proses ekstraksi dan kompilasi terisolasi di `/tmp/forge/build/` berbasis memori RAM tmpfs.
- **🔄 Staging Sandboxing (DESTDIR):** Paket dikompilasi dan dipasang ke staging area sebelum transaksi merge ke rootfs riil.
- **⚙️ OpenRC Native Integration:** Mendeteksi otomatis skrip daemon di `/etc/init.d/` dan terintegrasi mulus dengan `rc-update`.
- **📦 Meta-Target `@system`:** Mendukung kompilasi dan pembaruan massal seluruh base system Kura Linux dalam satu perintah.
- **🏗️ Stage Exporter (`forge stage-export`):** Utilitas bawaan untuk mengemas rootfs menjadi tarball distribusi Kura Linux (`kura-stage.tar.xz`).
- **🔒 Flat-File Database:** Database `/var/db/forge/` yang tangguh dan mandiri tanpa ketergantungan pada runtime SQL eksternal.

---

## 📁 Standar Lokasi Filesystem

| Jalur | Fungsi |
| :--- | :--- |
| `/etc/forge/forge.conf` | Berkas konfigurasi global Forge & flag kompilasi distro |
| `/var/db/forge/installed/` | Database paket terpasang (manifest, metadata.json, dependencies) |
| `/var/db/forge/world` | Daftar paket eksplisit yang diminta oleh pengguna |
| `/var/cache/forge/distfiles/` | Cache penyimpanan berkas arsip sumber (tarball / zip) |
| `/tmp/forge/build/` | Area ekstraksi dan kompilasi sumber (RAM tmpfs) |
| `/tmp/forge/stage/` | Area staging sementara sebelum merge ke sistem |

---

## 🛠️ Perintah CLI Dasar

```bash
# --- Manajemen Paket ---
forge install <pkg>         # Unduh, verifikasi, kompilasi, & pasang paket ke rootfs
forge build <pkg>           # Kompilasi paket hanya sampai tahap staging (tanpa pasang)
forge remove <pkg>          # Hapus paket secara bersih berdasarkan manifest
forge update                # Sinkronisasi & perbarui pohon resep lokal
forge clean                 # Bersihkan cache sementara di /tmp/forge/

# --- Informasi & Query ---
forge list                  # Tampilkan daftar seluruh paket yang terpasang
forge query <pkg>           # Tampilkan metadata, dependensi, dan isi file paket
forge search <query>        # Cari resep paket berdasarkan nama/deskripsi

# --- Fitur Distro Khusus ---
forge install @system       # Rebuild seluruh sistem Kura Linux 100% native
forge stage-export --output kura-stage.tar.xz  # Kemas rootfs menjadi stage tarball
```

---

## 📜 Standar Format Resep (`recipe`)

```bash
pkgname="nano"
pkgver="8.3"
pkgrel="1"
pkgdesc="Pico editor clone with enhanced features"
url="https://www.nano-editor.org/"
license="GPL-3.0-or-later"
depends=("glibc" "ncurses")
makedepends=("gcc" "make" "pkgconf")
sources=(
  "https://www.nano-editor.org/dist/v8/nano-${pkgver}.tar.xz"
)
sha256sums=(
  "5fb7d206f582f3496f30a91ca5dc6f9de5efb2beab314227f465c490a2a5dc94"
)

build() {
  cd "${srcdir}/nano-${pkgver}"
  ./configure \
    --prefix=/usr \
    --sysconfdir=/etc \
    --enable-utf8
  make
}

package() {
  cd "${srcdir}/nano-${pkgver}"
  make DESTDIR="${DESTDIR}" install
}
```

---

## 🧭 Dokumentasi Pengembangan & Pedoman AI

- **Panduan Pengembangan & Aturan HITL:** Lihat [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md).
- **Blueprint & Desain Arsitektur:** Lihat [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md).
- **Memori Persisten, Roadmap & ADR:** Lihat [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md).
