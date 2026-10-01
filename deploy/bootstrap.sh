#!/bin/sh
# Independent official bootstrap. Generated with the audited release verifier and roots.
set -eu
PATH=/usr/sbin:/usr/bin:/sbin:/bin
export PATH
umask 077

if [ "${1:-}" = --help ]; then
  printf '%s\n' '用法: bootstrap.sh --tag agent-v版本 --panel https://面板 --token 一次性接入令牌'
  exit 0
fi
if [ "$(id -u)" != 0 ]; then
  command -v sudo >/dev/null || { echo '请以 root 执行安装命令，或先安装 sudo' >&2; exit 1; }
  exec sudo /bin/sh "$0" "$@"
fi
[ "$(uname -s)" = Linux ] || { echo '此接入入口需要 Linux systemd 或 OpenRC' >&2; exit 1; }
case "$(uname -m)" in x86_64|aarch64|arm64) ;; *) echo '此接入入口仅支持 amd64 和 arm64' >&2; exit 1 ;; esac

NEEDS_PACKAGES=0
for tool in python3 minisign install getent cmp mv seq; do
  command -v "$tool" >/dev/null || NEEDS_PACKAGES=1
done
if [ "$NEEDS_PACKAGES" = 1 ]; then
  printf '%s\n' '正在通过系统软件源准备 Python、minisign 和安装工具。'
  if command -v apt-get >/dev/null; then
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends python3 minisign ca-certificates coreutils libc-bin passwd
  elif command -v apk >/dev/null; then
    apk add --no-cache python3 minisign ca-certificates coreutils musl-utils shadow
  elif command -v dnf >/dev/null; then
    dnf install -y python3 minisign ca-certificates coreutils glibc-common shadow-utils
  elif command -v yum >/dev/null; then
    yum install -y python3 minisign ca-certificates coreutils glibc-common shadow-utils
  else
    echo '无法自动准备安装工具：需要 apt-get、apk、dnf 或 yum 软件源' >&2
    exit 1
  fi
fi
for tool in python3 minisign install getent cmp mv seq; do
  command -v "$tool" >/dev/null || { echo "系统软件源未提供所需工具: $tool" >&2; exit 1; }
done

# A protected parent is required because the bootstrap rejects writable trust paths.
STAGING=$(mktemp -d /run/sinan-bootstrap.XXXXXX)
cleanup() { rm -rf "$STAGING"; }
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' HUP TERM
cat > "$STAGING/bootstrap.py" <<'SINAN_BOOTSTRAP_89B24407E0B9F17187979BBC89C5656C2034ECB4BF65F135C1A601CC3A1C3AFA'
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

from release import (REPOSITORY, VERSION, ensure, load_roots, read_regular,
                     require_protected_file, validate_manifest, verify_manifest)

GITHUB_DOWNLOAD_HOSTS = frozenset(("github.com", "release-assets.githubusercontent.com",
                                    "objects.githubusercontent.com"))


def validate_github_url(url):
    parsed = urllib.parse.urlsplit(url)
    ensure(not any(ord(character) < 32 or ord(character) == 127 for character in url)
           and parsed.scheme == "https" and parsed.hostname in GITHUB_DOWNLOAD_HOSTS
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

    def redirect_request(self, request, response, code, message, headers, new_url):
        validate_github_url(new_url)
        return super().redirect_request(request, response, code, message, headers, new_url)


def github_opener():
    # Bootstrap proof downloads never inherit HTTP_PROXY/HTTPS_PROXY/ALL_PROXY.
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), GithubRedirect())


def download(base, name, destination, limit):
    ensure(Path(name).name == name and name not in (".", ".."), "unsafe asset name")
    url = base + "/" + urllib.parse.quote(name, safe="")
    validate_github_url(url)
    with github_opener().open(url, timeout=120) as response:
        validate_github_url(response.url)
        data = response.read(limit + 1)
    ensure(0 < len(data) <= limit, "download size outside permitted range")
    destination.write_bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--panel", required=True)
    parser.add_argument("--token")
    parser.add_argument("--trusted-keys", default="/etc/sinan/trust/public-keys.json",
                        help="Operator-provisioned root-owned JSON key set, independent of panel")
    parser.add_argument("--minisign", default="minisign")
    parser.add_argument("--trusted-agent", help="Previously trusted signed Agent for offline proof verification")
    parser.add_argument("--release-dir", help="Pre-downloaded proof and signed installer; CI/offline proof only")
    args = parser.parse_args()
    ensure(os.getuid() == 0, "bootstrap requires root")
    ensure(args.tag.startswith("agent-v") and VERSION.fullmatch(args.tag[7:]), "invalid tag")
    version = args.tag[7:]
    validate_panel_origin(args.panel)
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
                download(base, name, bundle / name, limit)
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
SINAN_BOOTSTRAP_89B24407E0B9F17187979BBC89C5656C2034ECB4BF65F135C1A601CC3A1C3AFA

cat > "$STAGING/release.py" <<'SINAN_BOOTSTRAP_99E3BAC561E20F854C3A33D3E94BDF91FF76AEA4CC7C2A05847CE31A673CCBCE'
#!/usr/bin/env python3
"""Build canonical release manifests and verify complete offline-signed bundles."""

import argparse
import base64
import hashlib
import gzip
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile


REPOSITORY = "theLucius7/sinan"
TEST_ONLY_PUBLIC_KEY = "RWS3NbDikg3VqWRlxJMUyaB1dTvErk0ptJ695xQ50Kyb+MmtynMhN/lq"
TEST_ONLY_ROTATION_PUBLIC_KEY = "RWRURVNUUk9UMjMuvo0ny3Mjs6QBwcE7XdZLzMDhDs2hwrXRGgN3moXl"
SOURCE_ROOT = Path(__file__).resolve().parents[1]
TEST_PUBLIC_KEY_DIRS = (SOURCE_ROOT / "fixtures", SOURCE_ROOT / "crates/protocol/tests/fixtures")
NODEQUALITY_VERSION = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r7"
SEGMENT = re.compile(r"[0-9A-Za-z][0-9A-Za-z.+_-]{0,127}\Z")
VERSION = re.compile(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.-]+)?\Z")
MAX_BINARY = 256 * 1024 * 1024


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_regular(path, limit=MAX_BINARY):
    ensure(path.is_file() and not path.is_symlink(), "asset must be an ordinary file")
    ensure(0 < path.stat().st_size <= limit, "asset size outside permitted range")
    return path.read_bytes()


def canonical_path(entry):
    ensure(entry["name"] in ("agent", "sing-box", "nodequality", "tcpquality"), "unsupported module")
    ensure(SEGMENT.fullmatch(entry["version"]), "invalid version segment")
    ensure(entry["arch"] in ("amd64", "arm64", "linux-gnu-amd64", "linux-gnu-arm64", "linux-musl-amd64", "linux-musl-arm64", "macos-arm64", "freebsd-amd64", "freebsd-arm64", "windows-amd64", "windows-arm64"), "unsupported architecture")
    return "/".join(entry[k] for k in ("name", "version", "arch"))


def asset_name(entry):
    name, version, arch = (entry[k] for k in ("name", "version", "arch"))
    ensure(entry["format"] in ("raw", "tar.gz"), "unsupported format")
    if arch not in ("amd64", "arm64"):
        return f"{name}-{version}-{arch}" + (".tar.gz" if entry["format"] == "tar.gz" else "")
    return (f"{name}-{version}-linux-musl-{arch}" if entry["format"] == "raw"
            else f"{name}-{version}-linux-{arch}.tar.gz")


def validate_auxiliary_files(value, binary_name, archive_format):
    ensure(isinstance(value, dict) and len(value) <= 7, "invalid auxiliary file list")
    ensure(not value or archive_format == "tar.gz", "raw artifacts cannot contain auxiliary files")
    for name, entry in value.items():
        ensure(isinstance(name, str) and re.fullmatch(r"[0-9A-Za-z._-]{1,128}", name)
               and name not in (".", "..", binary_name, "release.json", "SHA256SUMS",
                                "SHA256SUMS.minisig", ".artifact.json") and not name.startswith("-"),
               "invalid auxiliary file name")
        ensure(isinstance(entry, dict) and set(entry) == {"sha256", "size"},
               "invalid auxiliary file metadata")
        ensure(type(entry["size"]) is int and 0 < entry["size"] <= MAX_BINARY,
               "invalid auxiliary file size")
        ensure(isinstance(entry["sha256"], str) and re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]),
               "invalid auxiliary file digest")
    return value


def binary_bytes(data, archive_format, binary_name, auxiliary_files=None):
    auxiliary = validate_auxiliary_files(auxiliary_files if auxiliary_files is not None else {},
                                         binary_name, archive_format)
    if archive_format == "raw":
        return data
    ensure(archive_format == "tar.gz", "unsupported format")
    expected = {binary_name} | set(auxiliary)
    seen, binary, consumed = set(), None, 0
    with gzip.GzipFile(fileobj=io.BytesIO(data), mode="rb") as compressed:
        def read(size):
            nonlocal consumed
            content = compressed.read(min(size, MAX_BINARY - consumed + 1))
            consumed += len(content)
            ensure(consumed <= MAX_BINARY, "archive exceeds unpacked size limit")
            return content

        while True:
            header = read(tarfile.BLOCKSIZE)
            ensure(len(header) in (0, tarfile.BLOCKSIZE), "truncated archive header")
            if not header or not any(header):
                break
            try:
                member = tarfile.TarInfo.frombuf(header, encoding="utf-8", errors="strict")
            except tarfile.HeaderError as error:
                raise ValueError("invalid archive header") from error
            ensure(member.name in expected and member.name not in seen
                   and member.type in (tarfile.REGTYPE, tarfile.AREGTYPE), "unsafe archive member")
            seen.add(member.name)
            ensure(0 < member.size <= MAX_BINARY, "invalid archive member size")
            if member.name != binary_name:
                ensure(member.size == auxiliary[member.name]["size"], "auxiliary file size mismatch")
            content = read(member.size)
            ensure(len(content) == member.size, "archive file length mismatch")
            if member.name == binary_name:
                binary = content
            else:
                ensure(digest(content) == auxiliary[member.name]["sha256"], "auxiliary file digest mismatch")
            padding = (-member.size) % tarfile.BLOCKSIZE
            ensure(len(read(padding)) == padding, "truncated archive padding")
        ensure(seen == expected, "archive does not contain the exact signed file set")
        while True:
            tail = read(8192)
            if not tail:
                break
            ensure(not any(tail), "archive contains trailing data")
    return binary


def assemble(args):
    ensure(VERSION.fullmatch(args.agent_version), "invalid Agent version")
    ensure(args.tag == "agent-v" + args.agent_version, "tag differs from Agent version")
    source, output = Path(args.source), Path(args.output)
    ensure(not output.exists(), "release output already exists")
    output.mkdir(parents=True)
    paths, artifacts = {}, []
    architectures = getattr(args, "arch", None) or ("amd64", "arm64")
    ensure(len(set(architectures)) == len(architectures)
           and set(architectures) <= {"amd64", "arm64"}, "invalid or duplicate architecture")
    modules = [
        ("agent", args.agent_version, "raw", "sinan-agent"),
        ("sing-box", args.runtime_version, "tar.gz", "sing-box"),
        ("nodequality", getattr(args, "nodequality_version", NODEQUALITY_VERSION), "tar.gz", "nodequality"),
    ]
    if getattr(args, "tcp_probe_version", None) is not None:
        modules.append(("tcpquality", args.tcp_probe_version, "tar.gz", "sinan-tcp-probe"))
    for name, version, archive_format, binary_name in modules:
        for arch in architectures:
            entry = {"name": name, "version": version, "arch": arch,
                     "format": archive_format, "binary_name": binary_name}
            path = canonical_path(entry)
            data = read_regular(source / path)
            auxiliary = {}
            if name == "tcpquality":
                from tcp_probe_artifact import archive_files, validate_files
                files = archive_files(data)
                validate_files(files, version, arch)
                auxiliary = {name: {"sha256": digest(content), "size": len(content)}
                             for name, content in files.items() if name != binary_name}
                entry["auxiliary_files"] = auxiliary
            binary = binary_bytes(data, archive_format, binary_name, auxiliary)
            entry.update(archive_size=len(data), binary_sha256=digest(binary),
                         binary_size=len(binary), asset_name=asset_name(entry))
            (output / entry["asset_name"]).write_bytes(data)
            paths[path] = digest(data)
            artifacts.append(entry)
    artifacts.sort(key=canonical_path)
    metadata = {"schema": 1, "source_repo": REPOSITORY, "tag": args.tag,
                "protocol_min": 1, "protocol_max": 1, "artifacts": artifacts}
    encoded = (json.dumps(metadata, sort_keys=True, separators=(",", ":")) + "\n").encode()
    ensure(len(encoded) <= 32768, "metadata too large")
    (output / "release.json").write_bytes(encoded)
    installer = read_regular(Path(args.installer), 256 * 1024)
    (output / "install.sh").write_bytes(installer)
    paths.update({"release.json": digest(encoded), "install.sh": digest(installer)})
    checksums = "".join(f"{paths[path]}  {path}\n" for path in sorted(paths))
    ensure(len(checksums.encode()) <= 8192, "checksums too large")
    (output / "SHA256SUMS").write_bytes(checksums.encode("utf-8"))


def render_installer(args):
    text = read_regular(Path(args.template), 262144).decode("utf-8")
    for marker, filename in (("@@AGENT_UNIT@@", args.agent_unit),
                             ("@@RUNTIME_UNIT@@", args.runtime_unit)):
        ensure(text.count(marker) == 1, "missing or duplicate installer unit marker")
        text = text.replace(marker, read_regular(Path(filename), 65536).decode("utf-8").rstrip())
    for marker, filename in (("@@AGENT_OPENRC@@", SOURCE_ROOT / "deploy/sinan-agent.openrc"),
                             ("@@RUNTIME_OPENRC@@", SOURCE_ROOT / "plugins/sing-box/sinan-singbox.openrc")):
        if marker in text:
            ensure(text.count(marker) == 1, "duplicate installer unit marker")
            text = text.replace(marker, read_regular(filename, 65536).decode("utf-8").rstrip())
    ensure("@@" not in text, "unexpanded installer marker")
    output = Path(args.output)
    ensure(not output.exists(), "installer output exists")
    output.write_bytes(text.encode("utf-8"))


def public_record(key):
    ensure(isinstance(key, str) and len(key.encode()) <= 4096, "invalid public key")
    lines = key.splitlines()
    record = lines[-1] if len(lines) in (1, 2) else ""
    if len(lines) == 2:
        ensure(lines[0].startswith("untrusted comment: "), "invalid public key comment")
    binary = base64.b64decode(record, validate=True)
    ensure(len(binary) == 42 and binary[:2] == b"Ed", "invalid minisign public key")
    return record, binary[10:]


def test_public_material():
    # Include the known roots when trusted bootstrap tools are copied without tests.
    denied = {public_record(key)[1] for key in
              (TEST_ONLY_PUBLIC_KEY, TEST_ONLY_ROTATION_PUBLIC_KEY)}
    for directory in TEST_PUBLIC_KEY_DIRS:
        for path in directory.glob("TEST_ONLY*.pub"):
            denied.add(public_record(read_regular(path, 4096).decode("utf-8"))[1])
    return denied


def load_roots(path, publication=False, require_protected=False):
    path = Path(path)
    if require_protected:
        for part in (path,) + tuple(path.parents):
            stat = part.lstat()
            ensure(not part.is_symlink() and stat.st_uid == 0 and stat.st_mode & 0o022 == 0,
                   "bootstrap trust file and parents must be root-owned and protected")
    value = json.loads(read_regular(path, 32768))
    ensure(isinstance(value, list) and 0 < len(value) <= 8, "invalid trusted key set")
    denied = test_public_material() if publication else set()
    roots, public_material = [], set()
    for key in value:
        record, material = public_record(key)
        ensure(material not in public_material, "duplicate public key")
        if publication:
            ensure(material not in denied, "TEST_ONLY key cannot publish an official release")
        roots.append(record)
        public_material.add(material)
    return roots


def require_protected_file(path):
    path = Path(path)
    for part in (path,) + tuple(path.parents):
        stat = part.lstat()
        ensure(not part.is_symlink() and stat.st_uid == 0 and stat.st_mode & 0o022 == 0,
               "trusted file and parents must be root-owned and protected")


def verify_signature(bundle, roots, minisign):
    signature = read_regular(bundle / "SHA256SUMS.minisig", 16384).decode("utf-8")
    lines = signature.splitlines()
    ensure(len(lines) == 4 and lines[0].startswith("untrusted comment: ")
           and lines[2].startswith("trusted comment: "), "signature must contain all four minisign lines")
    ensure(base64.b64decode(lines[1], validate=True)[:2] == b"ED", "legacy signature prohibited")
    for key in roots:
        result = subprocess.run([minisign, "-V", "-H", "-q", "-m", str(bundle / "SHA256SUMS"),
                                 "-x", str(bundle / "SHA256SUMS.minisig"), "-P", key],
                                capture_output=True, check=False)
        if result.returncode == 0:
            return
    raise ValueError("no trusted key verifies the complete signature")


def verify_manifest(bundle, roots, minisign, expected_tag=None):
    verify_signature(Path(bundle), roots, minisign)
    return validate_manifest(bundle, expected_tag)


def validate_manifest(bundle, expected_tag=None):
    """Validate contents only after an independently successful signature verifier."""
    bundle = Path(bundle)
    ensure(not bundle.is_symlink(), "bundle must not be a symlink")
    checksums = read_regular(bundle / "SHA256SUMS", 8192).decode("utf-8")
    rows = {}
    for line in checksums.splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([0-9A-Za-z.+_/-]+)", line)
        ensure(match is not None, "noncanonical checksum entry")
        value, path = match.groups()
        ensure(path not in rows and not path.startswith("/") and ".." not in path.split("/"),
               "duplicate or unsafe checksum path")
        rows[path] = value
    canonical = "".join(f"{rows[path]}  {path}\n" for path in sorted(rows))
    ensure(checksums == canonical, "checksums must be sorted canonical LF text")
    metadata_bytes = read_regular(bundle / "release.json", 32768)
    ensure(rows.get("release.json") == digest(metadata_bytes), "metadata hash mismatch")
    metadata = json.loads(metadata_bytes)
    ensure(set(metadata) == {"schema", "source_repo", "tag", "protocol_min", "protocol_max", "artifacts"},
           "unexpected metadata fields")
    ensure(type(metadata["schema"]) is int and metadata["schema"] == 1
           and metadata["source_repo"] == REPOSITORY, "wrong release identity")
    ensure(type(metadata["protocol_min"]) is int and type(metadata["protocol_max"]) is int
           and metadata["protocol_min"] == 1 and metadata["protocol_max"] == 1,
           "unsupported protocol range")
    ensure(expected_tag is None or metadata["tag"] == expected_tag, "wrong release tag")
    ensure(isinstance(metadata["artifacts"], list) and 0 < len(metadata["artifacts"]) <= 30,
           "invalid artifact list")
    expected_paths = {"release.json", "install.sh"}
    for entry in metadata["artifacts"]:
        required = {"name", "version", "arch", "format", "binary_name", "archive_size",
                    "binary_sha256", "binary_size", "asset_name"}
        ensure(isinstance(entry, dict) and set(entry) in (required, required | {"auxiliary_files"}),
               "unexpected artifact fields")
        path = canonical_path(entry)
        ensure(path not in expected_paths, "duplicate artifact identity")
        expected_paths.add(path)
        ensure(entry["asset_name"] == asset_name(entry), "asset name differs from signed identity")
        ensure(type(entry["archive_size"]) is int and 0 < entry["archive_size"] <= MAX_BINARY
               and type(entry["binary_size"]) is int and 0 < entry["binary_size"] <= MAX_BINARY,
               "invalid signed sizes")
        ensure(isinstance(entry["binary_sha256"], str)
               and re.fullmatch(r"[0-9a-f]{64}", entry["binary_sha256"]), "invalid binary digest")
        ensure(isinstance(entry["binary_name"], str) and SEGMENT.fullmatch(entry["binary_name"])
               and entry["binary_name"] not in (".", ".."), "invalid signed binary name")
        validate_auxiliary_files(entry.get("auxiliary_files", {}), entry["binary_name"], entry["format"])
        if entry["name"] == "tcpquality":
            from tcp_probe_artifact import BINARY, FILES, TOOL_VERSION
            ensure(entry["format"] == "tar.gz" and entry["binary_name"] == BINARY
                   and entry["arch"] in ("amd64", "arm64")
                   and re.fullmatch(re.escape(TOOL_VERSION) + r"-[0-9a-f]{40}-r1", entry["version"])
                   and set(entry.get("auxiliary_files", {})) == FILES - {BINARY},
                   "wrong or incomplete native TCP artifact identity")
        if entry["name"] == "agent":
            binary_name = "sinan-agent.exe" if entry["arch"].startswith("windows-") else "sinan-agent"
            ensure(metadata["tag"] == "agent-v" + entry["version"] and entry["format"] == "raw"
                   and entry["binary_name"] == binary_name, "wrong Agent release identity")
        if entry["format"] == "raw":
            ensure(entry["archive_size"] == entry["binary_size"]
                   and rows.get(path) == entry["binary_sha256"], "raw binary proof mismatch")
    ensure(set(rows) == expected_paths, "unsigned or unused checksum paths")
    ensure(rows["install.sh"] == digest(read_regular(bundle / "install.sh", 256 * 1024)), "installer mismatch")
    return metadata, rows


def verify_bundle(bundle, roots, minisign, expected_tag=None, exact_assets=True):
    bundle = Path(bundle)
    metadata, rows = verify_manifest(bundle, roots, minisign, expected_tag)
    expected_files = {"release.json", "install.sh", "SHA256SUMS", "SHA256SUMS.minisig"}
    for entry in metadata["artifacts"]:
        path = canonical_path(entry)
        expected_files.add(entry["asset_name"])
        data = read_regular(bundle / entry["asset_name"])
        ensure(len(data) == entry["archive_size"] and rows.get(path) == digest(data), "archive mismatch")
        binary = binary_bytes(data, entry["format"], entry["binary_name"], entry.get("auxiliary_files", {}))
        ensure(len(binary) == entry["binary_size"] and digest(binary) == entry["binary_sha256"], "binary mismatch")
        if entry["name"] == "tcpquality":
            from tcp_probe_artifact import archive_files, validate_files
            validate_files(archive_files(data), entry["version"], entry["arch"])
    if exact_assets:
        ensure({p.name for p in bundle.iterdir()} == expected_files, "missing or extra release assets")
    return metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("assemble")
    for argument in ("source", "output", "tag", "agent-version", "runtime-version", "installer"):
        build.add_argument("--" + argument, required=True)
    build.add_argument("--nodequality-version", default=NODEQUALITY_VERSION)
    build.add_argument("--tcp-probe-version", help="opt-in native TCP version with its full source SHA")
    build.add_argument("--arch", action="append", choices=("amd64", "arm64"),
                       help="CI test bundle architectures; production requires both")
    render = commands.add_parser("render-installer")
    for argument in ("template", "agent-unit", "runtime-unit", "output"):
        render.add_argument("--" + argument, required=True)
    verify = commands.add_parser("verify")
    verify.add_argument("--bundle", required=True)
    verify.add_argument("--trusted-keys", required=True)
    verify.add_argument("--minisign", default="minisign")
    verify.add_argument("--tag")
    verify.add_argument("--publication", action="store_true")
    args = parser.parse_args()
    if args.command == "assemble":
        assemble(args)
    elif args.command == "render-installer":
        render_installer(args)
    else:
        roots = load_roots(args.trusted_keys, args.publication)
        verify_bundle(args.bundle, roots, args.minisign, args.tag)
        print("Complete signature, canonical metadata, installer, and every artifact verified.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, json.JSONDecodeError) as error:
        raise SystemExit(f"Release verification failed: {error}") from error
SINAN_BOOTSTRAP_99E3BAC561E20F854C3A33D3E94BDF91FF76AEA4CC7C2A05847CE31A673CCBCE

cat > "$STAGING/tcp_probe_artifact.py" <<'SINAN_BOOTSTRAP_7C9C790035F22EC0554D1B922A5B960571792B991659DDFBF9C0A8E337C0BD0A'
"""Validate the complete, pinned native TCP artifact without executing it."""
import gzip
import hashlib
import io
import json
import re
import struct
import tarfile

TOOL_VERSION = "0.3.0"
BINARY = "sinan-tcp-probe"
FILES = {BINARY, "build-info.json", "LICENSE", "source.tar.gz", "Cargo.lock", "THIRD_PARTY_NOTICES.txt"}
TARGETS = {"amd64": "x86_64-unknown-linux-musl", "arm64": "aarch64-unknown-linux-musl"}
SOURCE_LIMIT = 16 * 1024 * 1024
BUNDLED_MUSL_FILES = ("tools/licenses/bundled-musl.json", "tools/licenses/musl-1.2.5-COPYRIGHT",
                      "tools/licenses/rust-1.98.1-musl-recipe.txt")
BUNDLED_MUSL_IDENTITY = {
    "schema": 1, "version": "1.2.5",
    "source_url": "https://musl.libc.org/releases/musl-1.2.5.tar.gz",
    "source_sha256": "a9a118bbe84d8764da0ea0d28b3ab3fae8477fc7e4085d90102b8596fc7c75e4",
    "copyright_sha256": "f9bc4423732350eb0b3f7ed7e91d530298476f8fec0c6c427a1c04ade22655af",
    "rustc_commit": "48a229ceaefd4985c50990b14116b6d856af0985",
    "rust_recipe_url": "https://raw.githubusercontent.com/rust-lang/rust/48a229ceaefd4985c50990b14116b6d856af0985/src/ci/docker/scripts/musl.sh",
    "rust_recipe_sha256": "2f218a2dc7b7e73509212bfd4319ebddc2ddac7c651fca142c2b29bd7ea0aa38",
    "patches": "CVE-2025-26519: two iconv patches in the pinned Rust recipe; copyright unchanged",
}


def ensure(condition, message):
    if not condition:
        raise ValueError(message)


def digest(value):
    return hashlib.sha256(value).hexdigest()


def artifact_version(commit):
    ensure(isinstance(commit, str) and re.fullmatch(r"[0-9a-f]{40}", commit),
           "TCP source commit must be an actual full lowercase SHA")
    return f"{TOOL_VERSION}-{commit}-r1"


def verify_elf(binary, arch):
    ensure(arch in TARGETS and len(binary) >= 64 and binary[:6] == b"\x7fELF\x02\x01",
           "TCP binary must be a little-endian ELF64")
    ensure(struct.unpack_from("<H", binary, 18)[0] == (62 if arch == "amd64" else 183),
           "TCP binary architecture mismatch")
    offset = struct.unpack_from("<Q", binary, 32)[0]
    size, count = struct.unpack_from("<HH", binary, 54)
    ensure(size == 56 and 0 < count <= 128 and offset + size * count <= len(binary),
           "invalid ELF program headers")
    for index in range(count):
        kind, _, start, _, _, length, _, _ = struct.unpack_from("<IIQQQQQQ", binary, offset + index * size)
        ensure(start + length <= len(binary), "ELF segment exceeds binary")
        ensure(kind != 3, "TCP binary has a dynamic interpreter")
        if kind == 2:
            ensure(length % 16 == 0, "invalid ELF dynamic segment")
            for entry in range(start, start + length, 16):
                tag = struct.unpack_from("<q", binary, entry)[0]
                ensure(tag != 1, "TCP binary has a dynamic library dependency")
                if tag == 0:
                    break


def source_files(data, commit):
    ensure(0 < len(data) <= SOURCE_LIMIT, "source archive size outside permitted range")
    with gzip.GzipFile(fileobj=io.BytesIO(data)) as compressed:
        unpacked = compressed.read(SOURCE_LIMIT + 1)
    ensure(len(unpacked) <= SOURCE_LIMIT, "source archive exceeds unpacked size limit")
    files, consumed = {}, 0
    with tarfile.open(fileobj=io.BytesIO(unpacked), mode="r:") as archive:
        ensure(archive.pax_headers.get("comment") == commit, "source archive commit mismatch")
        for member in archive:
            ensure(len(files) < 10000 and len(member.name) <= 1024
                   and not member.name.startswith("/") and "\\" not in member.name
                   and all(part not in ("", ".", "..") for part in member.name.rstrip("/").split("/")),
                   "unsafe source archive path")
            ensure(member.isdir() or member.isfile(), "source archive must contain ordinary files")
            if member.isdir():
                continue
            ensure(member.name not in files and 0 <= member.size <= SOURCE_LIMIT,
                   "duplicate or oversized source file")
            consumed += member.size
            ensure(consumed <= SOURCE_LIMIT, "source archive exceeds unpacked size limit")
            files[member.name] = archive.extractfile(member).read()
    required = {"Cargo.toml", "Cargo.lock", "LICENSE", "crates/tcp-probe/Cargo.toml",
                "crates/tcp-probe/src/lib.rs", "crates/tcp-probe/src/main.rs",
                "crates/tcp-probe/src/cli.rs", "crates/tcp-probe/src/engine.rs",
                "crates/tcp-probe/src/journal.rs", "crates/tcp-probe/src/model.rs",
                "tools/build-tcp-probe.py", "tools/tcp_probe_artifact.py", "tools/artifact_manifest.py", "tools/tcp_probe_notices.py"}
    ensure(required | set(BUNDLED_MUSL_FILES) <= files.keys(), "source archive is missing the tool or its build recipe")
    return files


def validate_files(files, version, arch):
    import tomllib

    ensure(set(files) == FILES, "TCP artifact must contain its exact complete provenance file set")
    ensure(all(0 < len(value) <= 256 * 1024 * 1024 for value in files.values()), "invalid TCP file size")
    info = json.loads(files["build-info.json"])
    expected = {"schema", "tool", "tool_version", "artifact_version", "source_repo", "source_commit",
                "target", "rustc", "cargo_locked", "source_sha256", "lock_sha256",
                "license_sha256", "binary_sha256", "notices_sha256"}
    ensure(isinstance(info, dict) and set(info) == expected, "invalid TCP build information")
    commit = info["source_commit"]
    ensure(version == artifact_version(commit) and info["artifact_version"] == version,
           "TCP artifact version must include its exact source commit")
    ensure(type(info["schema"]) is int and info["schema"] == 1 and info["tool"] == BINARY
           and info["tool_version"] == TOOL_VERSION and info["source_repo"] == "theLucius7/sinan"
           and info["cargo_locked"] is True and arch in TARGETS and info["target"] == TARGETS[arch]
           and isinstance(info["rustc"], str) and info["rustc"].startswith("rustc ") and len(info["rustc"]) <= 4096,
           "wrong TCP build identity or unlocked build")
    for filename, field in [(BINARY, "binary_sha256"), ("source.tar.gz", "source_sha256"),
                            ("Cargo.lock", "lock_sha256"), ("LICENSE", "license_sha256"),
                            ("THIRD_PARTY_NOTICES.txt", "notices_sha256")]:
        ensure(info[field] == digest(files[filename]), "TCP provenance digest mismatch")
    source = source_files(files["source.tar.gz"], commit)
    ensure(source["Cargo.lock"] == files["Cargo.lock"] and source["LICENSE"] == files["LICENSE"],
           "TCP lockfile or license differs from pinned source")
    workspace = tomllib.loads(source["Cargo.toml"].decode())
    package = tomllib.loads(source["crates/tcp-probe/Cargo.toml"].decode())["package"]
    ensure(workspace["workspace"]["package"]["version"] == TOOL_VERSION
           and workspace["workspace"]["package"]["license"] == "AGPL-3.0-only"
           and package["name"] == BINARY and package["version"] == {"workspace": True}
           and package["license"] == {"workspace": True}, "TCP source version or license mismatch")
    validate_notices(files["THIRD_PARTY_NOTICES.txt"], files["Cargo.lock"], arch, source, info["rustc"])
    verify_elf(files[BINARY], arch)
    return info


def archive_files(data):
    result, consumed = {}, 0
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive:
            ensure(member.name in FILES and member.name not in result and member.isfile()
                   and 0 < member.size <= 256 * 1024 * 1024, "unsafe TCP artifact member")
            consumed += member.size
            ensure(consumed <= 256 * 1024 * 1024, "TCP artifact exceeds unpacked file budget")
            result[member.name] = archive.extractfile(member).read()
    ensure(set(result) == FILES, "TCP artifact is missing provenance files")
    return result


def pack(files):
    output = io.BytesIO()
    with gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.USTAR_FORMAT) as archive:
            for name in sorted(files):
                member = tarfile.TarInfo(name)
                member.size = len(files[name])
                member.mode = 0o755 if name == BINARY else 0o644
                archive.addfile(member, io.BytesIO(files[name]))
    return output.getvalue()

def bundled_musl(source, rustc_info):
    ensure(set(BUNDLED_MUSL_FILES) <= source.keys(), "bundled musl originals are missing from pinned source")
    identity = json.loads(source[BUNDLED_MUSL_FILES[0]])
    ensure(identity == BUNDLED_MUSL_IDENTITY, "unknown bundled musl source inventory")
    copyright_text, recipe = (source[name] for name in BUNDLED_MUSL_FILES[1:])
    ensure(digest(copyright_text) == identity["copyright_sha256"]
           and digest(recipe) == identity["rust_recipe_sha256"], "bundled musl source originals differ")
    ensure(isinstance(rustc_info, str)
           and [line for line in rustc_info.splitlines() if line.startswith("commit-hash:")]
           == ["commit-hash: " + identity["rustc_commit"]],
           "bundled musl inventory does not cover this rustc commit")
    return dict(name="Rust bundled musl libc", version=identity["version"], notices=[
        dict(path="musl-1.2.5/COPYRIGHT", text=copyright_text.decode()),
        dict(path="rust-1.98.1/musl-recipe.txt", text=recipe.decode()),
        dict(path="bundled-musl/source.json", text=source[BUNDLED_MUSL_FILES[0]].decode()),
    ])


def validate_notices(encoded, lock_bytes, arch, source, rustc_info):
    import tomllib

    ensure(0 < len(encoded) <= 8 * 1024 * 1024, "invalid third-party notice inventory size")
    data = json.loads(encoded)
    ensure(isinstance(data, dict) and set(data) == {"schema", "target", "lock_sha256", "dependencies", "toolchain"}
           and type(data["schema"]) is int and data["schema"] == 1 and data["target"] == TARGETS[arch]
           and data["lock_sha256"] == digest(lock_bytes), "third-party inventory identity mismatch")
    locked = {(p["name"], p["version"], p.get("source")): p for p in tomllib.loads(lock_bytes.decode())["package"]}
    packages = data["dependencies"]
    ensure(isinstance(packages, list) and 0 < len(packages) <= 256, "native dependency notices are missing")
    seen = set()
    def originals(notices):
        ensure(isinstance(notices, list) and 0 < len(notices) <= 128, "license originals are missing")
        names = set()
        for item in notices:
            ensure(isinstance(item, dict) and set(item) == {"path", "text"}
                   and isinstance(item["path"], str) and 0 < len(item["path"]) <= 1024
                   and item["path"] not in names and not item["path"].startswith("/")
                   and all(p not in ("", ".", "..") for p in item["path"].split("/"))
                   and isinstance(item["text"], str) and 0 < len(item["text"]) <= 8 * 1024 * 1024,
                   "invalid license original")
            names.add(item["path"])
    for package in packages:
        ensure(isinstance(package, dict) and set(package) == {"name", "version", "source", "checksum", "license", "notices"},
               "invalid locked dependency notice")
        identity = (package["name"], package["version"], package["source"])
        ensure(all(isinstance(value, str) for value in identity) and identity not in seen
               and identity in locked and locked[identity].get("checksum") == package["checksum"]
               and isinstance(package["license"], str) and package["license"], "notice differs from locked dependency")
        originals(package["notices"])
        if "Unicode" in package["license"]:
            ensure(any("unicode" in n["path"].lower() for n in package["notices"]), "Unicode license original is missing")
        seen.add(identity)
    libraries = data["toolchain"]
    ensure(isinstance(libraries, list) and len(libraries) == 3
           and {p.get("name") for p in libraries} == {"Rust standard library and bundled native libraries", "system musl build tooling", "Rust bundled musl libc"},
           "native library license originals are missing")
    for library in libraries:
        ensure(set(library) == {"name", "version", "notices"} and isinstance(library["version"], str)
               and library["version"], "native library notice identity mismatch")
        originals(library["notices"])
    bundled = next(p for p in libraries if p["name"] == "Rust bundled musl libc")
    ensure(bundled == bundled_musl(source, rustc_info), "bundled musl notice differs from pinned source")
    standard = next(p for p in libraries if p["name"] == "Rust standard library and bundled native libraries")
    ensure(standard["version"] == rustc_info, "Rust notice and binary toolchain versions differ")
    return data
SINAN_BOOTSTRAP_7C9C790035F22EC0554D1B922A5B960571792B991659DDFBF9C0A8E337C0BD0A

cat > "$STAGING/tcp_probe_notices.py" <<'SINAN_BOOTSTRAP_7FE018A190D8AB15F25955182B8FFACA7BA35C543C7E1D562D9D68BE1F1EBA77'
"""Collect original notices from checksum-locked native Cargo packages and the toolchain."""
import io
import json
from pathlib import Path
import re
import subprocess
import tarfile
import tomllib

from tcp_probe_artifact import BINARY, TARGETS, BUNDLED_MUSL_FILES, bundled_musl, digest, ensure

LIMIT = 8 * 1024 * 1024


def capture(command):
    return subprocess.run(command, check=True, capture_output=True, timeout=60).stdout


def collect(repository, arch):
    target = TARGETS[arch]
    rustc_info = capture(["rustc", "-vV"]).decode()
    pinned = {name: (repository / name).read_bytes() for name in BUNDLED_MUSL_FILES}
    libc = bundled_musl(pinned, rustc_info)
    metadata = json.loads(capture(["cargo", "metadata", "--locked", "--format-version", "1",
                                  "--filter-platform", target, "--manifest-path", str(repository / "Cargo.toml")]))
    # Select normal/build edges for this package, not features unified across all workspace roots.
    tree = capture(["cargo", "tree", "--locked", "--package", BINARY, "--target", target,
                    "--edges", "normal,build", "--prefix", "none", "--format", "{p}",
                    "--manifest-path", str(repository / "Cargo.toml")]).decode()
    selected = set()
    for line in tree.splitlines():
        match = re.match(r"^([A-Za-z0-9_-]+) v([^ ]+)(?: |$)", line)
        ensure(match, "cannot parse locked native dependency selection")
        selected.add(match.groups())
    lock_bytes = (repository / "Cargo.lock").read_bytes()
    lock = {(p["name"], p["version"], p.get("source")): p for p in tomllib.loads(lock_bytes.decode())["package"]}
    dependencies = []
    for package in metadata["packages"]:
        identity = (package["name"], package["version"])
        if identity not in selected or package["name"] == BINARY:
            continue
        source = package.get("source")
        ensure(source and source.startswith("registry+"), "native dependencies require locked registry checksums")
        entry = lock.get((*identity, source))
        ensure(entry and re.fullmatch(r"[0-9a-f]{64}", entry.get("checksum", "")),
               "native dependency is absent from the locked checksum inventory")
        directory = Path(package["manifest_path"]).parent
        crate = directory.parents[2] / "cache" / directory.parent.name / (directory.name + ".crate")
        ensure(crate.is_file() and not crate.is_symlink(), "locked dependency source archive is missing")
        packed = crate.read_bytes()
        ensure(0 < len(packed) <= 16 * 1024 * 1024 and digest(packed) == entry["checksum"],
               "locked dependency source checksum mismatch")
        notices, count, total = [], 0, 0
        with tarfile.open(fileobj=io.BytesIO(packed), mode="r:gz") as archive:
            for member in archive:
                count += 1
                ensure(count <= 10000 and (member.isdir() or member.isfile()),
                       "unsafe locked dependency source")
                if member.isdir():
                    continue
                parts = member.name.split("/")
                ensure(parts[0] == directory.name and len(parts) > 1 and all(p not in ("", ".", "..") for p in parts),
                       "unsafe locked dependency source path")
                relative = "/".join(parts[1:])
                total += member.size
                ensure(0 <= member.size <= 16 * 1024 * 1024 and total <= 64 * 1024 * 1024,
                       "locked dependency source exceeds size limits")
                original = archive.extractfile(member).read()
                installed = directory / relative
                ensure(installed.is_file() and not installed.is_symlink() and installed.read_bytes() == original,
                       "installed dependency source differs from its locked archive")
                if re.match(r"^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE)(?:[._-]|$)", parts[-1], re.IGNORECASE):
                    ensure(0 < len(original) <= 512 * 1024, "dependency notice exceeds permitted size")
                    notices.append(dict(path=relative, text=original.decode("utf-8")))
        license_id = package.get("license")
        ensure(isinstance(license_id, str) and license_id and notices, "dependency license originals are missing")
        if "Unicode" in license_id:
            ensure(any("unicode" in n["path"].lower() for n in notices), "Unicode license original is missing")
        dependencies.append(dict(name=identity[0], version=identity[1], source=source,
                                 checksum=entry["checksum"], license=license_id, notices=sorted(notices, key=lambda n:n["path"])))
    ensure({(p["name"], p["version"]) for p in dependencies} == selected - {(BINARY, "0.3.0")},
           "native dependency selection is incomplete or ambiguous")
    sysroot = Path(capture(["rustc", "--print", "sysroot"]).decode().strip())
    rust_docs = sysroot / "share/doc/rust"
    originals = [rust_docs / "COPYRIGHT-library.html", *(rust_docs / "licenses").glob("*")]
    rust_notices = []
    for path in sorted(originals):
        ensure(path.is_file() and not path.is_symlink(), "Rust library notice original is missing")
        text = path.read_text()
        ensure(0 < len(text.encode()) <= LIMIT, "Rust library notice exceeds permitted size")
        rust_notices.append(dict(path=str(path.relative_to(rust_docs)), text=text))
    ensure(len(rust_notices) >= 2, "Rust library license originals are missing")
    musl_path = Path("/usr/share/doc/musl/copyright")
    ensure(musl_path.is_file() and not musl_path.is_symlink(), "system musl copyright original is missing")
    musl_version = capture(["dpkg-query", "-W", "-f", "${Version}", "musl"]).decode().strip()
    result = dict(schema=1, target=target, lock_sha256=digest(lock_bytes),
                  dependencies=sorted(dependencies, key=lambda p:(p["name"], p["version"], p["source"])),
                  toolchain=[
                      dict(name="Rust standard library and bundled native libraries",
                           version=rustc_info, notices=rust_notices),
                      dict(name="system musl build tooling", version=musl_version,
                           notices=[dict(path="musl/copyright", text=musl_path.read_text())]),
                      libc,
                  ])
    encoded = (json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    ensure(len(encoded) <= LIMIT, "third-party notice inventory exceeds size limit")
    return encoded
SINAN_BOOTSTRAP_7FE018A190D8AB15F25955182B8FFACA7BA35C543C7E1D562D9D68BE1F1EBA77

cat > "$STAGING/artifact_manifest.py" <<'SINAN_BOOTSTRAP_8B75C06855E5CA6222579D9DD5B14185451D81B0CB68219B1B2540F92415324D'
"""Publish immutable flat artifacts with a verified checksum manifest."""
import hashlib
import os
from pathlib import Path
import re
import uuid

KEY = re.compile(r"(?:amd64|arm64|linux-(?:gnu|musl)-(?:amd64|arm64)|macos-arm64|(?:freebsd|windows)-(?:amd64|arm64))\Z")


def publish(directory, name, payload):
    directory = Path(directory)
    if not KEY.fullmatch(name) or directory.is_symlink():
        raise ValueError("unsafe artifact target or directory")
    directory.mkdir(parents=True, exist_ok=True)
    manifest = directory / "SHA256SUMS"
    target = directory / name
    if target.exists() or target.is_symlink():
        raise FileExistsError(target)
    checksums = {}
    if manifest.is_symlink():
        raise ValueError("checksum manifest cannot be a symlink")
    if manifest.exists():
        for line in manifest.read_text(encoding="utf-8").splitlines():
            parts = line.split()
            if len(parts) != 2 or not re.fullmatch(r"[0-9a-fA-F]{64}", parts[0]) or not KEY.fullmatch(parts[1]) or parts[1] in checksums:
                raise ValueError("invalid checksum manifest")
            checksums[parts[1]] = parts[0].lower()
    for path in directory.iterdir():
        if KEY.fullmatch(path.name):
            if path.is_symlink() or not path.is_file() or checksums.get(path.name) != hashlib.sha256(path.read_bytes()).hexdigest():
                raise ValueError("existing artifact lacks a valid checksum")
    for key in checksums:
        if not (directory / key).is_file():
            raise ValueError("checksum references a missing artifact")
    temporary = directory / f".artifact-{uuid.uuid4()}"
    manifest_temporary = directory / f".checksums-{uuid.uuid4()}"
    try:
        with temporary.open("xb") as output:
            output.write(payload)
            output.flush()
            os.fsync(output.fileno())
        os.link(temporary, target)
        checksums[name] = hashlib.sha256(payload).hexdigest()
        with manifest_temporary.open("x", encoding="utf-8", newline="\n") as output:
            output.write("".join(f"{digest}  {key}\n" for key, digest in sorted(checksums.items())))
            output.flush()
            os.fsync(output.fileno())
        os.replace(manifest_temporary, manifest)
    finally:
        temporary.unlink(missing_ok=True)
        manifest_temporary.unlink(missing_ok=True)
    return target
SINAN_BOOTSTRAP_8B75C06855E5CA6222579D9DD5B14185451D81B0CB68219B1B2540F92415324D

cat > "$STAGING/public-keys.json" <<'SINAN_BOOTSTRAP_51121348A57E37D3396225828114D19A3F62EDB0AAD0F6157E7FBCBEBF56B576'
["RWS4aZYmyBmwROpGKjfADJqNedYCNRhlg0+UoIBjQHxXZxYL7XMlkGJN"]
SINAN_BOOTSTRAP_51121348A57E37D3396225828114D19A3F62EDB0AAD0F6157E7FBCBEBF56B576


# Isolated mode ignores Python environment/path overrides; imports use only this bundle.
python3 -I -c 'import runpy, sys; sys.path.insert(0, sys.argv.pop(1)); runpy.run_module("bootstrap", run_name="__main__")' "$STAGING" "$@" --trusted-keys "$STAGING/public-keys.json" --minisign "$(command -v minisign)"
