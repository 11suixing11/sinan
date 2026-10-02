#!/usr/bin/env python3
"""Owned collection fixtures; mocks do not certify a Debian signature or image."""

import contextlib
import email.message
import hashlib
import http.server
import importlib.util
import io
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
import types
import unittest
from unittest import mock
import urllib.error
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('rootfs_collect', ROOT / 'tools/nodequality-rootfs-collect.py')
COLLECT = importlib.util.module_from_spec(spec)
spec.loader.exec_module(COLLECT)
BUILD = COLLECT.BUILD
MAIN = {'id': 'main', 'archive': 'debian', 'timestamp': '20261001T142720Z', 'suite': 'bookworm'}
SECURITY = {'id': 'security', 'archive': 'debian-security', 'timestamp': '20261001T142623Z', 'suite': 'bookworm-security'}
REQUEST = {'schema': 1, 'arch': 'amd64', 'source_epoch': 1790864840, 'repositories': [MAIN, SECURITY]}


def identity(content):
    return {'sha256': hashlib.sha256(content).hexdigest(), 'size': len(content)}


def encoded(value):
    return BUILD.canonical(value) + b'\n'


def package(name='owned', arch='all', version='1', source='fixture (1)'):
    return {'Package': name, 'Version': version, 'Architecture': arch, 'Source': source,
            'Filename': 'pool/main/f/fixture/' + name + '_1.deb', 'Size': '12', 'SHA256': identity(b'owned-binary')['sha256']}


def source(name='fixture', version='1'):
    body = b'owned-source'
    sums = '\n'.join(identity(body)['sha256'] + ' ' + str(len(body)) + ' ' + filename
                     for filename in ('fixture.dsc', 'fixture.orig.tar.xz'))
    return {'Package': name, 'Version': version, 'Directory': 'pool/main/f/fixture', 'Checksums-Sha256': sums}


def fixture_solver_capacity():
    return {'reserved_expansion_bytes': 2 * BUILD.MAX_INDEX, 'derived_directories_removed': True,
            'phase_observations': [
                {'operation': 'update', 'lists_bytes': 1, 'archives_bytes': 0, 'mirrors_bytes': 2, 'solver_owned_bytes': 10},
                {'operation': 'plan', 'lists_bytes': 3, 'archives_bytes': 0, 'mirrors_bytes': 2, 'solver_owned_bytes': 12}]}


def capacity_fixture(root):
    collector = COLLECT.Collector(root / 'output', 4 * 1024**2, BUILD.Deadline(30))
    # This fixture's caller has created the owned output, as collect does.
    (collector.cache / 'owned-metadata').write_bytes(b'owned metadata')
    work = collector.output / 'solver'
    work.mkdir(mode=0o700)
    (work / 'selection.json').write_bytes(encoded({'fixture': True}))
    packages = []
    for name in sorted(set(BUILD.TOOL_PACKAGES.values()) | {'apt', 'base-files'}):
        row = package(name)
        packages.append({'repository': 'main', 'name': row['Package'], 'version': row['Version'],
            'architecture': row['Architecture'], 'filename': row['Filename'], 'size': int(row['Size']),
            'sha256': row['SHA256'], 'blob': row['SHA256'] + '.deb', 'source_name': 'fixture', 'source_version': '1'})
    repositories = []
    for requested in (MAIN, SECURITY):
        repositories.append(dict(requested, inrelease=dict(identity(b'owned'), blob='owned-release'),
            indices=[dict(identity(b'owned'), blob='owned-' + kind + '.xz', kind=kind, path=path)
                     for kind, path in (('Packages', 'main/binary-amd64/Packages.xz'), ('Sources', 'main/source/Sources.xz'))]))
    sources = COLLECT.source_closure(packages, repositories, {'main': {'Sources': [source()]}, 'security': {'Sources': []}})
    for value in sources[0]['files']:
        value['blob'] = value['sha256'] + '.source'
    materials = dict(REQUEST, keyring=dict(identity(b'owned key'), blob='owned-key.gpg'),
                     repositories=repositories, packages=packages, sources=sources)
    return collector, materials, {'capacity': fixture_solver_capacity()}


class CollectionContracts(unittest.TestCase):
    def test_request_requires_exact_debian12_pair_and_native_architecture(self):
        self.assertEqual(COLLECT.validate_request(REQUEST), {'debian': MAIN['timestamp'], 'debian-security': SECURITY['timestamp']})
        for altered in (dict(REQUEST, arch='i386'), dict(REQUEST, repositories=[MAIN]),
                        dict(REQUEST, repositories=[MAIN, MAIN]), dict(REQUEST, builder={})):
            with self.subTest(altered=altered), self.assertRaises(ValueError):
                COLLECT.validate_request(altered)

    def test_import_observation_rejects_nearest_earlier_and_duplicates(self):
        content = encoded({'result': {'debian': ['20261001T082322Z', MAIN['timestamp']]}})
        self.assertIn(MAIN['timestamp'], COLLECT.validate_imports(content, 'debian', MAIN['timestamp']))
        for value in ({'result': {'debian': ['20261001T082322Z']}},
                      {'result': {'debian': [MAIN['timestamp'], MAIN['timestamp']]}},
                      {'result': {'debian-security': [MAIN['timestamp']]}}):
            with self.subTest(value=value), self.assertRaises(ValueError):
                COLLECT.validate_imports(encoded(value), 'debian', MAIN['timestamp'])

    def test_official_transport_has_no_foreign_redirect_or_userinfo(self):
        self.assertEqual(COLLECT.allowed_url(COLLECT.import_url('debian', MAIN['timestamp'])),
                         COLLECT.import_url('debian', MAIN['timestamp']))
        for value in ('http://snapshot.debian.org/archive/debian/',
                      'https://snapshot.debian.org.evil.example/archive/debian/',
                      'https://user@snapshot.debian.org/archive/debian/',
                      'https://snapshot.debian.org/arbitrary/',
                      'https://snapshot.debian.org:443/archive/debian/'):
            with self.subTest(value=value), self.assertRaises(ValueError):
                COLLECT.allowed_url(value)

    def test_ordinary_release_and_unapproved_release_link(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            ordinary = root / 'os-release'
            ordinary.write_text('ID=debian\nVERSION_ID="12"\n')
            self.assertEqual(COLLECT.debian_release(ordinary)['ID'], 'debian')
            linked = root / 'release-link'
            linked.symlink_to(ordinary)
            with self.assertRaisesRegex(ValueError, 'release link'):
                COLLECT.debian_release(linked)
            ordinary.write_text('ID=ubuntu\nVERSION_ID="12"\n')
            with self.assertRaises(ValueError):
                COLLECT.debian_release(ordinary)

    def test_standard_debian_release_link_has_one_exact_target(self):
        with mock.patch.object(Path, 'is_symlink', return_value=True), \
             mock.patch.object(Path, 'resolve', return_value=Path('/usr/lib/os-release')), \
             mock.patch.object(BUILD, 'read_regular', return_value=b'ID=debian\nVERSION_ID=12\n') as reader:
            COLLECT.debian_release()
            reader.assert_called_once_with(Path('/usr/lib/os-release'), 65536)
        with mock.patch.object(Path, 'is_symlink', return_value=True), \
             mock.patch.object(Path, 'resolve', return_value=Path('/tmp/unowned-release')):
            with self.assertRaises(ValueError):
                COLLECT.debian_release()

    def test_stream_checks_byte_limit_before_write(self):
        with tempfile.TemporaryDirectory() as name, mock.patch.object(COLLECT, 'free_disk'):
            output = Path(name) / 'body'
            with self.assertRaisesRegex(ValueError, 'byte budget'):
                COLLECT.stream_response(io.BytesIO(b'x' * 1025), output, 1024)
            self.assertLessEqual(output.stat().st_size, 1024)
            exact = Path(name) / 'exact'
            self.assertEqual(COLLECT.stream_response(io.BytesIO(b'x' * 1024), exact, 1024), 1024)
            self.assertEqual(exact.read_bytes(), b'x' * 1024)

    def test_free_disk_reserve_and_finite_publication(self):
        usage = types.SimpleNamespace(f_bavail=1, f_frsize=4096)
        with mock.patch.object(os, 'statvfs', return_value=usage):
            with self.assertRaisesRegex(ValueError, 'free-disk reserve'):
                COLLECT.free_disk('/tmp', COLLECT.MIN_FREE)
        with tempfile.TemporaryDirectory() as name:
            output = Path(name) / 'json'
            with self.assertRaisesRegex(ValueError, 'reader limit'):
                COLLECT.publish(output, b'x' * 11, 10)
            self.assertFalse(output.exists())

    def test_success_retains_selected_headers_and_redacted_urls(self):
        with tempfile.TemporaryDirectory() as name, mock.patch.object(COLLECT, 'free_disk'):
            root = Path(name)
            headers = email.message.Message()
            headers['Content-Length'] = '5'
            headers['Set-Cookie'] = 'owned-secret'
            headers['Authorization'] = 'Bearer owned-secret'
            headers['Location'] = 'https://user:owned-secret@snapshot.debian.org/file/fixture?token=owned-secret'
            response = mock.MagicMock()
            response.__enter__.return_value = response
            response.status, response.headers = 200, headers
            response.geturl.return_value = 'https://snapshot.debian.org/file/fixture?token=owned-secret'
            response.read.side_effect = [b'owned', b'']
            opener = mock.Mock()
            opener.open.return_value = response
            with mock.patch.object(COLLECT.urllib.request, 'build_opener', return_value=opener):
                report = COLLECT.fetch_worker('https://snapshot.debian.org/file/fixture?token=owned-secret',
                    root / 'body', 16, root / 'headers')
            retained = (root / 'headers').read_bytes()
            self.assertNotIn(b'owned-secret', retained)
            self.assertNotIn('owned-secret', json.dumps(report))
            self.assertEqual(report['header_representation'], 'selected-parsed-headers')
            self.assertEqual(json.loads(retained)['headers']['Content-Length'], ['5'])
            self.assertEqual((root / 'body').read_bytes(), b'owned')

    def test_ambiguous_content_length_rejects_before_body_write(self):
        for transfer in (False, True):
            with self.subTest(transfer=transfer), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                headers = email.message.Message()
                headers['Content-Length'] = '5'
                headers['Transfer-Encoding' if transfer else 'Content-Length'] = 'chunked' if transfer else '7'
                response = mock.MagicMock()
                response.__enter__.return_value = response
                response.status, response.headers = 200, headers
                response.geturl.return_value = 'https://snapshot.debian.org/file/fixture'
                opener = mock.Mock()
                opener.open.return_value = response
                context = {}
                with mock.patch.object(COLLECT.urllib.request, 'build_opener', return_value=opener):
                    with self.assertRaisesRegex(ValueError, 'ambiguous'):
                        COLLECT.fetch_worker(response.geturl(), root / 'body', 16, root / 'headers', context=context)
                response.read.assert_not_called()
                self.assertFalse((root / 'body').exists())
                self.assertEqual(context['http_status'], 200)
                self.assertEqual(context['content_length_validation_issue'], 'ambiguous_framing')

    def test_selected_header_record_is_filtered_again_when_retained(self):
        headers = COLLECT.retained_headers(encoded({'representation': 'selected-parsed-headers',
            'headers': {'Content-Length': ['5'], 'Set-Cookie': ['owned-secret'],
                        'Location': ['https://user:owned-secret@snapshot.debian.org/file/fixture?token=owned-secret']}}))
        self.assertNotIn('Set-Cookie', headers)
        self.assertNotIn('owned-secret', json.dumps(headers))

    def test_failure_receipt_respects_budget_after_original_deadline_expires(self):
        with tempfile.TemporaryDirectory() as name, mock.patch.object(COLLECT, 'free_disk'):
            root = Path(name)
            output = root / 'output'
            output.mkdir(mode=0o700)
            collector = COLLECT.Collector(output, 128, BUILD.Deadline(1))
            collector.deadline.end = time.monotonic() - 1
            (output / 'owned').write_bytes(b'x' * 120)
            with self.assertRaisesRegex(ValueError, 'byte budget'):
                collector.failure_budget(b'oversized failure', BUILD.Deadline(1))
            collector.failure_budget(b'failure', BUILD.Deadline(1))
            replacement = root / 'replacement'
            output.rename(root / 'retained-original')
            replacement.mkdir(mode=0o700)
            replacement.rename(output)
            with self.assertRaisesRegex(ValueError, 'identity changed'):
                collector.failure_budget(b'failure', BUILD.Deadline(1))

    def test_success_publication_registers_partial_fsync_failure_for_rollback(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / 'collection.json'
            owned = []
            with mock.patch.object(os, 'fsync', side_effect=OSError('owned fsync failure')):
                with self.assertRaisesRegex(OSError, 'owned fsync failure'):
                    COLLECT.publish_success(path, b'owned partial publication', 64, owned)
            self.assertTrue(path.exists())
            self.assertEqual(len(owned), 1)
            cleanup = COLLECT.rollback_success(owned)
            self.assertEqual(cleanup, [{'path': 'collection.json', 'removed': True}])
            self.assertFalse(path.exists())

    def test_success_publication_never_overwrites_or_registers_existing_file(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            original = root / 'original'
            original.write_bytes(b'preexisting data')
            link = root / 'linked'
            link.symlink_to(original)
            for path in (original, link):
                with self.subTest(path=path.name):
                    owned = []
                    with self.assertRaises(FileExistsError):
                        COLLECT.publish_success(path, b'replacement', 64, owned)
                    self.assertEqual(owned, [])
                    self.assertEqual(COLLECT.rollback_success(owned), [])
                    self.assertEqual(original.read_bytes(), b'preexisting data')
                    self.assertTrue(link.is_symlink())

    def test_success_rollback_refuses_replaced_file_identity(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            path, replacement = root / 'materials.json', root / 'replacement'
            owned = []
            COLLECT.publish_success(path, b'owned publication', 64, owned)
            replacement.write_bytes(b'foreign replacement')
            os.replace(replacement, path)
            cleanup = COLLECT.rollback_success(owned)
            self.assertFalse(cleanup[0]['removed'])
            self.assertIn('identity changed', cleanup[0]['error'])
            self.assertEqual(path.read_bytes(), b'foreign replacement')

    def test_capacity_plan_records_exact_references_unique_storage_and_phase_observations(self):
        with tempfile.TemporaryDirectory() as name, mock.patch.object(COLLECT, 'free_disk'):
            root = Path(name)
            (root / 'output').mkdir(mode=0o700)
            collector, materials, solver = capacity_fixture(root)
            observed = COLLECT.owned_size(collector.output, collector.deadline)
            receipt = COLLECT.write_capacity_plan(collector, materials, solver)
            raw = (collector.output / receipt['path']).read_bytes()
            plan = json.loads(raw)
            self.assertEqual(receipt, dict(identity(raw), path='capacity-plan.json'))
            self.assertEqual(plan['packages'], materials['packages'])
            self.assertEqual(plan['sources'], materials['sources'])
            count = len(materials['packages'])
            self.assertEqual(plan['binary']['file_references'], count)
            self.assertEqual(plan['binary']['reference_bytes'], 12 * count)
            self.assertEqual(plan['binary']['unique_blobs'], 1)
            self.assertEqual(plan['binary']['unique_blob_bytes'], 12)
            self.assertEqual(plan['source']['name_version_pairs'], 1)
            self.assertEqual(plan['source']['file_references'], 2)
            self.assertEqual(plan['source']['reference_bytes'], 24)
            self.assertEqual(plan['source']['unique_blobs'], 1)
            self.assertEqual(plan['source']['unique_blob_bytes'], 12)
            self.assertEqual(plan['unique_cache_payload_bytes'], 24)
            self.assertEqual(plan['conservative_payload_admission_bytes'], 12 * count + 24)
            self.assertEqual(plan['observed_owned_metadata_bytes_before_plan'], observed)
            self.assertEqual(plan['plan_metadata_bytes'], len(raw))
            self.assertEqual(plan['required_owned_bytes_before_payloads'], observed + len(raw) + 12 * count + 24)
            self.assertEqual(plan['apt'], solver['capacity'])
            self.assertIn('phase samples', plan['accounting_scope'])
            self.assertIn('excluded', plan['future_overhead_scope'])
            for key in ('complete', 'payload_authenticated', 'builder_approved', 'full_ready', 'reproducibility_verified'):
                self.assertFalse(plan[key])

    def test_capacity_plan_cannot_bypass_its_own_byte_or_disk_guards(self):
        for reason in ('total', 'disk', 'metadata', 'conflicting_blob'):
            with self.subTest(reason=reason), tempfile.TemporaryDirectory() as name:
                root = Path(name)
                (root / 'output').mkdir(mode=0o700)
                collector, materials, solver = capacity_fixture(root)
                with contextlib.ExitStack() as stack:
                    if reason == 'total':
                        collector.max_total = COLLECT.owned_size(collector.output, collector.deadline)
                    elif reason == 'disk':
                        stack.enter_context(mock.patch.object(os, 'statvfs', return_value=types.SimpleNamespace(
                            f_bavail=COLLECT.MIN_FREE, f_frsize=1)))
                    elif reason == 'metadata':
                        stack.enter_context(mock.patch.object(COLLECT, 'MAX_RECEIPT', 16))
                    else:
                        materials['sources'][0]['files'][0]['size'] += 1
                    with self.assertRaises(ValueError):
                        COLLECT.write_capacity_plan(collector, materials, solver)
                self.assertFalse((collector.output / 'capacity-plan.json').exists())

    def test_fetch_errors_preserve_http_dns_tls_and_timeout_categories(self):
        now = time.monotonic()
        for status in (403, 429):
            error = urllib.error.HTTPError('https://snapshot.debian.org/', status, 'owned error', {}, None)
            result = COLLECT.fetch_error(error, error.url, now)
            self.assertEqual(result['http_status'], status)
            self.assertEqual(result['category'], 'http_' + str(status))
            self.assertFalse(result['complete'])
        result = COLLECT.fetch_error(urllib.error.URLError(socket.gaierror('owned DNS failure')), 'owned-url', now)
        self.assertEqual(result['category'], 'dns')
        self.assertIsNone(result['http_status'])
        self.assertFalse(result['response_received'])
        self.assertIsNone(result['final_url'])
        tls = COLLECT.fetch_error(urllib.error.URLError(COLLECT.ssl.SSLError('owned TLS failure')), 'owned-url', now)
        self.assertEqual(tls['category'], 'tls')
        self.assertIsNone(tls['http_status'])
        self.assertFalse(tls['response_received'])
        self.assertEqual(COLLECT.fetch_error(TimeoutError(), 'owned-url', now)['category'], 'timeout')

    def test_fetch_error_context_preserves_observations_without_credentials_or_unbounded_text(self):
        url = 'https://user:owned-secret@snapshot.debian.org/archive/debian/fixture?token=owned-secret#owned-secret'
        context = {'failure_stage': 'content_length_validation', 'response_received': True, 'http_status': 200,
                   'final_url': url, 'response_content_length': 'invalid' + 'x' * 1024,
                   'response_bytes_read': 3, 'response_bytes_written': 3}
        error = ValueError('invalid response Content-Length\x00 ' + url + ' ' + 'x' * 4096)
        result = COLLECT.fetch_error(error, url, time.monotonic(), context)
        self.assertEqual(result['category'], 'response_invalid')
        self.assertEqual(result['http_status'], 200)
        self.assertTrue(result['response_received'])
        self.assertEqual(result['error_type'], 'ValueError')
        self.assertEqual(result['failure_stage'], 'content_length_validation')
        self.assertLessEqual(len(result['error_message']), 1024)
        self.assertLessEqual(len(result['response_content_length']), 128)
        self.assertTrue(result['response_content_length_truncated'])
        self.assertNotIn('owned-secret', json.dumps(result))
        self.assertNotIn('\x00', result['error_message'])
        self.assertEqual(result['final_url'], 'https://snapshot.debian.org/archive/debian/fixture')
        response = urllib.error.HTTPError(url, 429, 'owned-secret response reason', {'Content-Length': '73'}, None)
        actual = COLLECT.fetch_error(response, url, time.monotonic())
        self.assertEqual(actual['category'], 'http_429')
        self.assertEqual(actual['http_status'], 429)
        self.assertTrue(actual['response_received'])
        self.assertEqual(actual['response_content_length'], '73')
        self.assertEqual(actual['error_message'], 'HTTP Error 429')
        self.assertNotIn('owned-secret', json.dumps(actual))
        response.close()
        redirect_context = {}
        redirect = COLLECT.OfficialRedirects(redirect_context)
        request = urllib.request.Request('https://snapshot.debian.org/archive/debian/fixture')
        with self.assertRaisesRegex(ValueError, 'non-official'):
            redirect.redirect_request(request, None, 302, 'Found', {'Content-Length': '0'},
                                      'https://user:owned-secret@evil.example/path?token=owned-secret')
        rejected = COLLECT.fetch_error(ValueError('non-official collection URL'), request.full_url,
                                       time.monotonic(), redirect_context)
        self.assertEqual(rejected['category'], 'response_invalid')
        self.assertEqual(rejected['http_status'], 302)
        self.assertEqual(rejected['failure_stage'], 'redirect_url_validation')
        self.assertEqual(rejected['redirect_target'], 'https://evil.example/path')
        self.assertNotIn('owned-secret', json.dumps(rejected))

    def test_missing_or_partial_worker_footer_preserves_bounded_evidence_and_unknown_status(self):
        for partial in (False, True):
            with self.subTest(partial=partial), tempfile.TemporaryDirectory() as name, \
                 mock.patch.object(COLLECT, 'free_disk'):
                root = Path(name)
                output = root / 'output'
                output.mkdir(mode=0o700)
                collector = COLLECT.Collector(output, 1024**2, BUILD.Deadline(30))
                work = output / 'download-owned'
                work.mkdir(mode=0o700)
                original = b'{"schema":1,"complete":false'
                if partial:
                    (work / 'receipt.json').write_bytes(original)
                (work / 'body').write_bytes(b'owned body that must not be retained')
                (work / 'headers').write_bytes(b'Content-Length: 12\r\nSet-Cookie: owned-secret\r\n\r\n')
                report = collector.failed_download(work, 'https://snapshot.debian.org/archive/debian/fixture',
                    ValueError('subprocess failed without a trustworthy footer'), 'producer_exit')
                self.assertIsNone(report['http_status'])
                self.assertIsNone(report['response_received'])
                self.assertIsNone(report['error_type'])
                self.assertIsNone(report['error_message'])
                self.assertIsNone(report['final_url'])
                self.assertEqual(report['failure_stage'], 'worker_receipt_unavailable')
                self.assertEqual(report['category'], 'producer_failed_or_deadline')
                evidence = report['failure_evidence']
                self.assertTrue(evidence['complete'])
                self.assertFalse(evidence['body_retained'])
                headers = (output / evidence['response_headers']['path']).read_bytes()
                self.assertNotIn(b'owned-secret', headers)
                self.assertNotIn(b'Set-Cookie', headers)
                self.assertEqual(json.loads(headers)['headers']['Content-Length'], ['12'])
                if partial:
                    self.assertEqual((output / evidence['worker_receipt']['path']).read_bytes(), original)
                else:
                    self.assertIsNone(evidence['worker_receipt'])
                self.assertFalse((output / evidence['directory'] / 'body').exists())

    def test_expired_request_deadline_still_retains_metadata_after_discarding_body(self):
        with tempfile.TemporaryDirectory() as name, mock.patch.object(COLLECT, 'free_disk'):
            output = Path(name) / 'output'
            output.mkdir(mode=0o700)
            collector = COLLECT.Collector(output, 4096, BUILD.Deadline(1))
            work = output / 'download-owned'
            work.mkdir(mode=0o700)
            worker = COLLECT.fetch_error(TimeoutError('owned read timed out'),
                'https://snapshot.debian.org/archive/debian/fixture', time.monotonic(),
                {'response_received': True, 'http_status': 200, 'failure_stage': 'body_read'})
            original = encoded(worker)
            (work / 'receipt.json').write_bytes(original)
            (work / 'headers').write_bytes(b'Content-Length: 8192\r\nSet-Cookie: owned-secret\r\n\r\n')
            (work / 'body').write_bytes(b'x' * 8192)
            collector.deadline.end = time.monotonic() - 1
            report = collector.failed_download(work, worker['url'],
                ValueError('overall operation deadline exceeded'), 'producer_exit')
            self.assertEqual(report['category'], 'timeout')
            self.assertEqual(report['http_status'], 200)
            self.assertEqual(report['parent_failure']['category'], 'timeout')
            self.assertTrue(report['failure_evidence']['complete'])
            self.assertFalse((work / 'body').exists())
            receipt = output / report['failure_evidence']['worker_receipt']['path']
            self.assertEqual(receipt.read_bytes(), original)
            headers = (output / report['failure_evidence']['response_headers']['path']).read_bytes()
            self.assertNotIn(b'owned-secret', headers)
            self.assertLessEqual(COLLECT.owned_size(output, BUILD.Deadline(1)), collector.max_total)
            self.assertLess(collector.deadline.end, time.monotonic())

    def test_failure_metadata_keeps_its_original_byte_and_disk_reserve_guards(self):
        for guard in ('bytes', 'disk'):
            with self.subTest(guard=guard), tempfile.TemporaryDirectory() as name:
                output = Path(name) / 'output'
                output.mkdir(mode=0o700)
                collector = COLLECT.Collector(output, 1 if guard == 'bytes' else 4096, BUILD.Deadline(1))
                work = output / 'download-owned'
                work.mkdir(mode=0o700)
                (work / 'headers').write_bytes(b'Content-Length: 12\r\n\r\n')
                (work / 'body').write_bytes(b'untrusted-body')
                with mock.patch.object(COLLECT, 'free_disk', side_effect=ValueError('owned reserve refusal')) as reserve:
                    report = collector.failed_download(work, 'https://snapshot.debian.org/archive/debian/fixture',
                        ValueError('overall operation deadline exceeded'), 'producer_exit')
                self.assertEqual(report['category'], 'timeout')
                self.assertIsNone(report['http_status'])
                self.assertFalse(report['failure_evidence']['complete'])
                self.assertIn('error', report['failure_evidence'])
                self.assertFalse((work / 'body').exists())
                self.assertEqual(list(output.glob('failed-download-*')), [])
                if guard == 'disk':
                    reserve.assert_called_once()

    def test_parent_diagnostics_are_bounded_and_never_copy_command_or_environment(self):
        error = ValueError('trusted build/verification command failed https://user:owned-secret@snapshot.debian.org/file/fixture?token=owned-secret ' + 'x' * 4096)
        error.factory_command = {'argv': ['owned-secret'], 'output': b'owned-secret',
            'returncode': 1, 'cleanup_returncode': 1, 'output_truncated': True}
        result = COLLECT.parent_failure(error)
        self.assertEqual(result['returncode'], 1)
        self.assertEqual(result['cleanup_returncode'], 1)
        self.assertTrue(result['output_truncated'])
        self.assertLessEqual(len(result['error_message']), 1024)
        self.assertNotIn('owned-secret', json.dumps(result))
        self.assertNotIn('argv', result)
        self.assertNotIn('output', result)
        for cancelled in (KeyboardInterrupt(), SystemExit(143)):
            self.assertEqual(COLLECT.parent_failure(cancelled)['category'], 'cancelled')
            self.assertEqual(COLLECT.fetch_error(cancelled, 'https://snapshot.debian.org/file/fixture',
                time.monotonic())['category'], 'cancelled')

    def test_apt_configuration_detaches_all_host_state_and_hooks(self):
        with tempfile.TemporaryDirectory() as name:
            work = Path(name)
            config = COLLECT.apt_config(work, [(MAIN, work / 'mirror')], 'amd64').read_text()
            for key in ('Dir::Etc::main', 'Dir::Etc::parts', 'Dir::Etc::sourceparts', 'Dir::Etc::preferencesparts',
                        'Dir::Etc::trustedparts', 'Dir::State::status', 'Dir::State::extended_states',
                        'Dir::Log', 'Dir::Log::History', 'Dir::Log::Terminal'):
                self.assertIn(key + ' "' + str(work), config)
            self.assertNotIn('/etc/apt', config)
            self.assertNotIn('/var/lib/dpkg', config)
            self.assertNotIn('/var/log/apt', config)
            self.assertEqual((work / 'status').read_bytes(), b'')
            self.assertEqual((work / 'extended_states').read_bytes(), b'')
            self.assertIn('#clear APT::Architectures;', config)
            self.assertIn('#clear DPkg::Pre-Install-Pkgs;', config)
            self.assertIn('APT::Sandbox::User "root";', config)
            sources = (work / 'sources.list').read_text()
            self.assertIn('file://', sources)
            self.assertIn('signed-by=', sources)
            self.assertNotIn('trusted=yes', sources)

    def test_essential_names_seed_apt_without_lexical_version_selection(self):
        row = dict(package('base-files', version='2:1'), Essential='yes')
        indices = {'main': {'Packages': [row]}, 'security': {'Packages': [package('base-files', version='10')]}}
        seeds, essentials = COLLECT.essential_seeds([MAIN, SECURITY], indices)
        self.assertEqual(essentials, ['base-files'])
        self.assertIn('apt', seeds)
        self.assertTrue(set(BUILD.TOOL_PACKAGES.values()) <= set(seeds))
        self.assertNotIn('base-files=2:1', seeds)

    def test_solver_uri_maps_exact_repository_native_all_and_source_version(self):
        with tempfile.TemporaryDirectory() as name:
            mirror = Path(name) / 'main'
            row = package(source='fixture (1)', version='1+b1')
            text = ("'file:" + str(mirror / row['Filename']) + "' owned.deb 12 MD5Sum:ignored\n").encode()
            selected = COLLECT.map_print_uris(text, [(MAIN, mirror)], {'main': {'Packages': [row]}}, 'amd64')
            self.assertEqual(selected[0]['architecture'], 'all')
            self.assertEqual(selected[0]['version'], '1+b1')
            self.assertEqual(selected[0]['source_version'], '1')
            self.assertEqual(selected[0]['sha256'], row['SHA256'])
            for changed in (dict(row, Architecture='arm64'), dict(row, Size='13')):
                with self.subTest(changed=changed), self.assertRaises(ValueError):
                    COLLECT.map_print_uris(text, [(MAIN, mirror)], {'main': {'Packages': [changed]}}, 'amd64')

    def test_solver_rejects_network_uri_duplicate_and_missing_authenticated_row(self):
        with tempfile.TemporaryDirectory() as name:
            mirror = Path(name)
            row = package()
            own = ("'file:" + str(mirror / row['Filename']) + "' owned.deb 12 MD5Sum:x\n").encode()
            for content, rows in ((b"'https://evil.example/package.deb' owned.deb 12 SHA256:x\n", [row]),
                                  (own + own, [row]), (own, []), (own, [row, row])):
                with self.subTest(content=content, rows=rows), self.assertRaises(ValueError):
                    COLLECT.map_print_uris(content, [(MAIN, mirror)], {'main': {'Packages': rows}}, 'amd64')

    def test_security_uri_optional_display_checksum_keeps_signed_identity(self):
        with tempfile.TemporaryDirectory() as name:
            mirror = Path(name) / 'security'
            row = dict(package(arch='arm64', version='1+security'),
                       Filename='pool/updates/main/f/fixture/owned_1+security_arm64.deb')
            url = "'file:" + str(mirror / row['Filename']).replace('+', '%2b') + "'"
            required = url + ' owned_1+security_arm64.deb 12'
            mirrors = [(SECURITY, mirror)]
            indices = {'security': {'Packages': [row]}}
            for suffix in ('', ' \t', ' MD5Sum:display-only', ' SHA256:not-authoritative \t'):
                with self.subTest(suffix=suffix):
                    selected = COLLECT.map_print_uris((required + suffix + '\n').encode(), mirrors, indices, 'arm64')
                    self.assertEqual(len(selected), 1)
                    self.assertEqual(selected[0]['repository'], 'security')
                    self.assertEqual(selected[0]['architecture'], 'arm64')
                    self.assertEqual(selected[0]['filename'], row['Filename'])
                    self.assertEqual(selected[0]['size'], int(row['Size']))
                    self.assertEqual(selected[0]['sha256'], row['SHA256'])
            malformed = (url + ' 12', url + ' owned.deb',
                         required + ' MD5Sum:display-only extra',
                         url + ' owned.deb 13 \t')
            for line in malformed:
                with self.subTest(line=line), self.assertRaises(ValueError):
                    COLLECT.map_print_uris((line + '\n').encode(), mirrors, indices, 'arm64')

    def test_source_closure_can_use_main_source_for_security_binary(self):
        packages = [{'source_name': 'fixture', 'source_version': '1', 'repository': 'security'}]
        indices = {'main': {'Sources': [source()]}, 'security': {'Sources': []}}
        result = COLLECT.source_closure(packages, [MAIN, SECURITY], indices)
        self.assertEqual(result[0]['repository'], 'main')
        self.assertEqual({value['name'] for value in result[0]['files']}, {'fixture.dsc', 'fixture.orig.tar.xz'})

    def test_sources_duplicate_identical_rows_across_repos_have_explicit_choice(self):
        packages = [{'source_name': 'fixture', 'source_version': '1'}]
        security_source = dict(source(), Directory='pool/updates/main/f/fixture')
        indices = {'main': {'Sources': [source()]}, 'security': {'Sources': [security_source]}}
        result = COLLECT.source_closure(packages, [MAIN, SECURITY], indices)
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]['repository'], 'main')
        indices['security']['Sources'][0] = dict(security_source,
            **{'Checksums-Sha256': source()['Checksums-Sha256'].replace(identity(b'owned-source')['sha256'], identity(b'changed-source')['sha256'])})
        with self.assertRaisesRegex(ValueError, 'conflicting'):
            COLLECT.source_closure(packages, [MAIN, SECURITY], indices)

    def test_security_pool_is_distinct_and_other_components_are_rejected(self):
        self.assertEqual(COLLECT.pool_path(SECURITY, 'pool/updates/main/f/fixture'), 'pool/updates/main/f/fixture')
        for repo, path in ((SECURITY, 'pool/main/f/fixture'), (MAIN, 'pool/updates/main/f/fixture'),
                           (SECURITY, 'pool/updates/non-free/f/fixture'), (MAIN, 'pool/contrib/f/fixture')):
            with self.subTest(repo=repo, path=path), self.assertRaises(ValueError):
                COLLECT.pool_path(repo, path)

    def test_sources_missing_orig_duplicate_or_wrong_version_are_rejected(self):
        packages = [{'source_name': 'fixture', 'source_version': '1'}]
        no_dsc = dict(source(), **{'Checksums-Sha256': identity(b'x')['sha256'] + ' 1 only.orig.tar.xz'})
        for rows in ([], [source(version='2')], [source(), source()], [no_dsc]):
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                COLLECT.source_closure(packages, [MAIN], {'main': {'Sources': rows}})

    def test_owned_factory_rejects_existing_output_links_and_hardlinks(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            output = root / 'output'
            output.mkdir(mode=0o700)
            with self.assertRaises(FileExistsError):
                BUILD.reserve_output(output)
            (output / 'one').write_bytes(b'owned')
            (output / 'linked').symlink_to(output / 'one')
            with self.assertRaises(ValueError):
                COLLECT.owned_size(output, BUILD.Deadline(5))
            (output / 'linked').unlink()
            os.link(output / 'one', output / 'two')
            with self.assertRaises(ValueError):
                COLLECT.owned_size(output, BUILD.Deadline(5))

    def test_solver_exec_is_read_only_file_selection_in_distinct_namespace(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            config = root / 'apt.conf'
            config.write_bytes(b'owned config')
            evidence = root / 'namespace.json'
            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(os, 'stat', wraps=os.stat) as metadata, \
                 mock.patch.object(socket, 'if_nameindex', return_value=[(1, 'lo')]), \
                 mock.patch.object(os, 'execve') as execute:
                def stat_owned(path, *args, **kwargs):
                    if str(path) == '/proc/self/ns/net':
                        return types.SimpleNamespace(st_ino=456)
                    return metadata._mock_wraps(path, *args, **kwargs)

                metadata.side_effect = stat_owned
                COLLECT.solver_worker(config, evidence, 123, 'plan', ['apt', 'bash'])
            program, argv, environment = execute.call_args.args
            self.assertEqual(program, '/usr/bin/apt-get')
            self.assertIn('--print-uris', argv)
            self.assertIn('--download-only', argv)
            self.assertIn('--no-remove', argv)
            self.assertEqual(set(environment), {'PATH', 'LANG', 'LC_ALL', 'APT_CONFIG', 'HOME'})
            receipt = json.loads(evidence.read_bytes())
            self.assertTrue(receipt['different_namespace'])
            self.assertFalse(receipt['installation'])

    def test_bind_refuses_empty_duplicate_extra_and_timestamp_drift_imports(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            materials = {'schema': 1, 'arch': 'amd64', 'source_epoch': REQUEST['source_epoch'],
                         'repositories': [MAIN, SECURITY], 'keyring': {}, 'packages': [], 'sources': []}
            raw = encoded(materials)
            (root / 'materials.json').write_bytes(raw)
            normal = [{'archive': 'debian', 'selected_timestamp': MAIN['timestamp']},
                      {'archive': 'debian-security', 'selected_timestamp': SECURITY['timestamp']}]
            for observations in ([], [normal[0], normal[0]],
                                  [dict(normal[0], selected_timestamp='20261002T000000Z'), normal[1]],
                                  [normal[0], dict(normal[1], archive='unrelated')]):
                receipt = {'kind': COLLECT.COLLECTION_KIND, 'complete': True, 'source_authenticated': True,
                           'materials_sha256': identity(raw)['sha256'], 'builder_approved': False, 'imports': observations}
                (root / 'collection.json').write_bytes(encoded(receipt))
                with self.subTest(observations=observations), \
                     mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                     mock.patch.object(BUILD, 'validate_materials'), \
                     mock.patch.object(BUILD, 'verify_authenticated_sources') as verify:
                    with self.assertRaisesRegex(ValueError, 'binding import'):
                        COLLECT.bind(root, root / 'absent-candidate', root / 'bound', 10)
                    verify.assert_not_called()

    def test_bind_rejects_changed_import_byte_and_failed_collection(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            cache = root / 'input-cache'
            cache.mkdir(mode=0o700)
            materials = {'schema': 1, 'arch': 'amd64', 'source_epoch': REQUEST['source_epoch'],
                         'repositories': [MAIN, SECURITY], 'keyring': {}, 'packages': [], 'sources': []}
            raw = encoded(materials)
            (root / 'materials.json').write_bytes(raw)
            observations = []
            for repo in (MAIN, SECURITY):
                content = encoded({'result': {repo['archive']: [repo['timestamp']]}})
                descriptor = dict(identity(content), blob=repo['id'] + '.json')
                (cache / descriptor['blob']).write_bytes(content)
                observations.append({'archive': repo['archive'], 'selected_timestamp': repo['timestamp'], 'response': descriptor})
            receipt = {'kind': COLLECT.COLLECTION_KIND, 'complete': True, 'source_authenticated': True,
                       'materials_sha256': identity(raw)['sha256'], 'builder_approved': False, 'imports': observations}
            (root / 'collection.json').write_bytes(encoded(receipt))
            (cache / 'main.json').write_bytes(b'x' + (cache / 'main.json').read_bytes()[1:])
            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(BUILD, 'validate_materials'), \
                 mock.patch.object(BUILD, 'verify_authenticated_sources') as verify:
                with self.assertRaisesRegex(ValueError, 'checksum'):
                    COLLECT.bind(root, root / 'candidate', root / 'bound', 10)
                verify.assert_not_called()
            (root / 'failure.json').write_bytes(encoded({'complete': False}))
            with self.assertRaisesRegex(ValueError, 'incomplete collection'):
                COLLECT.bind(root, root / 'candidate', root / 'bound', 10)

    def test_collect_unbound_emits_materials_without_fabricating_builder_lock(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            request = root / 'request.json'
            request.write_bytes(encoded(REQUEST))
            keyring = root / 'keyring.gpg'
            keyring.write_bytes(b'owned fixture, not a real key')
            provenance = root / 'provenance.json'
            provenance.write_bytes(encoded(dict(identity(keyring.read_bytes()), schema=1,
                source='owned fixture channel', reviewer='fixture reviewer', obtained_at='fixture time')))
            binary_rows = [package(value) for value in sorted(set(BUILD.TOOL_PACKAGES.values()) | {'apt', 'base-files'})]
            binary_rows[0]['Essential'] = 'yes'
            parsed = {'main': {'Packages': binary_rows, 'Sources': [source()]},
                      'security': {'Packages': [], 'Sources': []}}
            payload_attempts = []

            def fake_obtain(collector, url, limit, expected=None, suffix=''):
                if '/mr/timestamp/' in url:
                    archive = 'debian-security' if 'archive=debian-security' in url else 'debian'
                    chosen = SECURITY['timestamp'] if archive == 'debian-security' else MAIN['timestamp']
                    body = encoded({'result': {archive: [chosen]}})
                else:
                    if suffix in ('.deb', '.source'):
                        payload_attempts.append(url)
                    body = b'owned-binary' if suffix == '.deb' else b'owned-source'
                value = dict(identity(body), blob=identity(body)['sha256'] + suffix)
                path = collector.cache / value['blob']
                if not path.exists():
                    path.write_bytes(body)
                return value

            def fake_auth(collector, repo, ring):
                value = dict(repo, inrelease=dict(identity(b'owned'), blob='owned-release'), indices=[])
                for kind, path in (('Packages', 'main/binary-amd64/Packages.xz'), ('Sources', 'main/source/Sources.xz')):
                    value['indices'].append(dict(identity(b'owned'), blob='owned-' + kind + '.xz', kind=kind, path=path))
                return value, parsed[repo['id']]

            def fake_solve(collector, repositories, indices, ring):
                rows = []
                for row in binary_rows:
                    rows.append({'repository': 'main', 'name': row['Package'], 'version': row['Version'],
                        'architecture': row['Architecture'], 'filename': row['Filename'], 'size': int(row['Size']),
                        'sha256': row['SHA256'], 'source_name': 'fixture', 'source_version': '1'})
                (collector.output / 'solver').mkdir(mode=0o700)
                (collector.output / 'solver/selection.json').write_bytes(encoded({'fixture': True}))
                return rows, {'fixture': True, 'capacity': fixture_solver_capacity()}

            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(COLLECT.Collector, 'obtain', fake_obtain), \
                 mock.patch.object(COLLECT, 'authenticate_metadata', fake_auth), \
                 mock.patch.object(COLLECT, 'solve', fake_solve), \
                 mock.patch.object(COLLECT, 'record_collection_tools', return_value={'fixture': True}), \
                 mock.patch.object(COLLECT, 'free_disk'), \
                 mock.patch.object(BUILD, 'verify_authenticated_sources', return_value=({}, [])) as verify, \
                 mock.patch.object(BUILD, 'prepare') as prepare, mock.patch.object(BUILD, 'build') as build:
                result = COLLECT.collect(request, keyring, provenance, None, root / 'result', 4 * 1024**2, 30)
                oversized = COLLECT.source_closure([{'source_name': 'fixture', 'source_version': '1'}], [MAIN, SECURITY], parsed)
                oversized[0]['files'][0].update(size=4 * 1024**2, sha256=identity(b'owned oversized source declaration')['sha256'])
                payload_attempts.clear()
                with mock.patch.object(COLLECT, 'source_closure', return_value=oversized):
                    with self.assertRaisesRegex(ValueError, 'factory byte budget exceeded'):
                        COLLECT.collect(request, keyring, provenance, None, root / 'refused', 4 * 1024**2, 30)
                self.assertEqual(payload_attempts, [])
                refused = root / 'refused'
                plan_raw = (refused / 'capacity-plan.json').read_bytes()
                plan = json.loads(plan_raw)
                failure = json.loads((refused / 'failure.json').read_bytes())
                self.assertEqual(plan['source']['reference_bytes'], 4 * 1024**2 + 12)
                self.assertEqual(plan['source']['file_references'], 2)
                self.assertEqual(plan['source']['unique_blobs'], 2)
                self.assertEqual(plan['admission_at_observation']['rejection_reasons'], ['factory_total_byte_admission'])
                self.assertFalse(plan['admission_at_observation']['allowed'])
                self.assertEqual(failure['failure_stage'], 'before_binary_and_source_payload_download')
                self.assertEqual(failure['capacity_plan'], dict(identity(plan_raw), path='capacity-plan.json'))
                self.assertFalse(failure['complete'])
                for filename in ('materials.json', 'inputs-lock.json', 'collection.json', 'unbound-inputs.json'):
                    self.assertFalse((refused / filename).exists())
                failed_worker = COLLECT.fetch_error(ValueError('official download did not return HTTP 200'),
                    'https://snapshot.debian.org/archive/debian/fixture', time.monotonic(),
                    {'response_received': True, 'http_status': 206, 'failure_stage': 'response_status_validation',
                     'final_url': 'https://snapshot.debian.org/archive/debian/fixture', 'response_content_length': '7'})

                def fail_payload(collector, url, limit, expected=None, suffix=''):
                    if suffix == '.deb':
                        collector.downloads.append(failed_worker)
                        raise ValueError('owned worker subprocess failed')
                    return fake_obtain(collector, url, limit, expected, suffix)

                with mock.patch.object(COLLECT.Collector, 'obtain', fail_payload):
                    with self.assertRaisesRegex(ValueError, 'owned worker subprocess failed'):
                        COLLECT.collect(request, keyring, provenance, None, root / 'worker-failed', 4 * 1024**2, 30)
                failed = json.loads((root / 'worker-failed/failure.json').read_bytes())
                self.assertEqual(failed['failed_download'], failed_worker)
                self.assertIn(failed_worker, failed['received_objects'])
                self.assertFalse(failed['complete'])
                self.assertEqual(failed['failure_stage'], 'binary_and_source_payload_download')
                self.assertFalse((root / 'worker-failed/materials.json').exists())
                self.assertFalse((root / 'worker-failed/collection.json').exists())
                candidate = {'image_sha256': identity(b'owned image fixture, not an approval')['sha256'], 'arch': 'amd64',
                             'tools': [dict(identity(b'owned tool'), name=key, path=value, version='owned fixture')
                                       for key, value in sorted(BUILD.TOOL_PATHS.items())]}
                candidate_path = root / 'publication-candidate.json'
                candidate_path.write_bytes(encoded(candidate))
                original_success, original_publish = COLLECT.publish_success, COLLECT.publish
                for bound in (False, True):
                    for fault in ('tools', 'marker', 'collection', 'failure_receipt', 'existing_collection'):
                        with self.subTest(bound=bound, fault=fault), contextlib.ExitStack() as stack:
                            target = root / ('publication-' + str(bound) + '-' + fault)
                            if bound:
                                stack.enter_context(mock.patch.object(COLLECT, 'verify_candidate', return_value=candidate))

                            def failed_tools(deadline):
                                if fault == 'existing_collection':
                                    (target / 'collection.json').write_bytes(b'preexisting publication must remain')
                                    return {'fixture': True}
                                raise ValueError('owned final collection failure')

                            if fault in ('tools', 'failure_receipt', 'existing_collection'):
                                stack.enter_context(mock.patch.object(COLLECT, 'record_collection_tools', side_effect=failed_tools))
                            else:
                                marker = 'inputs-lock.json' if bound else 'unbound-inputs.json'

                                def failed_publication(path, content, limit, owned):
                                    original_success(path, content, limit, owned)
                                    if path.name == (marker if fault == 'marker' else 'collection.json'):
                                        raise ValueError('owned final collection failure')

                                stack.enter_context(mock.patch.object(COLLECT, 'publish_success', side_effect=failed_publication))
                            if fault == 'failure_receipt':
                                def failed_receipt(path, content, limit):
                                    if path.name == 'failure.json':
                                        raise OSError('owned failure receipt write refusal')
                                    return original_publish(path, content, limit)

                                stack.enter_context(mock.patch.object(COLLECT, 'publish', side_effect=failed_receipt))
                            expected_error = FileExistsError if fault == 'existing_collection' else ValueError
                            with self.assertRaises(expected_error) as caught:
                                COLLECT.collect(request, keyring, provenance, candidate_path if bound else None,
                                                target, 4 * 1024**2, 30)
                            self.assertTrue(target.exists())
                            self.assertTrue(any((target / 'input-cache').iterdir()))
                            for filename in ('materials.json', 'inputs-lock.json', 'unbound-inputs.json'):
                                self.assertFalse((target / filename).exists())
                            if fault == 'existing_collection':
                                self.assertEqual((target / 'collection.json').read_bytes(), b'preexisting publication must remain')
                            else:
                                self.assertFalse((target / 'collection.json').exists())
                            if fault == 'failure_receipt':
                                self.assertFalse((target / 'failure.json').exists())
                                self.assertTrue(any('directory retained' in note for note in caught.exception.__notes__))
                            else:
                                failure = json.loads((target / 'failure.json').read_bytes())
                                self.assertFalse(failure['complete'])
                                self.assertTrue(failure['success_publication_cleanup'])
                                self.assertTrue(all(row['removed'] for row in failure['success_publication_cleanup']))
                                with self.assertRaisesRegex(ValueError, 'incomplete collection'):
                                    COLLECT.bind(target, candidate_path, root / ('unexpected-bind-' + target.name), 30)
            output = root / 'result'
            self.assertTrue((output / 'materials.json').is_file())
            self.assertTrue((output / 'unbound-inputs.json').is_file())
            self.assertFalse((output / 'inputs-lock.json').exists())
            self.assertIsNone(result['builder'])
            self.assertFalse(result['lock_ready'])
            self.assertFalse(result['builder_approved'])
            self.assertFalse(result['full_ready'])
            self.assertFalse(result['reproducibility_verified'])
            self.assertNotIn('builder', verify.call_args.args[0])
            prepare.assert_not_called()
            build.assert_not_called()
            candidate = {'image_sha256': identity(b'owned image fixture, not an approval')['sha256'], 'arch': 'amd64',
                         'tools': [dict(identity(b'owned tool'), name=key, path=value, version='owned fixture')
                                   for key, value in sorted(BUILD.TOOL_PATHS.items())]}
            candidate_path = root / 'candidate.json'
            candidate_path.write_bytes(encoded(candidate))
            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(COLLECT, 'verify_candidate', return_value=candidate), \
                 mock.patch.object(BUILD, 'verify_authenticated_sources', return_value=({}, [])), \
                 mock.patch.object(BUILD, 'verify_tools') as approve, \
                 mock.patch.object(BUILD, 'prepare') as prepare:
                binding = COLLECT.bind(output, candidate_path, root / 'bound', 30)
            lock = json.loads((root / 'bound/inputs-lock.json').read_bytes())
            BUILD.validate_lock(lock)
            self.assertTrue(binding['lock_ready'])
            self.assertFalse(binding['builder_approved'])
            self.assertFalse(binding['runtime_image_identity_verified'])
            self.assertFalse(binding['full_ready'])
            approve.assert_not_called()
            prepare.assert_not_called()
            replaced = root / 'replaced-binding'
            preserved = root / 'retained-owned-binding'

            def replace_binding_output(path, content, limit):
                original_publish(path, content, limit)
                if path.name == 'binding.json':
                    replaced.rename(preserved)
                    replaced.mkdir(mode=0o700)
                    (replaced / 'foreign').write_bytes(b'foreign replacement must remain')
                    raise ValueError('owned binding publication failure')

            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(COLLECT, 'verify_candidate', return_value=candidate), \
                 mock.patch.object(BUILD, 'verify_authenticated_sources', return_value=({}, [])), \
                 mock.patch.object(COLLECT, 'publish', side_effect=replace_binding_output):
                with self.assertRaisesRegex(ValueError, 'owned binding publication failure') as caught:
                    COLLECT.bind(output, candidate_path, replaced, 30)
            self.assertEqual((replaced / 'foreign').read_bytes(), b'foreign replacement must remain')
            self.assertTrue((preserved / 'binding.json').exists())
            self.assertTrue(any('cleanup failed' in note for note in caught.exception.__notes__))

            def cancel_binding_output(path, content, limit):
                original_publish(path, content, limit)
                if path.name == 'binding.json':
                    raise SystemExit(143)

            cancelled = root / 'cancelled-binding'
            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(COLLECT, 'verify_candidate', return_value=candidate), \
                 mock.patch.object(BUILD, 'verify_authenticated_sources', return_value=({}, [])), \
                 mock.patch.object(COLLECT, 'publish', side_effect=cancel_binding_output):
                with self.assertRaises(SystemExit) as caught:
                    COLLECT.bind(output, candidate_path, cancelled, 30)
            self.assertEqual(caught.exception.code, 143)
            self.assertFalse(cancelled.exists())


@unittest.skipUnless(sys.platform == 'linux' and hasattr(os, 'waitid'), 'owned worker lifecycle requires Linux waitid')
class OwnedWorkers(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.arrived = threading.Event()
        self.release = threading.Event()
        outer = self

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                outer.arrived.set()
                if self.path in ('/403', '/429'):
                    self.send_error(int(self.path[1:]))
                    return
                fixed = {'/206': (206, '7', b'partial'), '/invalid-length': (200, 'not-a-number', b'owned'),
                         '/short-length': (200, '10', b'abc'), '/empty': (200, '0', b''),
                         '/identity-mismatch': (200, '12', b'owned-binary')}
                if self.path in fixed:
                    status, advertised, body = fixed[self.path]
                    self.send_response(status)
                    self.send_header('Content-Length', advertised)
                    self.send_header('Set-Cookie', 'owned-secret')
                    self.end_headers()
                    try:
                        self.wfile.write(body)
                        self.wfile.flush()
                    except (BrokenPipeError, ConnectionResetError):
                        pass
                    return
                self.send_response(200)
                self.send_header('Transfer-Encoding', 'chunked')
                self.end_headers()
                if self.path == '/pause':
                    outer.release.wait(5)
                try:
                    for _ in range(64):
                        body = b'x' * 65536
                        self.wfile.write(b'10000\r\n' + body + b'\r\n')
                        self.wfile.flush()
                    self.wfile.write(b'0\r\n\r\n')
                except (BrokenPipeError, ConnectionResetError):
                    pass

            def log_message(self, *args):
                pass

        self.server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.url = 'http://127.0.0.1:' + str(self.server.server_address[1])
        self.worker = self.root / 'owned-worker.py'
        # Only this private fixture admits its owned loopback server. Production
        # URL validation is unchanged and has no public test-mode switch.
        self.worker.write_text('import importlib.util\nimport sys\nfrom pathlib import Path\n'
            + 's=importlib.util.spec_from_file_location("owned_collect",' + repr(str(COLLECT.HERE)) + ')\n'
            + 'm=importlib.util.module_from_spec(s);s.loader.exec_module(m)\n'
            + 'def owned_url(value):\n'
            + '    m.require(value.startswith(' + repr(self.url + '/') + '),"foreign fixture URL");return value\n'
            + 'm.allowed_url=owned_url\n'
            + 'with m.BUILD.cli_signals():\n    m.main()\n')

    def tearDown(self):
        self.release.set()
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(5)
        self.temp.cleanup()

    def run_owned(self, route, limit=1024, seconds=5, expected=None):
        output = self.root / ('output-' + route.strip('/'))
        output.mkdir(mode=0o700)
        instance = COLLECT.Collector(output, 16 * 1024**2, BUILD.Deadline(seconds))
        processes = []
        original = subprocess.Popen

        def remember(*args, **kwargs):
            process = original(*args, **kwargs)
            processes.append(process)
            return process

        with mock.patch.object(COLLECT, 'HERE', self.worker), \
             mock.patch.object(COLLECT, 'allowed_url', side_effect=lambda value: value), \
             mock.patch.object(COLLECT, 'free_disk'), \
             mock.patch.object(subprocess, 'Popen', side_effect=remember):
            with self.assertRaises(ValueError):
                instance.obtain(self.url + route, limit, expected)
        self.assertEqual(len(processes), 1)
        for process in processes:
            self.assertIsNotNone(process.returncode)
            with self.assertRaises(ProcessLookupError):
                os.kill(process.pid, 0)
        self.assertEqual(list(output.glob('download-*')), [])
        self.assertEqual(list(instance.cache.iterdir()), [])
        return instance.downloads[-1]

    def test_real_chunked_body_is_bounded_and_producer_reaped(self):
        result = self.run_owned('/chunked')
        self.assertEqual(result['category'], 'response_too_large')
        self.assertFalse(result['complete'])

    def test_real_http_403_and_429_are_single_attempt_failures(self):
        for route in ('/403', '/429'):
            with self.subTest(route=route):
                result = self.run_owned(route)
                self.assertEqual(result['http_status'], int(route[1:]))
                self.assertEqual(result['category'], 'http_' + route[1:])
                self.assertTrue(result['response_received'])
                self.assertEqual(result['error_type'], 'HTTPError')
                self.assertEqual(result['failure_stage'], 'request_open')

    def test_real_non200_response_keeps_status_before_validation(self):
        result = self.run_owned('/206')
        self.assertEqual(result['category'], 'response_invalid')
        self.assertEqual(result['http_status'], 206)
        self.assertTrue(result['response_received'])
        self.assertEqual(result['response_content_length'], '7')
        self.assertEqual(result['final_url'], self.url + '/206')
        self.assertEqual(result['failure_stage'], 'response_status_validation')
        self.assertEqual(result['error_type'], 'ValueError')
        self.assertEqual(result['error_message'], 'official download did not return HTTP 200')
        output = self.root / 'output-206'
        evidence = result['failure_evidence']
        self.assertTrue(evidence['complete'])
        original = json.loads((output / evidence['worker_receipt']['path']).read_bytes())
        self.assertEqual(original['http_status'], 206)
        self.assertEqual(original['error_message'], result['error_message'])
        headers = (output / evidence['response_headers']['path']).read_bytes()
        self.assertNotIn(b'owned-secret', headers)

    def test_real_invalid_short_content_length_and_empty_body_keep_context(self):
        scenarios = (('/invalid-length', 'not-a-number', 'content_length_validation', 'advertised download exceeds budget'),
                     ('/short-length', '10', 'content_length_verification', 'download differs from Content-Length'),
                     ('/empty', '0', 'body_nonempty_validation', 'empty official download'))
        for route, advertised, phase, message in scenarios:
            with self.subTest(route=route):
                result = self.run_owned(route)
                self.assertEqual(result['category'], 'response_invalid')
                self.assertEqual(result['http_status'], 200)
                self.assertTrue(result['response_received'])
                self.assertEqual(result['response_content_length'], advertised)
                self.assertEqual(result['final_url'], self.url + route)
                self.assertEqual(result['failure_stage'], phase)
                self.assertEqual(result['error_type'], 'ValueError')
                self.assertEqual(result['error_message'], message)
                if route == '/invalid-length':
                    self.assertEqual(result['content_length_validation_issue'], 'not_decimal')
                elif route == '/short-length':
                    self.assertEqual(result['content_length_validation_issue'], 'declared_length_mismatch')
                self.assertFalse(result['complete'])
                self.assertEqual(result['response_bytes_written'], 3 if route == '/short-length' else 0)

    def test_real_signed_identity_mismatch_retains_diagnostics_without_caching_body(self):
        expected = dict(identity(b'owned-binary'), sha256=identity(b'different fixture bytes')['sha256'])
        result = self.run_owned('/identity-mismatch', expected=expected)
        self.assertEqual(result['category'], 'signed_identity_mismatch')
        self.assertEqual(result['failure_stage'], 'signed_payload_validation')
        self.assertEqual(result['error_origin'], 'collector')
        self.assertEqual(result['http_status'], 200)
        self.assertEqual(result['expected_identity'], expected)
        self.assertEqual(result['actual_identity'], identity(b'owned-binary'))
        output = self.root / 'output-identity-mismatch'
        evidence = result['failure_evidence']
        self.assertTrue(evidence['complete'])
        self.assertFalse(evidence['body_retained'])
        original = json.loads((output / evidence['worker_receipt']['path']).read_bytes())
        self.assertTrue(original['complete'])
        self.assertEqual(original['size'], 12)
        headers = (output / evidence['response_headers']['path']).read_bytes()
        self.assertNotIn(b'owned-secret', headers)
        self.assertFalse((output / evidence['directory'] / 'body').exists())
        self.assertEqual(list((output / 'input-cache').iterdir()), [])

    def test_real_paused_body_hits_deadline_and_reaps_producer(self):
        result = self.run_owned('/pause', seconds=1)
        self.assertTrue(self.arrived.is_set())
        self.assertEqual(result['category'], 'timeout')
        self.assertIsNone(result['http_status'])
        self.assertEqual(result['parent_failure']['error_type'], 'ValueError')
        self.assertEqual(result['parent_failure']['error_message'], 'overall operation deadline exceeded')
        self.assertTrue(result['failure_evidence']['complete'])
        self.assertIsNone(result['failure_evidence']['worker_receipt'])
        self.assertIsNotNone(result['failure_evidence']['response_headers'])

    def test_parent_term_and_hup_reap_only_the_owned_download_worker(self):
        for number in (signal.SIGTERM, signal.SIGHUP):
            with self.subTest(number=number):
                self.arrived.clear()
                self.release.clear()
                attempt = self.root / ('signal-' + str(number))
                attempt.mkdir(mode=0o700)
                script = attempt / 'parent.py'
                pidfile, finished = attempt / 'worker.pid', attempt / 'finished'
                script.write_text('import importlib.util\nfrom pathlib import Path\nimport subprocess\n'
                    + 's=importlib.util.spec_from_file_location("owned_collect",' + repr(str(COLLECT.HERE)) + ')\n'
                    + 'm=importlib.util.module_from_spec(s);s.loader.exec_module(m)\n'
                    + 'm.HERE=Path(' + repr(str(self.worker)) + ')\n'
                    + 'm.allowed_url=lambda value:value\n'
                    + 'popen=subprocess.Popen\n'
                    + 'def remember(*a,**kw):\n'
                    + '    p=popen(*a,**kw);Path(' + repr(str(pidfile)) + ').write_text(str(p.pid));return p\n'
                    + 'subprocess.Popen=remember\n'
                    + 'try:\n'
                    + '    with m.BUILD.cli_signals():\n'
                    + '        c=m.Collector(Path(' + repr(str(attempt)) + '),16*1024**2,m.BUILD.Deadline(10))\n'
                    + '        c.obtain(' + repr(self.url + '/pause') + ',1024)\n'
                    + 'finally:\n'
                    + '    if "c" in locals() and c.downloads:\n'
                    + '        Path(' + repr(str(attempt / 'parent-failure.json')) + ').write_bytes(m.BUILD.canonical(c.downloads[-1]))\n'
                    + '    Path(' + repr(str(finished)) + ').write_text("parent finally reached")\n')
                parent = subprocess.Popen([sys.executable, str(script)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                try:
                    self.assertTrue(self.arrived.wait(5), 'owned worker never reached the loopback server')
                    self.assertTrue(pidfile.is_file())
                    worker_pid = int(pidfile.read_text())
                    parent.send_signal(number)
                    self.assertEqual(parent.wait(timeout=5), 128 + number)
                    self.assertTrue(finished.is_file())
                    with self.assertRaises(ProcessLookupError):
                        os.kill(worker_pid, 0)
                    self.assertEqual(list(attempt.glob('download-*')), [])
                    report = json.loads((attempt / 'parent-failure.json').read_bytes())
                    self.assertEqual(report['category'], 'cancelled')
                    self.assertEqual(report['parent_failure']['error_type'], 'SystemExit')
                    self.assertTrue(report['failure_evidence']['complete'])
                    self.assertFalse(report['failure_evidence']['body_retained'])
                finally:
                    self.release.set()
                    if parent.poll() is None:
                        parent.kill()
                        parent.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
