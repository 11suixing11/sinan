#!/usr/bin/env python3
"""Apply the independent IPQuality tool profile to the authenticated Debian factory.

The existing source authentication, native builder approval, exact package/source
closure, capacity guards and failure evidence are retained. Profile selection does
not approve a builder, grant a license or run a diagnostic.
"""
import importlib.util
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[1]
TOOLS = {
    'bash': 'bash', 'python3': 'python3', 'curl': 'curl', 'jq': 'jq',
    'base64': 'coreutils', 'head': 'coreutils', 'wc': 'coreutils',
    'date': 'coreutils', 'timeout': 'coreutils', 'numfmt': 'coreutils',
    'uname': 'coreutils', 'cat': 'coreutils', 'cut': 'coreutils',
    'sort': 'coreutils', 'tr': 'coreutils', 'printf': 'coreutils',
    'grep': 'grep', 'sed': 'sed', 'awk': 'gawk', 'bc': 'bc',
    'openssl': 'openssl', 'update-ca-certificates': 'ca-certificates',
}
KIND = 'sinan-ipquality-debian12-preparation'


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def factory():
    build = module('sinan_ipquality_debian_factory', ROOT / 'tools/nodequality-rootfs-build.py')
    build.TOOL_PACKAGES = dict(TOOLS)
    build.PROVENANCE_KIND = KIND
    build.PENDING = ['IPQuality: independent source policy, license review and node acceptance are separate']
    return build


def collector():
    collect = module('sinan_ipquality_debian_collector', ROOT / 'tools/nodequality-rootfs-collect.py')
    collect.BUILD = factory()
    collect.COLLECTION_KIND = 'sinan-ipquality-debian-input-collection'
    return collect


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ('collect', 'bind', 'prepare', 'build', 'export'):
        raise SystemExit('Usage: ipquality-rootfs.py collect|bind|prepare|build|export <explicit factory arguments>')
    chosen = sys.argv[1]
    sys.argv = [sys.argv[0], *sys.argv[1:]]
    return (collector() if chosen in ('collect', 'bind') else factory()).main()


if __name__ == '__main__':
    raise SystemExit(main())
