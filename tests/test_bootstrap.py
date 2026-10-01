#!/usr/bin/env python3
"""Bootstrap network boundaries and independently provisioned trust file tests."""

import io
import importlib.util

import json
import os
from pathlib import Path
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


class BootstrapTests(unittest.TestCase):
    def test_official_standalone_bootstrap_matches_all_sources_and_refuses_test_root(self):
        self.assertEqual((ROOT / "deploy/bootstrap.sh").read_text(), RENDER.render())
        with self.assertRaises(ValueError):
            RENDER.render(trusted_keys=FIXTURES / "public-keys.json")

    def test_signed_installer_requires_preloaded_agent_contract(self):
        with tempfile.TemporaryDirectory() as directory:
            installer = Path(directory) / "install.sh"
            installer.write_bytes(b"#!/bin/sh\n# Old signed installer\n")
            with self.assertRaisesRegex(ValueError, "agent-v0.3.0 is incompatible"):
                bootstrap.require_preloaded_installer(directory)
            marker = bootstrap.PRELOADED_INSTALLER_MARKER + b"\n"
            installer.write_bytes(b"#!/bin/sh\n" + marker)
            bootstrap.require_preloaded_installer(directory)
            installer.write_bytes(b"#!/bin/sh\n" + marker + marker)
            with self.assertRaises(ValueError):
                bootstrap.require_preloaded_installer(directory)

    def test_slow_stream_has_total_budget_and_leaves_no_partial_file(self):
        class SlowResponse(Response):
            def read1(self, size):
                return b"x"

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "agent"
            with patch.object(bootstrap, "github_opener") as opener, \
                 patch.object(bootstrap.time, "monotonic", side_effect=[0, 0, 200, 200, 301]):
                opener.return_value.open.return_value = SlowResponse(b"", "https://github.com/asset")
                with self.assertRaisesRegex(ValueError, "total time budget"):
                    bootstrap.download("https://github.com/allowed", "agent", target, 1024)
                self.assertEqual(opener.return_value.open.call_args.kwargs["timeout"], 20)
            self.assertFalse(target.exists())

    def test_socket_timeout_uses_remaining_file_budget(self):
        from types import SimpleNamespace
        from unittest.mock import Mock

        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "agent"
            response = Response(b"signed", "https://github.com/asset")
            sock = Mock()
            response.fp = SimpleNamespace(raw=SimpleNamespace(_sock=sock))
            with patch.object(bootstrap, "github_opener") as opener, \
                 patch.object(bootstrap.time, "monotonic", side_effect=[0, 295, 296, 297, 298]):
                opener.return_value.open.return_value = response
                bootstrap.download("https://github.com/allowed", "agent", target, 1024)
            self.assertEqual([call.args for call in sock.settimeout.call_args_list], [(5,), (3,)])
            self.assertEqual(target.read_bytes(), b"signed")

    def test_signed_agent_length_and_digest_are_checked_before_installer_execution(self):
        valid = b"signed!!"
        entry = {"name": "agent", "version": "0.3.1", "arch": "amd64", "format": "raw",
                 "asset_name": "agent-0.3.1-linux-musl-amd64", "archive_size": len(valid),
                 "binary_sha256": release.digest(valid)}
        for payload, error in ((b"short", "length differs"), (b"modified", "digest differs")):
            payloads = {"SHA256SUMS": b"fixture", "SHA256SUMS.minisig": b"fixture",
                        "release.json": json.dumps({"artifacts": [entry]}).encode(),
                        "install.sh": b"#!/bin/sh\n" + bootstrap.PRELOADED_INSTALLER_MARKER + b"\n",
                        entry["asset_name"]: payload}

            def download(base, name, destination, limit, mirror=""):
                destination.write_bytes(payloads[name])

            with self.subTest(payload=payload), \
                 patch.object(bootstrap.os, "getuid", return_value=0), \
                 patch.object(bootstrap.platform, "machine", return_value="x86_64"), \
                 patch.object(bootstrap, "load_roots", return_value=[]), \
                 patch.object(bootstrap, "verify_manifest") as verify, \
                 patch.object(bootstrap, "download", side_effect=download), \
                 patch.object(bootstrap.subprocess, "run") as execute, \
                 patch.object(sys, "argv", ["bootstrap", "--tag", "agent-v0.3.1", "--panel",
                                            "https://panel.example.com", "--token", "TEST_ONLY_token"]):
                with self.assertRaisesRegex(ValueError, error):
                    bootstrap.main()
                verify.assert_called_once()
                execute.assert_not_called()

    def test_mirror_is_explicit_https_prefix_without_panel_credentials(self):
        base = "https://github.com/theLucius7/sinan/releases/download/agent-v0.3.1"
        mirror = "https://mirror.example.com"
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "agent"
            with patch.object(bootstrap, "github_opener") as opener:
                opener.return_value.open.return_value = Response(b"signed", mirror + "/" + base + "/agent")
                bootstrap.download(base, "agent", target, 1024, mirror)
                self.assertEqual(opener.call_args.args, (mirror,))
                self.assertEqual(opener.return_value.open.call_args.args, (mirror + "/" + base + "/agent",))
                self.assertEqual(target.read_bytes(), b"signed")
        for prefix in ("http://mirror.example.com", "https://secret@mirror.example.com", "https://127.0.0.1",
                       "https://[::1]", "https://mirror.example.com?token=secret", "https://mirror.example.com/#fragment",
                       "https://panel.example.com:443"):
            with self.subTest(prefix=prefix), self.assertRaises(ValueError):
                bootstrap.validate_mirror(prefix,"https://panel.example.com")
        request = urllib.request.Request(mirror + "/" + base + "/agent")
        with self.assertRaises(ValueError):
            bootstrap.GithubRedirect(mirror).redirect_request(request,None,302,"Found",{},"https://panel.example.com/agent")

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
        installer = b"#!/bin/sh\n# SINAN_BOOTSTRAP_AGENT_SOURCE=preloaded-github-v1\nset -eu\nprintf '%s\\n' INSTALLER_VERIFIED\n"
        (self.bundle / "install.sh").write_bytes(installer)
        binary = b"TEST ONLY Agent never executed"
        (self.bundle / "agent-0.3.0-linux-musl-arm64").write_bytes(binary)
        metadata = dict(schema=1, source_repo=release.REPOSITORY, tag="agent-v0.3.0",
                        protocol_min=1, protocol_max=1, artifacts=[dict(
                            name="agent", version="0.3.0", arch="arm64", format="raw",
                            binary_name="sinan-agent", archive_size=len(binary),
                            binary_size=len(binary), binary_sha256=release.digest(binary),
                            asset_name="agent-0.3.0-linux-musl-arm64")])
        encoded = (json.dumps(metadata, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (self.bundle / "release.json").write_bytes(encoded)
        checksums = {"agent/0.3.0/arm64": release.digest(binary),
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
