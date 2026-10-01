#!/usr/bin/env python3
"""Trusted, operator-provisioned bootstrap; never fetched from the panel and executed."""

import argparse
import ipaddress
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import urllib.parse
import urllib.request

from release import (REPOSITORY, VERSION, digest, ensure, load_roots, read_regular,
                     require_protected_file, validate_manifest, verify_manifest)

GITHUB_DOWNLOAD_HOSTS = frozenset(("github.com", "release-assets.githubusercontent.com",
                                    "objects.githubusercontent.com"))


def validate_mirror(value, panel=None):
    if not value:
        return ""
    parsed = urllib.parse.urlsplit(value)
    ensure(len(value) <= 512 and not any(ord(c) < 32 or ord(c) == 127 for c in value)
           and parsed.scheme == "https" and parsed.hostname and parsed.hostname != "localhost"
           and parsed.port in (None, 443) and not parsed.username and not parsed.password
           and not parsed.query and not parsed.fragment, "invalid HTTPS mirror prefix")
    try:
        ipaddress.ip_address(parsed.hostname)
    except ValueError:
        pass
    else:
        raise ValueError("mirror must be a hostname")
    if panel:
        origin = urllib.parse.urlsplit(panel)
        ensure((parsed.hostname, parsed.port or 443) != (origin.hostname, origin.port or (443 if origin.scheme == "https" else 80)), "Agent cannot be downloaded from panel")
    return value.rstrip("/")


def validate_github_url(url, mirror=""):

    parsed = urllib.parse.urlsplit(url)
    ensure(not any(ord(character) < 32 or ord(character) == 127 for character in url)
           and parsed.scheme == "https" and (parsed.hostname in GITHUB_DOWNLOAD_HOSTS or
               (mirror and parsed.netloc == urllib.parse.urlsplit(mirror).netloc))
           and parsed.port in (None, 443) and not parsed.username and not parsed.password
           and not parsed.fragment, "GitHub download URL is outside the fixed HTTPS allowlist")


def validate_panel_origin(value):
    parsed = urllib.parse.urlsplit(value)
    ensure(not any(ord(character) < 32 or ord(character) == 127 for character in value)
           and parsed.scheme in ("http", "https") and parsed.hostname
           and not parsed.username and not parsed.password and parsed.path in ("", "/")
           and not parsed.query and not parsed.fragment
           and (parsed.port is None or 0 < parsed.port <= 65535), "panel must be a valid origin")
    if parsed.scheme == "http":
        try:
            address = ipaddress.ip_address(parsed.hostname)
            address = getattr(address, "ipv4_mapped", None) or address
            loopback = address.is_loopback
        except ValueError:
            loopback = parsed.hostname == "localhost"
        ensure(loopback, "HTTP panel origins must use a loopback address; use HTTPS")


class GithubRedirect(urllib.request.HTTPRedirectHandler):
    max_redirections = 5
    max_repeats = 2

    def __init__(self, mirror=""):
        super().__init__()
        self.mirror = mirror

    def redirect_request(self, request, response, code, message, headers, new_url):
        validate_github_url(new_url, self.mirror)
        return super().redirect_request(request, response, code, message, headers, new_url)


def github_opener(mirror=""):
    # Bootstrap proof downloads never inherit HTTP_PROXY/HTTPS_PROXY/ALL_PROXY.
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), GithubRedirect(mirror))


def download(base, name, destination, limit, mirror=""):
    ensure(Path(name).name == name and name not in (".", ".."), "unsafe asset name")
    url = base + "/" + urllib.parse.quote(name, safe="")
    mirror = validate_mirror(mirror)
    if mirror:
        url = mirror + "/" + url
    validate_github_url(url, mirror)
    with github_opener(mirror).open(url, timeout=120) as response:
        validate_github_url(response.url, mirror)
        data = response.read(limit + 1)
    ensure(0 < len(data) <= limit, "download size outside permitted range")
    destination.write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--panel", required=True)
    parser.add_argument("--token")
    parser.add_argument("--mirror", default="", help="HTTPS prefix for GitHub downloads")
    parser.add_argument("--trusted-keys", default="/etc/sinan/trust/public-keys.json",
                        help="Operator-provisioned root-owned JSON key set, independent of panel")
    parser.add_argument("--minisign", default="minisign")
    parser.add_argument("--trusted-agent", help="Previously trusted signed Agent for offline proof verification")
    parser.add_argument("--release-dir", help="Pre-downloaded signed release, including Agent binary; offline installation")
    args = parser.parse_args()
    ensure(os.getuid() == 0, "bootstrap requires root")
    ensure(args.tag.startswith("agent-v") and VERSION.fullmatch(args.tag[7:]), "invalid tag")
    version = args.tag[7:]
    validate_panel_origin(args.panel)
    mirror = validate_mirror(args.mirror, args.panel)
    ensure(platform.machine() in ("x86_64", "aarch64"), "unsupported architecture")
    token = args.token or os.environ.pop("SINAN_ENROLLMENT_TOKEN", None)
    ensure(token, "provide one-time token through SINAN_ENROLLMENT_TOKEN")
    roots = None if args.trusted_agent else load_roots(args.trusted_keys, require_protected=True)
    tag = args.tag
    base = f"https://github.com/{REPOSITORY}/releases/download/{tag}"
    os.umask(0o077)
    with tempfile.TemporaryDirectory(prefix="sinan-bootstrap-") as temporary:
        bundle = Path(temporary)
        for name, limit in (("SHA256SUMS", 8192), ("SHA256SUMS.minisig", 16384),
                            ("release.json", 32768), ("install.sh", 262144)):
            if args.release_dir:
                (bundle / name).write_bytes(read_regular(Path(args.release_dir) / name, limit))
            else:
                download(base, name, bundle / name, limit, mirror)
        if args.trusted_agent:
            trusted_agent = Path(args.trusted_agent).resolve(strict=True)
            require_protected_file(trusted_agent)
            ensure(trusted_agent.name == "sinan-agent", "trusted verifier must be the installed Agent")
            for command in ([str(trusted_agent), "verify-installed", "--binary", str(trusted_agent),
                             "--name", "agent", "--format", "raw"],
                            [str(trusted_agent), "verify-release", "--proof-dir", str(bundle)]):
                result = subprocess.run(command, capture_output=True, check=False)
                ensure(result.returncode == 0, "previous Agent refused release verification")
            validate_manifest(bundle, tag)
        else:
            verify_manifest(bundle, roots, args.minisign, tag)
        metadata = json.loads((bundle / "release.json").read_text())
        arch = {"x86_64": "amd64", "aarch64": "arm64"}[platform.machine()]
        entries = [item for item in metadata["artifacts"] if
                   (item["name"], item["version"], item["arch"]) == ("agent", version, arch)]
        ensure(len(entries) == 1, "release lacks a unique compatible Agent")
        entry = entries[0]
        name, limit = entry["asset_name"], entry["archive_size"]
        ensure(entry["format"] == "raw" and 0 < limit <= 128 * 1024 * 1024, "invalid Agent size or format")
        if args.release_dir:
            (bundle / name).write_bytes(read_regular(Path(args.release_dir) / name, limit))
        else:
            download(base, name, bundle / name, limit, mirror)
        ensure(digest(read_regular(bundle / name, limit)) == entry["binary_sha256"], "Agent digest differs from signed release")
        token_file = bundle / ".enrollment-token"
        token_file.write_text(token)
        result = subprocess.run(["/bin/sh", str(bundle / "install.sh"), "--bundle", str(bundle),
                                 "--panel", args.panel, "--version", version,
                                 "--token-file", str(token_file)], check=False)
        raise SystemExit(result.returncode)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"Bootstrap refused: {error}") from error
