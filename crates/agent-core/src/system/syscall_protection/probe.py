"""Test fixture: call swap APIs only for an absent path inside a root-owned 0700 directory."""
import ctypes
import errno
import json
import os
from pathlib import Path
import stat
import subprocess
import sys

directory = Path(sys.argv[1])
expected = int(sys.argv[2])
metadata = directory.lstat()
assert os.geteuid() == 0
assert stat.S_ISDIR(metadata.st_mode) and stat.S_IMODE(metadata.st_mode) == 0o700
assert metadata.st_uid == 0
target = directory / "never-created-swap"
assert not target.exists() and not target.is_symlink()
assert expected in (errno.ENOENT, errno.EPERM)


def probe():
    before = Path("/proc/swaps").read_bytes()
    fields = {}
    for line in Path("/proc/self/status").read_text().splitlines():
        name, _, value = line.partition(":")
        if name in ("NoNewPrivs", "Seccomp", "Seccomp_filters"):
            fields[name] = int(value.strip())
    if expected == errno.EPERM:
        assert fields["NoNewPrivs"] == 1 and fields["Seccomp"] == 2
        assert fields["Seccomp_filters"] >= 2
    else:
        assert fields["Seccomp"] == 0
    libc = ctypes.CDLL(None, use_errno=True)
    outcomes = {}
    for operation in ("swapon", "swapoff"):
        function = getattr(libc, operation)
        function.restype = ctypes.c_int
        function.argtypes = [ctypes.c_char_p] + ([ctypes.c_int] if operation == "swapon" else [])
        ctypes.set_errno(0)
        arguments = [os.fsencode(target)] + ([0] if operation == "swapon" else [])
        result = function(*arguments)
        observed = ctypes.get_errno()
        assert result == -1 and observed == expected, (operation, result, observed, expected)
        outcomes[operation] = observed
    assert Path("/proc/swaps").read_bytes() == before
    assert not target.exists() and not target.is_symlink()
    return {"errno": outcomes, "status": fields}


if len(sys.argv) > 3 and sys.argv[3] == "leaf":
    print(json.dumps(probe(), sort_keys=True))
else:
    reports = {"direct": probe()}
    reader, writer = os.pipe()
    child = os.fork()
    if child == 0:
        os.close(reader)
        try:
            os.write(writer, json.dumps(probe()).encode())
        except BaseException:
            os._exit(1)
        os._exit(0)
    os.close(writer)
    reports["fork"] = json.loads(os.read(reader, 4096))
    os.close(reader)
    _, status = os.waitpid(child, 0)
    assert os.waitstatus_to_exitcode(status) == 0
    completed = subprocess.run(
        [sys.executable, __file__, str(directory), str(expected), "leaf"],
        capture_output=True, text=True, timeout=3, check=True,
    )
    reports["exec"] = json.loads(completed.stdout)
    output = json.dumps(reports, sort_keys=True)
    if len(sys.argv) > 3:
        (directory / sys.argv[3]).write_text(output)
    else:
        print(output)
