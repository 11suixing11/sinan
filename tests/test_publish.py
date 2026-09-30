#!/usr/bin/env python3
"""Publication CI gates and asset/tag identity checks with real signed bundles."""

import copy
from pathlib import Path
import sys
import unittest
import urllib.parse
from unittest.mock import patch

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
        self.listed_releases = None
        self.release_detail_updates = {}
        self.changed_release_identity = {}
        self.api_calls = []
        self.assets = [{"id": index, "name": path.name, "state": "uploaded",
                        "size": len(path.read_bytes()), "digest": "sha256:" + release.digest(path.read_bytes())}
                       for index, path in enumerate(sorted(bundle.iterdir()), start=1)]

    def release_record(self):
        value = {"id": 7, "tag_name": TAG, "draft": self.draft,
                 "target_commitish": self.build_commit}
        if self.downloads:
            value.update(self.changed_release_identity)
        return value

    def api(self, endpoint, method="GET", fields=()):
        self.api_calls.append((endpoint, method))
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
            if self.draft:
                raise ValueError("GitHub API request failed (HTTP 404)")
            return self.release_record()
        if endpoint.startswith(f"repos/{publish.REPOSITORY}/releases?"):
            query = urllib.parse.parse_qs(urllib.parse.urlsplit(endpoint).query)
            page, per_page = int(query["page"][0]), int(query["per_page"][0])
            records = self.listed_releases
            if records is None:
                records = [self.release_record()]
            return copy.deepcopy(records[(page - 1) * per_page:page * per_page])
        if endpoint == f"repos/{publish.REPOSITORY}/releases/{self.release_record()['id']}":
            return dict(self.release_record(), **self.release_detail_updates)
        if f"/releases/{self.release_record()['id']}/assets?" in endpoint:
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

    def test_draft_tag_endpoint_is_unavailable_but_id_lookup_succeeds(self):
        with self.assertRaisesRegex(ValueError, "HTTP 404"):
            self.github.api(f"repos/{publish.REPOSITORY}/releases/tags/{TAG}")
        self.github.api_calls.clear()
        evidence = self.verify()
        self.assertEqual(evidence["release"]["id"], 7)
        self.assertIn((f"repos/{publish.REPOSITORY}/releases/7", "GET"), self.github.api_calls)
        self.assertFalse(any("/releases/tags/" in endpoint
                             for endpoint, _ in self.github.api_calls))

    def other_releases(self, count):
        return [dict(self.github.release_record(), id=100 + index,
                     tag_name=TAG + f".{index}") for index in range(count)]

    def refuse_before_download(self, message):
        with self.assertRaisesRegex(ValueError, message):
            self.verify(True)
        self.assertEqual(self.github.downloads, 0)
        self.assertEqual(self.github.patches, [])

    def test_exact_tag_on_second_page_ignores_similar_tags(self):
        self.github.listed_releases = self.other_releases(publish.RELEASES_PER_PAGE)
        self.github.listed_releases.append(self.github.release_record())
        self.assertEqual(self.verify()["release"]["id"], 7)
        self.assertIn((f"repos/{publish.REPOSITORY}/releases?per_page=100&page=2", "GET"),
                      self.github.api_calls)

    def test_published_duplicate_tag_on_later_page_refuses_publication(self):
        self.github.listed_releases = [self.github.release_record()]
        self.github.listed_releases += self.other_releases(publish.RELEASES_PER_PAGE - 1)
        self.github.listed_releases.append(dict(self.github.release_record(), id=8, draft=False))
        self.refuse_before_download("multiple releases match the exact tag")

    def test_repeated_release_id_across_pages_refuses_publication(self):
        self.github.listed_releases = self.other_releases(publish.RELEASES_PER_PAGE)
        self.github.listed_releases += [self.github.release_record(),
                                       self.github.listed_releases[0]]
        self.refuse_before_download("duplicate release ID")

    def test_full_final_page_refuses_incomplete_scan_even_after_match(self):
        self.github.listed_releases = self.other_releases(
            publish.RELEASES_PER_PAGE * publish.MAX_RELEASE_PAGES)
        self.github.listed_releases[0] = self.github.release_record()
        self.refuse_before_download("release listing exceeds the page limit")
        pages = [endpoint for endpoint, _ in self.github.api_calls if "/releases?" in endpoint]
        self.assertEqual(len(pages), publish.MAX_RELEASE_PAGES)

    def test_missing_exact_tag_refuses_publication(self):
        for records in ([], self.other_releases(1)):
            with self.subTest(records=records):
                self.github.listed_releases = records
                self.refuse_before_download("no release matches the exact tag")

    def test_malformed_listing_refuses_before_asset_lookup(self):
        record = self.github.release_record()
        pages = [None, {}, [None], [dict(record, tag_name=None)],
                 self.other_releases(publish.RELEASES_PER_PAGE + 1)]
        pages += [[dict(record, id=value)] for value in (True, "7", 0, -1)]
        for page in pages:
            with self.subTest(page=page):
                with patch.object(self.github, "api", return_value=page) as api:
                    with self.assertRaises(ValueError):
                        publish.release_snapshot(self.github, TAG)
                    self.assertEqual(api.call_count, 1)
                self.assertEqual(self.github.patches, [])

    def test_listed_build_commit_requires_full_sha(self):
        for commit in ("main", COMMIT[:7], None, True):
            with self.subTest(commit=commit):
                self.github.build_commit = commit
                self.refuse_before_download("release draft must record the exact build commit")

    def test_id_lookup_rechecks_listed_release_identity(self):
        for updates in ({"id": 8}, {"id": "7"}, {"tag_name": TAG + "-rc.1"},
                        {"target_commitish": "b" * 40}, {"target_commitish": COMMIT[:7]},
                        {"target_commitish": "main"}, {"target_commitish": None}):
            with self.subTest(updates=updates):
                self.github.release_detail_updates = updates
                self.refuse_before_download("release identity changed after listing")

    def test_changed_release_identity_during_verification_refuses_publication(self):
        for updates in ({"id": 8}, {"tag_name": TAG + "-rc.1"},
                        {"target_commitish": "b" * 40}):
            with self.subTest(updates=updates):
                self.github = FakeGithub(self.fixture.bundle)
                self.github.changed_release_identity = updates
                with self.assertRaises(ValueError):
                    self.verify(True)
                self.assertGreater(self.github.downloads, 0)
                self.assertEqual(self.github.patches, [])

    def test_new_duplicate_tag_during_download_refuses_publication(self):
        self.github.listed_releases = [self.github.release_record()]
        download = self.github.download

        def add_duplicate(asset, destination):
            download(asset, destination)
            if self.github.downloads == 1:
                self.github.listed_releases.append(dict(self.github.release_record(), id=8))

        with patch.object(self.github, "download", side_effect=add_duplicate):
            with self.assertRaisesRegex(ValueError, "multiple releases match the exact tag"):
                self.verify(True)
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
