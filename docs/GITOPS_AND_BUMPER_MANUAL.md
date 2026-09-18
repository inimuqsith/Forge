# Manual Operasional: GitOps Recipe Registry, GitHub Webhook & Upstream Bumper

> **Panduan Teknis Resmi Kura Linux**: Arsitektur GitOps, konfigurasi Webhook GitHub, penggunaan CLI `forge-server audit` & `forge-server bump`, integrasi GitHub Actions Cron Bot, dan sinkronisasi live server `https://pkgkura.amqs.net`.

---

## 1. Topologi GitOps & Arsitektur Single Source of Truth (SSOT)

Seluruh definisi resep paket Kura Linux (`recipes/`) dikelola secara terpusat di repositori GitHub **`https://github.com/inimuqsith/Forge`** pada branch **`main`**.

```mermaid
flowchart TD
    Maintainer["👨‍💻 Maintainer / Bot"] -->|git push origin main| GitHub["🐙 GitHub (inimuqsith/Forge:main)"]
    GitHub -->|Event Push Webhook| WebhookEndpoint["⚡ POST https://pkgkura.amqs.net/v1/webhook/github"]
    WebhookEndpoint -->|git pull --rebase| VPS["🏢 Server VPS (Ubuntu Docker Container)"]
    VPS -->|bundle_recipes()| Tarball["📦 /var/cache/forge/server/recipes.tar.zst"]
    VPS -->|generate_sha256()| SHA["🔑 /var/cache/forge/server/latest.sha256"]
    VPS -->|scan_packages()| Explorer["🌐 In-Memory Public Web Explorer"]

    UserClient["💻 Client forge sync"] -->|GET latest.sha256| VPS
    UserClient -->|GET latest.tar.zst| VPS
    UserClient -->|Extract Atomik| LocalRecipes["📁 /var/db/forge/recipes/"]
```

---

## 2. Pengecekan Versi Hulu Multi-Tier (Zero Quota Probing)

Engine `RecipeBumper` menggunakan sistem 3-tingkat probing untuk menjamin audit 100% andal tanpa terblokir batas kuota API:

1. **Tier 1 — GitHub REST API (Authenticated):**
   - Mendeteksi environment variable `GITHUB_TOKEN` atau `GH_TOKEN`.
   - Melakukan query ke `https://api.github.com/repos/{owner}/{repo}/releases/latest`.
   - Kuota: 1,000 – 5,000 requests/jam.
2. **Tier 2 — GitHub Atom Feed (`/releases.atom`):**
   - Fallback otomatis saat tidak ada token atau saat kuota habis (*Rate-Limit 403*).
   - Membaca feed XML `https://github.com/{owner}/{repo}/releases.atom`.
   - **Bebas Kuota (Zero Quota):** Tidak memiliki batasan kuota request dan sangat cepat.
3. **Tier 3 — Anitya / Release-Monitoring.org v2 Projects API:**
   - Membaca database rilis terbuka `https://release-monitoring.org/api/v2/projects/?name={pkg}`.
   - Digunakan untuk paket non-GitHub (GNU, Linux Kernel, OpenRC, Sourceforge, dll.).

---

## 3. Panduan Penggunaan CLI `forge-server`

### A. Memeriksa Status Versi Hulu Seluruh Paket (`audit`):
```bash
forge-server audit [--recipes-path <DIR>]
```
Output menampilkan tabel ringkasan:
```text
=== Forge Server Upstream Recipe Version Audit ===
Memindai direktori resep: recipes

PAKET                  KATEGORI   VERSI LOKAL  VERSI HULU     STATUS   SUMBER / PROVIDER
-----------------------------------------------------------------------------------------------
acl                    core       2.4.0        2.4.0          [LATEST] Anitya (Release-Monitoring.org)
fastfetch              extra      2.38.0       2.38.0         [LATEST] GitHub (fastfetch-cli/fastfetch)
gcc                    system     16.2.0       16.2.0         [LATEST] Anitya (Release-Monitoring.org)
glibc                  system     2.44         2.44           [LATEST] Anitya (Release-Monitoring.org)
mold                   system     2.42.1       2.42.1         [LATEST] GitHub (rui314/mold)
-----------------------------------------------------------------------------------------------
Total paket: 107 | Paket mutakhir: 107 | Perlu diperbarui: 0
```

### B. Memperbarui Paket ke Versi Hulu Terbaru (`bump`):
```bash
# 1. Bump satu paket spesifik dan push otomatis ke GitHub SSOT:
forge-server bump fastfetch

# 2. Bump satu paket ke versi kustom tertentu:
forge-server bump fastfetch --version 2.39.0

# 3. Bump SEMUA paket yang memiliki rilis hulu baru secara massal:
forge-server bump --all

# 4. Bump tanpa langsung melakukan git push (hanya edit lokal):
forge-server bump --all --no-push
```

---

## 4. Konfigurasi GitHub Actions Bot (Otomasi 6 Jam)

Berkas alur kerja di `.github/workflows/recipe-auto-updater.yml`:
- **Jadwal:** Berjalan setiap 6 jam (`cron: '0 */6 * * *'`) atau dapat dipicu manual dari tab *Actions* di GitHub (*Workflow Dispatch*).
- **Alur Eksekusi:**
  1. Melakukan `checkout` repositori dengan `secrets.GITHUB_TOKEN`.
  2. Mengompilasi `forge-server`.
  3. Menjalankan `forge-server audit`.
  4. Menjalankan `forge-server bump --all --no-push`.
  5. Jika terdapat perubahan resep: membuat commit `chore(recipes): automated upstream recipe version bump [skip ci]` dan melakukan `git push origin main`.

---

## 5. Konfigurasi Webhook di Server VPS

- **Endpoint Webhook:** `POST https://pkgkura.amqs.net/v1/webhook/github`
- **Events:** `push` event pada branch `main`.
- **Status Health Check Webhook:** Buka browser di `https://pkgkura.amqs.net/v1/webhook/github` untuk melihat status kesiapan dan jumlah paket aktif.
