#!/usr/bin/env python3
"""Owned collection fixtures; mocks do not certify a Debian signature or image."""

import contextlib
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
        self.assertEqual(COLLECT.fetch_error(TimeoutError(), 'owned-url', now)['category'], 'timeout')

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

            def fake_obtain(collector, url, limit, expected=None, suffix=''):
                if '/mr/timestamp/' in url:
                    archive = 'debian-security' if 'archive=debian-security' in url else 'debian'
                    chosen = SECURITY['timestamp'] if archive == 'debian-security' else MAIN['timestamp']
                    body = encoded({'result': {archive: [chosen]}})
                else:
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
                (collector.output / 'solver').mkdir()
                (collector.output / 'solver/selection.json').write_bytes(encoded({'fixture': True}))
                return rows, {'fixture': True}

            with mock.patch.object(COLLECT, 'native_arch', return_value='amd64'), \
                 mock.patch.object(COLLECT.Collector, 'obtain', fake_obtain), \
                 mock.patch.object(COLLECT, 'authenticate_metadata', fake_auth), \
                 mock.patch.object(COLLECT, 'solve', fake_solve), \
                 mock.patch.object(COLLECT, 'record_collection_tools', return_value={'fixture': True}), \
                 mock.patch.object(COLLECT, 'free_disk'), \
                 mock.patch.object(BUILD, 'verify_authenticated_sources', return_value=({}, [])) as verify, \
                 mock.patch.object(BUILD, 'prepare') as prepare, mock.patch.object(BUILD, 'build') as build:
                result = COLLECT.collect(request, keyring, provenance, None, root / 'result', 4 * 1024**2, 30)
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

    def run_owned(self, route, limit=1024, seconds=5):
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
                instance.obtain(self.url + route, limit)
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

    def test_real_paused_body_hits_deadline_and_reaps_producer(self):
        result = self.run_owned('/pause', seconds=1)
        self.assertTrue(self.arrived.is_set())
        self.assertEqual(result['category'], 'producer_failed_or_deadline')
        self.assertIsNone(result['http_status'])

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
                finally:
                    self.release.set()
                    if parent.poll() is None:
                        parent.kill()
                        parent.wait(timeout=5)


if __name__ == '__main__':
    unittest.main()
