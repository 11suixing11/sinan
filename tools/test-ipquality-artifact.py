#!/usr/bin/env python3
"""Check source-only preparation, inventory and publication guards."""
import gzip
import io
from pathlib import Path
import tarfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import ipquality_artifact as artifact


class ArtifactTests(unittest.TestCase):
    def test_signed_inventory_requires_source_license_and_every_auxiliary(self):
        for name in artifact.FILES:
            files = {entry: b'fixture' for entry in artifact.FILES - {name}}
            with self.assertRaisesRegex(ValueError, 'signed inventory'):
                artifact.validate_files(files, artifact.VERSION, 'amd64')

    def test_source_archive_refuses_duplicate_traversal_and_links(self):
        for name, kind in (('../source.py', tarfile.REGTYPE), ('source.py', tarfile.SYMTYPE)):
            output = io.BytesIO()
            with tarfile.open(fileobj=output, mode='w:gz', format=tarfile.USTAR_FORMAT) as archive:
                member = tarfile.TarInfo(name)
                member.type = kind
                member.size = 1 if kind == tarfile.REGTYPE else 0
                member.linkname = '/etc/passwd' if kind == tarfile.SYMTYPE else ''
                archive.addfile(member, io.BytesIO(b'x') if member.size else None)
            with self.assertRaisesRegex(ValueError, 'unsafe'):
                artifact.unpack(output.getvalue(), maximum=artifact.MAX_SOURCE)
        data = artifact.pack({'source.py': b'print("fixture")\n'})
        self.assertEqual(artifact.unpack(data, maximum=artifact.MAX_SOURCE), {'source.py': b'print("fixture")\n'})

    def test_small_compressed_archive_with_two_gib_limit_uses_bounded_reads(self):
        expected = {'source.py': b'x' * (2 * artifact.READ_CHUNK + 31)}
        data = artifact.pack(expected)
        self.assertLess(len(data), artifact.READ_CHUNK)
        requests = {'gzip': [], 'member': []}
        gzip_read = gzip.GzipFile.read
        member_read = tarfile.ExFileObject.read

        def bounded_read(kind, original):
            def read(source, size=-1):
                self.assertIs(type(size), int)
                self.assertGreater(size, 0)
                self.assertLessEqual(size, artifact.READ_CHUNK)
                requests[kind].append(size)
                return original(source, size)
            return read

        with patch.object(gzip.GzipFile, 'read', bounded_read('gzip', gzip_read)), \
                patch.object(tarfile.ExFileObject, 'read', bounded_read('member', member_read)):
            self.assertEqual(artifact.unpack(data, maximum=artifact.MAX_SOURCE_OFFER), expected)
        self.assertGreater(len(requests['gzip']), 2)
        self.assertGreater(len(requests['member']), 2)

    def test_expansion_limit_allows_exact_size_and_rejects_one_byte_more(self):
        expected = {'source.py': b'print("fixture")\n'}
        data = artifact.pack(expected)
        expanded = len(gzip.decompress(data))
        self.assertLess(len(data), expanded - 1)
        self.assertEqual(artifact.unpack(data, maximum=expanded), expected)
        with self.assertRaisesRegex(ValueError, 'archive expansion exceeds limit'):
            artifact.unpack(data, maximum=expanded - 1)

    def test_unpack_rejects_invalid_limits_before_opening_gzip(self):
        data = artifact.pack({'source.py': b'print("fixture")\n'})
        with patch.object(gzip, 'GzipFile') as reader:
            for maximum in (True, False, 0, -1, 1.5, '2147483648', None):
                with self.subTest(maximum=maximum), \
                        self.assertRaisesRegex(ValueError, 'invalid archive expansion limit'):
                    artifact.unpack(data, maximum=maximum)
            reader.assert_not_called()

    def test_unpack_reads_through_gzip_crc_size_and_complete_trailer(self):
        data = artifact.pack({'source.py': b'print("fixture")\n'})
        corrupt_crc = data[:-8] + bytes([data[-8] ^ 1]) + data[-7:]
        corrupt_size = data[:-4] + bytes([data[-4] ^ 1]) + data[-3:]
        for malformed in (corrupt_crc, corrupt_size, data[:-1], data + b'not a gzip trailer'):
            with self.subTest(trailer=malformed[-8:]), \
                    self.assertRaises((gzip.BadGzipFile, EOFError)):
                artifact.unpack(malformed, maximum=artifact.MAX_SOURCE_OFFER)

    def test_independent_profile_cannot_satisfy_full_hardware_factory(self):
        profile = artifact.module('sinan_ipquality_profile_fixture', artifact.ROOT / 'tools/ipquality-rootfs.py')
        node = profile.factory()
        original = artifact.module('sinan_original_nodequality_factory_fixture', artifact.ROOT / 'tools/nodequality-rootfs-build.py')
        self.assertIn('python3', node.TOOL_PACKAGES)
        self.assertNotIn('fio', node.TOOL_PACKAGES)
        self.assertNotIn('iperf3', node.TOOL_PACKAGES)
        self.assertIn('fio', original.TOOL_PACKAGES)
        self.assertNotEqual(node.PROVENANCE_KIND, original.PROVENANCE_KIND)
        self.assertTrue(original.PENDING)
        self.assertNotEqual(node.PENDING, original.PENDING)
        self.assertIsNone(original.INPUT_PROFILE)
        self.assertIsNotNone(node.INPUT_PROFILE)

    def minimal_metadata(self):
        # Shape/replay admission is exercised by test-ipquality-profile.py.
        # These inert bytes isolate the signed archive/provenance connection.
        prefix = 'usr/share/sinan-rootfs/'
        proof = {'schema': 1, 'scope': 'TEST_ONLY signed metadata propagation'}
        content = artifact.canonical(proof)
        metadata = {prefix + 'ipquality-profile.json': content,
                    prefix + 'inputs-lock.json': b'{"scope":"TEST_ONLY"}',
                    prefix + 'provenance.json': artifact.canonical({'profile_proof_sha256': artifact.digest(content)})}
        manifest = {'entries': [{'path': prefix + 'ipquality-profile.json', 'type': 'file',
                                 'sha256': artifact.digest(content), 'size': len(content)}]}
        validator = SimpleNamespace(INPUT_PROFILE=SimpleNamespace(validate_public=lambda value, lock: value),
                                    canonical=lambda value: artifact.canonical(value).rstrip(b'\n'))
        return metadata, manifest, validator

    def test_minimal_profile_proof_is_mandatory_even_before_license_claim(self):
        with self.assertRaisesRegex(ValueError, 'profile proof is absent'):
            artifact.check_minimal_profile({}, {'entries': []})
        with self.assertRaisesRegex(ValueError, 'profile proof is absent'):
            artifact.check_license_review({'schema': 1, 'reviewed': True}, {}, {'entries': []})

    def test_signed_minimal_profile_bytes_require_both_provenance_and_runtime_binding(self):
        metadata, manifest, validator = self.minimal_metadata()
        with patch.object(artifact, 'factory', return_value=validator):
            self.assertEqual(artifact.check_minimal_profile(metadata, manifest)['schema'], 1)
            for key, value in (('sha256', 'f' * 64), ('size', 0), ('type', 'symlink')):
                changed = {'entries': [dict(manifest['entries'][0], **{key: value})]}
                with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'runtime manifest'):
                    artifact.check_minimal_profile(metadata, changed)
            changed = dict(metadata)
            changed['usr/share/sinan-rootfs/provenance.json'] = artifact.canonical({'profile_proof_sha256': 'f' * 64})
            with self.assertRaisesRegex(ValueError, 'factory provenance'):
                artifact.check_minimal_profile(changed, manifest)

    def test_signed_archive_rejects_raw_private_factory_evidence(self):
        metadata, manifest, validator = self.minimal_metadata()
        for private in ('ipquality-profile-private.json', 'input-ledger.json', 'tool-evidence.json',
                        'ipquality-profile-replay-host.json'):
            changed = {'entries': manifest['entries'] + [{'path': 'usr/share/sinan-rootfs/' + private,
                                                         'type': 'file', 'sha256': 'f' * 64, 'size': 1}]}
            with self.subTest(private=private), patch.object(artifact, 'factory', return_value=validator), \
                    self.assertRaisesRegex(ValueError, 'private factory evidence'):
                artifact.check_minimal_profile(metadata, changed)

    def test_packaged_runner_embeds_exact_verifier_and_no_online_bootstrap(self):
        value = artifact.runner()
        self.assertNotIn(b"ROOTFS_SOURCE = '@ROOTFS_HELPER@'", value)
        self.assertIn(b'private mount namespace', value)
        self.assertIn(b'--artifact-sha256', value)
        self.assertNotIn(b'curl -', value)
        self.assertNotIn(b'apt-get', value)

    def test_license_claim_without_inventory_and_evidence_is_refused(self):
        with self.assertRaises((ValueError, KeyError)):
            artifact.check_license_review({'schema': 1, 'reviewed': True}, {}, {'entries': []})

    def test_paired_source_requires_complete_matching_bytes_not_just_urls(self):
        source_files = {'sinan-source.tar.gz': b'inert fixture source', 'debian-sources/test.source': b'complete source fixture'}
        expected = {name: {'size': len(content), 'sha256': artifact.digest(content)} for name, content in source_files.items()}
        data = artifact.pack(source_files)
        offer = {'asset': f'ipquality-{artifact.VERSION}-linux-amd64-sources.tar.gz',
                 'sha256': artifact.digest(data), 'size': len(data)}
        files = {'THIRD_PARTY_NOTICES.txt': b'Sinan IPQuality node self-query\n' + artifact.canonical({
            'source_offer': offer, 'notice': 'TEST_ONLY complete byte-stream fixture; no approved build', 'license': 'AGPL-3.0-only'})}
        with patch.object(artifact, 'source_inventory', return_value=expected):
            artifact.validate_source_offer(data, files, artifact.VERSION, 'amd64')
            for contents in ({'sinan-source.tar.gz': source_files['sinan-source.tar.gz']},
                             {**source_files, 'debian-sources/test.source': b'altered source'},
                             {**source_files, 'unexpected.txt': b'extra'}):
                changed = artifact.pack(contents)
                notice = artifact.decode(files['THIRD_PARTY_NOTICES.txt'].split(b'\n', 1)[1])
                notice['source_offer'].update(size=len(changed), sha256=artifact.digest(changed))
                modified = {'THIRD_PARTY_NOTICES.txt': b'Sinan IPQuality node self-query\n' + artifact.canonical(notice)}
                with self.assertRaises(ValueError):
                    artifact.validate_source_offer(changed, modified, artifact.VERSION, 'amd64')
            with self.assertRaises(ValueError):
                artifact.validate_source_offer(data + b'changed', files, artifact.VERSION, 'amd64')
            trailing = data + gzip.compress(b'unlisted source bytes')
            notice = artifact.decode(files['THIRD_PARTY_NOTICES.txt'].split(b'\n', 1)[1])
            notice['source_offer'].update(size=len(trailing), sha256=artifact.digest(trailing))
            modified = {'THIRD_PARTY_NOTICES.txt': b'Sinan IPQuality node self-query\n' + artifact.canonical(notice)}
            with self.assertRaisesRegex(ValueError, 'unlisted'):
                artifact.validate_source_offer(trailing, modified, artifact.VERSION, 'amd64')

    def test_paired_source_cannot_name_external_or_another_architecture_asset(self):
        for asset_name in ('https://example.invalid/source.tar.gz', '../source.tar.gz',
                           f'ipquality-{artifact.VERSION}-linux-arm64-sources.tar.gz'):
            files = {'THIRD_PARTY_NOTICES.txt': b'Sinan IPQuality node self-query\n' + artifact.canonical({
                'source_offer': {'asset': asset_name, 'size': 1, 'sha256': 'a' * 64},
                'notice': 'TEST_ONLY', 'license': 'AGPL-3.0-only'})}
            with self.assertRaisesRegex(ValueError, 'source-offer descriptor'):
                artifact.source_offer(files, artifact.VERSION, 'amd64')


if __name__ == '__main__':
    unittest.main()
