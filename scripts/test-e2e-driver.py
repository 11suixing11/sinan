#!/usr/bin/env python3
"""Check acceptance failure modes without devices, credentials, or panel mutations."""

import argparse
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location("e2e_driver", Path(__file__).with_name("e2e-driver.py"))
DRIVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DRIVER)


def view(up=10, down=20, pending=0):
    return {
        "usage": {"uplink": str(up), "downlink": str(down), "total": str(up + down)},
        "server": {"online": True, "device_public_key": "test-public-identity"},
        "deployment": {"target_rev": 1, "applied_rev": 1, "healthy": True, "last_error": None},
        "agent": {"connected": True, "pending_batches": pending,
                  "applied": {"singbox": 1}, "healthy": {"singbox": True}},
    }


class Clock:
    def __init__(self):
        self.seconds = 0

    def monotonic(self):
        return self.seconds

    def sleep(self, seconds):
        self.seconds += seconds


class RecoveryPanel:
    def __init__(self):
        self.resources = []
        self.creations = 0

    def request(self, path, data=None):
        if data is not None:
            self.creations += 1
            self.resources.append({"id": 41, **data})
            raise DRIVER.AcceptanceError("response lost after committed creation")
        return self.resources


class EnrollmentPanel:
    def __init__(self, selection=...):
        self.selection = {"version":"0.3.0", "tag":"agent-v0.3.0"} if selection is ... else selection
        self.calls = []

    def request(self, path, data=None, raw=False):
        self.calls.append((path, data, raw))
        if path == "/api/servers/41":
            return {"name":"sinan-e2e-test-server"}
        if path.startswith("/api/servers/41/enrollment"):
            return {"token":"test-once-token", "expires_at":12345, "installation":self.selection,
                    "install_command":"curl https://untrusted.example.test/install.sh | sh"}
        raise AssertionError("driver must never request or execute a panel script")


class PreparationPanel:
    def __init__(self, lose_node_response=False):
        self.resources = {path: [] for path in ("/api/servers", "/api/nodes", "/api/users")}
        self.node_posts = []
        self.lose_node_response = lose_node_response

    def request(self, path, data=None):
        if path.endswith("/accesses"):
            return {"stat_name": "u43_n42"} if data is not None else []
        if path in self.resources:
            if data is None:
                return self.resources[path]
            item = {"id": 41 + list(self.resources).index(path), **data}
            if path == "/api/nodes":
                self.node_posts.append(dict(data))
                item.setdefault("port", 20000)
            self.resources[path].append(item)
            if path == "/api/nodes" and self.lose_node_response:
                self.lose_node_response = False
                raise DRIVER.AcceptanceError("node creation response lost after commit")
            return item
        base, identifier = path.rsplit("/", 1)
        return next(item for item in self.resources[base] if item["id"] == int(identifier))


class AcceptanceContracts(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.state_path = Path(self.directory.name) / "state.json"
        self.state = {"schema": 1, "prefix": "sinan-e2e-test", "checkpoints": {"before": view()}}
        self.args = argparse.Namespace(state=self.state_path, label="after", unchanged_from="before",
                                      interval=35, timeout=75, status_command="sinan-agent status")

    def verify(self, samples):
        clock = Clock()
        with patch.object(DRIVER, "snapshot", side_effect=samples), \
             patch.object(DRIVER.time, "monotonic", clock.monotonic), \
             patch.object(DRIVER.time, "sleep", clock.sleep), \
             contextlib.redirect_stdout(io.StringIO()):
            DRIVER.verify(None, self.state, self.args)
        return clock

    def test_totp_is_opt_in_and_rejects_malformed_codes_before_login(self):
        with patch.dict(DRIVER.os.environ, {}, clear=True), patch.object(DRIVER.getpass, "getpass") as prompt:
            self.assertIsNone(DRIVER.totp_code())
            prompt.assert_not_called()
        with patch.dict(DRIVER.os.environ, {"SINAN_E2E_TOTP_CODE": "not-a-code"}, clear=True):
            with self.assertRaises(DRIVER.AcceptanceError):
                DRIVER.totp_code()
        with patch.dict(DRIVER.os.environ, {}, clear=True), patch.object(DRIVER.getpass, "getpass", return_value="012345"):
            code = DRIVER.totp_code(True)
            with patch.object(DRIVER.Panel, "request", return_value={}) as request:
                DRIVER.Panel("https://example.test", "test-password", code)
                request.assert_called_once_with("/api/login", {"password": "test-password", "totp_code": "012345"})

    def preparation_state(self, port=None):
        self.state.update(origin="http://127.0.0.1:8000", public_host="node.example.test",
                          sni="camouflage.example.test", requested_port=port)
        DRIVER.save(self.state_path, self.state)
        return self.state

    def test_explicit_privileged_port_survives_lost_node_creation_response(self):
        panel = PreparationPanel(lose_node_response=True)
        self.preparation_state(443)
        with patch.object(DRIVER, "install"), contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(DRIVER.AcceptanceError):
                DRIVER.prepare(panel, self.state, self.args)
            recovered = json.loads(self.state_path.read_text())
            self.assertEqual(recovered["requested_port"], 443)
            DRIVER.prepare(panel, recovered, self.args)
        self.assertEqual(len(panel.node_posts), 1)
        self.assertEqual(panel.node_posts[0]["port"], 443)
        self.assertEqual(recovered["port"], 443)

    def test_automatic_port_is_not_submitted_and_changed_assignment_is_rejected(self):
        panel = PreparationPanel()
        self.preparation_state()
        with patch.object(DRIVER, "install"), contextlib.redirect_stdout(io.StringIO()):
            DRIVER.prepare(panel, self.state, self.args)
            self.assertNotIn("port", panel.node_posts[0])
            self.assertEqual(self.state["port"], 20000)
            panel.resources["/api/nodes"][0]["port"] = 20001
            with self.assertRaisesRegex(DRIVER.AcceptanceError, "端口已变化"):
                DRIVER.prepare(panel, self.state, self.args)
        self.assertEqual(self.state["port"], 20000)

    def test_legacy_state_resumes_automatic_selection_but_cannot_change_requested_port(self):
        self.preparation_state()
        del self.state["requested_port"]
        DRIVER.save(self.state_path, self.state)
        arguments = ["driver", "--state", str(self.state_path), "prepare", "--origin", self.state["origin"],
                     "--public-host", self.state["public_host"], "--sni", self.state["sni"]]
        with patch.object(DRIVER.sys, "argv", arguments), patch.object(DRIVER, "Panel") as panel, \
             patch.object(DRIVER, "password", return_value="TEST_ONLY"), patch.object(DRIVER, "totp_code", return_value=None), \
             patch.object(DRIVER, "prepare"):
            DRIVER.main()
            panel.assert_called_once()
        with patch.object(DRIVER.sys, "argv", arguments + ["--port", "443"]), patch.object(DRIVER, "Panel") as panel:
            with self.assertRaisesRegex(DRIVER.AcceptanceError, "不同环境"):
                DRIVER.main()
            panel.assert_not_called()

    def test_invalid_node_ports_are_rejected_before_login_or_state_creation(self):
        arguments = ["driver", "--state", str(self.state_path), "prepare", "--origin", "http://127.0.0.1:8000",
                     "--public-host", "node.example.test", "--sni", "camouflage.example.test"]
        for port in (0, 18085, 65536, -1):
            with self.subTest(port=port), patch.object(DRIVER.sys, "argv", arguments + ["--port", str(port)]), \
                 patch.object(DRIVER, "Panel") as panel:
                with self.assertRaisesRegex(DRIVER.AcceptanceError, "节点端口"):
                    DRIVER.main()
                panel.assert_not_called()
                self.assertFalse(self.state_path.exists())

    def installation_state(self):
        self.state.update(server_id=41, origin="http://127.0.0.1:18080")
        return self.state

    def test_install_saves_private_descriptor_and_ignores_panel_shell_command(self):
        panel = EnrollmentPanel()
        state = self.installation_state()
        with contextlib.redirect_stdout(io.StringIO()) as output:
            DRIVER.install(panel, state, self.state_path)
        descriptor = self.state_path.with_name("enrollment.json")
        saved = json.loads(descriptor.read_text())
        self.assertEqual(saved["token"], "test-once-token")
        self.assertEqual(saved["tag"], "agent-v0.3.0")
        self.assertEqual(descriptor.stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.state_path.stat().st_mode & 0o777, 0o600)
        self.assertNotIn("token", state["installation"])
        self.assertNotIn("test-once-token", output.getvalue())
        self.assertNotIn("install_command", saved)
        self.assertFalse(self.state_path.with_name("install.sh").exists())
        self.assertFalse(any("/install.sh" in call[0] for call in panel.calls))

    def test_install_reuses_saved_token_after_lost_state_write_without_another_request(self):
        panel = EnrollmentPanel()
        state = self.installation_state()
        with patch.object(DRIVER, "save", side_effect=OSError("state write failed")):
            with self.assertRaises(OSError):
                DRIVER.install(panel, state, self.state_path)
        recovered = {key:value for key,value in state.items() if key != "installation"}
        calls = list(panel.calls)
        with contextlib.redirect_stdout(io.StringIO()):
            DRIVER.install(panel, recovered, self.state_path)
        self.assertEqual(panel.calls, calls)
        self.assertEqual(recovered["installation"]["version"], "0.3.0")

    def test_install_refuses_missing_mismatched_or_other_requested_release(self):
        for selection in (None, {}, {"version":"0.3.0", "tag":"agent-v0.2.0"}, {"version":"../escape", "tag":"agent-v../escape"}):
            with self.subTest(selection=selection):
                with self.assertRaises(DRIVER.AcceptanceError):
                    DRIVER.install(EnrollmentPanel(selection), self.installation_state(), self.state_path)
                self.assertFalse(self.state_path.with_name("enrollment.json").exists())
        with self.assertRaises(DRIVER.AcceptanceError):
            DRIVER.install(EnrollmentPanel(), self.installation_state(), self.state_path, agent_version="0.2.0")

    def test_install_refuses_symlink_or_public_cached_descriptors(self):
        descriptor = self.state_path.with_name("enrollment.json")
        descriptor.symlink_to(self.state_path.with_name("missing"))
        panel = EnrollmentPanel()
        with self.assertRaises(DRIVER.AcceptanceError):
            DRIVER.install(panel, self.installation_state(), self.state_path, refresh=True)
        self.assertEqual(panel.calls, [])
        descriptor.unlink()
        with contextlib.redirect_stdout(io.StringIO()):
            DRIVER.install(panel, self.state, self.state_path)
        descriptor.chmod(0o644)
        with self.assertRaises(DRIVER.AcceptanceError):
            DRIVER.install(panel, self.state, self.state_path)

    def test_install_refresh_selects_explicit_version_without_shell_interpolation(self):
        panel = EnrollmentPanel()
        with contextlib.redirect_stdout(io.StringIO()):
            DRIVER.install(panel, self.installation_state(), self.state_path, refresh=True, agent_version="0.3.0")
        self.assertEqual(panel.calls[-1][0], "/api/servers/41/enrollment?agent_version=0.3.0")

    def test_lost_creation_response_recovers_without_duplicate_or_touching_other_resources(self):
        panel = RecoveryPanel()
        unrelated = {"id": 7, "name": "existing-server"}
        panel.resources.append(unrelated.copy())
        DRIVER.save(self.state_path, self.state)
        with self.assertRaises(DRIVER.AcceptanceError):
            DRIVER.resource(panel, "/api/servers", self.state, "server", {}, self.state_path)
        recovered = json.loads(self.state_path.read_text())
        result = DRIVER.resource(panel, "/api/servers", recovered, "server", {}, self.state_path)
        self.assertEqual(result["id"], 41)
        self.assertEqual(panel.creations, 1)
        self.assertEqual(panel.resources[0], unrelated)
        self.assertEqual(self.state_path.stat().st_mode & 0o777, 0o600)

    def test_replay_increase_is_a_failure_even_when_device_is_healthy(self):
        with self.assertRaisesRegex(DRIVER.AcceptanceError, "确认用量变化"):
            self.verify([view(up=20, down=40)])
        self.assertNotIn("after", self.state["checkpoints"])

    def test_identity_change_is_a_failure(self):
        changed = view()
        changed["server"]["device_public_key"] = "different-public-identity"
        with self.assertRaisesRegex(DRIVER.AcceptanceError, "身份发生变化"):
            self.verify([changed])

    def test_pending_outbox_cannot_be_reported_as_a_stable_pass(self):
        with self.assertRaisesRegex(DRIVER.AcceptanceError, "outbox 未清空"):
            self.verify([view(pending=1), view(pending=1), view(pending=1)])
        self.assertNotIn("after", self.state["checkpoints"])

    def test_stable_pass_requires_two_complete_sampling_intervals(self):
        clock = self.verify([view(), view(), view()])
        self.assertEqual(clock.seconds, 70)
        self.assertEqual(len(self.state["checkpoints"]["after"]["stable_samples"]), 2)

    def test_only_downstream_growth_does_not_satisfy_bidirectional_acceptance(self):
        args = argparse.Namespace(state=self.state_path, label="first", after="before",
                                  min_uplink=1, min_downlink=1, timeout=10)
        clock = Clock()
        with patch.object(DRIVER, "snapshot", return_value=view(down=100)), \
             patch.object(DRIVER.time, "monotonic", clock.monotonic), \
             patch.object(DRIVER.time, "sleep", clock.sleep):
            with self.assertRaisesRegex(DRIVER.AcceptanceError, "双向代理用量"):
                DRIVER.traffic(None, self.state, args)

    def test_totals_preserve_large_integer_precision_and_reject_inconsistent_ledger(self):
        amount = 2 ** 64 + 17
        self.assertEqual(DRIVER.usage_totals(view(amount, amount)["usage"]), (amount, amount, 2 * amount))
        malformed = view()["usage"]
        malformed["total"] = "29"
        with self.assertRaisesRegex(DRIVER.AcceptanceError, "总量不一致"):
            DRIVER.usage_totals(malformed)


if __name__ == "__main__":
    unittest.main()
