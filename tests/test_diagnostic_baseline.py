import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).resolve().parents[1] / 'scripts' / 'diagnostic-baseline.py'
spec = importlib.util.spec_from_file_location('baseline', SCRIPT)
baseline = importlib.util.module_from_spec(spec)
spec.loader.exec_module(baseline)


class BaselineEvidenceTests(unittest.TestCase):
    def test_old_agent_has_unknown_metrics_time_and_secrets_are_excluded(self):
        value = baseline.panel_times({'last_seen': 123, 'device_public_key': 'private',
                                     'latest_metrics': {'cpu_percent': 12},
                                     'static_info': {'ip_addresses': ['192.0.2.1']}})
        self.assertEqual(value['last_heartbeat_at'], 123)
        self.assertIsNone(value['last_metrics_collected_at'])
        self.assertFalse(value['metrics_timestamp_available'])
        self.assertNotIn('private', str(value))
        self.assertNotIn('192.0.2.1', str(value))

    def test_metrics_and_heartbeat_times_remain_distinct(self):
        value = baseline.panel_times({'last_seen': 123, 'latest_metrics': {'collected_at': 100},
                                     'last_metrics_received_at': 102})
        self.assertEqual(value['last_metrics_collected_at'], 100)
        self.assertEqual(value['last_metrics_received_at'], 102)
        self.assertEqual(value['last_heartbeat_at'], 123)

    def test_evidence_is_private_and_never_overwrites(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'evidence'
            baseline.private_file(path, 'first')
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            with self.assertRaises(FileExistsError):
                baseline.private_file(path, 'second')
            self.assertEqual(path.read_text(), 'first')

    def test_cookie_rejects_public_files_symlinks_and_header_injection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'cookie'
            path.write_text('session=private')
            path.chmod(0o644)
            with self.assertRaises(ValueError):
                baseline.read_cookie(path)
            path.chmod(0o600)
            link = root / 'link'
            link.symlink_to(path)
            with self.assertRaises(ValueError):
                baseline.read_cookie(link)
            path.write_text('session=private\nInjected: yes')
            with self.assertRaises(ValueError):
                baseline.read_cookie(path)

    def test_remote_http_never_sends_cookie(self):
        with patch.object(baseline.urllib.request, 'build_opener') as opener:
            for origin in ('http://panel.example.com', 'http://192.0.2.1',
                           'http://[2001:db8::1]', 'http://localhost.example.com',
                           'http://127.0.0.1.example.com', 'http://2130706433'):
                with self.subTest(origin=origin), self.assertRaises(ValueError):
                    baseline.read_panel(origin, 123, 'session=private')
            opener.assert_not_called()

    def test_loopback_http_and_https_still_request_panel_times(self):
        with patch.object(baseline.urllib.request, 'build_opener') as opener:
            response = opener.return_value.open.return_value.__enter__.return_value
            response.read.return_value = b'{"last_seen":123,"latest_metrics":{}}'
            for origin in ('http://127.0.0.1:8080', 'http://localhost:8080',
                           'http://[::1]:8080', 'http://[::ffff:127.0.0.1]:8080',
                           'https://panel.example.com'):
                with self.subTest(origin=origin):
                    result = baseline.read_panel(origin, 123, 'session=private')
                    self.assertEqual(result['last_heartbeat_at'], 123)
                    request = opener.return_value.open.call_args.args[0]
                    self.assertEqual(request.full_url, origin + '/api/servers/123')
                    self.assertEqual(request.get_header('Cookie'), 'session=private')

    def test_redirect_never_forwards_cookie(self):
        with self.assertRaises(ValueError):
            baseline.NoRedirect().redirect_request(None, None, 302, None, None, 'https://other.example')


if __name__ == '__main__':
    unittest.main()
