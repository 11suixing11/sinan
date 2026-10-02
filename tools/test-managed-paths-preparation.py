#!/usr/bin/env python3
"""Tool contracts only; these do not attest a running Agent or proxy path."""
import argparse
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
from unittest.mock import patch

from managed_paths_support import capture, identity, regular, within

TOOLS = Path(__file__).resolve().parent


def module(name, filename):
    specification = importlib.util.spec_from_file_location(name, TOOLS / filename)
    value = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(value)
    return value


PREPARE = module("managed_prepare_contract", "prepare-managed-paths-linux.py")
CONTROL = module("managed_controller_contract", "managed-paths-controller.py")


class PrivateInputs(unittest.TestCase):
    def test_symlink_fifo_and_outside_owned_paths_are_refused(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source = root / "ordinary"
            source.write_bytes(b"TEST_ONLY")
            (root / "link").symlink_to(source)
            os.mkfifo(root / "fifo")
            for path in (root / "link", root / "fifo"):
                with self.subTest(path=path.name), self.assertRaises((OSError, ValueError)):
                    regular(path)
            with self.assertRaises(ValueError):
                within(root, root.parent / "unowned")
            (root / "directory").symlink_to(root, target_is_directory=True)
            with self.assertRaises(ValueError):
                within(root, root / "directory/ordinary")
            self.assertEqual(identity(source), {"sha256": hashlib.sha256(b"TEST_ONLY").hexdigest(), "size": 9})

    def test_deadline_kills_child_that_outlives_parent_without_touching_sentinel(self):
        sentinel = subprocess.Popen([sys.executable, "-c", "import time;time.sleep(30)"], start_new_session=True)
        try:
            with tempfile.TemporaryDirectory() as temporary:
                pidfile = Path(temporary).resolve() / "child-pid"
                code = ("import os,time,pathlib; p=os.fork(); "
                        "pathlib.Path(" + repr(str(pidfile)) + ").write_text(str(p)) if p else None; "
                        "os._exit(0) if p else time.sleep(30)")
                started = time.monotonic()
                with self.assertRaisesRegex(ValueError, "controlled_command_timeout"):
                    capture([sys.executable, "-c", code], timeout=0.5)
                self.assertLess(time.monotonic() - started, 4)
                child = int(pidfile.read_text())
                # Linux orphan zombies can remain until PID 1 reaps them.
                alive = subprocess.run(["ps", "-o", "stat=", "-p", str(child)], capture_output=True, text=True, timeout=2)
                self.assertTrue(not alive.stdout.strip() or alive.stdout.strip().startswith("Z"))
                self.assertIsNone(sentinel.poll())
        finally:
            os.killpg(sentinel.pid, signal.SIGKILL)
            sentinel.wait(timeout=3)

    def test_output_overflow_is_bounded_and_original_failure_is_preserved(self):
        with tempfile.TemporaryDirectory() as temporary:
            log = Path(temporary).resolve() / "failed-command.json"
            with self.assertRaisesRegex(ValueError, "controlled_command_output_limit"):
                capture([sys.executable, "-c", "import os;os.write(1,b'x'*400000)"], log=log)
            saved = json.loads(log.read_text())
            self.assertEqual(saved["failure_type"], "ValueError")
            self.assertLessEqual(len(saved["stdout"]) + len(saved["stderr"]), 256 * 1024)
            with self.assertRaises(FileExistsError):
                capture([sys.executable, "-c", "print('second')"], log=log)

    def test_cleanup_observation_failure_preserves_original_command_log(self):
        with tempfile.TemporaryDirectory() as temporary:
            log = Path(temporary).resolve() / "failed-cleanup.json"
            with patch("managed_paths_support.group_has_live_members",
                       side_effect=[False, False, ValueError("cleanup_observation_failed")]):
                with self.assertRaisesRegex(ValueError, "cleanup_observation_failed"):
                    capture([sys.executable, "-c", "print('original failure');raise SystemExit(1)"], log=log)
            saved = json.loads(log.read_text())
            self.assertEqual(saved["returncode"], 1)
            self.assertEqual(saved["failure_type"], "ValueError")
            self.assertEqual(saved["cleanup_failure_type"], "ValueError")
            self.assertEqual(saved["stdout"], "original failure\n")


class NativePreparation(unittest.TestCase):
    def test_archive_is_deterministic_and_has_one_actual_binary(self):
        source = b"TEST_ONLY actual input bytes"
        first = PREPARE.archive_runtime(source)
        self.assertEqual(first, PREPARE.archive_runtime(source))
        with tarfile.open(fileobj=io.BytesIO(first), mode="r:gz") as archive:
            members = archive.getmembers()
            self.assertEqual(len(members), 1)
            self.assertEqual((members[0].name, members[0].size, members[0].mode, members[0].mtime),
                             ("sing-box", len(source), 0o755, 0))
            self.assertEqual(archive.extractfile(members[0]).read(), source)

    def test_capacity_rejection_precedes_build_signing_or_deletion(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            source, old = root / "source", root / "old-material"
            source.mkdir()
            old.write_bytes(b"preserve previous evidence")
            args = argparse.Namespace(dedicated_test_node=True, target="aarch64-unknown-linux-gnu",
                source_root=source, output_dir=root / "new-output", target_dir=root / "new-target",
                frozen_inputs=root / "freeze.json", frozen_inputs_sha256="a" * 64,
                runtime_binary=old, runtime_sha256=identity(old)["sha256"])
            with patch.object(PREPARE.platform, "system", return_value="Linux"), \
                 patch.object(PREPARE.platform, "machine", return_value="aarch64"), \
                 patch.object(PREPARE, "verify_frozen", return_value={"head": "b" * 40}), \
                 patch.object(PREPARE, "memory_available", return_value=2 * 1024**3), \
                 patch.object(PREPARE, "capacity", side_effect=ValueError("managed_build_disk_reserve_rejected")), \
                 patch.object(PREPARE, "capture") as command:
                with self.assertRaisesRegex(ValueError, "disk_reserve_rejected"):
                    PREPARE.prepare(args)
                command.assert_not_called()
            receipt = json.loads((args.output_dir / "prepare-failure.json").read_text())
            self.assertEqual(receipt["managed_acceptance"], "not_run")
            self.assertTrue(receipt["materials_retained"])
            self.assertFalse(args.target_dir.exists())
            self.assertEqual(old.read_bytes(), b"preserve previous evidence")
            self.assertFalse((args.output_dir / "prepared-artifacts.json").exists())

    def test_new_build_must_not_reuse_a_prior_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            output = root / "prior"
            output.mkdir()
            sentinel = output / "evidence.json"
            sentinel.write_bytes(b"original")
            args = argparse.Namespace(dedicated_test_node=True, target="aarch64-unknown-linux-gnu",
                source_root=root / "src", output_dir=output, target_dir=root / "target")
            with patch.object(PREPARE.platform, "system", return_value="Linux"), \
                 patch.object(PREPARE.platform, "machine", return_value="aarch64"):
                with self.assertRaisesRegex(ValueError, "fresh_build_and_output_required"):
                    PREPARE.prepare(args)
            self.assertEqual(sentinel.read_bytes(), b"original")


class ControllerContracts(unittest.TestCase):
    def test_enrollment_uses_bound_origin_and_private_single_argument_token(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            root.chmod(0o700)
            run_id = "8eabca26-702f-4fde-890b-60594eaf528d"
            marker = root / ".sinan-managed-test-run"
            marker.write_text(run_id + "\n")
            marker.chmod(0o600)
            path = root / "enrollment.json"
            row = {"agent_binary": "/opt/sinan/core/current/sinan-agent",
                   "agent_config": "/etc/sinan/agent.toml"}
            origin = "https://owned-panel.example.invalid:3443"
            manifest = {"run_id": run_id, "run_root": str(root),
                        "roles": {"A": row}, "panel": {"origin": origin}}
            request = {"schema": 1, "run_id": run_id, "operation": "enroll", "role": "A",
                       "arguments": {"descriptor_file": str(path)}}
            token = "--TEST_ONLY_private_token"
            descriptor = {"schema": 1, "run_id": run_id, "role": "A", "token": token,
                          "origin": "https://untrusted.example.invalid", "panel": "https://other.example.invalid",
                          "agent_binary": "/bin/false", "argv": ["--panel=https://untrusted.example.invalid"],
                          "enrollment": {"token": "TEST_ONLY_untrusted_extra"}}
            path.write_text(json.dumps(descriptor))
            path.chmod(0o600)
            with patch.object(CONTROL, "role_capture") as command, patch.object(CONTROL, "service") as service:
                result = CONTROL.dispatch(manifest, [], request)
                command.assert_called_once_with(row, [row["agent_binary"], "--config", row["agent_config"],
                    "enroll", "--panel=" + origin, "--token=" + token], timeout=45)
                service.assert_called_once_with(row, "start")
                self.assertEqual(result, {"role": "A", "ordinary_enrollment_completed": True})
            invalid_fields = (("run_id", "48a7f5bc-e160-43c2-9001-03e2f73f5493"), ("role", "B"),
                              ("token", None), ("token", True), ("token", 1), ("token", [token]),
                              ("token", {"value": token}), ("token", ""), ("token", "x" * 513))
            for field, value in invalid_fields:
                with self.subTest(field=field, value_type=type(value).__name__):
                    path.write_text(json.dumps({**descriptor, field: value}))
                    with patch.object(CONTROL, "role_capture") as command, patch.object(CONTROL, "service") as service:
                        with self.assertRaisesRegex(ValueError, "enrollment_descriptor_identity_invalid"):
                            CONTROL.dispatch(manifest, [], request)
                        command.assert_not_called()
                        service.assert_not_called()
            path.write_text(json.dumps(descriptor))
            extra_arguments = {**request, "arguments": {**request["arguments"],
                                "panel": "https://untrusted.example.invalid"}}
            with patch.object(CONTROL, "role_capture") as command, patch.object(CONTROL, "service") as service:
                with self.assertRaisesRegex(ValueError, "enrollment_descriptor_required"):
                    CONTROL.dispatch(manifest, [], extra_arguments)
                command.assert_not_called()
                service.assert_not_called()

    def test_panel_private_environment_cannot_point_at_another_database_or_store(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            panel = {"origin": "https://panel.example.invalid:3443", "data_dir": str(root / "panel-data"),
                     "postgres": {"database": "sinan_managed_TEST_ONLY", "username": "postgres", "port": 55437}}
            valid = {"SINAN_PUBLIC_URL": panel["origin"], "SINAN_DATA_DIR": panel["data_dir"],
                     "SINAN_DATABASE_URL": "postgresql://postgres@127.0.0.1:55437/sinan_managed_TEST_ONLY"}
            CONTROL.validate_panel_environment(valid, panel, root)
            for key, value in (
                    ("SINAN_DATABASE_URL", "postgresql://postgres@127.0.0.1:55437/other_database"),
                    ("SINAN_DATABASE_URL", "postgresql://postgres@127.0.0.1:55438/sinan_managed_TEST_ONLY"),
                    ("SINAN_DATABASE_URL", "postgresql://postgres@192.0.2.10:55437/sinan_managed_TEST_ONLY"),
                    ("SINAN_DATA_DIR", str(root.parent / "unowned")),
                    ("SINAN_PUBLIC_URL", "https://other.example.invalid:3443")):
                with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                    CONTROL.validate_panel_environment({**valid, key: value}, panel, root)
            with self.assertRaises(ValueError):
                CONTROL.validate_panel_environment({**valid, "SINAN_DATA_DIR": str(root.parent / "unowned")},
                                                   {**panel, "data_dir": str(root.parent / "unowned")}, root)

    def test_id_filter_refuses_injection_duplicates_booleans_and_unbounded_lists(self):
        for values in (["1);DELETE FROM servers;--"], [True], [1, 1], list(range(1, 66)), [0], [2**63]):
            with self.subTest(values=values), self.assertRaises(ValueError):
                CONTROL.evidence_sql({"owned_ids": {"servers": values}})
        with self.assertRaises(ValueError):
            CONTROL.evidence_sql({"owned_ids": {"password": []}})

    def test_fixed_sql_is_readonly_bounded_and_omits_sensitive_columns(self):
        sql = CONTROL.evidence_sql({"owned_ids": {"servers": [1, 2, 3], "chains": [4], "users": [5]}})
        self.assertTrue(sql.startswith("BEGIN READ ONLY;"))
        self.assertIn("statement_timeout='5s'", sql)
        self.assertIn("LIMIT 512", sql)
        for forbidden in ("UPDATE ", "INSERT ", "DELETE ", "source_json", "controller_secret", "private_key", "subscription_token", "password"):
            self.assertNotIn(forbidden.lower(), sql.lower())
        self.assertIn("ARRAY[1,2,3]::bigint[]", sql)
        self.assertIn("request_digest AS digest", sql)
        self.assertIn("m.applied_rev=d.revision", sql)

    def test_no_arbitrary_command_or_wrong_run_can_reach_system(self):
        for operation, run_id in (("execute", "owned"), ("cleanup", "another"), ("confirm_devices", "owned")):
            with self.subTest(operation=operation), patch.object(CONTROL, "capture") as command:
                with self.assertRaises(ValueError):
                    CONTROL.dispatch({"run_id": "owned"}, [], {"schema": 1, "run_id": run_id,
                                     "operation": operation, "arguments": {}})
                command.assert_not_called()


if __name__ == "__main__":
    unittest.main()
