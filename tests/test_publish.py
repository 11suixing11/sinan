#!/usr/bin/env python3
"""Publication CI gates and asset/tag identity checks with real signed bundles."""

import copy
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import publish
import release
import test_release

COMMIT = "a" * 40
TAG = "agent-v0.3.0"


class FakeGithub:
    def __init__(self, bundle):
        self.bundle = bundle
        self.patches = []
        self.downloads = 0
        self.draft = True
        self.reference = {"type": "commit", "sha": COMMIT}
        self.changed_tag = False
        self.changed_assets = False
        self.changed_checks = False
        self.missing_job = False
        self.run_failed = False
        self.build_commit = COMMIT
        self.assets = [{"id": index, "name": path.name, "state": "uploaded",
                        "size": len(path.read_bytes()), "digest": "sha256:" + release.digest(path.read_bytes())}
                       for index, path in enumerate(sorted(bundle.iterdir()), start=1)]

    def api(self, endpoint, method="GET", fields=()):
        if method == "PATCH":
            self.patches.append((endpoint, fields))
            self.draft = False
            return {"id": 7, "draft": False}
        if "/git/ref/tags/" in endpoint:
            reference = self.reference
            if self.changed_tag and self.downloads:
                reference = {"type": "commit", "sha": "b" * 40}
            return {"ref": "refs/tags/" + TAG, "object": copy.deepcopy(reference)}
        if "/git/tags/" in endpoint:
            return {"object": {"type": "commit", "sha": COMMIT}}
        if "/actions/workflows/ci.yml/runs?" in endpoint:
            failed = self.run_failed or self.changed_checks and self.downloads
            return {"total_count": 1, "workflow_runs": [{
                "id": 5, "run_attempt": 1, "head_sha": COMMIT, "head_branch": "main",
                "event": "push", "path": ".github/workflows/ci.yml@refs/heads/main",
                "status": "completed", "conclusion": "failure" if failed else "success"}]}
        if "/actions/runs/5/jobs?" in endpoint:
            names = sorted(publish.REQUIRED_JOBS)
            if self.missing_job:
                names.remove("Reality installation and accounting")
            jobs = [{"id": index, "name": name, "status": "completed", "conclusion": "success",
                     "head_sha": COMMIT} for index, name in enumerate(names, start=1)]
            return {"total_count": len(jobs), "jobs": jobs}
        if "/releases/tags/" in endpoint:
            return {"id": 7, "tag_name": TAG, "draft": self.draft,
                    "target_commitish": self.build_commit}
        if "/releases/7/assets?" in endpoint:
            assets = copy.deepcopy(self.assets)
            if self.changed_assets and self.downloads:
                assets[0]["id"] += 100
            return assets
        raise AssertionError("unexpected fake API endpoint")

    def download(self, asset, destination):
        destination.write_bytes((self.bundle / asset["name"]).read_bytes())
        self.downloads += 1


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.fixture = test_release.ReleaseTests("test_valid_complete_signature_and_every_asset")
        self.fixture.setUp()
        self.github = FakeGithub(self.fixture.bundle)

    def tearDown(self):
        self.fixture.tearDown()

    def verify(self, publish_release=False):
        return publish.checked_publication(self.github, TAG, self.fixture.roots,
                                           "minisign", publish_release)

    def test_verified_draft_remains_draft_without_publish_flag(self):
        evidence = self.verify()
        self.assertFalse(evidence["published"])
        self.assertEqual(len(evidence["release"]["assets"]), 10)
        self.assertEqual(self.github.patches, [])

    def test_publish_is_one_mutation_after_all_checks(self):
        evidence = self.verify(True)
        self.assertTrue(evidence["published"])
        self.assertEqual(len(self.github.patches), 1)
        self.assertEqual(self.github.patches[0][1], ("draft=false",))

    def test_changed_asset_id_refuses_publication(self):
        self.github.changed_assets = True
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.patches, [])

    def test_changed_tag_refuses_publication(self):
        self.github.changed_tag = True
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.patches, [])

    def test_changed_ci_refuses_publication(self):
        self.github.changed_checks = True
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.patches, [])

    def test_missing_required_reality_check_refuses_publication(self):
        self.github.missing_job = True
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.patches, [])

    def test_failed_main_ci_refuses_before_download(self):
        self.github.run_failed = True
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.downloads, 0)
        self.assertEqual(self.github.patches, [])

    def test_tag_must_match_the_recorded_draft_build_commit(self):
        self.github.build_commit = "b" * 40
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.downloads, 0)
        self.assertEqual(self.github.patches, [])

    def test_signed_single_arch_ci_bundle_cannot_be_published(self):
        self.fixture.arguments.arch = ["amd64"]
        self.fixture.arguments.output = str(self.fixture.directory / "single-arch")
        release.assemble(self.fixture.arguments)
        self.fixture.bundle = Path(self.fixture.arguments.output)
        self.fixture.sign()
        self.github = FakeGithub(self.fixture.bundle)
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.patches, [])

    def test_matching_github_digest_does_not_replace_release_signature(self):
        target = self.fixture.bundle / "agent-0.3.0-linux-musl-amd64"
        target.write_bytes(target.read_bytes() + b"tampered")
        self.github = FakeGithub(self.fixture.bundle)
        with self.assertRaises(ValueError):
            self.verify(True)
        self.assertEqual(self.github.patches, [])

    def test_annotated_tag_resolves_exact_build_commit(self):
        self.github.reference = {"type": "tag", "sha": "c" * 40}
        self.assertEqual(publish.tag_identity(self.github, TAG),
                         {"ref_type": "tag", "ref_sha": "c" * 40, "commit": COMMIT})


if __name__ == "__main__":
    unittest.main()
