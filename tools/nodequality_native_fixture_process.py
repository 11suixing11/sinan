"""Bounded subprocess ownership for inert, independent native runner fixtures."""
import contextlib
import hashlib
import json
import math
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
import threading
import uuid

from managed_paths_support import group_has_live_members as linux_group_has_live_members


MAX_OUTPUT = 2 * 1024 * 1024


def group_has_live_members(pgid):
    if sys.platform == "linux":
        return linux_group_has_live_members(pgid)
    if sys.platform != "darwin":
        raise ValueError("fixture_process_observation_platform_unsupported")
    # Observe only PID/group/state metadata, without operator command lines.
    selector = selectors.DefaultSelector()
    process = None
    captured = {}
    deadline = time.monotonic() + 2
    try:
        process = subprocess.Popen(["/bin/ps", "-axo", "pid=,pgid=,stat="],
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env={"PATH": "/usr/bin:/bin", "LANG": "C", "LC_ALL": "C"})
        captured = {process.stdout: bytearray(), process.stderr: bytearray()}
        for stream, target in captured.items():
            os.set_blocking(stream.fileno(), False)
            selector.register(stream, selectors.EVENT_READ, target)
        while selector.get_map():
            if time.monotonic() >= deadline:
                raise ValueError("fixture_process_observation_timeout")
            for key, _ in selector.select(min(0.05, max(0, deadline-time.monotonic()))):
                block = os.read(key.fd, 4096)
                if not block:
                    selector.unregister(key.fileobj)
                    continue
                if sum(map(len, captured.values())) + len(block) > 256 * 1024:
                    raise ValueError("fixture_process_observation_output_limit")
                key.data.extend(block)
        process.wait(timeout=max(0.01, deadline-time.monotonic()))
        if process.returncode != 0 or captured[process.stderr]:
            raise ValueError("fixture_process_observation_failed")
        rows = captured[process.stdout].decode("ascii").splitlines()
        if len(rows) > 8192:
            raise ValueError("fixture_process_observation_row_limit")
        for row in rows:
            fields = row.split()
            if len(fields) != 3 or not all(value.isdigit() for value in fields[:2]) or not fields[2]:
                raise ValueError("fixture_process_observation_shape")
            if int(fields[1]) == pgid and not fields[2].startswith("Z"):
                return True
        return False
    finally:
        # ps is a direct, trusted metadata child; never signal it after reaping.
        try:
            if process is not None and process.returncode is None:
                process.kill()
                process.wait(timeout=3)
        finally:
            try:
                for stream in captured:
                    stream.close()
            finally:
                selector.close()


def signal_group_if_live(pgid, number):
    if not group_has_live_members(pgid):
        return
    try:
        os.killpg(pgid, number)
    except ProcessLookupError:
        pass
    except PermissionError:
        # Darwin can retain an inaccessible zombie-only group. A live member
        # still matching the unreaped leader makes this a cleanup failure.
        if group_has_live_members(pgid):
            raise


class ObservedProcess(subprocess.Popen):
    """Keep the direct leader unreaped until all group signals have finished."""
    def __init__(self, *args, **kwargs):
        if (sys.platform not in ("linux", "darwin") or not hasattr(os, "waitid")
                or not hasattr(os, "WNOWAIT")):
            raise ValueError("fixture_nonreaping_process_observation_required")
        if kwargs.get("start_new_session") is not True:
            raise ValueError("fixture_owned_session_required")
        self.reaped = False
        super().__init__(*args, **kwargs)

    def poll(self):
        if not self.reaped:
            observed = os.waitid(os.P_PID, self.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            if observed is not None:
                self.returncode = (observed.si_status if observed.si_code == os.CLD_EXITED
                                   else -observed.si_status)
        return self.returncode

    def wait(self, timeout=None):
        if not finite_timeout(timeout, allow_zero=True):
            raise ValueError("fixture_process_wait_budget_required")
        deadline = time.monotonic() + timeout
        while self.poll() is None:
            if time.monotonic() >= deadline:
                raise subprocess.TimeoutExpired(self.args, timeout)
            time.sleep(min(0.02, max(0, deadline-time.monotonic())))
        return self.returncode

    def reap(self, timeout=3):
        if not finite_timeout(timeout, allow_zero=True):
            raise ValueError("fixture_process_wait_budget_required")
        if not self.reaped:
            self.returncode = None
            result = super().wait(timeout=timeout)
            self.reaped = True
            return result
        return self.returncode


def finite_timeout(timeout, *, allow_zero=False):
    return (isinstance(timeout, (int, float)) and not isinstance(timeout, bool)
            and (0 <= timeout if allow_zero else 0 < timeout) and timeout <= 3600
            and math.isfinite(timeout))


def note(error, message):
    if hasattr(error, "add_note"):
        error.add_note(message)


@contextlib.contextmanager
def cancellation_signals(defer=False):
    if threading.current_thread() is not threading.main_thread():
        yield
        return
    pending = []
    def interrupted(number, frame):
        if defer:
            pending.append(number)
        else:
            raise KeyboardInterrupt("fixture_cancelled_by_signal_" + str(number))
    previous = {number: signal.getsignal(number) for number in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP)}
    try:
        for number in previous:
            signal.signal(number, interrupted)
        yield
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)
        if pending:
            raise KeyboardInterrupt("fixture_cancelled_by_signal_" + str(pending[0]))


class OwnedProcess:
    def __init__(self, command, directory, env=None, owner=None):
        self.command = command
        self.log = Path(directory) / ("fixture-process-" + uuid.uuid4().hex + ".json")
        self.process = None
        self.selector = selectors.DefaultSelector()
        self.stdout, self.stderr = bytearray(), bytearray()
        self.cleanup_confirmed = False
        self.failure = None
        self.cleanup_failures = []
        if owner is not None:
            owner.children.append(self)
        try:
            # Defer parent cancellation only until the new group and both pipes
            # have been registered. The child inherits no blocked signal mask.
            with cancellation_signals(defer=True):
                self.process = ObservedProcess(command, stdin=subprocess.DEVNULL,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, start_new_session=True)
                for stream, output in ((self.process.stdout, self.stdout), (self.process.stderr, self.stderr)):
                    os.set_blocking(stream.fileno(), False)
                    self.selector.register(stream, selectors.EVENT_READ, output)
        except BaseException as error:
            self.failure = error
            self.stop(error)
            raise

    @property
    def pid(self):
        return self.process.pid

    def read_ready(self, timeout):
        for key, _ in self.selector.select(timeout):
            block = os.read(key.fd, 4096)
            if not block:
                self.selector.unregister(key.fileobj)
                key.fileobj.close()
                continue
            available = MAX_OUTPUT - len(self.stdout) - len(self.stderr)
            key.data.extend(block[:available])
            if len(block) > available:
                raise ValueError("fixture_process_output_limit")

    def collect(self, timeout, guard=None):
        if not finite_timeout(timeout):
            raise ValueError("fixture_process_deadline_required")
        end = time.monotonic() + timeout
        while self.selector.get_map() or self.process.poll() is None:
            if time.monotonic() >= end:
                raise subprocess.TimeoutExpired(self.command, timeout,
                    output=bytes(self.stdout), stderr=bytes(self.stderr))
            if guard is not None:
                guard(self)
            self.read_ready(min(0.05, max(0, end - time.monotonic())))
        return subprocess.CompletedProcess(self.command, self.process.returncode,
                                           bytes(self.stdout), bytes(self.stderr))

    def write_log(self):
        value = {"pid": self.process.pid if self.process else None,
                 "returncode": self.process.returncode if self.process else None,
                 "command_sha256": hashlib.sha256(json.dumps(self.command).encode()).hexdigest(),
                 "cleanup_confirmed": self.cleanup_confirmed,
                 "primary_failure": None if self.failure is None else {
                     "type": type(self.failure).__name__, "message": str(self.failure)[:4096]},
                 "cleanup_failures": self.cleanup_failures,
                 "stdout": self.stdout.decode("utf-8", "replace"),
                 "stderr": self.stderr.decode("utf-8", "replace")}
        descriptor = os.open(self.log, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w") as target:
            json.dump(value, target, sort_keys=True)
            target.write("\n")
            target.flush()
            os.fsync(target.fileno())

    def stop(self, primary=None):
        try:
            with cancellation_signals(defer=True):
                self._stop(primary)
        except KeyboardInterrupt as error:
            if primary is None:
                raise
            note(primary, "Additional fixture cancellation was deferred through owned cleanup")

    def _stop(self, primary=None):
        if primary is not None:
            self.failure = primary
        if self.cleanup_confirmed:
            return
        errors = []
        try:
            if self.process is not None:
                # Only Popen(start_new_session=True) gives us this group. The
                # leader can have exited while a descendant still holds a pipe.
                if not self.process.reaped:
                    signal_group_if_live(self.process.pid, signal.SIGTERM)
                    try:
                        self.process.wait(timeout=1)
                    except subprocess.TimeoutExpired:
                        pass
                    signal_group_if_live(self.process.pid, signal.SIGKILL)
                    self.process.reap(timeout=3)
                end = time.monotonic() + 2
                while group_has_live_members(self.process.pid):
                    if time.monotonic() >= end:
                        raise ValueError("fixture_owned_group_cleanup_not_confirmed")
                    time.sleep(0.02)
                self.cleanup_confirmed = True
            else:
                self.cleanup_confirmed = True
        except BaseException as error:
            errors.append(error)
        finally:
            # Every failure path closes pipes and records both failures before
            # the owning fixture can remove its temporary directory.
            try:
                if self.selector.get_map():
                    self.read_ready(0)
            except BaseException as error:
                errors.append(error)
            for stream in (self.process.stdin, self.process.stdout, self.process.stderr) if self.process else ():
                if stream is not None:
                    try:
                        stream.close()
                    except BaseException as error:
                        errors.append(error)
            try:
                self.selector.close()
            except BaseException as error:
                errors.append(error)
            self.cleanup_failures.extend({"type": type(error).__name__, "message": str(error)[:4096]}
                                         for error in errors)
            try:
                if self.log.exists():
                    self.log = self.log.with_name("fixture-process-" + uuid.uuid4().hex + ".json")
                self.write_log()
            except BaseException as error:
                errors.append(error)
                self.cleanup_failures.append({"type": type(error).__name__, "message": str(error)[:4096]})
        if errors:
            if primary is not None:
                note(primary, "Fixture cleanup failed; evidence retained at " + str(self.log))
                return
            raise RuntimeError("fixture_process_cleanup_failed: " + str(self.log)) from errors[0]


class OwnedProcesses:
    def __init__(self, directory):
        self.directory = Path(directory)
        self.children = []
        self.cleanup_failed = False

    def spawn(self, command, env=None):
        return OwnedProcess(command, self.directory, env, self)

    def stop(self, child, primary=None):
        try:
            child.stop(primary)
        except BaseException:
            self.cleanup_failed = True
            raise
        if not child.cleanup_confirmed or child.cleanup_failures:
            self.cleanup_failed = True

    def run(self, command, *, timeout, env=None, capture_output=True, text=False, check=False, guard=None):
        if not capture_output:
            raise ValueError("fixture_capture_required")
        with cancellation_signals():
            child = None
            primary = None
            try:
                with cancellation_signals(defer=True):
                    child = self.spawn(command, env)
                result = child.collect(timeout, guard)
                if text:
                    result.stdout = result.stdout.decode()
                    result.stderr = result.stderr.decode()
                if check:
                    result.check_returncode()
                return result
            except BaseException as error:
                primary = error
                raise
            finally:
                if child is not None:
                    self.stop(child, primary)

    def close(self):
        errors = []
        for child in self.children:
            try:
                self.stop(child)
            except BaseException as error:
                errors.append(error)
        if errors or self.cleanup_failed:
            raise RuntimeError("fixture_cleanup_failed_evidence_retained: " + str(self.directory)) from (
                errors[0] if errors else None)

    def cleanup_temporary(self, temporary, test_case=None):
        try:
            self.close()
        except BaseException:
            # TemporaryDirectory's destructor otherwise erases failure evidence.
            temporary._finalizer.detach()
            raise
        outcome = getattr(test_case, "_outcome", None)
        if any(child.failure is not None for child in self.children) or (
                outcome is not None and not outcome.success):
            # A verified cleanup does not erase primary failures, nor a later
            # business assertion's logs and unchanged private workspace.
            temporary._finalizer.detach()
            return
        temporary.cleanup()
