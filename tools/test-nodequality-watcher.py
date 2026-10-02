#!/usr/bin/env python3
"""Real watcher ownership regressions; no benchmark or hardware test is run."""
import contextlib
import fcntl
import json
import os
from pathlib import Path
import select
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "plugins/nodequality/report.py"
MAX_LOG = 128 * 1024
WAIT_SECONDS = 6


def private_json(path, value):
    with path.open("x", encoding="utf-8") as stream:
        os.chmod(path, 0o600)
        json.dump(value, stream)
        stream.flush()
        os.fsync(stream.fileno())


def pid_identity(pid):
    """Bind an orphan's start time, unique fixture argv, group and session."""
    result = subprocess.run(["/bin/ps", "-ww", "-o", "stat=,lstart=,command=", "-p", str(pid)],
                            stdin=subprocess.DEVNULL, capture_output=True, timeout=2,
                            env={"PATH": "/usr/bin:/bin", "LANG": "C"})
    if len(result.stdout) + len(result.stderr) > 16384:
        raise RuntimeError("owned PID observation exceeded its output budget")
    if result.returncode not in (0, 1):
        raise RuntimeError("owned PID observation failed")
    fields = result.stdout.decode("utf-8", errors="strict").split(None, 6)
    # An orphan zombie has exited; this test cannot reap another PID's child.
    if not fields or fields[0].startswith("Z"):
        return None
    if len(fields) != 7:
        raise RuntimeError("owned PID observation has an invalid shape")
    try:
        return (" ".join(fields[1:6]), fields[6].strip(), os.getpgid(pid), os.getsid(pid))
    except ProcessLookupError:
        return None


def pid_live(pid):
    return pid_identity(pid) is not None


def watcher_live(row):
    expected = row.get("watcher_identity")
    return expected is not None and pid_identity(row["watcher_pid"]) == expected


def signal_owned_group(row, number):
    # Each process here was created with start_new_session=True by this test.
    # A live, unreaped direct child reserves its PID/group. After it is reaped,
    # never signal a historical PGID without a still-matching watcher identity.
    process = row["process"]
    parent_live = process.poll() is None
    orphan_live = watcher_live(row)
    if not parent_live and not orphan_live:
        return
    if parent_live and (os.getpgid(process.pid), os.getsid(process.pid)) != (process.pid, process.pid):
        raise RuntimeError("owned direct child's process group changed")
    if orphan_live and row["watcher_identity"][2:] != (process.pid, process.pid):
        raise RuntimeError("owned watcher's process group changed")
    try:
        os.killpg(process.pid, number)
    except ProcessLookupError:
        pass
    except PermissionError:
        # Darwin may retain an inaccessible group containing only zombies.
        if process.poll() is None or watcher_live(row):
            raise


def owner_main(workspace, evidence, ignored_signals):
    """Be the actual direct parent, retaining watcher output even after SIGKILL."""
    if ignored_signals:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGHUP, signal.SIG_IGN)
    watcher = None
    with (evidence / "watcher.stdout").open("xb") as stdout, \
            (evidence / "watcher.stderr").open("xb") as stderr:
        os.chmod(evidence / "watcher.stdout", 0o600)
        os.chmod(evidence / "watcher.stderr", 0o600)
        try:
            watcher = subprocess.Popen([sys.executable, "-B", str(REPORT), "watch-sections",
                str(workspace), str(os.getpid())], stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
            private_json(evidence / "owner-ready.json", {"owner_pid": os.getpid(), "watcher_pid": watcher.pid})
            deadline = time.monotonic() + 60
            recorded_exit = False
            while time.monotonic() < deadline:
                code = watcher.poll()
                if code is not None and not recorded_exit:
                    private_json(evidence / "watcher-exit.json", {"returncode": watcher.wait(timeout=2)})
                    recorded_exit = True
                readable, _, _ = select.select([sys.stdin], [], [], 0.05)
                if readable and not os.read(sys.stdin.fileno(), 1):
                    return
            raise RuntimeError("owned test parent exceeded its deadline")
        finally:
            if watcher is not None:
                if watcher.poll() is None:
                    watcher.terminate()
                    try:
                        watcher.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        watcher.kill()
                watcher.wait(timeout=2)


@unittest.skipUnless(os.name == "posix", "watcher ownership requires POSIX signals and flock")
class WatcherLifecycle(unittest.TestCase):
    def setUp(self):
        parent = os.environ.get("SINAN_WATCHER_TEST_EVIDENCE")
        if parent is not None:
            candidate = Path(parent)
            if not candidate.is_absolute() or candidate.is_symlink() or not candidate.is_dir():
                raise ValueError("evidence parent must be an existing absolute ordinary directory")
            parent = str(candidate.resolve())
        self.evidence = Path(tempfile.mkdtemp(prefix="sinan-watcher-test-", dir=parent)).resolve()
        self.evidence.chmod(0o700)
        self.groups = []
        self.handles = []
        self.logs = []
        self.addCleanup(self.cleanup)
        # This process is deliberately outside all owner/watcher groups.
        self.sentinel = subprocess.Popen([sys.executable, "-B", "-c", "import time;time.sleep(60)"],
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            start_new_session=True)
        self.groups.append({"process": self.sentinel, "watcher_pid": None, "watcher_identity": None})
        private_json(self.evidence / "test-identity.json", {"test": self.id(), "sentinel_pid": self.sentinel.pid})

    def diagnostics(self):
        rows = []
        for path in self.logs:
            if path.exists():
                with path.open("rb") as stream:
                    rows.append(path.name + ": " + stream.read(8192).decode("utf-8", errors="replace"))
        return "Evidence retained at " + str(self.evidence) + "\n" + "\n".join(rows)

    def bounded_logs(self):
        for path in self.logs:
            self.assertLessEqual(path.stat().st_size, MAX_LOG, self.diagnostics())

    def wait_for(self, predicate, description, seconds=WAIT_SECONDS):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.bounded_logs()
            if predicate():
                return
            time.sleep(0.03)
        self.fail(description + "\n" + self.diagnostics())

    def assert_sentinel(self):
        self.assertIsNone(self.sentinel.poll(), self.diagnostics())

    def cleanup(self):
        failures = []
        for row in reversed(self.groups):
            process, watcher_pid = row["process"], row["watcher_pid"]
            try:
                if process.stdin is not None and not process.stdin.closed:
                    process.stdin.close()
                signal_owned_group(row, signal.SIGTERM)
                try:
                    process.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    pass
                signal_owned_group(row, signal.SIGKILL)
                process.wait(timeout=2)
                deadline = time.monotonic() + 3
                while watcher_live(row) and time.monotonic() < deadline:
                    time.sleep(0.03)
                if watcher_live(row):
                    raise RuntimeError("owned watcher remained alive after cleanup")
            except BaseException as error:
                failures.append(type(error).__name__)
        for handle in self.handles:
            handle.close()
        private_json(self.evidence / "cleanup.json", {"failures": failures, "materials_retained": True,
            "direct_children_reaped": all(row["process"].returncode is not None for row in self.groups)})
        if failures:
            raise AssertionError("Owned process cleanup failed: " + ",".join(failures) + "\n" + self.diagnostics())

    def workspace(self, name):
        workspace = self.evidence / name
        workspace.mkdir(mode=0o700)
        (workspace / ".runner").mkdir(mode=0o700)
        results = workspace / ".nodequality-owned" / "BenchOs" / "result"
        results.mkdir(parents=True)
        for name, content in {
                "header_info.log": "TEST_ONLY header snapshot",
                "hardware_quality.log": "TEST_ONLY incomplete newer hardware",
                "hardware_quality.json": "{",
                "ip_quality.log": "TEST_ONLY completed IP snapshot",
                "ip_quality.json": '{"owned":true}',
                "net_quality.log": "TEST_ONLY partial network snapshot",
                "net_quality.json": '{"owned":true}'}.items():
            (results / name).write_text(content)
        historical = {"name": "hardware_quality", "text": "TEST_ONLY completed historical hardware",
                      "complete": True, "revision": 9, "collected_at": 1}
        private_json(workspace / "section-hardware_quality.json", historical)
        return workspace, results

    def start_owner(self, workspace, ignored_signals=False):
        directory = self.evidence / ("owner-" + str(len(self.groups)))
        directory.mkdir(mode=0o700)
        stdout, stderr = directory / "owner.stdout", directory / "owner.stderr"
        for path in (stdout, stderr):
            handle = path.open("xb")
            os.chmod(path, 0o600)
            self.handles.append(handle)
        self.logs.extend((stdout, stderr))
        process = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()), "--owner",
            str(workspace), str(directory), "ignore" if ignored_signals else "normal"],
            stdin=subprocess.PIPE, stdout=self.handles[-2], stderr=self.handles[-1], start_new_session=True)
        row = {"process": process, "watcher_pid": None, "watcher_identity": None, "directory": directory}
        self.groups.append(row)
        ready = directory / "owner-ready.json"
        self.wait_for(lambda: ready.exists(), "actual direct owner did not start")
        identity = json.loads(ready.read_bytes())
        self.assertEqual(identity["owner_pid"], process.pid)
        row["watcher_pid"] = identity["watcher_pid"]
        row["watcher_identity"] = pid_identity(row["watcher_pid"])
        self.assertIsNotNone(row["watcher_identity"], self.diagnostics())
        self.assertEqual(row["watcher_identity"][2:], (process.pid, process.pid))
        self.logs.extend((directory / "watcher.stdout", directory / "watcher.stderr"))
        self.wait_for(lambda: (workspace / "section-header_info.json").exists(), "watcher did not publish a real snapshot")
        self.assertIsNone(process.poll(), self.diagnostics())
        self.assertTrue(watcher_live(row), self.diagnostics())
        self.assert_sentinel()
        return row

    def assert_watcher_exits(self, row):
        self.wait_for(lambda: not watcher_live(row), "owned watcher did not exit")
        self.assert_sentinel()

    def test_sigkill_owner_exits_watcher_with_workspace_retained_or_removed(self):
        for remove_workspace in (False, True):
            with self.subTest(remove_workspace=remove_workspace):
                workspace, _ = self.workspace("kill-" + str(remove_workspace))
                historical = (workspace / "section-hardware_quality.json").read_bytes()
                row = self.start_owner(workspace)
                row["process"].kill()
                self.assertEqual(row["process"].wait(timeout=2), -signal.SIGKILL)
                if remove_workspace:
                    shutil.rmtree(workspace)
                self.assert_watcher_exits(row)
                if not remove_workspace:
                    self.assertEqual((workspace / "section-hardware_quality.json").read_bytes(), historical)

    def test_noncanonical_or_nonparent_owner_is_rejected_without_publication(self):
        workspace, _ = self.workspace("mismatch")
        historical = (workspace / "section-hardware_quality.json").read_bytes()
        for index, value in enumerate((str(self.sentinel.pid), "1", "0", "-1", "02", "+" + str(os.getpid()), "bad")):
            with self.subTest(owner=value):
                stdout, stderr = self.evidence / ("mismatch-" + str(index) + ".stdout"), self.evidence / ("mismatch-" + str(index) + ".stderr")
                handles = [path.open("xb") for path in (stdout, stderr)]
                for path in (stdout, stderr):
                    os.chmod(path, 0o600)
                self.handles.extend(handles)
                self.logs.extend((stdout, stderr))
                process = subprocess.Popen([sys.executable, "-B", str(REPORT), "watch-sections", str(workspace), value],
                    stdin=subprocess.DEVNULL, stdout=handles[0], stderr=handles[1], start_new_session=True)
                self.groups.append({"process": process, "watcher_pid": None, "watcher_identity": None})
                self.wait_for(lambda: process.poll() is not None, "invalid owner was accepted")
                self.assertNotEqual(process.wait(timeout=2), 0, self.diagnostics())
                self.assertFalse((workspace / "section-header_info.json").exists())
                self.assertEqual((workspace / "section-hardware_quality.json").read_bytes(), historical)
                self.assert_sentinel()

    def test_inherited_ignored_term_and_hup_are_restored_in_watcher(self):
        for number in (signal.SIGTERM, signal.SIGHUP):
            with self.subTest(signal=number):
                workspace, _ = self.workspace("signal-" + str(number))
                row = self.start_owner(workspace, ignored_signals=True)
                os.kill(row["watcher_pid"], number)
                self.assert_watcher_exits(row)
                exit_record = row["directory"] / "watcher-exit.json"
                self.wait_for(lambda: exit_record.exists(), "actual parent did not reap stopped watcher")
                self.assertIsInstance(json.loads(exit_record.read_bytes())["returncode"], int)
                self.assertIsNone(row["process"].poll(), self.diagnostics())

    def test_busy_chapter_lock_cannot_keep_an_orphan_alive(self):
        workspace, results = self.workspace("busy-lock")
        row = self.start_owner(workspace)
        previous = (workspace / "section-header_info.json").read_bytes()
        with (workspace / ".sections.lock").open("r+") as lock:
            def acquire_fixture_lock():
                try:
                    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    return True
                except BlockingIOError:
                    return False
            self.wait_for(acquire_fixture_lock, "test fixture could not acquire its independent chapter lock")
            (results / "header_info.log").write_text("TEST_ONLY newer header while lock busy")
            # Give a live iteration a chance to encounter the independently held
            # lock; keep it held throughout the parent-death exit deadline.
            end = time.monotonic() + 1.3
            while time.monotonic() < end:
                self.bounded_logs()
                self.assertTrue(watcher_live(row), self.diagnostics())
                time.sleep(0.03)
            row["process"].kill()
            row["process"].wait(timeout=2)
            self.assert_watcher_exits(row)
            self.assertEqual((workspace / "section-header_info.json").read_bytes(), previous)

    def test_runtime_disappearance_or_inode_replacement_exits_with_owner_alive(self):
        for replacement in (False, True):
            with self.subTest(replacement=replacement):
                workspace, _ = self.workspace("runtime-" + str(replacement))
                row = self.start_owner(workspace)
                historical = (workspace / "section-hardware_quality.json").read_bytes()
                (workspace / ".runner").rename(workspace / ".runner-retained")
                if replacement:
                    (workspace / ".runner").mkdir(mode=0o700)
                self.assert_watcher_exits(row)
                self.assertIsNone(row["process"].poll(), self.diagnostics())
                self.assertEqual((workspace / "section-hardware_quality.json").read_bytes(), historical)

    def test_workspace_inode_replacement_is_not_followed(self):
        workspace, _ = self.workspace("workspace-original")
        row = self.start_owner(workspace)
        historical = (workspace / "section-hardware_quality.json").read_bytes()
        displaced = self.evidence / "workspace-retained"
        workspace.rename(displaced)
        workspace.mkdir(mode=0o700)
        (workspace / ".runner").mkdir(mode=0o700)
        self.assert_watcher_exits(row)
        self.assertIsNone(row["process"].poll(), self.diagnostics())
        self.assertEqual((displaced / "section-hardware_quality.json").read_bytes(), historical)
        self.assertEqual(set(path.name for path in workspace.iterdir()), {".runner"})

    def test_real_snapshots_keep_completed_history_and_do_not_invent_completeness(self):
        workspace, results = self.workspace("snapshots")
        historical = (workspace / "section-hardware_quality.json").read_bytes()
        row = self.start_owner(workspace)
        ip = workspace / "section-ip_quality.json"
        network = workspace / "section-net_quality.json"
        self.wait_for(lambda: ip.exists() and network.exists(), "available chapters did not publish")
        self.assertTrue(json.loads(ip.read_bytes())["complete"])
        self.assertFalse(json.loads(network.read_bytes())["complete"])
        first = json.loads(network.read_bytes())
        self.assertEqual(first["text"], "TEST_ONLY partial network snapshot")
        (results / "net_quality.log").write_text("TEST_ONLY later network snapshot")
        self.wait_for(lambda: json.loads(network.read_bytes())["text"] == "TEST_ONLY later network snapshot",
                      "live watcher did not collect the next snapshot")
        later = json.loads(network.read_bytes())
        self.assertGreater(later["revision"], first["revision"])
        self.assertFalse(later["complete"])
        self.assertEqual((workspace / "section-hardware_quality.json").read_bytes(), historical)
        row["process"].kill()
        row["process"].wait(timeout=2)
        self.assert_watcher_exits(row)
        self.assertEqual((workspace / "section-hardware_quality.json").read_bytes(), historical)


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--owner":
        owner_main(Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4] == "ignore")
    else:
        unittest.main()
