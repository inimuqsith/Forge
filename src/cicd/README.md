# CI/CD Builder & Importer Module (`src/cicd/`)

Mengimplementasikan fungsionalitas worker build server dan `forge-server import`:
- Eksekusi kompilasi otomatis di build farm yang di-lock ke profil CPU target pengguna (`znver4`, dll.).
- Otomatisasi pengemasan hasil build staging ke arsip biner `.forge.tar.zst` + manifest + hash metadata.
- Pembangunan ulang database index repositori biner server (`forge-server index`).
- Otomatisasi publikasi/upload artefak ke Binary Library / CDN.
