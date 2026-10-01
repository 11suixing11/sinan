"""Keep native smoke state observations reliable during Windows replacement."""
import json
from pathlib import Path
import runpy
import unittest
from unittest.mock import Mock, patch

SMOKE = runpy.run_path(str(Path(__file__).resolve().parents[1] / 'tools/agent-smoke.py'))


class SharedStateTests(unittest.TestCase):
    def test_windows_sharing_error_retries_and_observes_the_actual_value(self):
        path = Mock()
        path.read_text.side_effect = [PermissionError('sharing'), '{"pending":true}']
        with patch('sys.platform', 'win32'), patch('time.sleep'):
            self.assertEqual(SMOKE['read_shared_json'](path), {'pending': True})
        self.assertEqual(path.read_text.call_count, 2)

    def test_permanent_denial_still_fails_within_the_deadline(self):
        path = Mock()
        path.read_text.side_effect = PermissionError('denied')
        with patch('sys.platform', 'win32'), patch('time.sleep'), patch('time.monotonic', side_effect=[0, 1, 3]):
            with self.assertRaises(PermissionError):
                SMOKE['read_shared_json'](path)
        self.assertEqual(path.read_text.call_count, 2)

    def test_non_windows_permissions_and_corrupt_json_are_not_retried(self):
        path = Mock()
        path.read_text.side_effect = PermissionError('denied')
        with patch('sys.platform', 'linux'), patch('time.sleep') as sleep:
            with self.assertRaises(PermissionError):
                SMOKE['read_shared_json'](path)
            sleep.assert_not_called()
        path.read_text.side_effect = None
        path.read_text.return_value = 'corrupt'
        with patch('sys.platform', 'win32'), patch('time.sleep') as sleep:
            with self.assertRaises(json.JSONDecodeError):
                SMOKE['read_shared_json'](path)
            sleep.assert_not_called()


if __name__ == '__main__':
    unittest.main()
