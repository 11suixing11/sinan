#!/usr/bin/env python3
"""Verify capture, upload failure and report boundaries without running tests."""

import base64
import hashlib
import importlib.util
import io
import pathlib
import subprocess
import sys
import tempfile
import unittest
import zipfile
import os
import signal
import time


sys.dont_write_bytecode = True
PLUGIN = pathlib.Path(__file__).resolve().parent.parent / "plugins/nodequality"
module_spec = importlib.util.spec_from_file_location("nodequality_report", PLUGIN / "report.py")
report = importlib.util.module_from_spec(module_spec)
module_spec.loader.exec_module(report)


def make_archive(extra=None, missing=None, large=False):
    target = io.BytesIO()
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, _ in report.SECTIONS:
            if name == missing:
                continue
            archive.writestr(name + ".log", "\x1b[31mActual " + name + " report\x1b[0m\n" + ("x" * 300000 if large else ""))
            if name != "header_info":
                archive.writestr(name + ".json", '{"Head":{"IP":"192.0.2.1"}}\n{"Head":{"IP":"2001:db8::1"}}\n')
        if extra is not None:
            archive.writestr(extra, "unexpected data")
    return target.getvalue()


class ReportTests(unittest.TestCase):
    def stage(self, directory, data):
        root = pathlib.Path(directory)
        (root / "upload.base64").write_bytes(base64.encodebytes(data))
        return root

    def test_complete_report_survives_failed_online_upload(self):
        with tempfile.TemporaryDirectory() as directory:
            data = make_archive()
            root = self.stage(directory, data)
            (root / "upload-response.txt").write_text("Access denied")
            (root / "upload-status.txt").write_text("403")
            report.render(root)
            text = (root / "result.txt").read_text()
            self.assertIn("Actual ip_quality report", text)
            self.assertIn("本地报告已保留", text)
            self.assertIn("HTTP 状态：403", text)
            self.assertNotIn("\x1b", text)
            self.assertEqual((root / "report.zip").read_bytes(), data)
            self.assertFalse((root / "report-url.txt").exists())
            self.assertFalse((root / "upload.base64").exists())

    def test_online_url_requires_a_successful_http_status_and_exact_host(self):
        for response, status, expected in (
            ("测试完成：https://nodequality.com/r/abc-DEF_123\n", "200", True),
            ('{"url":"https://nodequality.com/r/abc-DEF_123"}', "200", True),
            ("https://nodequality.com/r/abc\n", "403", False),
            ("https://evil.example/r/abc\n", "200", False),
            ("https://nodequality.com/r/abc?redirect=evil\n", "200", False),
            ("https://nodequality.com/r/abc/extra\n", "200", False),
        ):
            with self.subTest(response=response, status=status):
                with tempfile.TemporaryDirectory() as directory:
                    root = self.stage(directory, make_archive())
                    (root / "upload-response.txt").write_text(response)
                    (root / "upload-status.txt").write_text(status)
                    report.render(root)
                    self.assertEqual((root / "report-url.txt").exists(), expected)

    def test_disabled_upload_keeps_the_report_local(self):
        with tempfile.TemporaryDirectory() as directory:
            data = make_archive()
            root = self.stage(directory, data)
            (root / "upload-disabled.txt").write_text("disabled\n")
            report.render(root)
            self.assertIn("公开报告上传已关闭", (root / "result.txt").read_text())
            self.assertEqual((root / "report.zip").read_bytes(), data)
            self.assertFalse((root / "report-url.txt").exists())

    def test_incomplete_report_is_not_published_as_complete(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.stage(directory, make_archive(missing="net_quality"))
            with self.assertRaises(ValueError):
                report.render(root)
            self.assertFalse((root / "result.txt").exists())
            self.assertTrue((root / "report.zip").exists())

    def test_zip_paths_and_invalid_json_are_rejected(self):
        for filename in ("../result.txt", "/tmp/result.txt", "hardware_quality.log"):
            with self.subTest(path=filename):
                with tempfile.TemporaryDirectory() as directory:
                    root = self.stage(directory, make_archive(extra=filename))
                    with self.assertRaises(ValueError):
                        report.render(root)
                    self.assertFalse((root / "result.txt").exists())
        with self.assertRaises(ValueError):
            report.validate_json(b'{"Head": {}} trailing invalid data')
        with self.assertRaises(ValueError):
            report.validate_json(b'{}')

    def test_render_and_stream_limits_preserve_full_local_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            data = make_archive(large=True)
            root = self.stage(directory, data)
            report.render(root)
            self.assertLessEqual((root / "result.txt").stat().st_size, report.MAX_TEXT)
            self.assertEqual((root / "report.zip").read_bytes(), data)
            self.assertIn("完整原始结果", (root / "result.txt").read_text())
            subprocess.run([sys.executable, str(PLUGIN / "report.py"), "stream-log", str(root / "log.txt")],
                           input=b"a" * (report.MAX_TEXT + 100) + b"final failure", check=True)
            log = (root / "log.txt").read_bytes()
            self.assertEqual(len(log), report.MAX_TEXT)
            self.assertTrue(log.endswith(b"final failure"))

    def test_capture_rejects_oversized_input(self):
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.run([sys.executable, str(PLUGIN / "report.py"), "capture", directory],
                                    input=b"a" * (report.MAX_CAPTURE + 1), capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((pathlib.Path(directory) / "upload.base64").exists())

    def test_chunked_upload_response_cannot_grow_the_output_file(self):
        with tempfile.TemporaryDirectory() as directory:
            subprocess.run([sys.executable, str(PLUGIN / "report.py"), "response", directory],
                           input=b"x" * 100000 + b"\nSINAN_RESPONSE_STATUS:403", stdout=subprocess.DEVNULL, check=True)
            root = pathlib.Path(directory)
            self.assertEqual((root / "upload-response.txt").stat().st_size, 65536)
            self.assertEqual((root / "upload-status.txt").read_text(), "403")


class UploadPolicyTests(unittest.TestCase):
    def test_upload_never_calls_the_service_without_explicit_true(self):
        for option in (None, "false", "true", "yes"):
            with self.subTest(option=option):
                with tempfile.TemporaryDirectory() as directory:
                    root = pathlib.Path(directory)
                    real = root / "real-curl"
                    real.write_text('''#!/usr/bin/env python3
import os, pathlib, sys
root = pathlib.Path(os.environ["SINAN_REPORT_WORKSPACE"])
(root / "curl-called.txt").write_text("called")
sys.stdout.write("https://nodequality.com/r/fixture\\nSINAN_RESPONSE_STATUS:200")
''')
                    real.chmod(0o755)
                    environment = dict(os.environ)
                    environment.update(SINAN_REAL_CURL=str(real), SINAN_REPORT_WORKSPACE=directory,
                                       SINAN_REPORT_HELPER=str(PLUGIN / "report.py"))
                    environment.pop("SINAN_UPLOAD_REPORT", None)
                    if option is not None:
                        environment["SINAN_UPLOAD_REPORT"] = option
                    data = make_archive()
                    subprocess.run(["bash", str(PLUGIN / "curl-shim.sh"), "-X", "POST", "--data-binary", "@-",
                                    "https://api.nodequality.com/api/v1/record"],
                                   input=base64.b64encode(data), env=environment,
                                   capture_output=True, check=True)
                    self.assertEqual((root / "curl-called.txt").exists(), option == "true")
                    self.assertEqual((root / "upload-disabled.txt").exists(), option != "true")
                    report.render(root)
                    self.assertEqual((root / "report.zip").read_bytes(), data)
                    self.assertEqual((root / "report-url.txt").exists(), option == "true")


class BuildTests(unittest.TestCase):
    def test_repeated_build_refuses_to_modify_the_existing_artifact_and_checksum(self):
        with tempfile.TemporaryDirectory() as directory:
            version = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2"
            root = pathlib.Path(directory) / "nodequality" / version
            root.mkdir(parents=True)
            artifact = root / "amd64"
            content = b"already verified artifact"
            artifact.write_bytes(content)
            manifest = root / "SHA256SUMS"
            checksum = hashlib.sha256(content).hexdigest() + "  amd64\n"
            manifest.write_text(checksum)
            result = subprocess.run(["bash", str(PLUGIN.parents[1] / "tools/build-nodequality.sh"), "amd64", directory],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("immutable artifact already exists", result.stderr)
            self.assertEqual(artifact.read_bytes(), content)
            self.assertEqual(manifest.read_text(), checksum)
            self.assertFalse((root / ".build.lock").exists())
    def test_trace_binary_translation_is_limited_to_the_fixed_upstream_command(self):
        fixed = "wget https://github.com/nxtrace/NTrace-core/releases/download/v1.3.7/nexttrace_linux_amd64 -qO /usr/local/bin/nexttrace"
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            real = root / "real-chroot"
            real.write_text('#!/usr/bin/env python3\nimport json, sys\nprint(json.dumps(sys.argv[1:]))\n')
            real.chmod(0o755)
            fake_uname = root / "uname"
            for arch in ("x86_64", "aarch64", "arm64"):
                fake_uname.write_text("#!/bin/sh\nprintf '%s\\n' '" + arch + "'\n")
                fake_uname.chmod(0o755)
                environment = dict(os.environ, PATH=str(root) + ":" + os.environ["PATH"], SINAN_REAL_CHROOT=str(real))
                for command in (fixed, fixed.replace("v1.3.7", "v1.3.8"), "printf ordinary-command"):
                    with self.subTest(arch=arch, command=command):
                        result = subprocess.run(["bash", str(PLUGIN / "chroot-shim.sh"), "/fixture/BenchOs", "/bin/bash", "-c", command],
                                                env=environment, capture_output=True, text=True, check=True)
                        arguments = report.json.loads(result.stdout)
                        expected = command.replace("nexttrace_linux_amd64", "nexttrace_linux_arm64") if command == fixed and arch in ("aarch64", "arm64") else command
                        self.assertEqual(arguments, ["/fixture/BenchOs", "/bin/bash", "-c", expected])

    def test_runner_rejects_workspace_expansion_before_starting_any_test(self):
        for directory in ("/tmp/space path", "/tmp/wild*card", "/tmp/question?mark", "/tmp/bracket[1]"):
            with self.subTest(directory=directory):
                result = subprocess.run(["bash", str(PLUGIN / "runner.sh.tmpl"), "--workspace", directory],
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("workspace must not contain whitespace or shell glob", result.stderr)

    def test_runner_rejects_ambiguous_upload_options_before_starting(self):
        for option in ("yes", "1", "", "$(id)"):
            with self.subTest(option=option):
                result = subprocess.run(["bash", str(PLUGIN / "runner.sh.tmpl"), "--workspace", "/tmp/fixture",
                                         "--upload-report", option], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("invalid report upload option", result.stderr)

    def test_shell_syntax_and_nonexecuting_help(self):
        for script in (PLUGIN / "runner.sh.tmpl", PLUGIN / "curl-shim.sh", PLUGIN / "chroot-shim.sh", PLUGIN.parents[1] / "tools/build-nodequality.sh"):
            subprocess.run(["bash", "-n", str(script)], check=True)
        result = subprocess.run(["bash", str(PLUGIN / "runner.sh.tmpl"), "--version"],
                                capture_output=True, text=True, check=True)
        self.assertEqual(result.stdout.strip(), "nodequality a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2")

    def test_existing_architecture_checksums_are_not_replaced(self):
        script = (PLUGIN.parents[1] / "tools/build-nodequality.sh").read_text()
        source = script.split("<<'PY'\n", 1)[1].split("\nPY\n", 1)[0]
        for state in ("empty", "valid", "tampered", "missing-sum", "missing-file", "duplicate", "symlink"):
            with self.subTest(state=state):
                with tempfile.TemporaryDirectory() as directory:
                    root = pathlib.Path(directory)
                    payload = b"previous pinned artifact"
                    digest = hashlib.sha256(payload).hexdigest()
                    artifact = root / "amd64"
                    manifest = root / "SHA256SUMS"
                    if state != "empty":
                        artifact.write_bytes(payload)
                        manifest.write_text(digest + "  amd64\n")
                    if state == "tampered":
                        artifact.write_bytes(b"changed")
                    elif state == "missing-sum":
                        manifest.unlink()
                    elif state == "missing-file":
                        artifact.unlink()
                    elif state == "duplicate":
                        manifest.write_text((digest + "  amd64\n") * 2)
                    elif state == "symlink":
                        artifact.rename(root / "original")
                        artifact.symlink_to(root / "original")
                    result = subprocess.run([sys.executable, "-", directory], input=source,
                                            text=True, capture_output=True)
                    self.assertEqual(result.returncode == 0, state in ("empty", "valid"), result.stderr)


@unittest.skipUnless(sys.platform.startswith("linux") and os.geteuid() == 0,
                     "runner fixtures need Linux/root; they use fake mount, curl and upstream tests")
class RunnerFixtureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="sinan-nodequality-runner-fixture-")
        self.root = pathlib.Path(self.temporary.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        self.binary = self.root / "bin"
        self.binary.mkdir()
        for name in ("mount", "chroot"):
            self.stub(name, "#!/bin/sh\nexit 0\n")
        self.stub("mountpoint", "#!/bin/sh\nexit 1\n")
        self.stub("umount", '#!/bin/sh\nprintf "%s\\n" "$*" >> "$NQ_FIXTURE_ROOT/cleanup.txt"\n')
        self.stub("curl", '''#!/usr/bin/env python3
import os, pathlib, sys
(pathlib.Path(os.environ["NQ_FIXTURE_ROOT"]) / "curl-called.txt").write_text("called")
sys.stdout.write(os.environ["NQ_FIXTURE_RESPONSE"] + "\\nSINAN_RESPONSE_STATUS:" + os.environ["NQ_FIXTURE_STATUS"])
''')
        self.environment = dict(os.environ)
        self.environment.update({
            "PATH": str(self.binary) + ":" + os.environ["PATH"],
            "NQ_FIXTURE_ROOT": str(self.root),
            "NQ_FIXTURE_RESPONSE": "测试完成：https://nodequality.com/r/fixture_REPORT-123\n",
            "NQ_FIXTURE_STATUS": "200",
        })

    def tearDown(self):
        self.temporary.cleanup()

    def stub(self, name, source):
        target = self.binary / name
        target.write_text(source)
        target.chmod(0o755)

    def runner(self, mode="report"):
        fixture = '''#!/usr/bin/env bash
set -euo pipefail
while [[ $# != 0 ]]; do
  case "$1" in -d) workspace=$2; shift 2 ;; -4|-6) ip=$1; shift ;; *) exit 5 ;; esac
done
read -r hardware; read -r ip_test; read -r network; read -r trace
printf '%s/%s/%s/%s/%s\\n' "$hardware" "$ip_test" "$network" "$trace" "${ip:-both}" > "$workspace/fixture-options.txt"
mkdir -p "$workspace/.nodequalityfixture/BenchOs/dev" "$workspace/.nodequalityfixture/BenchOs/sys" "$workspace/.nodequalityfixture/BenchOs/proc"
'''
        if mode == "report":
            encoded = base64.b64encode(make_archive()).decode()
            fixture += "printf '%s' '" + encoded + "' | curl -X POST --data-binary @- https://api.nodequality.com/api/v1/record\nexit 1\n"
        elif mode == "sleep":
            fixture += 'touch "$workspace/fixture-ready"\nsleep 30\nexit 1\n'
        else:
            fixture += "printf 'no actual reports were produced\\n'\nexit 0\n"
        content = (PLUGIN / "runner.sh.tmpl").read_text()
        for marker, payload in (
            ("NODEQUALITY_SOURCE", fixture),
            ("NODEQUALITY_LICENSE", "Synthetic test fixture; no upstream tests run.\n"),
            ("REPORT_HELPER", (PLUGIN / "report.py").read_text()),
            ("CURL_SHIM", (PLUGIN / "curl-shim.sh").read_text()),
            ("CHROOT_SHIM", (PLUGIN / "chroot-shim.sh").read_text()),
        ):
            content = content.replace("@" + marker + "@\n", payload)
        path = self.root / "nodequality"
        path.write_text(content)
        path.chmod(0o755)
        return ["bash", str(path), "--workspace", str(self.workspace),
                "--ip-version", "ipv6", "--network-mode", "low"]

    def test_upstream_nonzero_exit_still_requires_four_actual_local_reports(self):
        result = subprocess.run(self.runner(), env=self.environment, capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual((self.workspace / "upstream-exit.txt").read_text().strip(), "1")
        self.assertEqual((self.workspace / "fixture-options.txt").read_text().strip(), "y/y/l/y/-6")
        text = (self.workspace / "result.txt").read_text()
        for name, _ in report.SECTIONS:
            self.assertIn("Actual " + name + " report", text)
        self.assertTrue((self.workspace / "report.zip").is_file())
        self.assertIn("公开报告上传已关闭", text)
        self.assertFalse((self.workspace / "report-url.txt").exists())
        self.assertFalse((self.root / "curl-called.txt").exists())
        self.assertFalse((self.workspace / ".nodequalityfixture").exists())
        self.assertFalse((self.workspace / ".runner").exists())

    def test_explicit_upload_true_produces_the_online_report(self):
        result = subprocess.run(self.runner() + ["--upload-report", "true"], env=self.environment,
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual((self.workspace / "report-url.txt").read_text().strip(),
                         "https://nodequality.com/r/fixture_REPORT-123")
        self.assertTrue((self.root / "curl-called.txt").exists())

    def test_upload_403_preserves_a_complete_local_report(self):
        self.environment.update(NQ_FIXTURE_STATUS="403", NQ_FIXTURE_RESPONSE="Access denied")
        result = subprocess.run(self.runner() + ["--upload-report", "true"], env=self.environment,
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn("HTTP 状态：403", (self.workspace / "result.txt").read_text())
        self.assertFalse((self.workspace / "report-url.txt").exists())
        self.assertTrue((self.workspace / "report.zip").is_file())

    def test_zero_exit_without_a_zip_is_failed(self):
        result = subprocess.run(self.runner("empty"), env=self.environment, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.workspace / "result.txt").exists())
        self.assertIn(b"a complete local NodeQuality report was not produced", result.stderr)

    def test_signal_cleans_only_the_private_workspace(self):
        process = subprocess.Popen(self.runner("sleep"), env=self.environment,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        try:
            deadline = time.monotonic() + 5
            while not (self.workspace / "fixture-ready").exists() and time.monotonic() < deadline:
                time.sleep(0.01)
            self.assertTrue((self.workspace / "fixture-ready").exists())
            os.killpg(process.pid, signal.SIGTERM)
            process.communicate(timeout=5)
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.communicate(timeout=5)
        self.assertNotEqual(process.returncode, 0)
        self.assertFalse((self.workspace / ".nodequalityfixture").exists())
        self.assertFalse((self.workspace / ".runner.lock").exists())
        cleanup = (self.root / "cleanup.txt").read_text().splitlines()
        self.assertEqual(len(cleanup), 3)
        self.assertTrue(all(str(self.workspace) in call for call in cleanup))


if __name__ == "__main__":
    unittest.main()
