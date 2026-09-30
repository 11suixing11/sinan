"""Validate the complete, pinned native TCP artifact without executing it."""
import gzip
import hashlib
import io
import json
import re
import struct
import tarfile
import tomllib

TOOL_VERSION = "0.3.0"
BINARY = "sinan-tcp-probe"
FILES = {BINARY, "build-info.json", "LICENSE", "source.tar.gz", "Cargo.lock"}
TARGETS = {"amd64": "x86_64-unknown-linux-musl", "arm64": "aarch64-unknown-linux-musl"}
SOURCE_LIMIT = 16 * 1024 * 1024


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
                "tools/build-tcp-probe.py", "tools/tcp_probe_artifact.py"}
    ensure(required <= files.keys(), "source archive is missing the tool or its build recipe")
    return files


def validate_files(files, version, arch):
    ensure(set(files) == FILES, "TCP artifact must contain its exact complete provenance file set")
    ensure(all(0 < len(value) <= 256 * 1024 * 1024 for value in files.values()), "invalid TCP file size")
    info = json.loads(files["build-info.json"])
    expected = {"schema", "tool", "tool_version", "artifact_version", "source_repo", "source_commit",
                "target", "rustc", "cargo_locked", "source_sha256", "lock_sha256",
                "license_sha256", "binary_sha256"}
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
                            ("Cargo.lock", "lock_sha256"), ("LICENSE", "license_sha256")]:
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
