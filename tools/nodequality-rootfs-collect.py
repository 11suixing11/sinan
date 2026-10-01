#!/usr/bin/env python3
"""Collect authenticated Debian inputs; never approve or build a rootfs."""

import argparse
import contextlib
import datetime
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import re
import shutil
import socket
import ssl
import stat
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request


HERE = Path(__file__).resolve()
spec = importlib.util.spec_from_file_location('sinan_rootfs_build', HERE.with_name('nodequality-rootfs-build.py'))
BUILD = importlib.util.module_from_spec(spec)
spec.loader.exec_module(BUILD)

MAX_TOTAL = 64 * 1024**3
MAX_SECONDS = 7200
MAX_IMPORTS = 1024**2
MAX_RECEIPT = 8 * 1024**2
MIN_FREE = 512 * 1024**2
REQUEST_FIELDS = {'schema', 'arch', 'source_epoch', 'repositories'}
REPOSITORY_FIELDS = {'id', 'archive', 'timestamp', 'suite'}
COLLECTION_KIND = 'sinan-nodequality-debian-input-collection'
BASE_ENV = {'PATH': '/usr/sbin:/usr/bin:/sbin:/bin', 'LANG': 'C', 'LC_ALL': 'C'}
APTPATH = '/usr/bin/apt-get'


def require(condition, message):
    BUILD.require(condition, message)


def timestamp():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def debian_release(path=Path('/etc/os-release')):
    path = Path(path)
    if path.is_symlink():
        require(path == Path('/etc/os-release') and path.resolve(strict=True) == Path('/usr/lib/os-release'),
                'unsupported operating-system release link')
        path = Path('/usr/lib/os-release')
    fields = {}
    for line in BUILD.read_regular(path, 65536).decode('utf-8').splitlines():
        if '=' in line:
            key, value = line.split('=', 1)
            fields[key] = value.strip('"\'')
    require(fields.get('ID') == 'debian' and fields.get('VERSION_ID') == '12', 'collection requires Debian 12')
    return fields


def native_arch():
    require(sys.platform == 'linux', 'collection requires a native Debian 12 Linux host')
    require(os.geteuid() == 0, 'isolated native collection requires root on its dedicated host')
    debian_release()
    value = {'x86_64': 'amd64', 'aarch64': 'arm64'}.get(platform.machine())
    require(value is not None, 'unsupported native collection architecture')
    return value


def validate_request(request):
    require(isinstance(request, dict) and set(request) == REQUEST_FIELDS, 'invalid collection request fields')
    require(type(request['schema']) is int and request['schema'] == 1 and request['arch'] in BUILD.ARCHES,
            'invalid collection request identity')
    require(type(request['source_epoch']) is int and 0 < request['source_epoch'] < 2**32, 'fixed source epoch required')
    require(isinstance(request['repositories'], list) and 2 <= len(request['repositories']) <= 3,
            'main and security repositories required')
    seen, imports, suites = set(), {}, set()
    for repo in request['repositories']:
        require(isinstance(repo, dict) and set(repo) == REPOSITORY_FIELDS, 'invalid requested repository')
        BUILD.relative(repo['id'])
        require('/' not in repo['id'] and repo['id'] not in seen, 'duplicate requested repository')
        require(repo['archive'] in BUILD.SIGNERS and re.fullmatch(r'[0-9]{8}T[0-9]{6}Z', repo['timestamp'] or ''),
                'exact requested snapshot timestamp required')
        datetime.datetime.strptime(repo['timestamp'], '%Y%m%dT%H%M%SZ')
        permitted = {'bookworm', 'bookworm-updates'} if repo['archive'] == 'debian' else {'bookworm-security'}
        require(repo['suite'] in permitted and repo['suite'] not in suites, 'duplicate or non-bookworm suite')
        require(repo['archive'] not in imports or imports[repo['archive']] == repo['timestamp'],
                'one archive must use one exact import')
        seen.add(repo['id'])
        imports[repo['archive']] = repo['timestamp']
        suites.add(repo['suite'])
    require(set(imports) == set(BUILD.SIGNERS) and 'bookworm' in suites, 'main/security pair is incomplete')
    return imports


def import_url(archive, chosen):
    day = datetime.datetime.strptime(chosen, '%Y%m%dT%H%M%SZ').date()
    query = urllib.parse.urlencode({'archive': archive, 'after': day.strftime('%Y%m%dT000000Z'),
                                   'before': (day + datetime.timedelta(days=1)).strftime('%Y%m%dT000000Z')})
    return 'https://snapshot.debian.org/mr/timestamp/?' + query


def validate_imports(content, archive, chosen):
    value = BUILD.decode(content)
    require(isinstance(value, dict) and isinstance(value.get('result'), dict)
            and set(value['result']) == {archive}, 'wrong snapshot import response')
    rows = value['result'][archive]
    require(isinstance(rows, list) and rows and len(rows) <= 4096, 'invalid snapshot import list')
    require(all(isinstance(row, str) and re.fullmatch(r'[0-9]{8}T[0-9]{6}Z', row) for row in rows)
            and len(rows) == len(set(rows)), 'ambiguous snapshot import list')
    for row in rows:
        datetime.datetime.strptime(row, '%Y%m%dT%H%M%SZ')
    require(chosen in rows, 'requested timestamp is absent; nearest earlier snapshot is not accepted')
    return rows


def allowed_url(value):
    parsed = urllib.parse.urlsplit(value)
    require(parsed.scheme == 'https' and parsed.hostname == 'snapshot.debian.org'
            and parsed.netloc == 'snapshot.debian.org' and not parsed.fragment, 'non-official collection URL')
    require(parsed.path.startswith(('/archive/debian/', '/archive/debian-security/', '/mr/timestamp/', '/file/')),
            'unsupported official collection path')
    return value


class OfficialRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, stream, code, message, headers, destination):
        allowed_url(destination)
        return super().redirect_request(request, stream, code, message, headers, destination)


def free_disk(path, reserve, additional=0):
    require(type(reserve) is int and MIN_FREE <= reserve <= MAX_TOTAL, 'invalid explicit free-disk reserve')
    usage = os.statvfs(path)
    require(usage.f_bavail * usage.f_frsize >= reserve + additional, 'factory free-disk reserve would be crossed')


def publish(path, content, limit):
    require(len(content) <= limit, 'published metadata exceeds its reader limit')
    BUILD.write_new(path, content)


def stream_response(response, output, limit, reserve=MIN_FREE):
    """The limit applies before every write, including chunked responses."""
    length = 0
    with open(output, 'xb') as target:
        os.fchmod(target.fileno(), 0o600)
        while True:
            chunk = response.read(min(65536, limit - length + 1))
            if not chunk:
                break
            require(length + len(chunk) <= limit, 'download exceeds its byte budget')
            free_disk(Path(output).parent, reserve, len(chunk))
            target.write(chunk)
            length += len(chunk)
    require(length > 0, 'empty official download')
    return length


def fetch_worker(url, output, limit, header_output, reserve=MIN_FREE):
    allowed_url(url)
    require(type(limit) is int and 0 < limit <= BUILD.MAX_SOURCE, 'invalid worker byte limit')
    output = Path(output).absolute()
    BUILD.private_directory(output.parent)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), OfficialRedirects())
    # The parent supervises DNS, TLS and every body read with an absolute deadline.
    with opener.open(url, timeout=30) as response:
        require(response.status == 200, 'official download did not return HTTP 200')
        allowed_url(response.geturl())
        advertised = response.headers.get('Content-Length')
        if advertised is not None:
            require(advertised.isdigit() and int(advertised) <= limit, 'advertised download exceeds budget')
        headers = response.headers.as_bytes()
        require(len(headers) <= 65536, 'oversized official response headers')
        BUILD.write_new(header_output, headers)
        size = stream_response(response, output, limit, reserve)
        require(advertised is None or size == int(advertised), 'download differs from Content-Length')
        return {'schema': 1, 'url': url, 'final_url': response.geturl(), 'http_status': 200,
                'received_at': timestamp(), 'size': size, 'header_representation': 'parsed-http-headers'}


def fetch_error(error, url, started):
    status = error.code if isinstance(error, urllib.error.HTTPError) else None
    reason = error.reason if isinstance(error, urllib.error.URLError) else error
    if status is not None:
        category = 'http_' + str(status)
    elif isinstance(reason, socket.gaierror):
        category = 'dns'
    elif isinstance(reason, ssl.SSLError):
        category = 'tls'
    elif isinstance(reason, (TimeoutError, socket.timeout)):
        category = 'timeout'
    elif isinstance(reason, ValueError) and 'budget' in str(reason):
        category = 'response_too_large'
    elif isinstance(reason, ValueError):
        category = 'response_invalid'
    else:
        category = 'connection'
    return {'schema': 1, 'url': url, 'http_status': status, 'category': category,
            'complete': False, 'received_at': timestamp(), 'elapsed_seconds': time.monotonic() - started}


def owned_size(root, deadline):
    total, members = 0, 0
    stack = [BUILD.private_directory(root)]
    while stack:
        directory = stack.pop()
        for path in directory.iterdir():
            deadline.check()
            info = path.lstat()
            members += 1
            require(members <= BUILD.MAX_MEMBERS, 'factory directory has too many entries')
            if stat.S_ISDIR(info.st_mode):
                require(not path.is_symlink(), 'factory directory has a link parent')
                stack.append(path)
            else:
                require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, 'factory directory contains a non-private ordinary file')
                total += info.st_size
    return total


class Collector:
    def __init__(self, output, max_total, deadline, reserve=MIN_FREE):
        require(type(max_total) is int and 0 < max_total <= MAX_TOTAL, 'invalid explicit factory byte budget')
        self.output, self.max_total, self.deadline = Path(output), max_total, deadline
        self.reserve = reserve
        self.cache = self.output / 'input-cache'
        self.cache.mkdir(mode=0o700)
        self.downloads, self.signatures = [], []

    def budget(self, additional=0):
        require(owned_size(self.output, self.deadline) + additional <= self.max_total, 'factory byte budget exceeded')
        free_disk(self.output, self.reserve, additional)

    def obtain(self, url, limit, expected=None, suffix=''):
        self.budget()
        available = self.max_total - owned_size(self.output, self.deadline)
        bound = min(limit, available)
        require(bound > 0 and (expected is None or expected['size'] <= bound), 'insufficient factory download budget')
        with tempfile.TemporaryDirectory(prefix='download-', dir=self.output) as name:
            work = Path(name)
            body, headers, receipt = work / 'body', work / 'headers', work / 'receipt.json'
            argv = [sys.executable, str(HERE), '_fetch', '--url', allowed_url(url), '--output', str(body),
                    '--headers', str(headers), '--receipt', str(receipt), '--limit', str(bound),
                    '--reserve-free-bytes', str(self.reserve)]
            try:
                raw = BUILD.run_bounded(argv, BUILD.Deadline(min(180, self.deadline.remaining())), 65536)
            except BaseException as error:
                report = {'schema': 1, 'url': url, 'complete': False, 'category': 'producer_failed_or_deadline',
                          'http_status': None, 'elapsed_seconds': None}
                if receipt.exists():
                    report = BUILD.decode(BUILD.read_regular(receipt, 65536))
                self.downloads.append(report)
                if hasattr(error, 'add_note'):
                    error.add_note('Official download failed: ' + str(report.get('category')))
                raise
            report = BUILD.decode(BUILD.read_regular(receipt, 65536, self.deadline))
            require(BUILD.decode(raw) == report, 'worker output and receipt differ')
            actual = BUILD.file_identity(body, bound, self.deadline)
            require(report.get('size') == actual['size'] and report.get('http_status') == 200, 'download worker identity mismatch')
            if expected is not None:
                require(actual == expected, 'download differs from authenticated SHA256/size')
            blob = actual['sha256'] + suffix
            destination = self.cache / blob
            if destination.exists():
                require(BUILD.file_identity(destination, limit, self.deadline) == actual, 'cache identity conflict')
            else:
                os.rename(body, destination)
            header_blob = 'http-' + str(len(self.downloads)) + '.headers'
            destination_header = self.output / header_blob
            os.rename(headers, destination_header)
            report.update(actual, blob=blob, headers=header_blob)
            self.downloads.append(report)
            self.budget()
            return dict(actual, blob=blob)


def verify_candidate(candidate, arch, deadline):
    require(isinstance(candidate, dict) and set(candidate) == {'image_sha256', 'arch', 'tools'}, 'invalid candidate builder record')
    require(candidate['arch'] == arch and BUILD.SHA256.fullmatch(candidate['image_sha256'] or ''), 'invalid candidate image identity')
    require(isinstance(candidate['tools'], list) and len(candidate['tools']) == len(BUILD.TOOL_PATHS), 'complete candidate tool record required')
    found = set()
    for row in candidate['tools']:
        require(isinstance(row, dict) and set(row) == {'name', 'path', 'version', 'sha256', 'size'}, 'invalid candidate tool')
        require(row['name'] in BUILD.TOOL_PATHS and row['name'] not in found
                and row['path'] == BUILD.TOOL_PATHS[row['name']], 'wrong candidate tool path')
        require(isinstance(row['version'], str) and 0 < len(row['version']) <= 128, 'candidate tool version required')
        require(BUILD.file_identity(row['path'], BUILD.MAX_ARCHIVE, deadline) ==
                {key: row[key] for key in ('sha256', 'size')}, 'candidate tool bytes differ from this host')
        found.add(row['name'])
    return candidate


def record_collection_tools(deadline):
    result = []
    paths = {'/usr/bin/gpgv', '/usr/bin/unshare', APTPATH, '/usr/bin/env', '/usr/bin/dpkg-query',
            '/usr/lib/apt/methods/file', '/usr/lib/apt/methods/gpgv', str(Path(sys.executable).resolve())}
    for requested in sorted(paths):
        resolved = Path(requested).resolve(strict=True)
        identity = BUILD.file_identity(resolved, BUILD.MAX_ARCHIVE, deadline)
        ownership = BUILD.run_bounded(['/usr/bin/dpkg-query', '--search', str(resolved)],
                                      BUILD.Deadline(min(15, deadline.remaining())), 65536)
        require(ownership.strip() and b'\0' not in ownership, 'collection tool lacks dpkg ownership evidence')
        result.append(dict(identity, requested_path=requested, resolved_path=str(resolved),
                           dpkg_ownership=ownership.decode('utf-8').strip()))
    versions = BUILD.run_bounded(['/usr/bin/dpkg-query', '--show',
            '--showformat=${binary:Package}\t${Version}\t${Architecture}\t${source:Package}\t${source:Version}\n',
            'apt', 'gpgv', 'util-linux', 'python3-minimal'],
            BUILD.Deadline(min(15, deadline.remaining())), 65536).decode('utf-8')
    actual_versions = {}
    for path in (APTPATH, '/usr/bin/gpgv', '/usr/bin/unshare', str(Path(sys.executable).resolve())):
        actual_versions[path] = BUILD.run_bounded([path, '--version'],
                BUILD.Deadline(min(15, deadline.remaining())), 65536).decode('utf-8').strip()
    return {'executables': result, 'dpkg_report': versions, 'actual_version_output': actual_versions,
            'evidence_scope': 'actual bytes and local dpkg claims; not independent builder approval'}


def repository_url(repo, path):
    BUILD.relative(path)
    return BUILD.repository_url(repo, path)


def pool_path(repo, value):
    path = BUILD.relative(value)
    prefix = {'debian': 'pool/main/', 'debian-security': 'pool/updates/main/'}[repo['archive']]
    require(path.startswith(prefix), 'package/source is outside this signed Debian main archive')
    return path


def authenticate_metadata(collector, requested, keyring):
    repo = dict(requested)
    repo['inrelease'] = collector.obtain(repository_url(repo, 'dists/' + repo['suite'] + '/InRelease'), BUILD.MAX_LOCK)
    with tempfile.TemporaryDirectory(prefix='signature-', dir=collector.output) as name:
        home = Path(name)
        release = home / 'Release'
        status = BUILD.run_bounded(['/usr/bin/gpgv', '--homedir', str(home), '--keyring', str(keyring),
                '--status-fd', '1', '--output', str(release), str(collector.cache / repo['inrelease']['blob'])],
                BUILD.Deadline(min(30, collector.deadline.remaining())), 65536, stderr=subprocess.DEVNULL)
        signers = BUILD.valid_signers(status, repo['archive'], repo['timestamp'])
        publish(collector.output / (repo['id'] + '.gpg-status'), status, 65536)
        records = list(BUILD.control_records(BUILD.read_regular(release, BUILD.MAX_LOCK, collector.deadline)))
        require(len(records) == 1 and records[0].get('Origin') == 'Debian'
                and records[0].get('Codename') == repo['suite'], 'wrong authenticated Release identity')
        sums = BUILD.checksum_rows(records[0].get('SHA256', ''))
        collector.signatures.append({'repository': repo['id'], 'primary_fingerprints': signers,
                                      'inrelease_sha256': repo['inrelease']['sha256'],
                                      'gpg_status_file': repo['id'] + '.gpg-status'})
    repo['indices'], indices = [], {}
    for kind, base in [('Packages', 'main/binary-' + collector.arch + '/Packages'), ('Sources', 'main/source/Sources')]:
        path = next((base + extension for extension in ('.xz', '.gz') if base + extension in sums), None)
        require(path is not None, 'authenticated Release lacks the required native/source index')
        descriptor = collector.obtain(repository_url(repo, 'dists/' + repo['suite'] + '/' + path),
                                      BUILD.MAX_INDEX, sums[path], Path(path).suffix)
        repo['indices'].append(dict(descriptor, kind=kind, path=path))
        indices[kind] = collector.cache / descriptor['blob']
    return repo, indices


def iter_rows(value, deadline):
    # Lists are owned small test fixtures. Production retains only cache paths.
    if isinstance(value, list):
        yield from value
    else:
        yield from BUILD.index_records(value, BUILD.MAX_INDEX, deadline)


def essential_seeds(repositories, indices, deadline=None):
    deadline = deadline or BUILD.Deadline(60)
    main = next(repo for repo in repositories if repo['suite'] == 'bookworm')
    rows = iter_rows(indices[main['id']]['Packages'], deadline)
    names = {row['Package'] for row in rows if row.get('Essential') == 'yes'}
    require(names and all(BUILD.NAME.fullmatch(name) for name in names), 'authenticated main essential set is missing')
    # APT, rather than string ordering, chooses versions across the same signed mirrors.
    return sorted(names | {'apt'} | set(BUILD.TOOL_PACKAGES.values())), sorted(names)


def apt_config(work, mirrors, arch):
    def quote(path):
        value = str(path)
        require('"' not in value and '\\' not in value and '\n' not in value, 'unsafe APT directory')
        return '"' + value + '"'
    for name in ('confparts', 'sourceparts', 'prefparts', 'trustparts', 'lists/partial', 'archives/partial', 'log'):
        (work / name).mkdir(parents=True, mode=0o700)
    for name in ('empty-main', 'status', 'extended_states', 'preferences', 'empty-trusted'):
        BUILD.write_new(work / name, b'')
    lines = []
    for repo, mirror in mirrors:
        signers = ','.join(sorted(BUILD.SIGNERS[repo['archive']]))
        lines.append('deb [arch=' + arch + ' check-valid-until=no signed-by=' + str(mirror / 'archive-keyring.gpg')
                     + ',' + signers + '] file://' + str(mirror) + ' ' + repo['suite'] + ' main')
    BUILD.write_new(work / 'sources.list', ('\n'.join(lines) + '\n').encode('ascii'))
    paths = {'Dir::Etc': '.', 'Dir::State': '.', 'Dir::Cache': '.', 'Dir::Log': 'log',
             'Dir::Etc::main': 'empty-main', 'Dir::Etc::parts': 'confparts',
             'Dir::Etc::sourcelist': 'sources.list', 'Dir::Etc::sourceparts': 'sourceparts',
             'Dir::Etc::preferences': 'preferences', 'Dir::Etc::preferencesparts': 'prefparts',
             'Dir::Etc::trusted': 'empty-trusted', 'Dir::Etc::trustedparts': 'trustparts',
             'Dir::State::status': 'status', 'Dir::State::extended_states': 'extended_states', 'Dir::State::lists': 'lists',
             'Dir::Log::History': 'log/history.log', 'Dir::Log::Terminal': 'log/term.log',
             'Dir::Cache::archives': 'archives'}
    config = [key + ' ' + quote(work / path) + ';' for key, path in sorted(paths.items())]
    config.extend(['#clear APT::Architectures;', 'APT::Architecture "' + arch + '";',
        'APT::Architectures { "' + arch + '"; };', 'Dir::Cache::pkgcache "";', 'Dir::Cache::srcpkgcache "";',
        'APT::Install-Recommends "false";', 'APT::Install-Suggests "false";',
        'APT::Sandbox::User "root";',
        'Acquire::Retries "0";', 'Acquire::Languages "none";', 'Acquire::PDiffs "false";',
        'Acquire::By-Hash "false";', 'Acquire::AllowInsecureRepositories "false";',
        'Acquire::AllowWeakRepositories "false";', 'Acquire::AllowDowngradeToInsecureRepositories "false";',
        '#clear DPkg::Pre-Invoke;', '#clear DPkg::Post-Invoke;', '#clear DPkg::Pre-Install-Pkgs;',
        '#clear APT::Update::Pre-Invoke;', '#clear APT::Update::Post-Invoke;',
        '#clear APT::Update::Post-Invoke-Success;'])
    BUILD.write_new(work / 'apt.conf', ('\n'.join(config) + '\n').encode('ascii'))
    return work / 'apt.conf'


def solver_worker(config, evidence, baseline, operation, seeds):
    require(native_arch() in BUILD.ARCHES, 'native solver required')
    inode = os.stat('/proc/self/ns/net').st_ino
    require(inode != baseline and all(name == 'lo' for _, name in socket.if_nameindex()), 'solver network isolation failed')
    evidence = Path(evidence).absolute()
    BUILD.private_directory(evidence.parent)
    require(operation in ('update', 'plan'), 'invalid internal solver operation')
    require(all(BUILD.NAME.fullmatch(seed) for seed in seeds), 'invalid internal solver seed')
    require(operation != 'update' or not seeds, 'update must not take package seeds')
    receipt = {'schema': 1, 'host_namespace_inode': baseline, 'solver_namespace_inode': inode,
               'different_namespace': True, 'interfaces': ['lo'], 'operation': operation,
               'configuration': str(config), 'installation': False}
    BUILD.write_new(evidence, BUILD.canonical(receipt) + b'\n')
    argv = [APTPATH, '--quiet', '--yes']
    argv += ['update'] if operation == 'update' else ['--print-uris', '--download-only', '--no-remove', 'install', *seeds]
    os.execve(APTPATH, argv, dict(BASE_ENV, APT_CONFIG=str(config), HOME=str(evidence.parent)))


def map_print_uris(content, mirrors, indices, arch, deadline=None):
    deadline = deadline or BUILD.Deadline(60)
    wanted, selections = {}, []
    for line in content.decode('utf-8').splitlines():
        if not line.startswith("'"):
            continue
        # APT may omit the display checksum when Packages has no MD5Sum.
        # Only the signed Packages SHA256 and size below authenticate a file.
        match = re.fullmatch(r"'([^']+)'\s+\S+\s+([0-9]+)(?:\s+\S+)?\s*", line)
        require(match is not None, 'unrecognized APT URI record')
        url = urllib.parse.urlsplit(match[1])
        require(url.scheme == 'file' and not url.netloc and not url.query and not url.fragment, 'solver selected a network URI')
        decoded = urllib.parse.unquote(url.path, errors='strict')
        matches = []
        for repo, mirror in mirrors:
            prefix = str(mirror) + '/'
            if decoded.startswith(prefix):
                filename = pool_path(repo, decoded[len(prefix):])
                matches.append((repo['id'], filename, match[2]))
        require(len(matches) == 1, 'APT selected outside the owned mirrors')
        repository, filename, size = matches[0]
        key = (repository, filename)
        require(key not in wanted, 'APT printed a duplicate package URI')
        wanted[key] = size
    require(wanted and len(wanted) <= 2048, 'APT produced no bounded complete selection')
    found = set()
    for repo, _mirror in mirrors:
        for row in iter_rows(indices[repo['id']]['Packages'], deadline):
            key = (repo['id'], row.get('Filename'))
            if key not in wanted:
                continue
            require(key not in found, 'APT filename has no unique signed package identity')
            require(row.get('Architecture') in (arch, 'all') and row.get('Size') == wanted[key]
                    and BUILD.SHA256.fullmatch(row.get('SHA256', '')), 'APT selection differs from signed native/all package')
            source_name, source_version = BUILD.source_pair(row)
            selections.append({'repository': repo['id'], 'name': row['Package'], 'version': row['Version'],
                'architecture': row['Architecture'], 'filename': key[1], 'sha256': row['SHA256'],
                'size': int(row['Size']), 'source_name': source_name, 'source_version': source_version})
            found.add(key)
    require(found == set(wanted), 'APT URI has no matching authenticated package')
    require(selections and len(selections) <= 2048, 'APT produced no bounded complete selection')
    require(len({row['name'] for row in selections}) == len(selections), 'APT selected duplicate package names')
    return sorted(selections, key=lambda row: row['name'])


def source_closure(packages, repositories, indices, deadline=None):
    deadline = deadline or BUILD.Deadline(60)
    wanted = {(row['source_name'], row['source_version']) for row in packages}
    candidates = {pair: [] for pair in wanted}
    for repo in repositories:
        found = set()
        for row in iter_rows(indices[repo['id']]['Sources'], deadline):
            pair = (row.get('Package'), row.get('Version'))
            if pair in wanted:
                require(pair not in found, 'ambiguous signed source index')
                found.add(pair)
                directory = pool_path(repo, row.get('Directory'))
                files = BUILD.checksum_rows(row.get('Checksums-Sha256', ''))
                require(any(path.endswith('.dsc') for path in files) and len(files) <= 64
                        and all('/' not in path for path in files), 'incomplete or unsafe corresponding source files')
                candidates[pair].append({'repository': repo['id'], 'name': pair[0], 'version': pair[1],
                                        'directory': directory, 'files': files})
    result = []
    for pair, matches in sorted(candidates.items()):
        require(matches, 'corresponding source is absent from every signed repository')
        identities = {BUILD.canonical(row['files']) for row in matches}
        require(len(identities) == 1, 'conflicting corresponding source across signed repositories')
        chosen = matches[0]
        chosen['files'] = [dict(value, name=path) for path, value in sorted(chosen['files'].items())]
        result.append(chosen)
    return result


def solve(collector, repositories, indices, keyring):
    seeds, essentials = essential_seeds(repositories, indices, collector.deadline)
    work = collector.output / 'solver'
    work.mkdir(mode=0o700)
    # APT may write derived list copies. Admission reserves their maximum index
    # size, and the caller's filesystem quota remains the hard aggregate guard.
    collector.budget(sum(BUILD.MAX_INDEX for repo in repositories for row in repo['indices'] if row['kind'] == 'Packages'))
    mirrors = []
    for repo in repositories:
        mirror = work / 'mirrors' / repo['id']
        mirror.mkdir(parents=True, mode=0o700)
        files = [(keyring, mirror / 'archive-keyring.gpg'),
                 (collector.cache / repo['inrelease']['blob'], mirror / 'dists' / repo['suite'] / 'InRelease')]
        files += [(collector.cache / row['blob'], mirror / 'dists' / repo['suite'] / row['path'])
                  for row in repo['indices'] if row['kind'] == 'Packages']
        for original, target in files:
            collector.budget(original.stat().st_size)
            target.parent.mkdir(parents=True, mode=0o700, exist_ok=True)
            shutil.copyfile(original, target)
            os.chmod(target, 0o644)
        mirrors.append((repo, mirror))
    config = apt_config(work, mirrors, collector.arch)
    outputs, commands = {}, []
    baseline = os.stat('/proc/self/ns/net').st_ino
    for operation in ('update', 'plan'):
        evidence = work / (operation + '-namespace.json')
        argv = ['/usr/bin/unshare', '--net', '--fork', '--kill-child', '/usr/bin/env', '-i',
                *[key + '=' + value for key, value in BASE_ENV.items()], 'APT_CONFIG=' + str(config),
                sys.executable, str(HERE), '_solver', '--config', str(config), '--evidence', str(evidence),
                '--baseline', str(baseline), '--operation', operation]
        if operation == 'plan':
            argv += ['--seeds', *seeds]
        outputs[operation] = BUILD.run_bounded(argv, BUILD.Deadline(min(180, collector.deadline.remaining())), MAX_RECEIPT)
        BUILD.write_new(work / (operation + '.stdout'), outputs[operation])
        commands.append(argv)
        collector.budget()
    packages = map_print_uris(outputs['plan'], mirrors, indices, collector.arch, collector.deadline)
    require(set(seeds) <= {row['name'] for row in packages}, 'APT selection omitted required or essential packages')
    report = {'schema': 1, 'seeds': seeds, 'main_essential_names': essentials,
              'commands': commands, 'package_selection': packages,
              'installation': False, 'binary_downloads_by_solver': False,
              'network_namespaces': [BUILD.decode(BUILD.read_regular(work / (op + '-namespace.json'), 65536))
                                     for op in ('update', 'plan')],
              'factory_budget_scope': 'owned material checked between phases; external filesystem quota remains separate'}
    publish(work / 'selection.json', BUILD.canonical(report) + b'\n', MAX_RECEIPT)
    # Keep the evidence/configuration, but not derived lists or duplicate mirrors.
    for name in ('lists', 'archives', 'mirrors'):
        shutil.rmtree(work / name)
    collector.budget()
    return packages, report


def collect(request_path, keyring_path, provenance_path, candidate_path, output_path, max_total, seconds, reserve=MIN_FREE):
    require(type(seconds) is int and 0 < seconds <= MAX_SECONDS, 'invalid overall collection timeout')
    deadline = BUILD.Deadline(seconds)
    request_bytes = BUILD.read_regular(request_path, BUILD.MAX_LOCK, deadline)
    request = BUILD.decode(request_bytes)
    imports = validate_request(request)
    require(request['arch'] == native_arch(), 'collection must run on the requested native architecture')
    candidate_bytes = BUILD.read_regular(candidate_path, BUILD.MAX_LOCK, deadline) if candidate_path else None
    candidate = verify_candidate(BUILD.decode(candidate_bytes), request['arch'], deadline) if candidate_bytes else None
    provenance_bytes = BUILD.read_regular(provenance_path, MAX_IMPORTS, deadline)
    provenance = BUILD.decode(provenance_bytes)
    keyring_identity = BUILD.file_identity(keyring_path, BUILD.MAX_LOCK, deadline)
    require(isinstance(provenance, dict) and provenance.get('schema') == 1
            and all(isinstance(provenance.get(field), str) and 0 < len(provenance[field]) <= 4096
                    for field in ('source', 'reviewer', 'obtained_at'))
            and {key: provenance.get(key) for key in ('sha256', 'size')} == keyring_identity,
            'independent keyring acquisition/review record is missing or mismatched')
    complete, output = False, None
    try:
        with BUILD.deferred_signals():
            output = BUILD.reserve_output(output_path)
        collector = Collector(output, max_total, deadline, reserve)
        collector.arch = request['arch']
        BUILD.write_new(output / 'request.json', request_bytes)
        if candidate_bytes is not None:
            BUILD.write_new(output / 'candidate-builder.json', candidate_bytes)
        BUILD.write_new(output / 'keyring-provenance.json', provenance_bytes)
        keyring = dict(keyring_identity, blob=keyring_identity['sha256'] + '.gpg')
        BUILD.copy_locked(Path(keyring_path).parent, dict(keyring_identity, blob=Path(keyring_path).name),
                          collector.cache / keyring['blob'], BUILD.MAX_LOCK, deadline)
        observed = []
        for archive, chosen in sorted(imports.items()):
            descriptor = collector.obtain(import_url(archive, chosen), MAX_IMPORTS, suffix='.json')
            rows = validate_imports(BUILD.read_regular(collector.cache / descriptor['blob'], MAX_IMPORTS, deadline), archive, chosen)
            observed.append({'archive': archive, 'selected_timestamp': chosen, 'response': descriptor,
                             'imports_in_response': rows, 'authentication_scope': 'official HTTPS discovery, not package signature'})
        repositories, indices = [], {}
        for requested in request['repositories']:
            repo, parsed = authenticate_metadata(collector, requested, collector.cache / keyring['blob'])
            repositories.append(repo)
            indices[repo['id']] = parsed
        packages, solver = solve(collector, repositories, indices, collector.cache / keyring['blob'])
        sources = source_closure(packages, repositories, indices, deadline)
        remaining = sum(row['size'] for row in packages) + sum(value['size'] for row in sources for value in row['files'])
        collector.budget(remaining)
        repos = {repo['id']: repo for repo in repositories}
        for package in packages:
            downloaded = collector.obtain(repository_url(repos[package['repository']], package['filename']),
                    BUILD.MAX_ARCHIVE, {key: package[key] for key in ('sha256', 'size')}, '.deb')
            package['blob'] = downloaded['blob']
        for source in sources:
            for value in source['files']:
                downloaded = collector.obtain(repository_url(repos[source['repository']], source['directory'] + '/' + value['name']),
                        BUILD.MAX_SOURCE, {key: value[key] for key in ('sha256', 'size')}, '.source')
                value['blob'] = downloaded['blob']
        materials = dict(request, keyring=keyring, repositories=repositories, packages=packages, sources=sources)
        BUILD.validate_materials(materials)
        inventory, final_signatures = BUILD.verify_authenticated_sources(materials, collector.cache, deadline)
        materials_bytes = BUILD.canonical(materials) + b'\n'
        publish(output / 'materials.json', materials_bytes, BUILD.MAX_LOCK)
        lock_identity = None
        if candidate is not None:
            lock = dict(materials, builder=candidate)
            BUILD.validate_lock(lock)
            lock_bytes = BUILD.canonical(lock) + b'\n'
            publish(output / 'inputs-lock.json', lock_bytes, BUILD.MAX_LOCK)
            lock_identity = hashlib.sha256(lock_bytes).hexdigest()
        else:
            publish(output / 'unbound-inputs.json', BUILD.canonical({
                'schema': 1, 'kind': COLLECTION_KIND, 'materials_sha256': hashlib.sha256(materials_bytes).hexdigest(),
                'builder': None, 'lock_ready': False, 'builder_approved': False,
                'full_ready': False, 'reproducibility_verified': False}) + b'\n', MAX_RECEIPT)
        receipt = {'schema': 1, 'kind': COLLECTION_KIND, 'complete': True, 'arch': request['arch'], 'collected_at': timestamp(),
            'inputs_lock_sha256': lock_identity, 'materials_sha256': hashlib.sha256(materials_bytes).hexdigest(),
            'lock_ready': candidate is not None, 'builder': candidate, 'imports': observed,
            'signatures': final_signatures, 'downloads': collector.downloads,
            'collection_tools': record_collection_tools(deadline),
            'collector_sha256': BUILD.file_identity(HERE, BUILD.MAX_LOCK, deadline)['sha256'],
            'verification_helper_sha256': BUILD.file_identity(HERE.with_name('nodequality-rootfs-build.py'), BUILD.MAX_LOCK, deadline)['sha256'],
            'candidate_builder_image_sha256': candidate['image_sha256'] if candidate is not None else None,
            'builder_approved': False, 'runtime_image_identity_verified': False,
            'keyring_acquisition_review_supplied': True, 'keyring_trust_independently_verified_by_collector': False,
            'source_authenticated': True, 'authentication_scope': 'Debian inputs under supplied independently reviewed keyring',
            'solver_selection_sha256': BUILD.file_identity(output / 'solver' / 'selection.json', MAX_RECEIPT, deadline)['sha256'],
            'full_ready': False, 'reproducibility_verified': False, 'max_total_bytes': max_total,
            'overall_timeout_seconds': seconds, 'reserve_free_bytes': reserve}
        raw_receipt = BUILD.canonical(receipt) + b'\n'
        require(len(raw_receipt) <= MAX_RECEIPT, 'collection receipt exceeds its reader limit')
        collector.budget(len(raw_receipt))
        with BUILD.deferred_signals():
            publish(output / 'collection.json', raw_receipt, MAX_RECEIPT)
            complete = True
        return receipt
    except BaseException as error:
        if output is not None:
            # Retain a bounded failure receipt and already obtained evidence; never mark it complete.
            with contextlib.suppress(BaseException):
                failure = {'schema': 1, 'kind': COLLECTION_KIND, 'complete': False,
                           'error_type': type(error).__name__, 'error': str(error)[:1024],
                           'notes': [str(note)[:1024] for note in getattr(error, '__notes__', [])[:4]],
                           'builder_approved': False,
                           'full_ready': False, 'reproducibility_verified': False,
                           'received_objects': getattr(locals().get('collector'), 'downloads', [])[-512:]}
                publish(output / 'failure.json', BUILD.canonical(failure) + b'\n', MAX_RECEIPT)
                if hasattr(error, 'add_note'):
                    error.add_note('Incomplete collection evidence retained at ' + str(output))
        raise
    finally:
        if not complete and output is not None and not (output / 'failure.json').exists():
            BUILD.cleanup_output(output)


def bind(materials_directory, candidate_path, output_path, seconds):
    require(type(seconds) is int and 0 < seconds <= MAX_SECONDS, 'invalid overall binding timeout')
    deadline = BUILD.Deadline(seconds)
    directory = BUILD.private_directory(materials_directory)
    require(not (directory / 'failure.json').exists(), 'binding refuses an incomplete collection')
    raw = BUILD.read_regular(directory / 'materials.json', BUILD.MAX_LOCK, deadline)
    materials = BUILD.decode(raw)
    BUILD.validate_materials(materials)
    require(materials['arch'] == native_arch(), 'binding requires the matching native architecture')
    old_receipt = BUILD.decode(BUILD.read_regular(directory / 'collection.json', MAX_RECEIPT, deadline))
    require(isinstance(old_receipt, dict) and old_receipt.get('kind') == COLLECTION_KIND
            and old_receipt.get('complete') is True and old_receipt.get('source_authenticated') is True
            and old_receipt.get('materials_sha256') == hashlib.sha256(raw).hexdigest()
            and old_receipt.get('builder_approved') is False, 'collection receipt is missing or inconsistent')
    expected_imports = {repo['archive']: repo['timestamp'] for repo in materials['repositories']}
    observations = old_receipt.get('imports')
    require(isinstance(observations, list) and len(observations) == len(expected_imports), 'binding import pair is incomplete')
    seen_imports = set()
    for observation in observations:
        require(isinstance(observation, dict) and observation.get('archive') in expected_imports
                and observation['archive'] not in seen_imports
                and observation.get('selected_timestamp') == expected_imports[observation['archive']],
                'binding import observation differs from exact repository timestamp')
        seen_imports.add(observation['archive'])
    for observation in observations:
        body = BUILD.checked_blob(directory / 'input-cache', observation['response'], MAX_IMPORTS, deadline)
        validate_imports(BUILD.read_regular(body, MAX_IMPORTS, deadline), observation['archive'], observation['selected_timestamp'])
    inventory, signatures = BUILD.verify_authenticated_sources(materials, directory / 'input-cache', deadline)
    candidate_bytes = BUILD.read_regular(candidate_path, BUILD.MAX_LOCK, deadline)
    candidate = verify_candidate(BUILD.decode(candidate_bytes), materials['arch'], deadline)
    lock = dict(materials, builder=candidate)
    BUILD.validate_lock(lock)
    output, complete = None, False
    try:
        with BUILD.deferred_signals():
            output = BUILD.reserve_output(output_path)
        raw_lock = BUILD.canonical(lock) + b'\n'
        publish(output / 'inputs-lock.json', raw_lock, BUILD.MAX_LOCK)
        BUILD.write_new(output / 'candidate-builder.json', candidate_bytes)
        receipt = {'schema': 1, 'kind': COLLECTION_KIND + '-binding', 'arch': materials['arch'],
                   'materials_directory': str(directory), 'materials_sha256': hashlib.sha256(raw).hexdigest(),
                   'inputs_lock_sha256': hashlib.sha256(raw_lock).hexdigest(), 'signatures': signatures,
                   'cache_directory': str(directory / 'input-cache'), 'lock_ready': True,
                   'builder_approved': False, 'runtime_image_identity_verified': False,
                   'full_ready': False, 'reproducibility_verified': False}
        publish(output / 'binding.json', BUILD.canonical(receipt) + b'\n', MAX_RECEIPT)
        complete = True
        return receipt
    finally:
        if output is not None and not complete:
            BUILD.cleanup_output(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    operations = parser.add_subparsers(dest='operation', required=True)
    public = operations.add_parser('collect')
    for name in ('request', 'keyring', 'keyring-provenance', 'output'):
        public.add_argument('--' + name, type=Path, required=True)
    public.add_argument('--candidate-builder', type=Path)
    public.add_argument('--max-total-bytes', type=int, required=True)
    public.add_argument('--timeout-seconds', type=int, required=True)
    public.add_argument('--reserve-free-bytes', type=int, default=MIN_FREE)
    binding = operations.add_parser('bind')
    binding.add_argument('--materials', type=Path, required=True)
    binding.add_argument('--candidate-builder', type=Path, required=True)
    binding.add_argument('--output', type=Path, required=True)
    binding.add_argument('--timeout-seconds', type=int, required=True)
    fetch = operations.add_parser('_fetch', help=argparse.SUPPRESS)
    fetch.add_argument('--url', required=True)
    fetch.add_argument('--output', type=Path, required=True)
    fetch.add_argument('--headers', type=Path, required=True)
    fetch.add_argument('--receipt', type=Path, required=True)
    fetch.add_argument('--limit', type=int, required=True)
    fetch.add_argument('--reserve-free-bytes', type=int, required=True)
    solver = operations.add_parser('_solver', help=argparse.SUPPRESS)
    solver.add_argument('--config', type=Path, required=True)
    solver.add_argument('--evidence', type=Path, required=True)
    solver.add_argument('--baseline', type=int, required=True)
    solver.add_argument('--operation', choices=('update', 'plan'), required=True, dest='solver_operation')
    solver.add_argument('--seeds', nargs='*', default=[])
    args = parser.parse_args()
    if args.operation == '_fetch':
        started = time.monotonic()
        try:
            result = fetch_worker(args.url, args.output, args.limit, args.headers, args.reserve_free_bytes)
        except BaseException as error:
            result = fetch_error(error, args.url, started)
            BUILD.write_new(args.receipt, BUILD.canonical(result) + b'\n')
            if isinstance(error, urllib.error.HTTPError):
                error.close()
            raise
        result.update(complete=True, elapsed_seconds=time.monotonic() - started)
        BUILD.write_new(args.receipt, BUILD.canonical(result) + b'\n')
    elif args.operation == '_solver':
        solver_worker(args.config, args.evidence, args.baseline, args.solver_operation, args.seeds)
        return
    elif args.operation == 'bind':
        result = bind(args.materials, args.candidate_builder, args.output, args.timeout_seconds)
    else:
        result = collect(args.request, args.keyring, args.keyring_provenance, args.candidate_builder,
                         args.output, args.max_total_bytes, args.timeout_seconds, args.reserve_free_bytes)
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    try:
        with BUILD.cli_signals():
            main()
    except (OSError, ValueError, subprocess.SubprocessError, urllib.error.URLError) as error:
        raise SystemExit('Collection error: ' + str(error)) from None
