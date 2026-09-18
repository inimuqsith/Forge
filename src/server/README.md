# Forge Server Daemon Module (`src/server/`)

Komponen backend server sentral untuk executable `forge-server`:
- `forge-server serve`: Menyajikan sinkronisasi pohon resep resmi ke seluruh klien (`forge sync`).
- Target CPU Profile Registry: Menerima dan mengindeks `cpu-profile.json` pengguna.
- Binary Library API & Catalog Indexer (`forge-server index` $\rightarrow$ `packages.db.zst`).
- Authentication & Upload Gateway untuk worker CI/CD.
