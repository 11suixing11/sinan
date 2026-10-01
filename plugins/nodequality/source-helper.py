#!/usr/bin/env python3
"""Validate canonical sources and serve the fixed public-report policy variant."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys

MAX_FILE = 2 * 1024 * 1024
MAX_BUNDLE = 8 * 1024 * 1024
REPORT_POLICY_SHA256 = '0c66e702084820e399a16b18b51ba331cd8edd406dd96ede7c2ee84f78c30245'
REPORT_ROLES = frozenset({'hardware.sh', 'ip.sh', 'net.sh'})
SWAP_POLICY_SHA256 = 'd43b3e6fa6bfc0a31fcff5e3bc7a3228cdd1501c9f15f7e27175d09dada9a96b'
FILES = {
    'NodeQuality.sh': ('LloydAsp/NodeQuality', 'NodeQuality.sh', 'LICENSE.nodequality'),
    'header.sh': ('LloydAsp/NodeQuality', 'part/header.sh', 'LICENSE.nodequality'),
    'swap.sh': ('LloydAsp/NodeQuality', 'part/swap.sh', 'LICENSE.nodequality'),
    'hardware.sh': ('xykt/HardwareQuality', 'hardware.sh', 'LICENSE.hardware'),
    'ip.sh': ('xykt/IPQuality', 'ip.sh', 'LICENSE.ip'),
    'net.sh': ('xykt/NetQuality', 'net.sh', 'LICENSE.net'),
    'LICENSE.nodequality': ('LloydAsp/NodeQuality', 'LICENSE', None),
    'LICENSE.hardware': ('xykt/HardwareQuality', 'LICENSE', None),
    'LICENSE.ip': ('xykt/IPQuality', 'LICENSE', None),
    'LICENSE.net': ('xykt/NetQuality', 'LICENSE', None),
}
ALIASES = {
    'https://raw.githubusercontent.com/LloydAsp/NodeQuality/refs/heads/main/part/header.sh': 'header.sh',
    'https://raw.githubusercontent.com/LloydAsp/NodeQuality/refs/heads/main/part/swap.sh': 'swap.sh',
    'https://Hardware.Check.Place': 'hardware.sh',
    'https://IP.Check.Place': 'ip.sh',
    'https://Net.Check.Place': 'net.sh',
}


def ordinary(path, limit):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, 'rb') as stream:
        metadata = os.fstat(stream.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > limit:
            raise ValueError('source input must be a bounded ordinary file')
        content = stream.read(limit + 1)
    if len(content) > limit:
        raise ValueError('source input exceeds its byte limit')
    return content


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('duplicate source manifest key')
        result[key] = value
    return result


def decode(content):
    return json.loads(content, object_pairs_hook=unique_object)


def validate(lock):
    if (not isinstance(lock, dict) or set(lock) != {'schema', 'files'}
            or type(lock['schema']) is not int or lock['schema'] != 1):
        raise ValueError('unsupported source manifest')
    rows = {}
    if not isinstance(lock['files'], list):
        raise ValueError('source manifest requires files')
    for row in lock['files']:
        if not isinstance(row, dict) or row.get('name') not in FILES or row['name'] in rows:
            raise ValueError('unknown or duplicate source file')
        repository, source_path, license_file = FILES[row['name']]
        expected = {'name', 'repository', 'commit', 'path', 'sha256', 'size'}
        if license_file:
            expected.add('license_file')
        if set(row) != expected or row['repository'] != repository or row['path'] != source_path:
            raise ValueError('source identity differs from its pinned role')
        if license_file and row['license_file'] != license_file:
            raise ValueError('source license identity mismatch')
        if not isinstance(row['commit'], str) or not re.fullmatch('[0-9a-f]{40}', row['commit']):
            raise ValueError('source commit must be a full immutable SHA')
        if not isinstance(row['sha256'], str) or not re.fullmatch('[0-9a-f]{64}', row['sha256']):
            raise ValueError('source requires an exact SHA256')
        if type(row['size']) is not int or not 0 < row['size'] <= MAX_FILE:
            raise ValueError('source size exceeds its byte limit')
        rows[row['name']] = row
    if set(rows) != set(FILES):
        raise ValueError('source manifest lacks a script or full license')
    for row in rows.values():
        related = [other for other in rows.values() if other['repository'] == row['repository']]
        if len({other['commit'] for other in related}) != 1:
            raise ValueError('one repository must have one pinned source commit')
    return rows


def verified(content, row):
    if len(content) != row['size'] or hashlib.sha256(content).hexdigest() != row['sha256']:
        raise ValueError('pinned source checksum or size mismatch: ' + row['name'])
    return content


def receive(path, stream):
    # Enforce the disk bound independently of curl version/Content-Length.
    content = stream.read(MAX_FILE + 1)
    if not content or len(content) > MAX_FILE:
        raise ValueError('download is empty or exceeds its byte limit')
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(descriptor, 'wb') as output:
            os.fchmod(output.fileno(), 0o600)
            output.write(content)
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def report_policy():
    path = Path(__file__).with_name('report-policy.py')
    content = ordinary(path, 65536)
    if hashlib.sha256(content).hexdigest() != REPORT_POLICY_SHA256:
        raise ValueError('signed report policy helper SHA256 mismatch')
    namespace = {'__name__': 'sinan_report_policy', '__file__': str(path)}
    # Execute only the exact verified helper bytes, never a second path lookup.
    # Its bounded transform does not execute upstream code or perform I/O.
    exec(compile(content, str(path), 'exec'), namespace)
    return namespace


def swap_policy():
    path = Path(__file__).with_name('swap-policy.py')
    content = ordinary(path, 65536)
    if hashlib.sha256(content).hexdigest() != SWAP_POLICY_SHA256:
        raise ValueError('signed swap policy helper SHA256 mismatch')
    namespace = {'__name__': 'sinan_swap_policy', '__file__': str(path)}
    exec(compile(content, str(path), 'exec'), namespace)
    return namespace


def without_swap(name, content):
    policy = swap_policy()
    result = policy['transform'](name, content)
    if (not isinstance(result, bytes) or len(result) > MAX_FILE + 2048
            or hashlib.sha256(result).hexdigest() != policy['SOURCES'][name]['patched_sha256']):
        raise ValueError('served swap policy output SHA256 or byte limit mismatch')
    return result


def entrypoint(bundle):
    rows = validate(bundle['lock'])
    original = verified(base64.b64decode(bundle['files']['NodeQuality.sh'], validate=True), rows['NodeQuality.sh'])
    return without_swap('NodeQuality.sh', original)


def pack(lock, directory):
    rows = validate(lock)
    report_policy()
    swap_policy()
    files = {name: base64.b64encode(verified(ordinary(directory / name, MAX_FILE), row)).decode()
             for name, row in rows.items()}
    result = (json.dumps(dict(schema=1, lock=lock, files=files), sort_keys=True, separators=(',', ':')) + '\n').encode()
    if len(result) > MAX_BUNDLE:
        raise ValueError('source bundle exceeds its byte limit')
    return result


def materialize(bundle, directory):
    if (not isinstance(bundle, dict) or set(bundle) != {'schema', 'lock', 'files'}
            or type(bundle['schema']) is not int or bundle['schema'] != 1):
        raise ValueError('unsupported source bundle')
    rows = validate(bundle['lock'])
    if not isinstance(bundle['files'], dict) or set(bundle['files']) != set(rows):
        raise ValueError('source bundle lacks an exact file set')
    content = {name: verified(base64.b64decode(bundle['files'][name], validate=True), row)
               for name, row in rows.items()}
    directory.mkdir(mode=0o700, exist_ok=False)
    for name, value in content.items():
        with (directory / name).open('xb') as output:
            os.fchmod(output.fileno(), 0o600)
            output.write(value)
    with (directory / 'source-lock.json').open('x') as output:
        os.fchmod(output.fileno(), 0o600)
        json.dump(bundle['lock'], output, sort_keys=True)


def serve(directory, arguments):
    if len(arguments) != 2 or arguments[0] not in ('-sL', '-Ls') or arguments[1] not in ALIASES:
        raise ValueError('unknown first-level source request; online fallback is forbidden')
    if directory.is_symlink() or not directory.is_dir():
        raise ValueError('source directory must be private and ordinary')
    rows = validate(decode(ordinary(directory / 'source-lock.json', 65536)))
    name = ALIASES[arguments[1]]
    canonical = verified(ordinary(directory / name, MAX_FILE), rows[name])
    if name not in REPORT_ROLES:
        return canonical
    policy = report_policy()
    patched = policy['transform'](name, canonical)
    if (not isinstance(patched, bytes) or len(patched) > MAX_FILE + 2048
            or hashlib.sha256(patched).hexdigest() != policy['SOURCES'][name]['patched_sha256']):
        raise ValueError('served report policy output SHA256 or byte limit mismatch')
    return without_swap(name, patched) if name == 'hardware.sh' else patched


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', choices=['downloads', 'receive', 'pack', 'materialize', 'serve', 'entrypoint'])
    parser.add_argument('input', type=Path)
    parser.add_argument('remaining', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.operation == 'entrypoint':
        if args.remaining:
            raise ValueError('entrypoint accepts only a pinned bundle')
        sys.stdout.buffer.write(entrypoint(decode(ordinary(args.input, MAX_BUNDLE))))
        return
    if args.operation == 'receive':
        if args.remaining:
            raise ValueError('receive accepts only a new private target')
        receive(args.input, sys.stdin.buffer)
        return
    if args.operation == 'serve':
        sys.stdout.buffer.write(serve(args.input, args.remaining))
        return
    if args.operation == 'materialize':
        if len(args.remaining) != 1:
            raise ValueError('materialize requires a private destination')
        materialize(decode(ordinary(args.input, MAX_BUNDLE)), Path(args.remaining[0]))
        return
    lock = decode(ordinary(args.input, 65536))
    rows = validate(lock)
    if args.operation == 'downloads':
        if args.remaining:
            raise ValueError('downloads accepts only the fixed manifest')
        for name, row in sorted(rows.items()):
            print(name + '\thttps://raw.githubusercontent.com/' + row['repository'] + '/' + row['commit'] + '/' + row['path'])
    else:
        if len(args.remaining) != 1:
            raise ValueError('pack requires a private source directory')
        sys.stdout.buffer.write(pack(lock, Path(args.remaining[0])))


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, TypeError, KeyError) as error:
        raise SystemExit('Error: ' + str(error)) from None
