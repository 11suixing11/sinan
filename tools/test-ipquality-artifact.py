#!/usr/bin/env python3
"""Check source-only preparation, inventory and publication guards."""
import gzip
import io
from pathlib import Path
import tarfile
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
