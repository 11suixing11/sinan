#!/usr/bin/env python3
"""Bootstrap network boundaries and independently provisioned trust file tests."""

import io
import os
from pathlib import Path
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


class Response(io.BytesIO):
    def __init__(self, data, url):
        super().__init__(data)
        self.url = url


class BootstrapTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
