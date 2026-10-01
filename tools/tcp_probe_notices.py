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
