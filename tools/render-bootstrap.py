#!/usr/bin/env python3
"""Render the standalone official bootstrap from audited source files."""

import argparse
import hashlib
from pathlib import Path
import sys

from release import load_roots

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ("tools/bootstrap.py", "tools/release.py", "tools/tcp_probe_artifact.py",
           "tools/tcp_probe_notices.py", "tools/artifact_manifest.py",
           "deploy/release-public-keys.json")


def render(root=ROOT, trusted_keys=None, publication=True):
    root = Path(root)
    trusted_keys = Path(trusted_keys) if trusted_keys else root / SOURCES[-1]
    load_roots(trusted_keys, publication=publication)
    sections = []
    for filename in SOURCES:
        path = trusted_keys if filename == SOURCES[-1] else root / filename
        data = path.read_text()
        if not data.endswith("\n"):
            data += "\n"
        delimiter = "SINAN_BOOTSTRAP_" + hashlib.sha256(data.encode()).hexdigest().upper()
        destination = "public-keys.json" if filename == SOURCES[-1] else Path(filename).name
        sections.append(f'cat > "$STAGING/{destination}" <<\'{delimiter}\'\n{data}{delimiter}\n')
    template = (root / "deploy/bootstrap.sh.tmpl").read_text()
    if template.count("@@BOOTSTRAP_FILES@@") != 1:
        raise ValueError("bootstrap template must contain one source marker")
    return template.replace("@@BOOTSTRAP_FILES@@", "\n".join(sections))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=ROOT / "deploy/bootstrap.sh")
    args = parser.parse_args()
    rendered = render()
    if args.check:
        if not args.output.is_file() or args.output.read_text() != rendered:
            print("bootstrap.sh is stale; run python3 tools/render-bootstrap.py", file=sys.stderr)
            raise SystemExit(1)
    else:
        args.output.write_text(rendered)
        args.output.chmod(0o755)


if __name__ == "__main__":
    main()
