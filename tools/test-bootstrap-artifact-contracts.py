#!/usr/bin/env python3
"""Inert standalone bootstrap metadata contracts; no signing or installation."""
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

import release

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("bootstrap_artifact_render", ROOT / "tools/render-bootstrap.py")
RENDER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RENDER)


class StandaloneMetadataTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="sinan-bootstrap-artifact-TEST_ONLY-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.stage = self.root / "standalone"
        self.stage.mkdir(mode=0o700)
        # Extract embedded ordinary source bytes without executing the shell,
        # installer, download path or any artifact factory.
        rendered = RENDER.render(test_installer="#!/bin/sh\nexit 99\n", publication=False,
                                 trusted_keys=ROOT / "crates/protocol/tests/fixtures/public-keys.json")
        blocks = re.findall(r'cat > "\$STAGING/([a-zA-Z0-9._-]+)" <<\'(SINAN_BOOTSTRAP_[A-F0-9]{64})\'\n(.*?)\n\2\n',
                            rendered, re.S)
        self.assertEqual(len(blocks), len(RENDER.SOURCES) + 1)
        for name, delimiter, content in blocks:
            data = (content + "\n").encode()
            self.assertEqual(delimiter, "SINAN_BOOTSTRAP_" + hashlib.sha256(data).hexdigest().upper())
            (self.stage / name).write_bytes(data)

    def isolated(self, program, *args):
        result = subprocess.run([sys.executable, "-I", "-c",
                                 "import os,sys;sys.path.insert(0,os.getcwd());" + program, *map(str, args)],
                                cwd=self.stage, capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def bundle(self, module, version, auxiliary):
        entry = {"name": module, "version": version, "arch": "amd64", "format": "tar.gz",
                 "binary_name": module, "archive_size": 4, "binary_size": 3,
                 "binary_sha256": "a" * 64,
                 "auxiliary_files": {name: {"sha256": "b" * 64, "size": 1}
                                     for name in auxiliary}}
        entry["asset_name"] = release.asset_name(entry)
        bundle = self.root / (module + "-" + version)
        bundle.mkdir(mode=0o700)
        metadata = {"schema": 1, "source_repo": release.REPOSITORY, "tag": "agent-v0.3.0",
                    "protocol_min": 1, "protocol_max": 1, "artifacts": [entry]}
        encoded = json.dumps(metadata, sort_keys=True, separators=(",", ":")).encode() + b"\n"
        installer = b"#!/bin/sh\nexit 99\n"
        (bundle / "release.json").write_bytes(encoded)
        (bundle / "install.sh").write_bytes(installer)
        rows = {"release.json": release.digest(encoded), "install.sh": release.digest(installer),
                release.canonical_path(entry): "c" * 64}
        (bundle / "SHA256SUMS").write_text("".join(f"{rows[path]}  {path}\n" for path in sorted(rows)))
        return bundle

    def test_standalone_metadata_accepts_exact_legacy_and_explicit_native_inventories(self):
        stem = "a92fca6c0067df29ddd03fdc2fee6f3000f64545"
        rootfs = {"rootfs.tar.gz", "rootfs-manifest.json"}
        vectors = [("nodequality", stem + suffix, auxiliary) for suffix, auxiliary in
                   (("-r19", set()), ("-r20", rootfs), ("-r21", set()),
                    ("-sinan-native-r1", set()), ("-offline-rootfs-r1", rootfs))]
        vectors.append(("ipquality", "87397e2c3196ec796f5477c83343c2354df601ea-node-r1",
                        rootfs | {"build-info.json", "LICENSE", "source.tar.gz", "THIRD_PARTY_NOTICES.txt"}))
        for module, version, auxiliary in vectors:
            with self.subTest(module=module, version=version):
                bundle = self.bundle(module, version, auxiliary)
                output = self.isolated("import release;release.validate_manifest(sys.argv[1],'agent-v0.3.0');"
                                       "print('TEST_ONLY metadata, no signature acceptance')", bundle)
                self.assertIn("TEST_ONLY metadata", output)

    def test_standalone_missing_offline_inventory_is_refused(self):
        bundle = self.bundle("nodequality", "a92fca6c0067df29ddd03fdc2fee6f3000f64545-offline-rootfs-r1",
                             {"rootfs.tar.gz"})
        output = self.isolated("import release\ntry:release.validate_manifest(sys.argv[1],'agent-v0.3.0')\n"
                               "except ValueError as error:print(error)\nelse:raise SystemExit(1)", bundle)
        self.assertIn("incomplete offline NodeQuality", output)

    def test_old_asset_limit_never_imports_optional_ipquality_validator(self):
        output = self.isolated("import builtins,release\noriginal=builtins.__import__\n"
                               "def guarded(name,*args,**kwargs):\n"
                               " if name=='ipquality_artifact':raise AssertionError('unexpected optional import')\n"
                               " return original(name,*args,**kwargs)\n"
                               "builtins.__import__=guarded\n"
                               "assert release.source_offer_asset_limit('agent-0.3.0-linux-musl-amd64')==release.MAX_BINARY\n"
                               "assert release.source_offer_asset_limit('nodequality-old-linux-amd64.tar.gz')==release.MAX_BINARY\n"
                               "print('old identity independent')")
        self.assertIn("independent", output)


if __name__ == "__main__":
    unittest.main()
