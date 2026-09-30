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
