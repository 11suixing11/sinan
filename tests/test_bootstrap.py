#!/usr/bin/env python3
"""Bootstrap network boundaries and independently provisioned trust file tests."""

import io
import importlib.util
import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "crates/protocol/tests/fixtures"
if not FIXTURES.is_dir():
    FIXTURES = ROOT / "fixtures"
sys.path.insert(0, str(ROOT / "tools"))
import bootstrap
import release

RENDER_SPEC = importlib.util.spec_from_file_location("render_bootstrap", ROOT / "tools/render-bootstrap.py")
RENDER = importlib.util.module_from_spec(RENDER_SPEC)
RENDER_SPEC.loader.exec_module(RENDER)


class Response(io.BytesIO):
    def __init__(self, data, url):
        super().__init__(data)
        self.url = url
        self.status = 200
        self.headers = {}


class BootstrapTests(unittest.TestCase):
    def test_actual_platform_detection_uses_cpu_os_and_libc(self):
        for system, machine, libc, target in (
                ("Linux", "x86_64", "glibc", "linux-gnu-amd64"),
                ("Linux", "aarch64", "musl", "linux-musl-arm64"),
                ("Darwin", "arm64", "", "macos-arm64"),
                ("FreeBSD", "amd64", "", "freebsd-amd64"),
                ("FreeBSD", "aarch64", "", "freebsd-arm64")):
            with self.subTest(target=target), patch.object(bootstrap.platform, "system", return_value=system), \
                    patch.object(bootstrap.platform, "machine", return_value=machine), \
                    patch.object(bootstrap.platform, "libc_ver", return_value=(libc, "")):
                self.assertEqual(bootstrap.host_target(), target)
        for system, machine in (("Darwin", "x86_64"), ("Linux", "riscv64"), ("Windows", "amd64")):
            with self.subTest(system=system), patch.object(bootstrap.platform, "system", return_value=system), \
                    patch.object(bootstrap.platform, "machine", return_value=machine), self.assertRaises(ValueError):
                bootstrap.host_target()

    def test_abi_selection_never_installs_gnu_on_musl_or_another_cpu(self):
        self.assertEqual(bootstrap.compatible_targets("linux-musl-arm64"), ["linux-musl-arm64", "arm64"])
        self.assertEqual(bootstrap.compatible_targets("linux-gnu-amd64", "linux-gnu-amd64"),
                         ["linux-gnu-amd64", "linux-musl-amd64", "amd64"])
        for actual, requested in (("linux-musl-arm64", "linux-gnu-arm64"),
                                  ("linux-gnu-arm64", "linux-musl-amd64"),
                                  ("macos-arm64", "freebsd-arm64")):
            with self.subTest(actual=actual, requested=requested), self.assertRaises(ValueError):
                bootstrap.compatible_targets(actual, requested)

    def test_signed_selection_requires_real_platform_version_and_protocol(self):
        metadata = {"tag": "agent-v0.3.0", "protocol_min": 1, "protocol_max": 2, "artifacts": [
            {"name": "agent", "version": "0.3.0", "arch": "linux-gnu-arm64", "format": "raw",
             "binary_name": "sinan-agent", "archive_size": 1, "binary_size": 1},
            {"name": "agent", "version": "0.3.0", "arch": "macos-arm64", "format": "raw",
             "binary_name": "sinan-agent", "archive_size": 1, "binary_size": 1},
        ]}
        self.assertEqual(bootstrap.select_artifact(metadata, "0.3.0", "macos-arm64")["arch"], "macos-arm64")
        with self.assertRaises(bootstrap.IncompatibleRelease):
            bootstrap.select_artifact(metadata, "0.3.0", "linux-musl-arm64")
        metadata["protocol_min"] = 2
        with self.assertRaises(bootstrap.IncompatibleRelease):
            bootstrap.select_artifact(metadata, "0.3.0", "macos-arm64")
        metadata["protocol_min"] = 1
        with self.assertRaises(ValueError):
            bootstrap.select_artifact(metadata, "0.4.0", "macos-arm64")

    def test_latest_catalog_is_data_only_and_uses_numeric_stable_order(self):
        versions = [{"version": v, "tag": "agent-v" + v, "targets": ["arm64"],
                     "protocol_min": 9, "protocol_max": 9}
                    for v in ("0.9.0", "0.10.0", "2.0.0-beta")]
        url = "https://panel.example.com/api/bootstrap/versions?token=fixture&target=linux-musl-arm64"
        with patch.object(bootstrap, "panel_opener") as opener:
            opener.return_value.open.return_value = Response(json.dumps({"versions": versions}).encode(), url)
            candidates = bootstrap.catalog("https://panel.example.com", "fixture", "linux-musl-arm64", "latest")
        self.assertEqual([item["version"] for item in candidates], ["0.10.0", "0.9.0"])
        self.assertEqual(opener.return_value.open.call_args.args[0], url)

    def test_agent_download_is_bounded_same_origin_and_rejects_tampering_before_execution(self):
        item = {"version": "0.3.0", "arch": "freebsd-arm64", "archive_size": 3,
                "binary_sha256": release.digest(b"raw")}
        url = "https://panel.example.com/api/bootstrap/0.3.0/freebsd-arm64?token=fixture"
        with tempfile.TemporaryDirectory() as directory:
            for data, response_url in ((b"evil", url), (b"bad", url), (b"raw", "https://elsewhere.example.com")):
                target = Path(directory) / "agent"
                with self.subTest(data=data), patch.object(bootstrap, "panel_opener") as opener:
                    opener.return_value.open.return_value = Response(data, response_url)
                    with self.assertRaises(ValueError):
                        bootstrap.download_agent("https://panel.example.com", "fixture", item, target)
                    self.assertFalse(target.exists())
            target = Path(directory) / "valid"
            with patch.object(bootstrap, "panel_opener") as opener:
                opener.return_value.open.return_value = Response(b"raw", url)
                bootstrap.download_agent("https://panel.example.com", "fixture", item, target)
            self.assertEqual(target.read_bytes(), b"raw")

    def test_native_preflight_rejects_download_and_cached_state_before_enrollment(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "0.3.0"
            bundle.mkdir()
            with patch.object(bootstrap, "download_agent", side_effect=ValueError("tampered")), \
                    patch.object(bootstrap, "checked_agent") as execute, self.assertRaises(ValueError):
                bootstrap.install_native(bundle, "https://panel.example.com", "fixture", {}, "macos-arm64")
            execute.assert_not_called()

    def test_failed_native_upgrade_restores_configuration_and_previous_service(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = root / "0.3.0"
            bundle.mkdir()
            configuration = root / "config/agent.toml"
            configuration.parent.mkdir()
            previous = b'panel_url="https://old.example.com"\n'
            configuration.write_bytes(previous)
            command = root / "bin/sinan-agent"
            command.parent.mkdir()
            agent_root = root / "core"
            old = agent_root / "0.2.0"
            old.mkdir(parents=True)
            old_agent = old / "sinan-agent"
            old_agent.write_bytes(b"TEST ONLY old signed Agent")
            (agent_root / "current").symlink_to(old)
            calls = []

            def execute(agent, arguments):
                calls.append((Path(agent), arguments))
                if "enroll" in arguments:
                    configuration.write_bytes(b'panel_url="https://new.example.com"\n')
                if "install-service" in arguments and Path(agent) == bundle / "sinan-agent":
                    raise ValueError("TEST ONLY activation failure")

            def download(_panel, _token, _item, target):
                target.write_bytes(b"TEST ONLY already independently validated new Agent")

            with patch.object(bootstrap, "download_agent", side_effect=download), \
                    patch.object(bootstrap, "checked_agent", side_effect=execute), \
                    patch.object(bootstrap, "require_protected_file"), \
                    patch.object(bootstrap, "native_paths", return_value=(configuration, command, root / "var", agent_root)), \
                    self.assertRaisesRegex(ValueError, "activation failure"):
                bootstrap.install_native(bundle, "https://panel.example.com", "fixture", {}, "freebsd-amd64")
            self.assertEqual(configuration.read_bytes(), previous)
            self.assertEqual(calls[-1][0], old_agent)
            self.assertIn("install-service", calls[-1][1])
            self.assertEqual(calls[0][1][0], "verify-installed")
            self.assertIn("verify-cache", calls[1][1])
            self.assertFalse(command.exists())

    def test_failed_first_enrollment_can_retry_with_the_same_private_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            bundle = root / "0.3.0"
            bundle.mkdir()
            configuration = root / "config/agent.toml"
            configuration.parent.mkdir()
            identity = configuration.parent / "identity"
            command = root / "bin/sinan-agent"
            command.parent.mkdir()
            agent_root = root / "core"
            key = b"K" * 32
            attempts = []

            def execute(_agent, arguments):
                if "enroll" not in arguments:
                    return
                identity.mkdir(mode=0o700, exist_ok=True)
                key_path = identity / "device.key"
                if not key_path.exists():
                    key_path.write_bytes(key)
                    key_path.chmod(0o600)
                    (identity / "panel_origin").write_text("https://panel.example.com")
                attempts.append(key_path.read_bytes())
                if len(attempts) == 1:
                    raise ValueError("TEST ONLY enrollment HTTP failure")
                configuration.write_text('panel_url="https://panel.example.com"\n')

            def download(_panel, _token, _item, target):
                target.write_bytes(b"TEST ONLY already independently validated new Agent")

            with patch.object(bootstrap, "download_agent", side_effect=download), \
                    patch.object(bootstrap, "checked_agent", side_effect=execute), \
                    patch.object(bootstrap, "require_protected_file"), \
                    patch.object(bootstrap, "native_paths", return_value=(configuration, command, root / "var", agent_root)):
                with self.assertRaisesRegex(ValueError, "HTTP failure"):
                    bootstrap.install_native(bundle, "https://panel.example.com", "first-token", {}, "macos-arm64")
                self.assertFalse(configuration.exists())
                bootstrap.install_native(bundle, "https://panel.example.com", "second-token", {}, "macos-arm64")
            self.assertEqual(attempts, [key, key])
            self.assertTrue(configuration.exists())
            self.assertTrue(command.is_symlink())
            (identity / "panel_origin").write_text("https://other.example.com")
            with patch.object(bootstrap, "require_protected_file"), self.assertRaises(ValueError):
                bootstrap.validate_partial_identity(identity, "https://panel.example.com")
            (identity / "panel_origin").write_text("https://panel.example.com")
            (identity / "unknown").write_bytes(b"unexpected")
            with patch.object(bootstrap, "require_protected_file"), self.assertRaises(ValueError):
                bootstrap.validate_partial_identity(identity, "https://panel.example.com")
            (identity / "unknown").unlink()
            (identity / "device.key").unlink()
            with patch.object(bootstrap, "require_protected_file"):
                bootstrap.validate_partial_identity(identity, "https://panel.example.com")
            (identity / "server_id").write_text("1")
            with patch.object(bootstrap, "require_protected_file"), self.assertRaises(ValueError):
                bootstrap.validate_partial_identity(identity, "https://panel.example.com")

    def test_official_standalone_bootstrap_matches_all_sources_and_refuses_test_root(self):
        self.assertEqual((ROOT / "deploy/bootstrap.sh").read_text(), RENDER.render())
        with self.assertRaises(ValueError):
            RENDER.render(trusted_keys=FIXTURES / "public-keys.json")

    def test_http_panel_is_only_allowed_for_loopback(self):
        for value in ("http://127.0.0.1:8000", "http://[::1]:8000", "http://localhost:8000",
                      "http://[::ffff:127.0.0.1]:8000", "https://panel.example.com"):
            bootstrap.validate_panel_origin(value)
        for value in ("http://panel.example.com", "http://192.0.2.1", "http://10.0.0.1",
                      "https://user:pass@panel.example.com", "https://panel.example.com/path",
                      "https://panel.example.com?token=value", "http://localhost:0"):
            with self.subTest(value=value), self.assertRaises(ValueError):
                bootstrap.validate_panel_origin(value)

    def test_only_fixed_github_https_hosts_are_allowed(self):
        for host in bootstrap.GITHUB_DOWNLOAD_HOSTS:
            bootstrap.validate_github_url(f"https://{host}:443/release?token=fixture")
        for url in ("http://github.com/a", "https://github.com:444/a",
                    "https://github.com.example.com/a", "https://example.com/a",
                    "https://127.0.0.1/a", "https://[::ffff:127.0.0.1]/a",
                    "https://user:pass@github.com/a", "https://github.com/a#fragment",
                    "https://github.com/a\n"):
            with self.subTest(url=url), self.assertRaises(ValueError):
                bootstrap.validate_github_url(url)

    def test_redirect_is_rejected_before_unapproved_connection(self):
        request = urllib.request.Request("https://github.com/asset")
        for destination in ("https://example.com/asset", "http://github.com/asset",
                            "https://127.0.0.1/asset"):
            with self.subTest(destination=destination), self.assertRaises(ValueError):
                bootstrap.GithubRedirect().redirect_request(request, None, 302, "Found", {}, destination)
        next_request = bootstrap.GithubRedirect().redirect_request(
            request, None, 302, "Found", {}, "https://release-assets.githubusercontent.com/asset")
        self.assertEqual(next_request.full_url, "https://release-assets.githubusercontent.com/asset")
        self.assertEqual(bootstrap.GithubRedirect.max_redirections, 5)

    def test_environment_proxy_is_never_loaded(self):
        with patch.dict(os.environ, {"HTTPS_PROXY": "http://127.0.0.1:9",
                                    "ALL_PROXY": "socks5://127.0.0.1:9"}):
            with patch("urllib.request.build_opener") as build:
                bootstrap.github_opener()
                proxy = build.call_args.args[0]
                self.assertIsInstance(proxy, urllib.request.ProxyHandler)
                self.assertEqual(proxy.proxies, {})

    def test_download_checks_final_host_and_size_without_partial_file(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "SHA256SUMS"
            for payload, url in ((b"12345", "https://github.com/asset"),
                                 (b"123", "https://example.com/asset")):
                with patch.object(bootstrap, "github_opener") as opener:
                    opener.return_value.open.return_value = Response(payload, url)
                    with self.assertRaises(ValueError):
                        bootstrap.download("https://github.com/allowed", "SHA256SUMS", target, 4)
                self.assertFalse(target.exists())

    def test_unprotected_operator_trust_file_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "public-keys.json"
            path.write_text((FIXTURES / "public-keys.json").read_text())
            path.chmod(0o666)
            with self.assertRaises(ValueError):
                release.load_roots(path, require_protected=True)

    @unittest.skipUnless(os.getuid() == 0, "root-owned trust file test requires container root")
    def test_protected_operator_root_is_accepted_and_symlink_refused(self):
        with tempfile.TemporaryDirectory(prefix="sinan-trust-test-", dir="/root") as directory:
            path = Path(directory) / "public-keys.json"
            path.write_text((FIXTURES / "public-keys.json").read_text())
            path.chmod(0o600)
            self.assertEqual(len(release.load_roots(path, require_protected=True)), 1)
            link = Path(directory) / "linked.json"
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                release.load_roots(link, require_protected=True)


@unittest.skipUnless(shutil.which("minisign"), "standalone signed bootstrap requires minisign")
class StandaloneBootstrapTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.root_command = [] if os.getuid() == 0 else ["sudo", "-n"]
        if cls.root_command:
            result = subprocess.run(cls.root_command + ["true"], capture_output=True, check=False)
            if result.returncode:
                raise unittest.SkipTest("standalone bootstrap needs root or passwordless sudo")

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="sinan-standalone-bootstrap-test-")
        self.directory = Path(self.temporary.name)
        self.bundle = self.directory / "release"
        self.bundle.mkdir()
        installer = b"#!/bin/sh\nset -eu\nprintf '%s\\n' INSTALLER_VERIFIED\n"
        (self.bundle / "install.sh").write_bytes(installer)
        binary = b"TEST ONLY Agent never executed"
        architecture = {"x86_64": "amd64", "amd64": "amd64", "aarch64": "arm64", "arm64": "arm64"}[platform.machine()]
        metadata = dict(schema=1, source_repo=release.REPOSITORY, tag="agent-v0.3.0",
                        protocol_min=1, protocol_max=1, artifacts=[dict(
                            name="agent", version="0.3.0", arch=architecture, format="raw",
                            binary_name="sinan-agent", archive_size=len(binary),
                            binary_size=len(binary), binary_sha256=release.digest(binary),
                            asset_name="agent-0.3.0-linux-musl-" + architecture)])
        encoded = (json.dumps(metadata, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (self.bundle / "release.json").write_bytes(encoded)
        checksums = {"agent/0.3.0/" + architecture: release.digest(binary),
                     "install.sh": release.digest(installer), "release.json": release.digest(encoded)}
        (self.bundle / "SHA256SUMS").write_text("".join(
            f"{checksums[name]}  {name}\n" for name in sorted(checksums)))
        result = subprocess.run(["minisign", "-S", "-m", str(self.bundle / "SHA256SUMS"),
                                 "-s", str(FIXTURES / "TEST_ONLY.key"), "-x",
                                 str(self.bundle / "SHA256SUMS.minisig"), "-t",
                                 "Sinan TEST ONLY standalone fixture"], capture_output=True)
        self.assertEqual(result.returncode, 0, "fixture signing failed")
        self.script = self.directory / "bootstrap.sh"
        self.script.write_text(RENDER.render(trusted_keys=FIXTURES / "public-keys.json", publication=False))

    def tearDown(self):
        self.temporary.cleanup()

    def run_bootstrap(self, script=None):
        return subprocess.run(self.root_command + ["/bin/sh", str(script or self.script),
                              "--tag", "agent-v0.3.0", "--panel", "http://127.0.0.1:8000",
                              "--token", "TEST_ONLY_token", "--release-dir", str(self.bundle)],
                              capture_output=True, check=False, timeout=30)

    def test_standalone_bootstrap_provisions_its_own_trust_and_verifies_before_execution(self):
        result = self.run_bootstrap()
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"INSTALLER_VERIFIED", result.stdout)

    def test_standalone_rejects_writable_staging_before_writing_or_importing_helpers(self):
        unsafe = self.directory / "unsafe"
        unsafe.mkdir(mode=0o777)
        unsafe.chmod(0o777)
        script = self.directory / "unsafe-bootstrap.sh"
        script.write_text(self.script.read_text().replace(
            "STAGING_BASE=/opt/sinan", "STAGING_BASE=" + shlex.quote(str(unsafe))))
        result = self.run_bootstrap(script)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("root 保护".encode(), result.stderr)
        self.assertNotIn(b"INSTALLER_VERIFIED", result.stdout)
        self.assertEqual(list(unsafe.iterdir()), [])

    def test_tampered_installer_or_metadata_never_executes(self):
        for filename in ("install.sh", "release.json"):
            path = self.bundle / filename
            original = path.read_bytes()
            path.write_bytes(original + b" ")
            result = self.run_bootstrap()
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn(b"INSTALLER_VERIFIED", result.stdout)
            path.write_bytes(original)

    def test_official_roots_reject_a_release_signed_by_test_root(self):
        result = self.run_bootstrap(ROOT / "deploy/bootstrap.sh")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn(b"INSTALLER_VERIFIED", result.stdout)
        self.assertIn(b"no trusted key verifies", result.stderr)


if __name__ == "__main__":
    unittest.main()
