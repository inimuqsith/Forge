# Hybrid Binary Adapter Module (`src/hybrid/`)

Menyediakan provider fallback terhadap repositori binary eksternal:
- Adapter repositori CachyOS (arsip x86-64-v4 / x86-64-v3 `.pkg.tar.zst`).
- Adapter repositori Arch Linux (arsip standar x86-64).
- Konversi metadata paket asing ke manifest format Forge.
- Penyesuaian layout path UsrMerge dan penanganan skrip OpenRC.
