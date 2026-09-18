# Core Engine (`src/core/`)

Modul inti Forge mencakup:
- DAG Resolver & Topological Sort untuk dependensi (`depends`, `makedepends`).
- Engine USE Flags & evaluasi status fitur paket (`forge_use`).
- Engine Slotting untuk koeksistensi multi-versi (`pkg:slot`).
- Engine Transactional Merger, collision checker, & flat-file database manager (`/var/db/forge/`).
- Staging runner & injektor flag native silikon.
- OpenRC hook dispatcher & triggers.
