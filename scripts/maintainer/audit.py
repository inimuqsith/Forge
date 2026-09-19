#!/usr/bin/env python3
"""
Recipe Auditor - Parallel, zero-quota upstream version scanner.
"""

import concurrent.futures
import json
import os
import re
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from typing import Dict, List, Optional, Tuple
from .catalog import MaintainerCatalog, CYAN, GREEN, YELLOW, RED, BOLD, GRAY, RESET


def clean_version_tag(tag: str) -> str:
    """Clean upstream tag into normalized semantic version string."""
    tag = tag.strip()
    tag = re.sub(r"^(?:v|release-|rel-|v_|ver-)", "", tag, flags=re.IGNORECASE)
    tag = re.sub(r"\.(?:tar\.gz|tar\.xz|tar\.bz2|tar\.zst|zip|tgz)$", "", tag, flags=re.IGNORECASE)
    return tag.strip()


def parse_semver_tuple(ver_str: str) -> Tuple[int, ...]:
    """Parse version string into tuple of ints for reliable comparison."""
    parts = re.findall(r"\d+", ver_str)
    return tuple(map(int, parts)) if parts else (0,)


def is_version_newer(latest: str, current: str) -> bool:
    """Check if latest version is strictly greater than current version."""
    t_latest = parse_semver_tuple(latest)
    t_current = parse_semver_tuple(current)
    return t_latest > t_current


class AuditResult:
    def __init__(self, name: str, category: str, current_version: str, latest_version: Optional[str], has_update: bool, provider: str):
        self.name = name
        self.category = category
        self.current_version = current_version
        self.latest_version = latest_version
        self.has_update = has_update
        self.provider = provider


class RecipeAuditor:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def extract_github_repo(self, url: str) -> Optional[Tuple[str, str]]:
        if not url or "github.com" not in url:
            return None
        match = re.search(r"github\.com[/:]([a-zA-Z0-9_\-\.]+)/([a-zA-Z0-9_\-\.]+)", url)
        if match:
            owner = match.group(1)
            repo = match.group(2).rstrip("/")
            if repo.endswith(".git"):
                repo = repo[:-4]
            return owner, repo
        return None

    def probe_github_atom(self, owner: str, repo: str) -> Optional[str]:
        feed_url = f"https://github.com/{owner}/{repo}/releases.atom"
        req = urllib.request.Request(feed_url, headers={"User-Agent": "ForgeAuditor/1.0"})
        try:
            with urllib.request.urlopen(req, timeout=5) as resp:
                if resp.status != 200:
                    return None
                tree = ET.fromstring(resp.read().decode("utf-8"))
                # Namespace handling for Atom feed
                ns = {"atom": "http://www.w3.org/2005/Atom"}
                entry = tree.find("atom:entry", ns)
                if entry is not None:
                    title = entry.find("atom:title", ns)
                    if title is not None and title.text:
                        return clean_version_tag(title.text)
        except Exception:
            pass
        return None

    def probe_anitya(self, pkg_name: str) -> Optional[str]:
        api_url = f"https://release-monitoring.org/api/v2/projects/?name={urllib.parse.quote(pkg_name)}"
        req = urllib.request.Request(api_url, headers={"User-Agent": "ForgeAuditor/1.0"})
        try:
            with urllib.request.urlopen(req, timeout=5) as resp:
                if resp.status == 200:
                    data = json.loads(resp.read().decode("utf-8"))
                    items = data.get("items", [])
                    if items:
                        return clean_version_tag(items[0].get("version", ""))
        except Exception:
            pass
        return None

    def audit_package(self, pkg_name: str) -> AuditResult:
        rec = self.catalog.recipes.get(pkg_name)
        if not rec:
            return AuditResult(pkg_name, "unknown", "-", None, False, "Not Found")

        # Git VCS packages are exempted
        is_git_vcs = rec.version == "git" or any(".git" in u or u.startswith("git://") for u in rec.source_urls)
        if is_git_vcs:
            return AuditResult(rec.name, rec.category, rec.version, "HEAD (Live VCS)", False, "Git VCS (Live)")

        upstream_url = rec.upstream
        gh_info = self.extract_github_repo(upstream_url)

        latest = None
        provider = "None"

        # Tier 1 & 2: GitHub Atom Probing (Rate-limit free)
        if gh_info:
            owner, repo = gh_info
            latest = self.probe_github_atom(owner, repo)
            if latest:
                provider = f"GitHub Atom ({owner}/{repo})"

        # Tier 3: Anitya fallback
        if not latest:
            latest = self.probe_anitya(rec.name)
            if latest:
                provider = "Anitya (release-monitoring.org)"

        has_update = False
        if latest and latest != rec.version:
            has_update = is_version_newer(latest, rec.version)

        return AuditResult(rec.name, rec.category, rec.version, latest, has_update, provider)

    def audit_all(self, max_workers: int = 16) -> List[AuditResult]:
        results = []
        pkg_names = sorted(self.catalog.recipes.keys())

        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
            future_to_pkg = {executor.submit(self.audit_package, name): name for name in pkg_names}
            for future in concurrent.futures.as_completed(future_to_pkg):
                res = future.result()
                results.append(res)

        results.sort(key=lambda x: x.name)
        return results

    def print_audit_report(self, results: List[AuditResult]) -> None:
        print(f"\n{BOLD}{CYAN}=== Forge Upstream Recipe Version Audit Report ==={RESET}\n")
        print(f"{'PAKET':<24} {'KATEGORI':<10} {'VERSI LOKAL':<14} {'VERSI HULU':<16} {'STATUS':<10} {'SUMBER / PROVIDER'}")
        print(f"{GRAY}{'-' * 95}{RESET}")

        updates_count = 0
        latest_count = 0

        for r in results:
            if r.has_update:
                status_str = f"{YELLOW}{BOLD}[UPDATE]{RESET}"
                latest_str = f"{YELLOW}{BOLD}{r.latest_version:<16}{RESET}"
                updates_count += 1
            elif r.latest_version:
                status_str = f"{GREEN}[LATEST]{RESET}"
                latest_str = f"{GRAY}{r.latest_version:<16}{RESET}"
                latest_count += 1
            else:
                status_str = f"{GRAY}[? UNK]{RESET}"
                latest_str = f"{GRAY}{'-':<16}{RESET}"

            print(f"{BOLD}{r.name:<24}{RESET} {CYAN}{r.category:<10}{RESET} {r.current_version:<14} {latest_str} {status_str} {GRAY}{r.provider}{RESET}")

        print(f"{GRAY}{'-' * 95}{RESET}")
        print(f"Total Resep: {BOLD}{len(results)}{RESET} | Mutakhir: {GREEN}{BOLD}{latest_count}{RESET} | Perlu Pembaruan: {YELLOW}{BOLD}{updates_count}{RESET}\n")
