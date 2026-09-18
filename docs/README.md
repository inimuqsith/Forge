# Direktori Dokumentasi & Spesifikasi Teknis Forge

> Direktori ini memuat spesifikasi teknis mendalam, manual operasional GitOps, dan panduan integrasi sistem untuk ekosistem **Forge Package Manager**.

---

## 📑 Daftar Panduan Teknis:

1. **[`GITOPS_AND_BUMPER_MANUAL.md`](file:///home/admin/Development/Forge/docs/GITOPS_AND_BUMPER_MANUAL.md)**
   - Manual operasional GitOps Recipe Registry.
   - Konfigurasi GitHub Webhook (`POST /v1/webhook/github`).
   - Panduan CLI `forge-server audit` & `forge-server bump <pkg|--all>`.
   - Konfigurasi Bot Cron GitHub Actions 6-jam (`recipe-auto-updater.yml`).

2. **[`RECIPE_SPECIFICATION.md`](file:///home/admin/Development/Forge/docs/RECIPE_SPECIFICATION.md)**
   - Standar format penulisan `recipe.toml` Kura Linux.
   - Anatomi metadata `[package]`, `[dependencies]`, `[sources]`, dan `[build]`.
   - Sintaks evaluasi USE Flags bersyarat (`flag? ( dep )`).
   - Multi-version slots (`slot = "0"`).
   - Variabel lingkungan subshell yang diinjeksi saat proses build.

---

## 🧭 Dokumen Induk Terkait:
- **Arsitektur Sistem Lengkap:** [`ARCHITECTURE.md`](file:///home/admin/Development/Forge/ARCHITECTURE.md)
- **Pedoman Pengembang & AI Assistant:** [`AGENTS.md`](file:///home/admin/Development/Forge/AGENTS.md)
- **Roadmap & ADR Tracker:** [`MEMORY.md`](file:///home/admin/Development/Forge/MEMORY.md)
