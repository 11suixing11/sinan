#!/usr/bin/env python3
"""Contract regressions for the TEST_ONLY managed-path network helper."""
import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location("managed_paths_fixtures", Path(__file__).with_name("managed-paths-fixtures.py"))
HELPER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HELPER)


def plan(root):
    return {"schema": 1, "test_only": True, "run_id": "00000000-0000-4000-8000-000000000001",
            "native_binary": str(root / "native"), "native_sha256": "0" * 64,
            "addresses": {"fixture": "10.83.0.1", "A": "10.83.0.2", "M": "10.83.0.3",
                          "B": "10.83.0.4", "client": "10.83.0.5"},
            "ports": {"x": 21001, "handshake": 21002, "tcp_echo": 21003, "udp_echo": 21004, "https": 21005},
            "managed_ports": {"A": [20011, 20012], "M": 20001, "B": 20001}}


def identity(pid, namespace):
    return {"pid": pid, "starttime": pid * 10, "netns_inode": namespace}


class FakeSocket:
    def __init__(self, data):
        self.data, self.sent = bytearray(data), bytearray()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        pass

    def settimeout(self, value):
        pass

    def recv(self, size):
        result = bytes(self.data[:size])
        del self.data[:size]
        return result

    def sendall(self, value):
        self.sent.extend(value)


def frame(kind, flags, body=b"", stream=1):
    return len(body).to_bytes(3, "big") + bytes([kind, flags]) + struct.pack("!I", stream) + body


def varint(value):
    result = bytearray()
    while value >= 128:
        result.append((value & 127) | 128)
        value >>= 7
    result.append(value)
    return bytes(result)


class HelperContractTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name).resolve()
        os.chmod(self.root, 0o700)
        HELPER.DEADLINE = time.monotonic() + 30
        HELPER.STDOUT_BYTES = 0

    def tearDown(self):
        self.temporary.cleanup()

    def assertRejected(self, code, action):
        with self.assertRaisesRegex(HELPER.Rejected, code):
            action()

    def test_plan_requires_test_only_and_has_no_product_or_exec_inputs(self):
        value = plan(self.root)
        self.assertEqual(HELPER.validate_plan(value), value)
        for key, replacement in (("test_only", False), ("schema", 2)):
            altered = {**value, key: replacement}
            with self.assertRaises(HELPER.Rejected):
                HELPER.validate_plan(altered)
        for key in ("product_configs", "receipt", "exec", "systemctl", "ledger"):
            with self.assertRaises(HELPER.Rejected):
                HELPER.validate_plan({**value, key: "forbidden"})
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            HELPER.parser().parse_args(["serve", "--manifest", str(self.root / "manifest.json"), "--exec", "anything"])

    def test_port_lists_preserve_two_entry_ports_and_shared_port_across_guests(self):
        value = plan(self.root)
        endpoints = HELPER.endpoint_roles(value)
        self.assertEqual(endpoints[("10.83.0.2", 20011)], "A")
        self.assertEqual(endpoints[("10.83.0.2", 20012)], "A")
        self.assertEqual(endpoints[("10.83.0.3", 20001)], "M")
        self.assertEqual(endpoints[("10.83.0.4", 20001)], "B")
        for ports in ([], [True], [20011, 20011], list(range(22000, 22009)), "20001"):
            changed = {**value, "managed_ports": {**value["managed_ports"], "A": ports}}
            self.assertRejected("managed_ports_required", lambda: HELPER.validate_plan(changed))

    def test_public_unspecified_alias_and_reserved_fixture_endpoints_are_rejected(self):
        value = plan(self.root)
        for address in ("8.8.8.8", "0.0.0.0", "224.0.0.1", "edge.example.com"):
            with self.assertRaises((HELPER.Rejected, ValueError)):
                HELPER.validate_plan({**value, "addresses": {**value["addresses"], "fixture": address}})
        self.assertRejected("independent_managed_addresses_required", lambda: HELPER.validate_plan(
            {**value, "addresses": {**value["addresses"], "B": value["addresses"]["M"]}}))
        for reserved in (18085, 18086, 2080):
            self.assertRejected("invalid_fixture_ports", lambda: HELPER.validate_plan(
                {**value, "ports": {**value["ports"], "x": reserved}}))

    def test_private_paths_reject_leaf_and_ancestor_symlinks_and_escape(self):
        source = self.root / "source.json"
        source.write_text("{}")
        link = self.root / "link"
        link.symlink_to(source)
        self.assertRejected("symlink_path_rejected", lambda: HELPER.read_json(link))
        directory = self.root / "alias"
        directory.symlink_to(self.root, target_is_directory=True)
        self.assertRejected("symlink_path_rejected", lambda: HELPER.read_json(directory / "source.json"))
        self.assertRejected("absolute_path_required", lambda: HELPER.absolute_path(str(self.root / ".." / "outside")))
        source.write_bytes(b"x" * 33)
        self.assertRejected("bounded_regular_file_required", lambda: HELPER.regular_file(source, 32))

    def test_prepare_emits_private_versions_without_product_configuration(self):
        value = plan(self.root)
        input_file = self.root / "plan.json"
        input_file.write_text(json.dumps(value))
        output = self.root / "prepared"
        args = argparse.Namespace(plan=str(input_file), work_dir=str(output))
        def openssl(command, directory, name, timeout=20):
            self.assertEqual(command[0], "openssl")
            for option in ("-keyout", "-out"):
                if option in command:
                    HELPER.private_bytes(Path(command[command.index(option) + 1]), b"TEST_ONLY synthetic contract bytes")
            return b""
        capacity = argparse.Namespace(f_bavail=1024 * 1024, f_frsize=4096, f_favail=1024)
        with mock.patch.object(HELPER, "verify_native"), mock.patch.object(HELPER, "capture", side_effect=openssl), \
                mock.patch.object(HELPER.os, "statvfs", return_value=capacity), \
                mock.patch.object(HELPER, "emit") as emitted:
            HELPER.prepare(args)
        manifest = HELPER.load_manifest(output / "manifest.json")
        self.assertEqual(emitted.call_args.args[0]["kind"], "prepared")
        self.assertEqual(manifest["run_id"], value["run_id"])
        configs = [json.loads(Path(record["path"]).read_text())["outbounds"][0]
                   for record in manifest["source_contents"].values()]
        self.assertEqual({(item["tag"], item["server"], item["server_port"]) for item in configs},
                         {("TEST_ONLY_X", "10.83.0.1", 21001)})
        self.assertEqual(len({(item["username"], item["password"]) for item in configs}), 4)
        self.assertFalse(any(path.name.startswith(("A-", "M-", "B-", "product-")) for path in output.iterdir()))
        self.assertEqual((output / "manifest.json").stat().st_mode & 0o777, 0o600)
        self.assertEqual(output.stat().st_mode & 0o777, 0o700)
        source = Path(manifest["source_contents"]["v1"]["path"])
        source.write_text("{}")
        self.assertRejected("prepared_source_changed", lambda: HELPER.load_manifest(output / "manifest.json"))

    def test_x_only_accepts_success_versions_and_spawns_its_owned_config(self):
        manifest = {**plan(self.root), "accounts": {version: {"username": "TEST_ONLY_" + version,
                    "password": "TEST_ONLY_secret_" + version} for version in ("v1", "v2", "bad", "v3")},
                    "tls": {"ca": "TEST_ONLY-ca", "empty_ca_directory": "TEST_ONLY-empty"}}
        targets = HELPER.Targets(manifest, self.root)
        fake_child = mock.Mock()
        try:
            with mock.patch.object(HELPER, "Child", return_value=fake_child) as child, \
                    mock.patch.object(HELPER, "wait_port"):
                targets.start_x()
            command = child.call_args.args[0]
            self.assertEqual(command, [manifest["native_binary"], "run", "-c", str(self.root / "X-0.json")])
            config = HELPER.read_json(self.root / "X-0.json")
            self.assertEqual(config["inbounds"][0]["users"], [manifest["accounts"][version] for version in ("v1", "v2", "v3")])
            self.assertNotIn(manifest["accounts"]["bad"], config["inbounds"][0]["users"])
            targets.command({"op": "stop_x"})
            fake_child.stop.assert_called_once_with()
            self.assertRejected("invalid_serve_command", lambda: targets.command({"op": "stop", "pid": 1}))
            self.assertRejected("invalid_serve_command", lambda: targets.command({"op": "exec", "command": "anything"}))
        finally:
            targets.journal.close()

    def test_snapshot_pagination_preserves_all_events_and_rejects_budget_overflow(self):
        journal = HELPER.Journal(self.root / "observations.jsonl")
        try:
            for index in range(141):
                journal.append({"kind": "udp", "bytes": index})
            first = journal.snapshot(0)
            self.assertEqual(len(first["events"]), 128)
            self.assertTrue(first["has_more"])
            second = journal.snapshot(first["next"])
            self.assertEqual(len(second["events"]), 13)
            self.assertFalse(second["has_more"])
            self.assertEqual([event["seq"] for event in first["events"] + second["events"]], list(range(1, 142)))
            self.assertRejected("snapshot_cursor_ahead", lambda: journal.snapshot(142))
            with mock.patch.object(HELPER, "MAX_FILE", journal.size):
                self.assertRejected("target_journal_budget_exceeded", lambda: journal.append({"kind": "tcp"}))
            self.assertEqual(journal.sequence, 141)
        finally:
            journal.close()

    def test_stdout_budget_reserves_a_failure_record_without_growing_output(self):
        with mock.patch.object(HELPER, "MAX_OUTPUT", 2048), mock.patch("builtins.print") as printed:
            HELPER.emit({"kind": "traffic", "padding": "x" * 900})
            self.assertRejected("stdout_budget_exceeded", lambda: HELPER.emit({"kind": "traffic", "padding": "x" * 900}))
            HELPER.emit({"kind": "failure", "code": "stdout_budget_exceeded"})
            self.assertLessEqual(HELPER.STDOUT_BYTES, 2048)
            self.assertEqual(printed.call_count, 2)

    def subscription(self):
        path = self.root / "subscription.json"
        data = b'{\n  "inbounds": [{"type":"mixed","listen":"127.0.0.1","listen_port":2080}], "outbounds":[]\n}\n'
        path.write_bytes(data)
        return path, data, hashlib.sha256(data).hexdigest()

    def test_subscription_is_read_without_rewriting_bytes_or_port(self):
        path, data, digest = self.subscription()
        self.assertEqual(HELPER.subscription_file(str(path), digest), path)
        self.assertEqual(path.read_bytes(), data)
        self.assertRejected("subscription_bytes_hash_mismatch", lambda: HELPER.subscription_file(str(path), "0" * 64))
        changed = json.loads(data)
        changed["inbounds"][0]["listen_port"] = 2081
        path.write_text(json.dumps(changed))
        changed_digest = hashlib.sha256(path.read_bytes()).hexdigest()
        self.assertRejected("unchanged_product_client_listener_required",
                            lambda: HELPER.subscription_file(str(path), changed_digest))

    def test_client_preserves_fault_events_and_only_cleans_its_own_child(self):
        path, data, digest = self.subscription()
        manifest = {**plan(self.root), "run_id": "TEST_ONLY-run", "root": str(self.root),
                    "tls": {"ca": "TEST_ONLY-ca", "empty_ca_directory": "TEST_ONLY-empty"}}
        args = argparse.Namespace(config=str(path), config_sha256=digest, phase="runtime_fault", iterations=2,
                                  duration_secs=None, interval_ms=0)
        passed = {name: {"ok": True, "code": "verified"} for name in ("tcp", "udp", "https")}
        failed = {name: {"ok": False, "code": "network_failure"} for name in ("tcp", "udp", "https")}
        child = mock.Mock()
        with mock.patch.object(HELPER, "verify_native"), mock.patch.object(HELPER, "reserve", return_value=[]), \
                mock.patch.object(HELPER, "Child", return_value=child) as spawn, \
                mock.patch.object(HELPER, "wait_port"), mock.patch.object(HELPER, "process_identity", return_value=identity(1, 2)), \
                mock.patch.object(HELPER, "traffic", side_effect=[failed, passed]), mock.patch.object(HELPER, "emit") as emitted:
            HELPER.client(args, manifest)
        self.assertEqual(spawn.call_args.args[0], [manifest["native_binary"], "run", "-c", str(path)])
        self.assertEqual(path.read_bytes(), data)
        result = emitted.call_args.args[0]
        self.assertEqual(result["kind"], "result")
        self.assertFalse(result["all_passed"])
        self.assertFalse(result["events"][0]["tcp"]["ok"])
        self.assertTrue(result["events"][1]["tcp"]["ok"])
        self.assertTrue(result["cleanup_confirmed"])
        child.stop.assert_called_once_with()

    def test_duration_expiry_keeps_results_and_restores_outer_deadline_for_final_hash(self):
        path, _, digest = self.subscription()
        manifest = {**plan(self.root), "run_id": "TEST_ONLY-run", "root": str(self.root),
                    "tls": {"ca": "TEST_ONLY-ca", "empty_ca_directory": "TEST_ONLY-empty"}}
        args = argparse.Namespace(config=str(path), config_sha256=digest, phase="fault", iterations=1,
                                  duration_secs=1, interval_ms=0)
        HELPER.DEADLINE = 1000
        outer, clock = HELPER.DEADLINE, [100.0]
        def expire(*args):
            clock[0] = 102.0
            return {name: {"ok": False, "code": "deadline_exceeded"} for name in ("tcp", "udp", "https")}
        with mock.patch.object(HELPER, "verify_native"), mock.patch.object(HELPER, "reserve", return_value=[]), \
                mock.patch.object(HELPER, "Child"), mock.patch.object(HELPER, "wait_port"), \
                mock.patch.object(HELPER, "process_identity", return_value=identity(1, 2)), \
                mock.patch.object(HELPER, "traffic", side_effect=expire), \
                mock.patch.object(HELPER.time, "monotonic", side_effect=lambda: clock[0]), \
                mock.patch.object(HELPER, "emit") as emitted:
            HELPER.client(args, manifest)
        self.assertEqual(HELPER.DEADLINE, outer)
        self.assertEqual(len(emitted.call_args.args[0]["events"]), 1)
        self.assertTrue(emitted.call_args.args[0]["cleanup_confirmed"])

    def test_pid_reuse_and_namespace_change_never_reuse_identity(self):
        expected = identity(123, 900)
        for actual in ({**expected, "starttime": 999}, {**expected, "netns_inode": 901}):
            with mock.patch.object(HELPER, "process_identity", return_value=actual):
                self.assertRejected("process_identity_changed", lambda: HELPER.validate_identity(expected))

    def test_shared_managed_namespace_is_rejected(self):
        path = self.root / "identities.json"
        path.write_text(json.dumps({"schema": 1, "run_id": "TEST_ONLY-run", "roles": {
            "A": identity(1, 100), "M": identity(2, 100)}}))
        with mock.patch.object(HELPER, "validate_identity"):
            self.assertRejected("independent_managed_namespaces_required",
                                lambda: HELPER.identities_file(path, {"run_id": "TEST_ONLY-run"}))

    def test_observation_uses_pid_namespace_and_address_not_shared_port_or_inode(self):
        manifest = plan(self.root)
        identities = {"roles": {"client": identity(11, 110), "A": identity(12, 120), "M": identity(13, 130)}}
        rows = {11: [(('10.83.0.5', 31001), ('10.83.0.2', 20012), "7")],
                12: [(('10.83.0.2', 31002), ('10.83.0.3', 20001), "7"),
                     (('10.83.0.2', 31003), ('10.83.0.4', 20001), "8")],
                13: [(('10.83.0.3', 31004), ('10.83.0.1', 21001), "7")]}
        def tables(pid, protocol):
            return rows[pid] if protocol == "tcp" else []
        with mock.patch.object(HELPER, "process_sockets", return_value={"7"}), \
                mock.patch.object(HELPER, "net_rows", side_effect=tables) as read, \
                mock.patch.object(HELPER, "validate_identity"):
            edges = HELPER.observe_once(manifest, identities)
        self.assertEqual({edge[:2] for edge in edges}, {("client", "A"), ("A", "M"), ("M", "X")})
        self.assertNotIn(("A", "B"), {edge[:2] for edge in edges})
        self.assertEqual({call.args[0] for call in read.call_args_list}, {11, 12, 13})

    def test_stats_wrong_namespace_is_rejected_before_connect(self):
        args = argparse.Namespace(identities="TEST_ONLY", role="A", inside_namespace=True)
        identities = {"roles": {"A": identity(123, 900)}}
        with mock.patch.object(HELPER, "identities_file", return_value=identities), \
                mock.patch.object(HELPER.os, "stat", return_value=argparse.Namespace(st_ino=901)), \
                mock.patch.object(HELPER, "grpc_stats") as stats:
            self.assertRejected("stats_namespace_mismatch", lambda: HELPER.stats(args, {"run_id": "TEST_ONLY-run"}))
        stats.assert_not_called()

    def test_stats_pins_namespace_fd_and_uses_only_fixed_self_worker(self):
        expected = identity(123, 900)
        args = argparse.Namespace(identities=str(self.root / "identities.json"), manifest=str(self.root / "manifest.json"),
                                  role="A", inside_namespace=False)
        manifest = {"run_id": "TEST_ONLY-run", "root": str(self.root)}
        result = {"kind": "result", "event": "stats", "run_id": manifest["run_id"],
                  "role": "A", "identity": expected, "counters": []}
        child = mock.Mock()
        child.process.returncode = 0
        child.reader.is_alive.return_value = False
        child.output = json.dumps(result).encode()
        with mock.patch.object(HELPER, "identities_file", return_value={"roles": {"A": expected}}), \
                mock.patch.object(HELPER, "validate_identity"), \
                mock.patch.object(HELPER.os, "open", return_value=127) as opened, \
                mock.patch.object(HELPER.os, "fstat", return_value=argparse.Namespace(st_ino=900)), \
                mock.patch.object(HELPER.os, "close") as closed, \
                mock.patch.object(HELPER, "Child", return_value=child) as spawned, \
                mock.patch.object(HELPER, "emit") as emitted:
            HELPER.stats(args, manifest)
        command = spawned.call_args.args[0]
        self.assertEqual(command[:3], ["nsenter", "--net=/proc/self/fd/127", "--"])
        self.assertIn("--inside-namespace", command)
        self.assertEqual(spawned.call_args.kwargs["pass_fds"], (127,))
        self.assertEqual(opened.call_args.args[0], "/proc/123/ns/net")
        closed.assert_called_once_with(127)
        child.stop.assert_called_once_with()
        self.assertEqual(emitted.call_args.args[0], result)

    def test_grpc_empty_success_is_read_only_and_trailers_only_are_failure(self):
        success = FakeSocket(frame(0, 1, b"\x00\x00\x00\x00\x00"))
        with mock.patch.object(HELPER.socket, "create_connection", return_value=success) as connect:
            self.assertEqual(HELPER.grpc_stats(), [])
        self.assertEqual(connect.call_args.args[0], ("127.0.0.1", 18085))
        self.assertIn(b"StatsService/QueryStats", success.sent)
        self.assertNotIn(b"GetStats", success.sent)
        self.assertTrue(success.sent.endswith(b"\x00\x00\x00\x00\x00"))
        trailers = FakeSocket(frame(1, 5, b"ignored-error-trailers"))
        with mock.patch.object(HELPER.socket, "create_connection", return_value=trailers):
            self.assertRejected("grpc_unary_response_missing", HELPER.grpc_stats)

    def test_grpc_counters_are_typed_decimal_strings_and_response_budget_is_enforced(self):
        name = b"user>>>u7_n1>>>traffic>>>uplink"
        stat = b"\x0a" + varint(len(name)) + name + b"\x10" + varint(5056)
        response = b"\x0a" + varint(len(stat)) + stat
        wire = b"\x00" + len(response).to_bytes(4, "big") + response
        with mock.patch.object(HELPER.socket, "create_connection", return_value=FakeSocket(frame(0, 1, wire))):
            self.assertEqual(HELPER.grpc_stats(), [{"name": name.decode(), "value": "5056"}])
        oversized = (65537).to_bytes(3, "big") + bytes([0, 1]) + struct.pack("!I", 1)
        with mock.patch.object(HELPER.socket, "create_connection", return_value=FakeSocket(oversized)):
            self.assertRejected("grpc_frame_budget_exceeded", HELPER.grpc_stats)

    @unittest.skipUnless(sys.platform == "linux" and hasattr(os, "waitid") and hasattr(os, "WNOWAIT"),
                         "requires nonreaping waitid child ownership (Linux)")
    def test_child_output_overflow_is_bounded_and_does_not_stop_foreign_process(self):
        sentinel = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(20)"], start_new_session=True)
        child = None
        try:
            with mock.patch.object(HELPER, "MAX_OUTPUT", 64):
                child = HELPER.Child([sys.executable, "-c", "print('x' * 4096)"], os.environ.copy(), self.root / "overflow.log")
                child.process.wait(timeout=3)
                child.reader.join(timeout=1)
                self.assertRejected("child_output_budget_exceeded", child.stop)
                self.assertEqual(len(child.output), 64)
                self.assertLessEqual((self.root / "overflow.log").stat().st_size, 64)
            self.assertIsNone(sentinel.poll())
        finally:
            if child and child.process.poll() is None:
                with contextlib.suppress(ProcessLookupError):
                    os.killpg(child.process.pid, signal.SIGKILL)
                child.process.wait(timeout=2)
            sentinel.terminate()
            sentinel.wait(timeout=2)

    @unittest.skipUnless(os.name == "posix", "active socket cleanup requires POSIX")
    def test_active_target_connection_is_closed_and_handler_is_joined(self):
        entered, finished = threading.Event(), threading.Event()
        class Handler(HELPER.socketserver.BaseRequestHandler):
            def handle(self):
                entered.set()
                try:
                    self.request.recv(1)
                finally:
                    finished.set()
        server = HELPER.BoundedTCP(("127.0.0.1", 0), Handler)
        server.fixture = argparse.Namespace(errors=threading.Event())
        worker = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.01})
        worker.start()
        connection = None
        try:
            connection = socket.create_connection(server.server_address, timeout=1)
            self.assertTrue(entered.wait(1))
            server.shutdown()
            server.server_close()
            worker.join(timeout=1)
            self.assertTrue(finished.is_set())
            self.assertFalse(worker.is_alive())
            self.assertFalse(server.active)
        finally:
            if connection:
                connection.close()
            if worker.is_alive():
                server.shutdown()
                worker.join(timeout=1)
            server.server_close()

    def test_exited_leader_does_not_skip_owned_descendant_pipe_cleanup(self):
        child = HELPER.Child.__new__(HELPER.Child)
        child.process = mock.Mock(pid=123)
        child.process.poll.return_value = 0
        child.reader = mock.Mock()
        child.reader.is_alive.return_value = False
        child.log = mock.Mock()
        child.overflow, child.reader_error = threading.Event(), False
        child.stop()
        child.process.stop_group.assert_called_once_with()
        child.process.stdout.close.assert_called_once_with()
        child.log.close.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
