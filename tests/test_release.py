#!/usr/bin/env python3
"""Real minisign interoperability and hostile-release contract tests."""

import argparse
import base64
import contextlib
import gzip
import http.server
import io
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "crates/protocol/tests/fixtures"
if not FIXTURES.is_dir():
    FIXTURES = ROOT / "fixtures"
sys.path.insert(0, str(ROOT / "tools"))
import release


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="sinan-release-test-")
        self.directory = Path(self.temporary.name)
        self.source = self.directory / "artifacts"
        for arch in ("amd64", "arm64"):
            agent = self.source / "agent" / "0.3.0"
            agent.mkdir(parents=True, exist_ok=True)
            (agent / arch).write_bytes(b"TEST ONLY Agent " + arch.encode())
            for name, version in (("sing-box", "1.14.2"),
                                  ("nodequality", release.NODEQUALITY_VERSION)):
                runtime = self.source / name / version
                runtime.mkdir(parents=True, exist_ok=True)
                output = io.BytesIO()
                with tarfile.open(fileobj=output, mode="w") as archive:
                    data = b"TEST ONLY " + name.encode() + b" " + arch.encode()
                    member = tarfile.TarInfo(name)
                    member.size = len(data)
                    member.mode = 0o755
                    archive.addfile(member, io.BytesIO(data))
                (runtime / arch).write_bytes(gzip.compress(output.getvalue(), mtime=0))
        self.installer = self.directory / "installer.sh"
        self.installer.write_text("#!/bin/sh\nexit 0\n")
        self.bundle = self.directory / "release"
        self.arguments = argparse.Namespace(source=str(self.source), output=str(self.bundle),
                                             agent_version="0.3.0", runtime_version="1.14.2",
                                             tag="agent-v0.3.0", installer=str(self.installer))
        release.assemble(self.arguments)
        self.roots = release.load_roots(FIXTURES / "public-keys.json")
        self.sign()

    def tearDown(self):
        self.temporary.cleanup()

    def sign(self):
        result = subprocess.run(["minisign", "-S", "-m", str(self.bundle / "SHA256SUMS"),
                                 "-s", str(FIXTURES / "TEST_ONLY.key"), "-x",
                                 str(self.bundle / "SHA256SUMS.minisig"), "-t",
                                 "Sinan TEST ONLY automated fixture"], capture_output=True)
        self.assertEqual(result.returncode, 0, "fixture signing failed")

    def verify(self):
        return release.verify_bundle(self.bundle, self.roots, "minisign", "agent-v0.3.0")

    def rewrite_metadata(self, operation):
        path = self.bundle / "release.json"
        data = json.loads(path.read_text())
        operation(data)
        path.write_text(json.dumps(data, sort_keys=True, separators=(",", ":")) + "\n")
        manifest = self.bundle / "SHA256SUMS"
        lines = manifest.read_text().splitlines()
        lines = [release.digest(path.read_bytes()) + "  release.json" if line.endswith("  release.json")
                 else line for line in lines]
        manifest.write_text("\n".join(lines) + "\n")
        self.sign()

    def test_valid_complete_signature_and_every_asset(self):
        self.assertEqual(len(self.verify()["artifacts"]), 6)

    def test_single_architecture_ci_bundle(self):
        self.arguments.arch = ["amd64"]
        self.arguments.output = str(self.directory / "single")
        release.assemble(self.arguments)
        data = json.loads((Path(self.arguments.output) / "release.json").read_text())
        self.assertEqual({entry["arch"] for entry in data["artifacts"]}, {"amd64"})
        self.assertEqual({entry["name"] for entry in data["artifacts"]},
                         {"agent", "sing-box", "nodequality"})

    def test_deterministic_manifests(self):
        second = self.directory / "second"
        self.arguments.output = str(second)
        release.assemble(self.arguments)
        for name in ("release.json", "SHA256SUMS"):
            self.assertEqual((second / name).read_bytes(), (self.bundle / name).read_bytes())

    def test_tampered_signed_checksum_file(self):
        with (self.bundle / "SHA256SUMS").open("a") as file:
            file.write("0" * 64 + "  extra\n")
        with self.assertRaises(ValueError):
            self.verify()

    def test_tampered_trusted_comment(self):
        path = self.bundle / "SHA256SUMS.minisig"
        lines = path.read_text().splitlines()
        lines[2] += " modified"
        path.write_text("\n".join(lines) + "\n")
        with self.assertRaises(ValueError):
            self.verify()

    def test_truncated_signature(self):
        path = self.bundle / "SHA256SUMS.minisig"
        path.write_text("\n".join(path.read_text().splitlines()[:3]) + "\n")
        with self.assertRaises(ValueError):
            self.verify()

    def test_legacy_signature_refused(self):
        path = self.bundle / "SHA256SUMS.minisig"
        lines = path.read_text().splitlines()
        data = base64.b64decode(lines[1])
        lines[1] = base64.b64encode(b"Ed" + data[2:]).decode()
        path.write_text("\n".join(lines) + "\n")
        with self.assertRaises(ValueError):
            self.verify()

    def test_modified_archive(self):
        path = self.bundle / "sing-box-1.14.2-linux-amd64.tar.gz"
        path.write_bytes(path.read_bytes() + b"modified")
        with self.assertRaises(ValueError):
            self.verify()

    def test_signed_wrong_binary_digest(self):
        self.rewrite_metadata(lambda data: data["artifacts"][0].update(binary_sha256="0" * 64))
        with self.assertRaises(ValueError):
            self.verify()

    def test_signed_wrong_asset_name(self):
        self.rewrite_metadata(lambda data: data["artifacts"][0].update(asset_name="../escape"))
        with self.assertRaises(ValueError):
            self.verify()

    def test_signed_wrong_protocol_range(self):
        self.rewrite_metadata(lambda data: data.update(protocol_max=2))
        with self.assertRaises(ValueError):
            self.verify()

    def test_signed_unknown_metadata_field(self):
        self.rewrite_metadata(lambda data: data.update(public_keys=self.roots))
        with self.assertRaises(ValueError):
            self.verify()

    def test_extra_asset_refused(self):
        (self.bundle / "unsigned").write_text("unsigned")
        with self.assertRaises(ValueError):
            self.verify()

    def test_duplicate_checksum_refused_even_when_signed(self):
        path = self.bundle / "SHA256SUMS"
        text = path.read_text()
        path.write_text(text + text.splitlines()[0] + "\n")
        self.sign()
        with self.assertRaises(ValueError):
            self.verify()

    def test_test_key_cannot_publish(self):
        with self.assertRaises(ValueError):
            release.load_roots(FIXTURES / "public-keys.json", publication=True)

    def test_duplicate_trust_root_refused(self):
        path = self.directory / "duplicate-public-keys.json"
        path.write_text(json.dumps(self.roots * 2))
        with self.assertRaises(ValueError):
            release.load_roots(path)

    def test_test_key_cannot_publish_with_another_key_id(self):
        material = bytearray(base64.b64decode(self.roots[0]))
        material[2:10] = b"other-id"
        path = self.directory / "renamed-key.json"
        path.write_text(json.dumps([base64.b64encode(material).decode()]))
        with self.assertRaises(ValueError):
            release.load_roots(path, publication=True)

    def test_rotation_key_cannot_publish_with_another_key_id(self):
        material = bytearray(base64.b64decode(release.TEST_ONLY_ROTATION_PUBLIC_KEY))
        material[2:10] = b"other-id"
        path = self.directory / "renamed-rotation-key.json"
        path.write_text(json.dumps([base64.b64encode(material).decode()]))
        with self.assertRaises(ValueError):
            release.load_roots(path, publication=True)

    def test_every_test_public_fixture_is_denied(self):
        material = b"Ed" + b"test-id!" + b"\x42" * 32
        record = base64.b64encode(material).decode()
        fixtures = self.directory / "fixture-pubs"
        fixtures.mkdir()
        (fixtures / "TEST_ONLY_FUTURE.pub").write_text("untrusted comment: TEST ONLY\n" + record + "\n")
        path = self.directory / "new-fixture-key.json"
        path.write_text(json.dumps([record]))
        with patch.object(release, "TEST_PUBLIC_KEY_DIRS", (fixtures,)):
            with self.assertRaises(ValueError):
                release.load_roots(path, publication=True)

    def test_archive_path_traversal_refused(self):
        output = io.BytesIO()
        with tarfile.open(fileobj=output, mode="w") as archive:
            member = tarfile.TarInfo("../sing-box")
            member.size = 1
            archive.addfile(member, io.BytesIO(b"x"))
        with self.assertRaises(ValueError):
            release.binary_bytes(gzip.compress(output.getvalue()), "tar.gz", "sing-box")

    def signed_executable_fixture(self):
        good_marker = self.directory / "trusted-agent-executed"
        bad_marker = self.directory / "untrusted-agent-executed"

        def executable(marker):
            # A fake verifier must reject incorrect role-binding arguments too.
            return ("#!/bin/sh\n"
                    '[ "$#" = 7 ] && [ "$1" = verify-installed ] && [ "$2" = --binary ] && '
                    '[ "$3" = "$0" ] && [ "$4" = --name ] && [ "$5" = agent ] && '
                    '[ "$6" = --format ] && [ "$7" = raw ] || exit 93\n'
                    "printf verified > " + shlex.quote(str(marker)) + "\n").encode()

        good, bad = executable(good_marker), executable(bad_marker)
        size = max(len(good), len(bad)) + 64
        good, bad = good.ljust(size, b"\n"), bad.ljust(size, b"\n")
        (self.bundle / "agent-0.3.0-linux-musl-amd64").write_bytes(good)

        def update(metadata):
            entry = next(item for item in metadata["artifacts"]
                         if item["name"] == "agent" and item["arch"] == "amd64")
            entry.update(archive_size=size, binary_size=size, binary_sha256=release.digest(good))

        self.rewrite_metadata(update)
        manifest = self.bundle / "SHA256SUMS"
        lines = [release.digest(good) + "  agent/0.3.0/amd64" if
                 line.endswith("  agent/0.3.0/amd64") else line
                 for line in manifest.read_text().splitlines()]
        manifest.write_text("\n".join(lines) + "\n")
        self.sign()
        self.verify()
        return good, bad, good_marker, bad_marker

    @contextlib.contextmanager
    def panel_response(self, payload, length=True, redirect=False):
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.server.requests += 1
                self.send_response(302 if redirect else 200)
                if redirect:
                    self.send_header("Location", "/never-follow")
                elif length:
                    self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                if not redirect:
                    try:
                        self.wfile.write(payload)
                    except (BrokenPipeError, ConnectionResetError):
                        pass

            def log_message(self, format, *arguments):
                pass

        server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        server.requests = 0
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            yield server, f"http://127.0.0.1:{server.server_port}"
        finally:
            server.shutdown()
            server.server_close()
            thread.join()

    def installer_prefix(self, panel, forced_optimization=False):
        # Execute the actual production path through its first Agent execution.
        text = (ROOT / "deploy/install.sh.tmpl").read_text()
        text = text.split("# Reject unverifiable legacy caches", 1)[0] + "\nexit 0\n"
        if forced_optimization:
            text = text.replace("python3 -I - ", "python3 -I -O - ", 1)
        script = self.directory / "verified-installer-prefix.sh"
        script.write_text(text)
        token = self.directory / "fixture-token"
        token.write_text("TEST_ONLY_token")
        commands = self.directory / "commands"
        commands.mkdir(exist_ok=True)
        for name in ("systemctl", "useradd"):
            path = commands / name
            path.write_text("#!/bin/sh\nexit 0\n")
            path.chmod(0o755)
        Path("/run/systemd/system").mkdir(parents=True, exist_ok=True)
        modules = self.directory / "untrusted-python-path"
        modules.mkdir(exist_ok=True)
        environment_marker = self.directory / "python-environment-injected"
        (modules / "json.py").write_text(
            "from pathlib import Path\nPath(" + repr(str(environment_marker)) + ").write_text('injected')\n")
        environment = dict(os.environ, PATH=str(commands) + ":" + os.environ["PATH"],
                           PYTHONOPTIMIZE="1", PYTHONPATH=str(modules),
                           HTTP_PROXY="http://127.0.0.1:9", HTTPS_PROXY="http://127.0.0.1:9",
                           ALL_PROXY="socks5://127.0.0.1:9", NO_PROXY="", no_proxy="")
        result = subprocess.run(["/bin/sh", str(script), "--bundle", str(self.bundle),
                                 "--version", "0.3.0", "--panel", panel,
                                 "--token-file", str(token)], env=environment,
                                capture_output=True, timeout=35)
        self.assertFalse(environment_marker.exists(), "installer imported an untrusted Python module")
        return result

    @unittest.skipUnless(os.getuid() == 0, "real installer prefix requires isolated Linux container root")
    def test_optimized_installer_refuses_tampered_raw_without_execution(self):
        good, bad, good_marker, bad_marker = self.signed_executable_fixture()
        self.assertEqual(len(good), len(bad))
        for forced in (False, True):
            with self.subTest(forced_optimization=forced), self.panel_response(bad) as (server, panel):
                result = self.installer_prefix(panel, forced)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(server.requests, 1)
                self.assertIn("摘要不匹配".encode(), result.stderr)
                self.assertFalse(bad_marker.exists())
                self.assertFalse(good_marker.exists())

    @unittest.skipUnless(os.getuid() == 0, "real installer prefix requires isolated Linux container root")
    def test_installer_accepts_signed_raw_and_passes_role_binding(self):
        good, _, good_marker, bad_marker = self.signed_executable_fixture()
        with self.panel_response(good) as (server, panel):
            result = self.installer_prefix(panel, True)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertEqual(server.requests, 1)
            self.assertEqual(good_marker.read_text(), "verified")
            self.assertFalse(bad_marker.exists())

    @unittest.skipUnless(os.getuid() == 0, "real installer prefix requires isolated Linux container root")
    def test_installer_refuses_unbounded_body_at_signed_size(self):
        good, _, good_marker, bad_marker = self.signed_executable_fixture()
        with self.panel_response(good + b"x" * 1048576, length=False) as (server, panel):
            result = self.installer_prefix(panel)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(server.requests, 1)
            self.assertIn("超出已签大小上限".encode(), result.stderr)
            self.assertFalse(good_marker.exists())
            self.assertFalse(bad_marker.exists())

    @unittest.skipUnless(os.getuid() == 0, "real installer prefix requires isolated Linux container root")
    def test_installer_does_not_follow_panel_redirect(self):
        good, _, good_marker, bad_marker = self.signed_executable_fixture()
        with self.panel_response(good, redirect=True) as (server, panel):
            result = self.installer_prefix(panel)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(server.requests, 1)
            self.assertIn("禁止重定向".encode(), result.stderr)
            self.assertFalse(good_marker.exists())
            self.assertFalse(bad_marker.exists())


if __name__ == "__main__":
    unittest.main()
