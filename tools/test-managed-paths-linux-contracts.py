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


class DriverContracts(unittest.TestCase):
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
