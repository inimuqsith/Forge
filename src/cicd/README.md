# CI/CD Builder & Import Module (`src/cicd/`)

Mengimplementasikan fungsionalitas worker server dan `forge import`:
- Eksekusi kompilasi otomatis yang di-lock ke profil CPU target pengguna.
- Otomatisasi pengemasan hasil build staging ke arsip `.forge.tar.zst` + manifest + hash metadata.
- Pembangunan ulang database index repositori biner server.
- Otomatisasi upload artefak ke Binary Library / CDN.
