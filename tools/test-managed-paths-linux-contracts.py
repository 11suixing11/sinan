#!/usr/bin/env python3
"""Tool boundary regressions; these never count as registered-device acceptance."""

import copy
import importlib.util
import json
import os
from pathlib import Path
import signal
import tempfile
import unittest
from unittest import mock


SPEC = importlib.util.spec_from_file_location("managed_paths_driver", Path(__file__).with_name("test-managed-paths-linux.py"))
DRIVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DRIVER)


def checkpoint(revision=3, instance="instance-1"):
    return {"binding": {"revision": revision, "bundle_sha256": "a" * 64,
                        "deployment_id": "deployment", "binding_digest": "b" * 64},
            "healthy": True, "activation_id": "activation", "instance": {"instance_id": instance}}


def proof_fixture():
    expected = checkpoint()
    vector = [[1, expected], [2, checkpoint(4)]]
    value = {"path_probes": [], "requests": [], "receipts": [], "vectors": []}
    for stage in ("candidate", "switched"):
        request_id = stage + "-request"
        probe_id = stage + "-probe"
        value["path_probes"].append({"chain_id": 8, "generation": 2, "stage": stage,
                                    "state": "verified", "request_id": request_id,
                                    "probe_id": probe_id, "dependency_vector": copy.deepcopy(vector)})
        value["requests"].append({"request_id": request_id, "server_id": 1, "kind": "probe",
                                 "expected": copy.deepcopy(expected), "digest": "d" * 64})
        value["receipts"].append({"request_id": request_id, "outcome": "verified",
                                 "result": {"success": True, "observed": copy.deepcopy(expected),
                                            "request_digest": "d" * 64, "probe_id": probe_id}})
    value["vectors"].append({"chain_id": 8, "generation": 2, "barrier_request_id": "barrier",
                              "barrier_vector": copy.deepcopy(vector), "revision": 3,
                              **{key: expected["binding"][key] for key in ("bundle_sha256", "deployment_id", "binding_digest")}})
    value["requests"].append({"request_id": "barrier", "server_id": 1, "kind": "barrier",
                              "expected": copy.deepcopy(expected), "digest": "e" * 64})
    value["receipts"].append({"request_id": "barrier", "outcome": "verified", "result": {
        "success": True, "observed": copy.deepcopy(expected), "request_digest": "e" * 64,
        "pending_intents_clear": True, "minimum_revision": 3}})
    return value


def manifest_fixture(root):
    """Actual private file shapes for pure binding checks, never live facts."""
    root.chmod(0o700)
    run_id = "ab12640c-270f-4a82-a506-41176e42e19f"
    DRIVER.write_bytes(root / ".sinan-managed-test-run", (run_id + "\n").encode())
    source = {"head": "a" * 40, "file_count": 782, "frozen_inputs_sha256": "b" * 64,
              "functional_sha256": "c" * 64, "cargo_lock_sha256": "d" * 64}
    programs = {}
    for name in ("controller", "fixture"):
        path = root / (name + ".py")
        DRIVER.write_bytes(path, b"# TEST_ONLY pure manifest fixture; do not execute.\n")
        programs[name] = {"path": str(path), "sha256": DRIVER.digest(path.read_bytes())}
    roles = {}
    for index, role in enumerate(("A", "M", "B"), 2):
        config = root / ("agent-" + role + ".toml")
        DRIVER.write_bytes(config, b'panel_url = "https://owned.test"\npanel_ca_file = "/etc/sinan/trust/panel-ca.pem"\n')
        roles[role] = {"address": "10.231.0." + str(index), "sni": "reality.test",
                       "agent_config_file": str(config)}
    ca, admin = root / "panel-ca.pem", root / "administrator.json"
    DRIVER.write_bytes(ca, b"TEST_ONLY CA file; not a cryptographic acceptance fixture\n")
    DRIVER.write_json(admin, {"password": "TEST_ONLY_private_password"})
    binaries = {}
    for name in ("sinan-agent", "sinan-panel"):
        path = root / name
        DRIVER.write_bytes(path, b"TEST_ONLY immutable binary identity, not executable\n")
        binaries[name] = {"path": str(path), "sha256": DRIVER.digest(path.read_bytes()),
                          "size": path.stat().st_size}
    receipt = root / "prepared-artifacts.json"
    DRIVER.write_json(receipt, {"status": "prepared", "test_only": True, "run_id": run_id,
        "source_identity": source, "release": {"official_publication_rejected": True},
        "binaries": {name: {key: row[key] for key in ("sha256", "size")} for name, row in binaries.items()}})
    controller = {"schema": 1, "test_only": True, "dedicated": True, "run_id": run_id,
        "run_root": str(root), "source_identity": copy.deepcopy(source),
        "artifacts": {"test_only": True, "prepared_receipt_file": str(receipt), "binaries": binaries},
        "roles": {role: {"init_pid": index, "starttime": index, "netns_id": index,
            "mountns_id": index, "pidns_id": index, "systemd_id": str(index).zfill(32),
            "filesystem_id": "1:" + str(index), "agent_config": "/etc/sinan/agent.toml",
            "agent_binary": "/opt/sinan/core/current/sinan-agent"}
            for index, role in enumerate(("A", "M", "B"), 2)},
        "panel": {"origin": "https://owned.test", "unit": "sinan-managed-panel-" + run_id + ".service",
            "data_dir": str(root / "panel-data"), "ownership_file": str(root / ".sinan-managed-test-run"),
            "unit_file": str(root / "panel.service"), "unit_sha256": "e" * 64,
            "environment_file": str(root / "panel.env"), "environment_sha256": "f" * 64,
            "postgres": {"socket_dir": str(root / "pg"), "port": 55437,
                         "username": "postgres", "database": "sinan_managed_" + run_id.replace("-", "")}}}
    network_root = root / "network"
    network_root.mkdir(mode=0o700)
    tls = network_root / "tls"
    tls.mkdir(mode=0o700)
    empty_roots = tls / "empty-ca-directory"
    empty_roots.mkdir(mode=0o700)
    tls_files = {"empty_ca_directory": str(empty_roots)}
    for name in ("ca", "cert", "key"):
        path = tls / (name + ".pem")
        DRIVER.write_bytes(path, b"TEST_ONLY bounded TLS identity; not a live certificate\n")
        tls_files[name] = str(path)
    accounts, contents = {}, {}
    for version in ("v1", "v2", "bad", "v3"):
        account = {"username": "TEST_ONLY_X_" + version, "password": "TEST_ONLY_account_" + version}
        accounts[version] = account
        path = network_root / ("source-" + version + ".json")
        DRIVER.write_json(path, {"outbounds": [{"type": "http", "tag": "TEST_ONLY_X",
            "server": "10.231.0.1", "server_port": 21001, **account}]})
        contents[version] = {"path": str(path), "sha256": DRIVER.digest(path.read_bytes())}
    runtime = root / "sing-box"
    DRIVER.write_bytes(runtime, b"TEST_ONLY native runtime identity, not executable\n")
    fixture = {"schema": 1, "test_only": True, "run_id": run_id, "root": str(network_root),
        "native_binary": str(runtime), "native_sha256": DRIVER.digest(runtime.read_bytes()),
        "addresses": {"fixture": "10.231.0.1", "client": "10.231.0.1",
                      **{role: row["address"] for role, row in roles.items()}},
        "ports": {"x": 21001, "handshake": 21002, "tcp_echo": 21003, "udp_echo": 21004, "https": 21005},
        "managed_ports": {"A": [20011, 20012], "M": 20001, "B": 20001},
        "accounts": accounts, "source_contents": contents, "tls": tls_files}
    programs["controller"]["manifest_file"] = str(root / "environment.json")
    programs["fixture"]["manifest_file"] = str(network_root / "manifest.json")
    prepared = {"schema": 1, "run_id": run_id, "source_identity": source,
        "panel": {"origin": "https://owned.test", "ca_file": str(ca), "admin_descriptor_file": str(admin)},
        "roles": roles, **programs,
        "release": {"test_only": True, "agent_version": "0.3.1", "agent_target": "aarch64-unknown-linux-gnu"},
        "evidence_dir": str(root / "evidence")}
    return prepared, controller, fixture


def save_manifests(prepared, controller, fixture):
    DRIVER.write_json(Path(prepared["controller"]["manifest_file"]), controller)
    DRIVER.write_json(Path(prepared["fixture"]["manifest_file"]), fixture)


class DriverContracts(unittest.TestCase):
    def assert_manifest_rejected_before_business(self, mutation):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            prepared, controller, fixture = manifest_fixture(root)
            mutation(prepared, controller, fixture, root)
            save_manifests(prepared, controller, fixture)
            with mock.patch.object(DRIVER, "Controller") as control, \
                    mock.patch.object(DRIVER, "Panel") as panel, \
                    mock.patch.object(DRIVER, "Fixtures") as helper:
                with self.assertRaises(DRIVER.Rejected):
                    DRIVER.Driver(prepared, ("baseline",))
                control.assert_not_called()
                panel.assert_not_called()
                helper.assert_not_called()
            self.assertFalse((root / "evidence").exists())

    def test_manifest_ids_origins_and_source_mismatch_before_business(self):
        mutations = (
            lambda p, c, f, r: p.update(run_id=p["run_id"].upper()),
            lambda p, c, f, r: p.update(run_id="00000000-0000-0000-0000-000000000000"),
            lambda p, c, f, r: p["panel"].update(origin="https://owned.test/"),
            lambda p, c, f, r: p["panel"].update(origin="https://OWNED.test"),
            lambda p, c, f, r: p["panel"].update(origin="https://operator:TEST_ONLY@owned.test"),
            lambda p, c, f, r: c.update(run_id="ea681d39-1a63-4b48-b4f5-a9e48614ebd0"),
            lambda p, c, f, r: c["source_identity"].update(functional_sha256="e" * 64),
            lambda p, c, f, r: c["panel"].update(origin="https://other.test"),
            lambda p, c, f, r: DRIVER.write_bytes(r / ".sinan-managed-test-run", b"different-owned-run\n"),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.assert_manifest_rejected_before_business(mutation)

    def test_evidence_path_is_owned_private_and_never_follows_symlinks(self):
        def public_evidence(p, c, f, root):
            (root / "public-evidence").mkdir(mode=0o755)
            (root / "public-evidence").chmod(0o755)
            p["evidence_dir"] = str(root / "public-evidence")
        def linked_evidence(p, c, f, root):
            (root / "real-evidence").mkdir(mode=0o700)
            (root / "linked-evidence").symlink_to(root / "real-evidence", target_is_directory=True)
            p["evidence_dir"] = str(root / "linked-evidence")
        def linked_parent(p, c, f, root):
            (root / "real-parent").mkdir(mode=0o700)
            (root / "linked-parent").symlink_to(root / "real-parent", target_is_directory=True)
            p["evidence_dir"] = str(root / "linked-parent" / "new-evidence")
        mutations = (
            lambda p, c, f, r: p.update(evidence_dir=str(r.parent / "unowned-evidence")),
            lambda p, c, f, r: p.update(evidence_dir=str(r)),
            lambda p, c, f, r: p.update(evidence_dir=str(r / "missing" / ".." / "escape")),
            public_evidence, linked_evidence, linked_parent,
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.assert_manifest_rejected_before_business(mutation)

    def test_helper_run_addresses_and_exact_ports_bind_before_business(self):
        mutations = (
            lambda p, c, f, r: f.update(run_id="ea681d39-1a63-4b48-b4f5-a9e48614ebd0"),
            lambda p, c, f, r: f.update(test_only=False),
            lambda p, c, f, r: f["addresses"].update(B="10.231.0.99"),
            lambda p, c, f, r: f["addresses"].update(fixture=p["roles"]["A"]["address"]),
            lambda p, c, f, r: f["addresses"].update(client=p["roles"]["M"]["address"]),
            lambda p, c, f, r: f["managed_ports"].update(A=[20011]),
            lambda p, c, f, r: f["managed_ports"].update(A=[20011, 20012, 20013]),
            lambda p, c, f, r: f["managed_ports"].update(M=20002),
            lambda p, c, f, r: f["managed_ports"].update(B=True),
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation):
                self.assert_manifest_rejected_before_business(mutation)

    def test_complete_private_manifest_accepts_new_and_resumed_host_layout(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            prepared, controller, fixture = manifest_fixture(root)
            save_manifests(prepared, controller, fixture)
            self.assertIs(DRIVER.manifest_contract(prepared), prepared)
            self.assertFalse((root / "evidence").exists())
            (root / "evidence").mkdir(mode=0o700)
            fixture["addresses"]["client"] = "10.231.0.10"
            fixture["managed_ports"] = {"A": [20012, 20011], "M": [20001], "B": [20001]}
            save_manifests(prepared, controller, fixture)
            self.assertIs(DRIVER.manifest_contract(prepared), prepared)
            fixture["addresses"]["client"] = "127.0.0.1"
            save_manifests(prepared, controller, fixture)
            self.assertIs(DRIVER.manifest_contract(prepared), prepared)

    def test_isolation_requires_three_actual_distinct_namespaces(self):
        facts = {"dedicated": True, "isolated": True, "test_only": True,
                 "roles": {role: {"filesystem_id": number, "netns_id": number, "systemd_id": str(number)}
                           for number, role in enumerate(("A", "M", "B"), 1)}}
        DRIVER.complete_inspection(facts)
        for field in ("filesystem_id", "netns_id", "systemd_id"):
            broken = copy.deepcopy(facts)
            broken["roles"]["B"][field] = broken["roles"]["M"][field]
            with self.assertRaises(DRIVER.Rejected):
                DRIVER.complete_inspection(broken)
        facts["dedicated"] = False
        with self.assertRaises(DRIVER.Rejected):
            DRIVER.complete_inspection(facts)

    def test_checkpoint_health_does_not_replace_probe_barrier_receipts(self):
        proof = proof_fixture()
        DRIVER.assert_proof_chain(proof, 8, 2)
        mutations = (
            lambda item: item["receipts"][0].update(outcome="superseded"),
            lambda item: item["receipts"][0]["result"].update(request_digest="different"),
            lambda item: item["receipts"][0]["result"].update(observed=checkpoint(instance="new-instance")),
            lambda item: item["path_probes"][0].update(dependency_vector=[]),
            lambda item: item["path_probes"][0]["dependency_vector"][0].__setitem__(1, checkpoint(instance="other")),
            lambda item: item["receipts"][-1]["result"].update(pending_intents_clear=False),
            lambda item: item["receipts"][-1]["result"].update(minimum_revision=2),
            lambda item: item["vectors"][0].update(bundle_sha256="unknown"),
            lambda item: item.update(vectors=[]),
        )
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                broken = copy.deepcopy(proof)
                mutate(broken)
                with self.assertRaises(DRIVER.Rejected):
                    DRIVER.assert_proof_chain(broken, 8, 2)

    def test_inspected_panel_and_source_bind_before_login_or_mutation(self):
        prepared = {"panel": {"origin": "https://owned.test"},
                    "source_identity": {"frozen_inputs_sha256": "a" * 64}}
        facts = {"dedicated": True, "isolated": True, "test_only": True,
                 "panel_origin": prepared["panel"]["origin"], "source_identity": prepared["source_identity"],
                 "roles": {role: {"filesystem_id": number, "netns_id": number, "systemd_id": str(number)}
                           for number, role in enumerate(("A", "M", "B"), 1)}}
        DRIVER.bind_inspection(facts, prepared)
        for field, replacement in (("panel_origin", None), ("panel_origin", "https://other.test"),
                                   ("source_identity", None), ("source_identity", {"frozen_inputs_sha256": "b" * 64})):
            broken = copy.deepcopy(facts)
            if replacement is None:
                del broken[field]
            else:
                broken[field] = replacement
            driver = object.__new__(DRIVER.Driver)
            driver.manifest = prepared
            driver.environment_owned = False
            driver.control = mock.Mock()
            driver.control.call.return_value = broken
            driver.panel, driver.fixtures = mock.Mock(), mock.Mock()
            with self.assertRaises(DRIVER.Rejected):
                driver.setup()
            driver.panel.login.assert_not_called()
            driver.fixtures.start.assert_not_called()
            self.assertFalse(driver.environment_owned)
            driver.control.call.assert_called_once_with("inspect")

    def test_batch_retry_persists_and_reuses_exact_request(self):
        with tempfile.TemporaryDirectory() as temporary:
            driver = object.__new__(DRIVER.Driver)
            driver.state_file = Path(temporary).resolve() / "state.json"
            driver.state = {"requests": {}}
            calls = []
            body = {"request_id": "opaque-request", "items": [{"name": "TEST_ONLY"}]}
            def request(path, method, supplied, expected):
                saved = json.loads(driver.state_file.read_bytes())
                self.assertEqual(saved["requests"]["batch"]["body"], body)
                calls.append(copy.deepcopy(supplied))
                if len(calls) == 1:
                    raise DRIVER.Rejected("panel_transport_failed")
                return {"chain_ids": [7]}
            driver.panel = mock.Mock(request=request)
            self.assertEqual(driver.request_once("batch", "/chains/batch", "POST", body), {"chain_ids": [7]})
            self.assertEqual(calls, [body, body])
            with self.assertRaises(DRIVER.Rejected):
                driver.request_once("batch", "/chains/batch", "POST", {**body, "items": []})
            self.assertEqual(len(calls), 2)

    def test_unkeyed_creation_is_not_blindly_retried(self):
        with tempfile.TemporaryDirectory() as temporary:
            driver = object.__new__(DRIVER.Driver)
            driver.state_file = Path(temporary).resolve() / "state.json"
            driver.state = {"requests": {}}
            driver.panel = mock.Mock()
            driver.panel.request.side_effect = DRIVER.Rejected("panel_transport_failed")
            with self.assertRaises(DRIVER.Rejected):
                driver.request_once("server", "/api/servers", "POST", {"name": "TEST_ONLY"})
            self.assertEqual(driver.panel.request.call_count, 1)

    def test_controller_binding_whitelist_and_fixed_manifest_argument(self):
        controller = object.__new__(DRIVER.Controller)
        controller.path = Path("/owned/controller.py")
        controller.manifest_file = Path("/owned/environment.json")
        controller.run_id = "run"
        controller.directory = Path("/owned/evidence")
        controller.deadline = mock.Mock(remaining=lambda maximum: maximum)
        controller.sequence = 0
        with mock.patch.object(DRIVER, "Process") as process:
            process.return_value.result.return_value = {"schema": 1, "run_id": "run",
                                                       "operation": "inspect", "ok": True, "facts": {}}
            controller.call("inspect")
            self.assertEqual(process.call_args.args[0][-2:], ["--manifest", "/owned/environment.json"])
            self.assertEqual(process.call_args.args[3]["operation"], "inspect")
            process.return_value.result.return_value["run_id"] = "other"
            with self.assertRaises(DRIVER.Rejected):
                controller.call("inspect")
            with self.assertRaises(DRIVER.Rejected):
                controller.call("confirm_devices")
            with self.assertRaises(DRIVER.Rejected):
                controller.call("panel_evidence", "production")
            self.assertEqual(process.call_count, 2)

    def test_parent_exit_still_cleans_owned_process_group(self):
        process = object.__new__(DRIVER.Process)
        process.process = mock.Mock(pid=9876)
        process.process.poll.return_value = 0
        process.threads = []
        process.log = Path("/owned/private.stderr")
        process.buffers = [bytearray(), bytearray()]
        with mock.patch.object(DRIVER.os, "killpg") as kill, mock.patch.object(DRIVER, "write_bytes"):
            process.stop()
        self.assertEqual(kill.call_args_list, [mock.call(9876, signal.SIGTERM), mock.call(9876, signal.SIGKILL)])

    def test_symlink_parent_and_unbounded_controller_source_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            real = root / "real"
            real.mkdir()
            source = real / "controller.py"
            source.write_bytes(b"# TEST_ONLY\n")
            descriptor = {"path": str(source), "sha256": DRIVER.digest(source.read_bytes())}
            self.assertEqual(DRIVER.verified_program(descriptor), source)
            (root / "linked").symlink_to(real, target_is_directory=True)
            with self.assertRaises(OSError):
                DRIVER.verified_program({**descriptor, "path": str(root / "linked" / "controller.py")})
            source.write_bytes(b"x" * (1024 * 1024 + 1))
            with self.assertRaises(DRIVER.Rejected):
                DRIVER.verified_program(descriptor)

    def test_graph_cannot_pass_when_a_shortcut_reaches_target(self):
        graph = {"kind": "result", "event": "observation", "socket_edges": [{"protocol": "tcp"}],
                 "edges": [["client", "A"], ["A", "M"], ["M", "X"], ["X", "B"], ["B", "target"]]}
        DRIVER.assert_graph(graph, True)
        graph["edges"].append(["A", "target"])
        with self.assertRaises(DRIVER.Rejected):
            DRIVER.assert_graph(graph, True)

    def test_fault_events_are_not_erased_or_promoted_to_baseline_success(self):
        successful = {"tcp": {"ok": True, "bytes": 4096}, "udp": {"ok": True, "packets": 5, "bytes": 960},
                      "https": {"ok": True}}
        failed = copy.deepcopy(successful)
        failed["tcp"]["ok"] = False
        result = {"events": [successful, failed, successful]}
        DRIVER.assert_traffic(result, all_rounds=False)
        with self.assertRaises(DRIVER.Rejected):
            DRIVER.assert_traffic(result)
        self.assertEqual(len(result["events"]), 3)

    def test_preflight_rejection_does_not_restore_or_stop_unproven_hosts(self):
        driver = object.__new__(DRIVER.Driver)
        driver.state = {"ids": {}, "requests": {}, "run_id": "run"}
        driver.manifest = {"run_id": "run", "source_identity": {"frozen": "digest"}}
        driver.scenarios = ("baseline",)
        driver.environment_owned = False
        driver.accounting_verified = False
        driver.results, driver.cleanup_errors = {}, []
        driver.directory = Path("/owned/evidence")
        driver.control, driver.fixtures = mock.Mock(), mock.Mock()
        driver.setup = mock.Mock(side_effect=DRIVER.Rejected("dedicated_isolation_not_proven"))
        driver.event = mock.Mock()
        driver.evidence = mock.Mock()
        with mock.patch.object(DRIVER, "write_json"):
            result = driver.run()
        self.assertEqual(result["status"], "failed")
        self.assertFalse(result["full_matrix"])
        driver.control.call.assert_not_called()

    def test_partial_protocol_bypass_cannot_pass_negative_traffic(self):
        failed = {"tcp": {"ok": False}, "udp": {"ok": False}, "https": {"ok": False}}
        DRIVER.assert_all_traffic_failed({"events": [failed]})
        for protocol in ("tcp", "udp", "https"):
            broken = copy.deepcopy(failed)
            broken[protocol]["ok"] = True
            with self.assertRaises(DRIVER.Rejected):
                DRIVER.assert_all_traffic_failed({"events": [broken]})

    def test_native_sparse_counter_response_is_typed_and_cannot_double_count(self):
        observed = {"kind": "result", "event": "stats", "counters": [{"name": "user>>>u1_n2>>>traffic>>>uplink", "value": "9007199254740993"}]}
        self.assertEqual(DRIVER.counter_values(observed), {"user>>>u1_n2>>>traffic>>>uplink": 9007199254740993})
        observed["counters"].append(copy.deepcopy(observed["counters"][0]))
        with self.assertRaises(DRIVER.Rejected):
            DRIVER.counter_values(observed)
        with self.assertRaises(DRIVER.Rejected):
            DRIVER.counter_values({"kind": "result", "event": "stats", "counters": [{"name": "counter", "value": 3}]})

    def test_client_jsonlines_preserve_fault_rows_and_exact_final_history(self):
        fixtures = object.__new__(DRIVER.Fixtures)
        fixtures.run_id = "run"
        fixtures.deadline = mock.Mock(remaining=lambda maximum: maximum)
        event = {"round": 1, "elapsed_ms": 0, "tcp": {"ok": False}, "udp": {"ok": False}, "https": {"ok": False}}
        result = {"kind": "result", "run_id": "run", "cleanup_confirmed": True, "events": [event]}
        for changed in (False, True):
            client = mock.Mock()
            final = copy.deepcopy(result)
            if changed:
                final["events"][0]["tcp"]["ok"] = True
            client.receive.side_effect = [{"kind": "traffic", "event": "client_round", "phase": "fault", **event}, final]
            fixtures.clients = [client]
            if changed:
                with self.assertRaises(DRIVER.Rejected):
                    fixtures.client_result(client)
            else:
                self.assertEqual(fixtures.client_result(client), result)
            client.stop.assert_called_once()
            self.assertEqual(fixtures.clients, [])

    def test_signal_enters_cleanup_once_instead_of_killing_driver(self):
        with mock.patch.object(DRIVER, "CANCELLED", False):
            with self.assertRaisesRegex(DRIVER.Rejected, "^acceptance_cancelled$"):
                DRIVER.interrupted(signal.SIGTERM, None)
            self.assertTrue(DRIVER.CANCELLED)
            DRIVER.interrupted(signal.SIGINT, None)


if __name__ == "__main__":
    unittest.main()
