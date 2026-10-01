#!/usr/bin/env python3
"""Embed approved release roots into the standalone PowerShell bootstrap."""
from pathlib import Path
from release import load_roots

ROOT = Path(__file__).resolve().parents[1]

def render():
    template = (ROOT / "deploy/bootstrap.ps1.tmpl").read_text(encoding="utf-8")
    if template.count("@@TRUSTED_KEYS@@") != 1:
        raise ValueError("missing or duplicate trust-root marker")
    roots = load_roots(ROOT / "deploy/release-public-keys.json", publication=True)
    literal = "@(" + ", ".join("'" + key.replace("'", "''") + "'" for key in roots) + ")"
    rendered = template.replace("@@TRUSTED_KEYS@@", literal)
    if "@@" in rendered:
        raise ValueError("unexpanded bootstrap marker")
    # BOM allows Windows PowerShell 5.1 to read the Chinese messages as UTF-8.
    return b"\xef\xbb\xbf" + rendered.encode("utf-8")

if __name__ == "__main__":
    (ROOT / "deploy/bootstrap.ps1").write_bytes(render())
