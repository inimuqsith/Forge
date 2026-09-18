# CLI Module (`src/cli/`)

Mengelola parsing argumen baris perintah, routing sub-perintah:
- `install`: Router resolusi hybrid & instalasi paket.
- `build`: Kompilasi terisolasi hingga tahap staging DESTDIR.
- `remove`: Penghapusan presisi berdasarkan manifest.
- `sync` / `update`: Sinkronisasi resep & pembaruan database.
- `cpu-dump`: Introspeksi CPU hardware & export profil.
- `import`: Server / CI/CD builder packaging & auto-upload.
- `stage-export`: Pengemasan stage tarball rootfs Kura Linux.
