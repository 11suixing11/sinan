#!/usr/bin/env python3
"""Exercise pinned packaging and the real curl shim using private inert sources."""
import base64
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest import mock

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
PLUGIN = ROOT / 'plugins/nodequality'
VERSION = 'a92fca6c0067df29ddd03fdc2fee6f3000f64545-r6'


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


helper = module('pinned_sources', PLUGIN / 'source-helper.py')


class SourceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='sinan-pinned-sources-')
        self.root = Path(self.temporary.name)
        self.sources = self.root / 'inputs'
        self.sources.mkdir(mode=0o700)
        self.lock = json.loads((PLUGIN / 'source-lock.json').read_bytes())
        self.executed = self.root / 'source-executed'
        for row in self.lock['files']:
            # This is shell code with an observable private side effect. Neither
            # packaging nor serving is allowed to execute even this inert code.
            content = ("#!/bin/sh\nprintf '%s' '" + row['name'] + "' > \"$NQ_SOURCE_EXECUTED\"\n").encode()
            row['sha256'] = hashlib.sha256(content).hexdigest()
            row['size'] = len(content)
            (self.sources / row['name']).write_bytes(content)
        self.lock_path = self.root / 'source-lock.json'
        self.lock_path.write_text(json.dumps(self.lock))
        self.bundle_path = self.root / 'pinned-chain.json'
        self.bundle_path.write_bytes(helper.pack(self.lock, self.sources))
        self.materialized = self.root / 'materialized'
        helper.materialize(helper.decode(self.bundle_path.read_bytes()), self.materialized)
        self.called = self.root / 'network-called'
        self.real_curl = self.root / 'real-curl'
        self.real_curl.write_text('#!/bin/sh\nprintf called > "' + str(self.called) + '"\nexit 0\n')
        self.real_curl.chmod(0o700)
        self.environment = dict(os.environ, SINAN_REAL_CURL=str(self.real_curl),
                                SINAN_CHAIN_HELPER=str(PLUGIN / 'source-helper.py'),
                                SINAN_CHAIN_DIRECTORY=str(self.materialized), NQ_SOURCE_EXECUTED=str(self.executed))

    def tearDown(self):
        self.temporary.cleanup()

    def shim(self, *args):
        return subprocess.run(['bash', str(PLUGIN / 'curl-shim.sh'), *args], env=self.environment,
                              capture_output=True, timeout=3)

    def test_all_five_actual_loader_requests_return_exact_original_bytes_without_execution_or_network(self):
        for url, name in helper.ALIASES.items():
            flag = '-sL' if name == 'swap.sh' else '-Ls'
            with self.subTest(name=name):
                result = self.shim(flag, url)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, (self.sources / name).read_bytes())
        self.assertFalse(self.called.exists())
        self.assertFalse(self.executed.exists())
        for row in self.lock['files']:
            self.assertEqual((self.materialized / row['name']).read_bytes(), (self.sources / row['name']).read_bytes())
        self.assertEqual(self.materialized.stat().st_mode & 0o777, 0o700)

    def test_modified_script_or_manifest_is_rejected_before_any_online_fallback(self):
        target = self.materialized / 'hardware.sh'
        target.write_bytes(target.read_bytes()[:-1] + b'!')
        result = self.shim('-Ls', 'https://Hardware.Check.Place')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'checksum or size mismatch', result.stderr)
        self.assertEqual(result.stdout, b'')
        self.assertFalse(self.called.exists())
        self.assertFalse(self.executed.exists())

    def test_unknown_urls_and_option_shapes_never_fall_back_online(self):
        for args in [('-Ls', 'https://Hardware.Check.Place/'),
                     ('-Ls', 'https://Hardware.Check.Place?main=1'),
                     ('-Ls', 'https://raw.githubusercontent.com/xykt/HardwareQuality/main/hardware.sh'),
                     ('-Ls', 'https://fixture.invalid/unrecognized.sh'),
                     ('--location', 'https://IP.Check.Place'),
                     ('-Ls', 'https://IP.Check.Place', '--user-agent', 'fixture'),
                     ('-X', 'POST', 'https://fixture.invalid/record'),
                     ('-L#o', 'BenchOs.tar.gz', 'https://fixture.invalid/BenchOs.tar.gz')]:
            with self.subTest(args=args):
                result = self.shim(*args)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b'online fallback is forbidden', result.stderr)
                self.assertEqual(result.stdout, b'')
        self.assertFalse(self.called.exists())

    def test_only_the_two_existing_exact_rootfs_download_shapes_remain_separate(self):
        for asset in ('BenchOs.tar.gz', 'BenchOs-arm.tar.gz'):
            url = 'https://github.com/LloydAsp/NodeQuality/releases/download/v0.0.2/' + asset
            self.assertEqual(self.shim('-L#o', 'BenchOs.tar.gz', url).returncode, 0)
            self.assertTrue(self.called.exists())
            self.called.unlink()
            self.assertNotEqual(self.shim('-Ls', url).returncode, 0)
            self.assertFalse(self.called.exists())

    def test_real_runner_path_routes_all_loader_calls_to_the_signed_bundle(self):
        workspace = self.root / 'workspace'
        workspace.mkdir(mode=0o700)
        binaries = self.root / 'bin'
        binaries.mkdir()
        for name in ('curl', 'chroot', 'mount', 'umount', 'mountpoint', 'base64', 'tar'):
            shutil.copy2(self.real_curl, binaries / name)
        uname = binaries / 'uname'
        uname.write_text("#!/bin/sh\nprintf 'Linux\\n'\n")
        uname.chmod(0o700)
        entry = '#!/usr/bin/env bash\nset -euo pipefail\n'
        for url, name in helper.ALIASES.items():
            flag = '-sL' if name == 'swap.sh' else '-Ls'
            entry += 'curl ' + flag + ' ' + url + ' > "$SINAN_REPORT_WORKSPACE/captured-' + name + '"\n'
        entry += 'exit 7\n'
        runner = (PLUGIN / 'runner.sh.tmpl').read_text()
        for guard in ("[[ $EUID == 0 ]] || die 'diagnostics require root'",
                      "[[ ${BASH_VERSINFO[0]} -ge 4 ]] || die 'diagnostics require Bash >= 4'"):
            self.assertEqual(runner.count(guard), 1)
            runner = runner.replace(guard, ':')
        for name, payload in [('NODEQUALITY_SOURCE', entry), ('NODEQUALITY_LICENSE', '# Synthetic license\n'),
                              ('PINNED_CHAIN', self.bundle_path.read_text())] + [
                              (name, (PLUGIN / path).read_text()) for name, path in [
                                  ('SOURCE_HELPER', 'source-helper.py'), ('REPORT_HELPER', 'report.py'),
                                  ('EXIT_OBSERVER', 'exit-observer.sh'), ('DAILY_HELPER', 'daily.py'),
                                  ('CURL_SHIM', 'curl-shim.sh'), ('CHROOT_SHIM', 'chroot-shim.sh')]]:
            runner = runner.replace('@' + name + '@\n', payload)
        path = self.root / 'nodequality'
        path.write_text(runner)
        result = subprocess.run(['bash', str(path), '--workspace', str(workspace), '--mode', 'full', '--ip-version', 'ipv4'],
                                env=dict(self.environment, PATH=str(binaries) + ':' + os.environ['PATH']),
                                capture_output=True, timeout=6)
        self.assertNotEqual(result.returncode, 0, 'synthetic sources produce no benchmark report')
        for name in helper.ALIASES.values():
            self.assertTrue((workspace / ('captured-' + name)).exists(), result.stderr.decode())
            self.assertEqual((workspace / ('captured-' + name)).read_bytes(), (self.sources / name).read_bytes())
        self.assertFalse(self.called.exists())
        self.assertFalse(self.executed.exists())
        self.assertFalse((workspace / '.runner').exists())
        self.assertFalse((workspace / '.runner.lock').exists())

    def test_download_manifest_requires_complete_fixed_source_and_license_identities(self):
        result = subprocess.run([sys.executable, str(PLUGIN / 'source-helper.py'), 'downloads', str(self.lock_path)],
                                capture_output=True, check=True, timeout=3)
        rows = result.stdout.decode().splitlines()
        self.assertEqual(len(rows), 10)
        for row in self.lock['files']:
            self.assertIn(row['name'] + '\thttps://raw.githubusercontent.com/' + row['repository']
                          + '/' + row['commit'] + '/' + row['path'], rows)
        mutations = [lambda x: x.update(schema=True), lambda x: x['files'].pop(),
                     lambda x: x['files'][0].update(commit='main'),
                     lambda x: x['files'][0].update(license_file='unknown'),
                     lambda x: x['files'][1].update(commit='0' * 40),
                     lambda x: x['files'][0].update(path='../NodeQuality.sh'),
                     lambda x: x['files'].append(copy.deepcopy(x['files'][0]))]
        for mutate in mutations:
            lock = copy.deepcopy(self.lock)
            mutate(lock)
            with self.assertRaises(ValueError):
                helper.pack(lock, self.sources)
        with self.assertRaises(ValueError):
            helper.decode(b'{"schema":1,"schema":1,"files":[]}')
        self.assertFalse(self.executed.exists())

    def test_pack_rejects_modified_or_missing_license_without_partial_materialization(self):
        path = self.sources / 'LICENSE.hardware'
        path.write_bytes(path.read_bytes()[:-1] + b'!')
        with self.assertRaisesRegex(ValueError, 'checksum'):
            helper.pack(self.lock, self.sources)
        bundle = helper.decode(self.bundle_path.read_bytes())
        bundle['files']['LICENSE.net'] = base64.b64encode(b'one byte changed').decode()
        destination = self.root / 'not-created'
        with self.assertRaisesRegex(ValueError, 'checksum'):
            helper.materialize(bundle, destination)
        self.assertFalse(destination.exists())
        path.unlink()
        with self.assertRaises(OSError):
            helper.pack(self.lock, self.sources)

    @unittest.skipUnless(hasattr(os, 'mkfifo'), 'requires POSIX FIFOs')
    def test_fifo_and_symlink_inputs_fail_without_waiting_for_a_writer(self):
        path = self.materialized / 'hardware.sh'
        path.unlink()
        os.mkfifo(path, 0o600)
        result = self.shim('-Ls', 'https://Hardware.Check.Place')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'ordinary file', result.stderr)
        path.unlink()
        path.symlink_to(self.sources / 'hardware.sh')
        self.assertNotEqual(self.shim('-Ls', 'https://Hardware.Check.Place').returncode, 0)
        self.assertFalse(self.called.exists())

    def build_tree(self):
        tree = self.root / 'project'
        (tree / 'tools').mkdir(parents=True)
        shutil.copytree(PLUGIN, tree / 'plugins/nodequality')
        shutil.copy2(ROOT / 'tools/build-nodequality.sh', tree / 'tools/build-nodequality.sh')
        # Only this private test tree has synthetic source hashes. The production
        # manifest remains unchanged and exposes no runtime source override.
        (tree / 'plugins/nodequality/source-lock.json').write_bytes(self.lock_path.read_bytes())
        binaries = self.root / 'bin'
        binaries.mkdir()
        fake = binaries / 'curl'
        fake.write_text('''#!/usr/bin/env python3
import json, os, pathlib, shutil, sys
args = sys.argv[1:]
index = args.index('-o')
url = args[index - 1]
lock = json.loads(pathlib.Path(os.environ['NQ_SOURCE_LOCK']).read_bytes())
matching = [row for row in lock['files'] if url == 'https://raw.githubusercontent.com/' + row['repository'] + '/' + row['commit'] + '/' + row['path']]
assert len(matching) == 1, 'unexpected online source URL'
with pathlib.Path(os.environ['NQ_DOWNLOAD_LOG']).open('a') as output:
    output.write(url + '\\n')
shutil.copyfile(pathlib.Path(os.environ['NQ_INPUTS']) / matching[0]['name'], args[index + 1])
''')
        fake.chmod(0o700)
        environment = dict(os.environ, PATH=str(binaries) + ':' + os.environ['PATH'],
                           NQ_SOURCE_LOCK=str(self.lock_path), NQ_INPUTS=str(self.sources),
                           NQ_DOWNLOAD_LOG=str(self.root / 'downloads'), NQ_SOURCE_EXECUTED=str(self.executed))
        return tree, environment

    def build(self, tree, environment, arch):
        return subprocess.run(['bash', str(tree / 'tools/build-nodequality.sh'), arch, str(self.root / 'artifacts')],
                              env=environment, capture_output=True, timeout=10)

    def test_real_builder_packages_both_architectures_immutably_without_executing_downloads(self):
        tree, environment = self.build_tree()
        output = self.root / 'artifacts/nodequality' / VERSION
        for arch in ('amd64', 'arm64'):
            result = self.build(tree, environment, arch)
            self.assertEqual(result.returncode, 0, result.stderr)
            with tarfile.open(output / arch, 'r:gz') as archive:
                self.assertEqual(archive.getnames(), ['nodequality'])
                runner = archive.extractfile('nodequality').read()
            runner_path = self.root / 'nodequality'
            runner_path.write_bytes(runner)
            for option in ('--version', '--help'):
                result = subprocess.run(['bash', str(runner_path), option], capture_output=True, check=True, timeout=3)
                self.assertTrue(result.stdout)
            self.assertFalse(self.executed.exists())
        self.assertEqual((output / 'amd64').read_bytes(), (output / 'arm64').read_bytes())
        self.assertEqual(len((self.root / 'downloads').read_text().splitlines()), 20)
        second_root = self.root / 'independent-artifacts'
        subprocess.run(['bash', str(tree / 'tools/build-nodequality.sh'), 'amd64', str(second_root)],
                       env=environment, capture_output=True, check=True, timeout=10)
        self.assertEqual((output / 'amd64').read_bytes(), (second_root / 'nodequality' / VERSION / 'amd64').read_bytes())
        prior = {name: (output / name).read_bytes() for name in ('amd64', 'arm64', 'SHA256SUMS')}
        self.assertNotEqual(self.build(tree, environment, 'amd64').returncode, 0)
        self.assertEqual(prior, {name: (output / name).read_bytes() for name in prior})
        self.assertFalse((output / '.build.lock').exists())

    def test_one_byte_source_change_blocks_new_architecture_and_preserves_existing_checksums(self):
        tree, environment = self.build_tree()
        self.assertEqual(self.build(tree, environment, 'amd64').returncode, 0)
        output = self.root / 'artifacts/nodequality' / VERSION
        prior = (output / 'SHA256SUMS').read_bytes()
        target = self.sources / 'ip.sh'
        target.write_bytes(target.read_bytes()[:-1] + b'!')
        result = self.build(tree, environment, 'arm64')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'checksum or size mismatch', result.stderr)
        self.assertFalse((output / 'arm64').exists())
        self.assertEqual((output / 'SHA256SUMS').read_bytes(), prior)
        self.assertFalse((output / '.build.lock').exists())
        self.assertFalse(self.executed.exists())

    def test_builder_rejects_a_source_commit_that_does_not_match_its_artifact_version(self):
        tree, environment = self.build_tree()
        for row in self.lock['files']:
            if row['repository'] == 'LloydAsp/NodeQuality':
                row['commit'] = '0' * 40
        self.lock_path.write_text(json.dumps(self.lock))
        (tree / 'plugins/nodequality/source-lock.json').write_bytes(self.lock_path.read_bytes())
        result = self.build(tree, environment, 'amd64')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'entrypoint source commit differs', result.stderr)
        output = self.root / 'artifacts/nodequality' / VERSION
        self.assertFalse((output / 'amd64').exists())
        self.assertFalse((output / 'SHA256SUMS').exists())
        self.assertFalse(self.executed.exists())

    @unittest.skipUnless(shutil.which('minisign'), 'requires minisign for disclosed TEST_ONLY signing')
    def test_signed_runner_covers_embedded_scripts_and_licenses_and_rejects_tampering(self):
        tree, environment = self.build_tree()
        self.assertEqual(self.build(tree, environment, 'arm64').returncode, 0)
        data = (self.root / 'artifacts/nodequality' / VERSION / 'arm64').read_bytes()
        release = module('source_release', ROOT / 'tools/release.py')
        fixture = module('source_fixture', ROOT / 'tools/ci-release-fixture.py')
        with mock.patch.dict(os.environ):
            os.environ.pop('SINAN_CI_FIXTURE_SIGNER', None)
            proof = fixture.proof('nodequality', VERSION, 'nodequality', data, 'tar.gz', 'arm64')
        bundle = self.root / 'signed'
        bundle.mkdir()
        fixture.install(bundle, proof)
        (bundle / 'install.sh').write_bytes(b'#!/bin/sh\nexit 0\n')
        entry = json.loads(proof['metadata_json'])['artifacts'][0]
        asset = bundle / entry['asset_name']
        asset.write_bytes(data)
        key = (ROOT / 'crates/protocol/tests/fixtures/TEST_ONLY.pub').read_text().splitlines()[1]
        self.assertEqual(release.verify_bundle(bundle, [key], shutil.which('minisign'))['artifacts'][0]['version'], VERSION)
        with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
            content = archive.extractfile('nodequality').read()
        # Mutate an embedded license's base64 representation inside the runner.
        bundle_text = helper.pack(self.lock, self.sources)
        self.assertIn(bundle_text, content)
        altered = content.replace(bundle_text, bundle_text.replace(b'LICENSE.net', b'LICENSE.Net', 1), 1)
        packed = io.BytesIO()
        with tarfile.open(fileobj=packed, mode='w:gz') as archive:
            member = tarfile.TarInfo('nodequality')
            member.size, member.mode = len(altered), 0o755
            archive.addfile(member, io.BytesIO(altered))
        asset.write_bytes(packed.getvalue())
        with self.assertRaisesRegex(ValueError, 'archive'):
            release.verify_bundle(bundle, [key], shutil.which('minisign'))
        checksums = bundle / 'SHA256SUMS'
        rows = checksums.read_text().splitlines()
        path = 'nodequality/' + VERSION + '/arm64'
        checksums.write_text('\n'.join(hashlib.sha256(asset.read_bytes()).hexdigest() + '  ' + path
                                       if row.endswith('  ' + path) else row for row in rows) + '\n')
        with self.assertRaisesRegex(ValueError, 'trusted key'):
            release.verify_bundle(bundle, [key], shutil.which('minisign'))


if __name__ == '__main__':
    unittest.main()
