#!/usr/bin/env python3
"""
Recipe Bumper - Auto-updates recipe versions, downloads tarball streams, calculates SHA256 checksums, and manages Git SSOT.
"""

import hashlib
import re
import subprocess
import urllib.request
from typing import Optional, Tuple
from .audit import RecipeAuditor
from .catalog import MaintainerCatalog, CYAN, GREEN, YELLOW, RED, BOLD, GRAY, RESET


class RecipeBumper:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog
        self.auditor = RecipeAuditor(catalog)

    def calculate_remote_sha256(self, url: str) -> Tuple[bool, str]:
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "ForgeBumper/1.0"})
            with urllib.request.urlopen(req, timeout=15) as resp:
                if resp.status != 200:
                    return False, f"HTTP Error {resp.status}"
                hasher = hashlib.sha256()
                while chunk := resp.read(65536):
                    hasher.update(chunk)
                return True, hasher.hexdigest()
        except Exception as e:
            return False, str(e)

    def bump_package(self, pkg_name: str, new_version: Optional[str] = None, fetch_sha: bool = True) -> Tuple[bool, str]:
        rec = self.catalog.recipes.get(pkg_name)
        if not rec:
            return False, f"Paket '{pkg_name}' tidak ditemukan di katalog."

        if not new_version:
            audit_res = self.auditor.audit_package(pkg_name)
            if not audit_res.latest_version or not audit_res.has_update:
                return False, f"Paket '{pkg_name}' sudah dalam versi terbaru (v{rec.version})."
            new_version = audit_res.latest_version

        content = rec.path.read_text(encoding="utf-8")
        old_ver = rec.version

        # 1. Update version and release
        new_content = re.sub(r'version\s*=\s*"[^"]*"', f'version = "{new_version}"', content)
        new_content = re.sub(r'release\s*=\s*\d+', 'release = 1', new_content)

        # 2. Update SHA256 if sources exist
        if fetch_sha and rec.source_urls:
            raw_url = rec.source_urls[0]
            # Interpolate new version into source url
            target_url = raw_url.replace("${pkgver}", new_version).replace(old_ver, new_version)
            print(f"  {GRAY}[↓] Mengunduh tarball & menghitung SHA256 dari: {target_url}{RESET}")
            ok, sha_res = self.calculate_remote_sha256(target_url)
            if ok:
                new_sha = sha_res
                # Replace sha256 in content
                pattern = r'(sha256\s*=\s*\[)[^\]]*(\])'
                new_content = re.sub(pattern, rf'\1\n    "{new_sha}"\n\2', new_content)
                print(f"  {GREEN}[✓] SHA256 baru: {new_sha}{RESET}")
            else:
                print(f"  {YELLOW}[!] Gagal kalkulasi SHA256: {sha_res} (mempertahankan SHA lama / lewati){RESET}")

        rec.path.write_text(new_content, encoding="utf-8")
        self.catalog.load_all_recipes()
        return True, f"Berhasil memperbarui {pkg_name} (v{old_ver} -> v{new_version})."

    def bump_all(self, no_push: bool = True) -> Tuple[int, int]:
        print(f"\n{BOLD}{CYAN}=== Memindai & Memperbarui Seluruh Resep yang Outdated ==={RESET}\n")
        audit_results = self.auditor.audit_all()
        outdated = [r for r in audit_results if r.has_update and r.latest_version]

        if not outdated:
            print(f"{GREEN}✓ Seluruh resep (227 paket) sudah dalam versi hulu terkini!{RESET}\n")
            return 0, 0

        print(f"Ditemukan {YELLOW}{len(outdated)}{RESET} paket yang perlu diperbarui:\n")
        success_count = 0
        failed_count = 0

        for r in outdated:
            print(f"🚀 Memperbarui {BOLD}{r.name}{RESET} (v{r.current_version} -> v{r.latest_version})...")
            ok, msg = self.bump_package(r.name, new_version=r.latest_version, fetch_sha=True)
            if ok:
                print(f"  {GREEN}✓ {msg}{RESET}")
                success_count += 1
            else:
                print(f"  {RED}✗ {msg}{RESET}")
                failed_count += 1

        print(f"\n{GREEN}{BOLD}Selesai: {success_count} berhasil di-bump, {failed_count} gagal.{RESET}\n")

        if not no_push and success_count > 0:
            print(f"{CYAN}[*] Mem-push perubahan ke GitHub SSOT...{RESET}")
            subprocess.run(["git", "add", "recipes"])
            subprocess.run(["git", "commit", "-m", "chore(recipes): automated upstream recipe version bump [skip ci]"])
            subprocess.run(["git", "push", "origin", "main"])

        return success_count, failed_count
