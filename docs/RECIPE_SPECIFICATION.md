# Spesifikasi Format Resep Paket Kura Linux (`recipe.toml`)

> **Standar Format Resep All-in-One Kura Linux**: Mendefinisikan struktur TOML deklaratif, bagian dependencies (`depends` & `makedepends`), USE flags, slots multi-versi, siklus build, dan variabel lingkungan compiler.

---

## 1. Struktur Standar `recipe.toml`

Setiap paket dalam ekosistem Kura Linux didefinisikan dalam sebuah berkas `recipe.toml` tunggal yang diletakkan pada:
`/var/db/forge/recipes/<category>/<pkgname>/recipe.toml`

### Kategori Resmi:
1. `system/`: Fondasi OS inti, meta-paket (`base`), package manager & compiler toolchain (`forge`), C runtime library, dan init system OpenRC.
2. `core/`: Utilitas sistem, filesystem tools, networking, dan libraries esensial.
3. `extra/`: Perangkat lunak pengembangan, CLI modern, text editor, dan runtime bahasa.

---

## 2. Anatomi Lengkap Resep

```toml
[package]
name = "fastfetch"
version = "2.38.0"
release = 1
slot = "0"
description = "Like neofetch, but much faster because written in C"
license = "MIT"
upstream = "https://github.com/fastfetch-cli/fastfetch"

[dependencies]
# Dependensi runtime wajib (dibutuhkan saat binary dieksekusi)
runtime = ["glibc", "zlib"]

# Dependensi build-time (hanya dibutuhkan saat proses kompilasi)
build = ["cmake", "ninja", "pkgconf", "gcc"]

[sources]
urls = [
    "https://github.com/fastfetch-cli/fastfetch/archive/refs/tags/2.38.0.tar.gz"
]
sha256 = [
    "d99a9a5fbe7e9eb0eb66da9bc298ff2aa14594c9794cbdb702fa10e7b257da2e"
]

[build]
type = "cmake" # autotools | cmake | meson | custom
script = """
cd "${srcdir}/fastfetch-${pkgver}"
cmake -B build -G Ninja \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX=/usr
ninja -C build ${MAKEFLAGS}
DESTDIR="${DESTDIR}" ninja -C build install
"""
```

---

## 3. Variabel Lingkungan Bawaan Saat Build

Saat script subshell dieksekusi oleh `RecipeBuilder` di dalam sandbox Bubblewrap, variabel-variabel berikut diinjeksikan secara otomatis:

| Variabel | Deskripsi | Contoh Nilai |
| :--- | :--- | :--- |
| `${pkgname}` | Nama paket | `fastfetch` |
| `${pkgver}` | Versi paket | `2.38.0` |
| `${pkgrel}` | Release build nomor | `1` |
| `${srcdir}` | Direktori ekstraksi source code | `/tmp/forge/build/fastfetch-2.38.0/src` |
| `${DESTDIR}` | Direktori staging hasil kompilasi | `/tmp/forge/stage/fastfetch/` |
| `CC` | C Compiler (dengan Ccache) | `ccache clang` (atau `ccache gcc` untuk Glibc) |
| `CXX` | C++ Compiler (dengan Ccache) | `ccache clang++` |
| `CFLAGS` | Compiler flags native silikon | `-O3 -march=native -pipe -flto=thin ...` |
| `CXXFLAGS` | C++ flags | `-O3 -march=native -pipe -flto=thin ...` |
| `LDFLAGS` | Linker flags Mold | `-Wl,-O3 -Wl,--gc-sections -fuse-ld=mold` |
| `MAKEFLAGS` | Paralelisasi jobs Make/Ninja | `-j16` |

---

## 4. Evaluasi USE Flags Bersyarat

Dependensi dapat diberi kondisi USE flags:
- `flag? ( dep )`: Dependensi `dep` hanya diikutsertakan jika `flag` aktif.
- `!flag? ( dep )`: Dependensi `dep` hanya diikutsertakan jika `flag` tidak aktif.

Contoh:
```toml
[dependencies]
runtime = [
    "glibc",
    "ssl? ( openssl )",
    "openrc? ( openrc )"
]
```
