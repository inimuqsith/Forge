#!/usr/bin/env python3
"""
Source Verifier - Verifies upstream tarball download streams and SHA256 hashes.
"""

import hashlib
import urllib.request
from typing import Optional, Tuple
from .catalog import MaintainerCatalog


class SourceVerifier:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def verify_package_sources(self, pkg_name: str, check_sha: bool = True) -> Tuple[bool, str, Optional[str]]:
        rec = self.catalog.recipes.get(pkg_name)
        if not rec:
            return False, f"Paket '{pkg_name}' tidak ditemukan", None
        if not rec.source_urls:
            return True, "Meta-Paket / Tanpa source URL (Valid)", None

        url = rec.source_urls[0].replace("${pkgver}", rec.version).replace("${pkgname}", rec.name)
        expected_sha = rec.source_sha256[0] if rec.source_sha256 else None

        try:
            req = urllib.request.Request(url, headers={"User-Agent": "ForgeMaintainer/1.0"})
            with urllib.request.urlopen(req, timeout=10) as resp:
                if resp.status != 200:
                    return False, f"HTTP Error {resp.status} dari {url}", None

                if not check_sha:
                    return True, f"HTTP 200 OK ({url})", None

                hasher = hashlib.sha256()
                while chunk := resp.read(65536):
                    hasher.update(chunk)
                calc_sha = hasher.hexdigest()

                if expected_sha and calc_sha.lower() == expected_sha.lower():
                    return True, f"SHA256 Match ({calc_sha[:12]}...)", calc_sha
                else:
                    return False, f"SHA256 Mismatch! Expected {expected_sha}, Got {calc_sha}", calc_sha
        except Exception as e:
            return False, f"Gagal mengunduh {url}: {e}", None
