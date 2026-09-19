#!/usr/bin/env python3
"""
Upstream Search Engine - Searches local catalog, Anitya API, and GitHub releases.
"""

import json
import urllib.parse
import urllib.request
from typing import Any, Dict, List
from .catalog import MaintainerCatalog


class UpstreamSearchEngine:
    def __init__(self, catalog: MaintainerCatalog):
        self.catalog = catalog

    def search_upstream(self, query: str) -> List[Dict[str, Any]]:
        results = []
        q = query.strip()
        seen_names = set()

        # 1. Local catalog search
        local_matches = self.catalog.find_matching_packages(q)
        for m in local_matches:
            rec = self.catalog.recipes[m]
            results.append({
                "name": rec.name,
                "version": rec.version,
                "description": rec.description,
                "homepage": rec.upstream,
                "source": "Catalog Kura",
                "category": rec.category,
                "installed": True,
            })
            seen_names.add(m.lower())

        # 2. Anitya (Release-Monitoring.org API)
        try:
            url = f"https://release-monitoring.org/api/v2/projects/?name={urllib.parse.quote(q)}"
            req = urllib.request.Request(url, headers={"User-Agent": "ForgeMaintainer/1.0"})
            with urllib.request.urlopen(req, timeout=3) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                for p in data.get("items", [])[:5]:
                    p_name = p.get("name", "").lower()
                    if p_name and p_name not in seen_names:
                        results.append({
                            "name": p_name,
                            "version": p.get("version", "1.0.0"),
                            "description": f"Upstream package from {p.get('backend', 'Anitya')}",
                            "homepage": p.get("homepage", ""),
                            "source": "Anitya (Upstream)",
                            "category": "extra",
                            "installed": False,
                        })
                        seen_names.add(p_name)
        except Exception:
            pass

        # 3. GitHub Search API
        try:
            gh_url = f"https://api.github.com/search/repositories?q={urllib.parse.quote(q)}+in:name&sort=stars&order=desc&per_page=5"
            req = urllib.request.Request(gh_url, headers={"User-Agent": "ForgeMaintainer/1.0", "Accept": "application/vnd.github.v3+json"})
            with urllib.request.urlopen(req, timeout=3) as resp:
                data = json.loads(resp.read().decode("utf-8"))
                for repo in data.get("items", []):
                    repo_name = repo.get("name", "").lower()
                    if repo_name and repo_name not in seen_names:
                        results.append({
                            "name": repo_name,
                            "version": "1.0.0",
                            "description": repo.get("description", "") or "GitHub Open Source Repository",
                            "homepage": repo.get("html_url", ""),
                            "source": f"GitHub ({repo.get('stargazers_count', 0)}★)",
                            "category": "extra",
                            "installed": False,
                        })
                        seen_names.add(repo_name)
        except Exception:
            pass

        return results
