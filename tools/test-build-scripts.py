#!/usr/bin/env python3
"""Exercise artifact guards without requiring a Linux compiler toolchain."""

import hashlib
import pathlib
import subprocess
import tempfile
import unittest


TOOLS = pathlib.Path(__file__).resolve().parent


class BuildScriptTests(unittest.TestCase):
    def test_syntax_and_help(self):
        for name in ("build-agent.sh", "build-singbox.sh"):
            with self.subTest(script=name):
                script = TOOLS / name
                subprocess.run(["bash", "-n", str(script)], check=True)
                result = subprocess.run(
                    ["bash", str(script), "--help"],
                    check=True,
                    capture_output=True,
                    text=True,
                )
                self.assertIn("<amd64|arm64> <ARTIFACT_ROOT>", result.stdout)

    def test_existing_artifact_checksums(self):
        cases = (
            "empty",
            "valid",
            "tampered",
            "missing-sum",
            "missing-file",
            "duplicate",
            "unsafe-name",
            "symlink-file",
            "symlink-sums",
        )
        for name in ("build-agent.sh", "build-singbox.sh"):
            script = (TOOLS / name).read_text()
            source = script.split("<<'PY'\n", 1)[1].split("\nPY\n", 1)[0]
            for case in cases:
                with self.subTest(script=name, case=case):
                    with tempfile.TemporaryDirectory() as directory:
                        root = pathlib.Path(directory)
                        artifact = root / "amd64"
                        manifest = root / "SHA256SUMS"
                        payload = b"previous verified build"
                        digest = hashlib.sha256(payload).hexdigest()
                        if case != "empty":
                            artifact.write_bytes(payload)
                            manifest.write_text(f"{digest}  amd64\n")
                        if case == "tampered":
                            artifact.write_bytes(b"changed")
                        elif case == "missing-sum":
                            manifest.unlink()
                        elif case == "missing-file":
                            artifact.unlink()
                        elif case == "duplicate":
                            manifest.write_text(f"{digest}  amd64\n{digest}  amd64\n")
                        elif case == "unsafe-name":
                            manifest.write_text(f"{digest}  ../amd64\n")
                        elif case == "symlink-file":
                            artifact.rename(root / "original")
                            artifact.symlink_to(root / "original")
                        elif case == "symlink-sums":
                            manifest.rename(root / "original")
                            manifest.symlink_to(root / "original")
                        result = subprocess.run(
                            ["python3", "-", str(root)],
                            input=source,
                            text=True,
                            capture_output=True,
                        )
                        self.assertEqual(
                            result.returncode == 0,
                            case in ("empty", "valid"),
                            result.stderr,
                        )


if __name__ == "__main__":
    unittest.main()
