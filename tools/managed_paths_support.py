"""Bounded private inputs and process ownership for TEST_ONLY managed acceptance."""
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import stat
import subprocess
import time
import uuid

MAX_JSON = 8 * 1024 * 1024
MAX_OUTPUT = 256 * 1024


def require(condition, code):
    if not condition:
        raise ValueError(code)


def absolute(value):
    path = Path(value)
    require(path.is_absolute() and ".." not in path.parts, "absolute_normal_path_required")
    return path


def regular(path, maximum=MAX_JSON, allow_empty=False):
    path = absolute(path)
    require(not any(parent.is_symlink() for parent in path.parents), "input_parent_symlink_refused")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(path, flags)
    try:
        before = os.fstat(fd)
        require(stat.S_ISREG(before.st_mode) and (before.st_size > 0 or allow_empty) and before.st_size <= maximum,
                "bounded_regular_input_required")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            data = stream.read(maximum + 1)
        after = os.fstat(fd)
        require(len(data) <= maximum and
                (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns) ==
                (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns),
                "input_changed_or_exceeded_limit")
        return data
    finally:
        os.close(fd)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def identity(path, maximum=256 * 1024 * 1024):
    data = regular(path, maximum, allow_empty=True)
    return {"sha256": digest(data), "size": len(data)}


def load(path):
    return json.loads(regular(path))


def write(path, value):
    data = (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    require(len(data) <= MAX_JSON, "private_evidence_limit")
    fd = os.open(absolute(path), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as target:
        target.write(data)
        target.flush()
        os.fsync(target.fileno())


def owned_root(root, run_id):
    root = absolute(root)
    require(not root.is_symlink() and root.is_dir() and root.stat().st_mode & 0o077 == 0,
            "private_owned_run_root_required")
    require(str(uuid.UUID(run_id)) == run_id and uuid.UUID(run_id).int != 0, "canonical_run_uuid_required")
    require(regular(root / ".sinan-managed-test-run", 128).decode().strip() == run_id,
            "run_ownership_mismatch")
    return root


def within(root, value):
    path = absolute(value)
    require(path.is_relative_to(root) and path != root, "path_outside_owned_run")
    cursor = path
    while cursor != root:
        require(not cursor.is_symlink(), "owned_path_symlink_refused")
        cursor = cursor.parent
    return path


def group_has_live_members(pgid):
    result = subprocess.run(["/bin/ps", "-A", "-o", "pid=,pgid=,stat="],
                            capture_output=True, check=False, timeout=2,
                            env={"PATH": "/usr/bin:/bin", "LANG": "C"})
    require(result.returncode == 0 and len(result.stdout) <= MAX_OUTPUT, "cleanup_observation_failed")
    rows = result.stdout.decode().splitlines()
    require(len(rows) <= 8192, "cleanup_process_budget")
    return any(len(fields := row.split()) == 3 and int(fields[1]) == pgid and
               not fields[2].startswith("Z") for row in rows)


def signal_group_if_live(pgid, number):
    if not group_has_live_members(pgid):
        return
    try:
        os.killpg(pgid, number)
    except ProcessLookupError:
        pass
    except PermissionError:
        # Darwin can return EPERM after TERM has left only orphan zombies.
        require(not group_has_live_members(pgid), "owned_group_cleanup_permission_denied")


def capture(command, timeout=30, input_bytes=None, cwd=None, env=None, log=None, guard=None, pass_fds=()):
    """Kill our complete group even when a parent exits with a pipe-holding child."""
    require(timeout > 0 and (input_bytes is None or len(input_bytes) <= MAX_OUTPUT),
            "command_budget_invalid")
    process = subprocess.Popen(command, stdin=subprocess.PIPE if input_bytes is not None else subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True,
                               cwd=cwd, env=env, pass_fds=pass_fds)
    deadline = time.monotonic() + timeout
    output, errors, failure = bytearray(), bytearray(), None
    selector = selectors.DefaultSelector()
    for stream, target in ((process.stdout, output), (process.stderr, errors)):
        os.set_blocking(stream.fileno(), False)
        selector.register(stream, selectors.EVENT_READ, target)
    if input_bytes is not None:
        os.set_blocking(process.stdin.fileno(), False)
        selector.register(process.stdin, selectors.EVENT_WRITE, memoryview(input_bytes))
    try:
        while selector.get_map() or process.poll() is None:
            require(time.monotonic() < deadline, "controlled_command_timeout")
            if guard is not None:
                guard()
            for key, _ in selector.select(min(0.25, max(0, deadline - time.monotonic()))):
                if key.fileobj is process.stdin:
                    try:
                        consumed = os.write(key.fd, key.data[:4096])
                    except BrokenPipeError:
                        consumed = len(key.data)
                    pending = key.data[consumed:]
                    if pending:
                        selector.modify(key.fileobj, selectors.EVENT_WRITE, pending)
                    else:
                        selector.unregister(key.fileobj)
                        key.fileobj.close()
                    continue
                block = os.read(key.fd, 4096)
                if not block:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                else:
                    allowance = MAX_OUTPUT - len(output) - len(errors)
                    key.data.extend(block[:allowance])
                    require(len(block) <= allowance, "controlled_command_output_limit")
        require(process.returncode == 0, "controlled_command_failed")
        return bytes(output)
    except BaseException as error:
        failure = type(error).__name__
        raise
    finally:
        cleanup_failure = None
        try:
            signal_group_if_live(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                pass
            # The parent may already have exited; its descendants still own this group.
            signal_group_if_live(process.pid, signal.SIGKILL)
            process.wait(timeout=3)
            cleanup_deadline = time.monotonic() + 2
            while group_has_live_members(process.pid):
                require(time.monotonic() < cleanup_deadline, "owned_group_cleanup_not_confirmed")
                time.sleep(0.02)
        except BaseException as error:
            cleanup_failure = type(error).__name__
            raise
        finally:
            for key in list(selector.get_map().values()):
                key.fileobj.close()
            selector.close()
            if log is not None:
                write(log, {"returncode": process.returncode, "failure_type": failure,
                            "cleanup_failure_type": cleanup_failure,
                            "stdout": output.decode("utf-8", "replace"),
                            "stderr": errors.decode("utf-8", "replace")})
