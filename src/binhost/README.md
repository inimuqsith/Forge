# Binhost Client Module (`src/binhost/`)

Mengelola interaksi klien dengan Forge Central Binary Library:
- Pengunduhan index repositori biner (`packages.db.zst`).
- Pencocokan biner yang 100% kompatibel dengan CPU microarchitecture & USE flags aktif.
- Pengunduhan arsip `.forge.tar.zst` & verifikasi checksum SHA256/BLAKE3.
- Ekstraksi biner langsung ke staging dan fast transactional merge.
