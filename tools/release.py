#!/usr/bin/env python3
"""Build canonical release manifests and verify complete offline-signed bundles."""

import argparse
import base64
import hashlib
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
NODEQUALITY_VERSION = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2"
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
    ensure(entry["name"] in ("agent", "sing-box", "nodequality"), "unsupported module")
    ensure(SEGMENT.fullmatch(entry["version"]), "invalid version segment")
    ensure(entry["arch"] in ("amd64", "arm64"), "unsupported architecture")
    return "/".join(entry[k] for k in ("name", "version", "arch"))


def asset_name(entry):
    name, version, arch = (entry[k] for k in ("name", "version", "arch"))
    ensure(entry["format"] in ("raw", "tar.gz"), "unsupported format")
    return (f"{name}-{version}-linux-musl-{arch}" if entry["format"] == "raw"
            else f"{name}-{version}-linux-{arch}.tar.gz")


def binary_bytes(data, archive_format, binary_name):
    if archive_format == "raw":
        return data
    ensure(archive_format == "tar.gz", "unsupported format")
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        members = archive.getmembers()
        ensure(len(members) == 1, "archive must contain exactly one binary")
        member = members[0]
        ensure(member.name == binary_name and member.isfile(), "unsafe archive member")
        ensure(0 < member.size <= MAX_BINARY, "invalid binary size")
        file = archive.extractfile(member)
        ensure(file is not None, "archive binary unavailable")
        binary = file.read(MAX_BINARY + 1)
        ensure(len(binary) == member.size, "archive binary length mismatch")
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
    for name, version, archive_format, binary_name in (
        ("agent", args.agent_version, "raw", "sinan-agent"),
        ("sing-box", args.runtime_version, "tar.gz", "sing-box"),
        ("nodequality", getattr(args, "nodequality_version", NODEQUALITY_VERSION), "tar.gz", "nodequality"),
    ):
        for arch in architectures:
            entry = {"name": name, "version": version, "arch": arch,
                     "format": archive_format, "binary_name": binary_name}
            path = canonical_path(entry)
            data = read_regular(source / path)
            binary = binary_bytes(data, archive_format, binary_name)
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
    (output / "SHA256SUMS").write_text(checksums)


def render_installer(args):
    text = read_regular(Path(args.template), 262144).decode("utf-8")
    for marker, filename in (("@@AGENT_UNIT@@", args.agent_unit),
                             ("@@RUNTIME_UNIT@@", args.runtime_unit)):
        ensure(text.count(marker) == 1, "missing or duplicate installer unit marker")
        text = text.replace(marker, read_regular(Path(filename), 65536).decode("utf-8").rstrip())
    ensure("@@" not in text, "unexpanded installer marker")
    output = Path(args.output)
    ensure(not output.exists(), "installer output exists")
    output.write_text(text)


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
    ensure(isinstance(metadata["artifacts"], list) and 0 < len(metadata["artifacts"]) <= 16,
           "invalid artifact list")
    expected_paths = {"release.json", "install.sh"}
    for entry in metadata["artifacts"]:
        ensure(set(entry) == {"name", "version", "arch", "format", "binary_name", "archive_size",
                              "binary_sha256", "binary_size", "asset_name"}, "unexpected artifact fields")
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
        if entry["name"] == "agent":
            ensure(metadata["tag"] == "agent-v" + entry["version"] and entry["format"] == "raw"
                   and entry["binary_name"] == "sinan-agent", "wrong Agent release identity")
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
        binary = binary_bytes(data, entry["format"], entry["binary_name"])
        ensure(len(binary) == entry["binary_size"] and digest(binary) == entry["binary_sha256"], "binary mismatch")
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
