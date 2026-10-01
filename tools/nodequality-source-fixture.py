"""Prepare private synthetic policy identities; never change production files."""
import ast
import hashlib
import importlib.util
from pathlib import Path


def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    result = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(result)
    return result


def replace_once(content, before, after):
    if content.count(before) != 1:
        raise ValueError('synthetic fixture replacement must be unique')
    return content.replace(before, after, 1)


def inert_source(name, policy):
    source = ('#!/bin/sh\n# Synthetic inert fixture: ' + name + '\n').encode()
    if name in policy.SOURCES:
        source += b'check_bash(){\n:; }\nfixture_report(){\n'
        if name == 'net.sh':
            source += policy.NET_OUTPUT
        source += policy.SOURCES[name]['original_guard'] + b'}\n'
    return source + swap_anchors(name) + dependency_anchors(name) + data_anchors(name) + ("printf '%s' '" + name + "' > \"$NQ_SOURCE_EXECUTED\"\n").encode()


def data_anchors(name):
    policy = module('fixture_data_policy', Path(__file__).resolve().parents[1] / 'plugins/nodequality/data-policy.py')
    if name not in policy.REQUESTS:
        return b''
    return b'fixture_unused_data(){\n' + b'\n'.join(policy.REQUESTS[name]) + b'\n}\n'


def undo_data(role, patched, contents):
    policy = module('fixture_undo_data', Path(__file__).resolve().parents[1] / 'plugins/nodequality/data-policy.py')
    for request, name in policy.REQUESTS.get(role, {}).items():
        patched = replace_once(patched, policy.data_command(contents[name]), request)
    return patched


def assignment(content, name, value):
    statements = [node for node in ast.parse(content).body if isinstance(node, ast.Assign)
                  and any(isinstance(target, ast.Name) and target.id == name for target in node.targets)]
    if len(statements) != 1:
        raise ValueError('fixture policy assignment must be unique')
    node = statements[0]
    lines = content.splitlines(keepends=True)
    return b''.join(lines[:node.lineno - 1]) + (name + ' = ' + repr(value) + '\n').encode() + b''.join(lines[node.end_lineno:])


def dependency_anchors(name):
    policy = module('fixture_dependency_policy', Path(__file__).resolve().parents[1] / 'plugins/nodequality/dependency-policy.py')
    if name == 'NodeQuality.sh':
        return b'fixture_unused_dependencies(){\n' + b''.join(a for a, _ in policy.ENTRY_REPLACEMENTS) + b'}\n'
    if name in policy.END_MARKERS:
        end = policy.END_MARKERS[name]
        suffix = b':\n}\n' if name == 'hardware.sh' else b')\n'
        return b'fixture_unused_dependencies(){\ninstall_dependencies(){\n:\n}\n' + end + suffix + policy.CHECK_CALL + b'}\n'
    return b''


def swap_anchors(name):
    policy = module('fixture_swap_policy', Path(__file__).resolve().parents[1] / 'plugins/nodequality/swap-policy.py')
    if name == 'NodeQuality.sh':
        return b'fixture_unused_entry_swaps(){\n' + b''.join(a for a, _ in policy.ENTRY_REPLACEMENTS) + b'}\n'
    if name == 'hardware.sh':
        return policy.HARDWARE_PREFIX + b':\ncleanup_local(){\n' + policy.SWAP_CLEANUP + b':\n}\n}\n'
    return b''


def undo_dependencies(role, patched, original):
    policy = module('fixture_undo_dependencies', Path(__file__).resolve().parents[1] / 'plugins/nodequality/dependency-policy.py')
    if role == 'NodeQuality.sh':
        for before, after in reversed(policy.ENTRY_REPLACEMENTS):
            patched = replace_once(patched, after, before)
        return patched
    start, stop = policy.installer_span(role, original)
    patched = replace_once(patched, policy.checks(role), original[start:stop])
    return replace_once(patched, policy.REQUIRED_CALL, policy.CHECK_CALL)


def prepare_policy(plugin, contents):
    """Caller owns a private copy of the plugin tree and all synthetic inputs."""
    path = Path(plugin) / 'report-policy.py'
    policy = module('synthetic_report_policy_input', path)
    content = path.read_bytes()
    original_helper_hash = hashlib.sha256(content).hexdigest().encode()
    outputs = {}
    for role, spec in policy.SOURCES.items():
        canonical = contents[role]
        patched = policy.exact_replace(canonical, b'check_bash(){\n', policy.POLICY + b'check_bash(){\n')
        patched = policy.exact_replace(patched, spec['original_guard'], spec['patched_guard'])
        if role == 'net.sh':
            patched = policy.exact_replace(patched, policy.NET_OUTPUT, b'local report_link=""\n' + policy.NET_OUTPUT)
        content = replace_once(content, spec['source_sha256'].encode(), hashlib.sha256(canonical).hexdigest().encode())
        content = replace_once(content, spec['patched_sha256'].encode(), hashlib.sha256(patched).hexdigest().encode())
        outputs[role] = patched
    path.write_bytes(content)
    helper = Path(plugin) / 'source-helper.py'
    helper.write_bytes(replace_once(helper.read_bytes(), original_helper_hash, hashlib.sha256(content).hexdigest().encode()))
    swap_path = Path(plugin) / 'swap-policy.py'
    swap = module('synthetic_swap_policy_input', swap_path)
    content = swap_path.read_bytes()
    original_swap_hash = hashlib.sha256(content).hexdigest().encode()
    for role, spec in swap.SOURCES.items():
        canonical = outputs.get(role, contents[role])
        patched = swap.patch(role, canonical)
        content = replace_once(content, spec['source_sha256'].encode(), hashlib.sha256(canonical).hexdigest().encode())
        content = replace_once(content, spec['patched_sha256'].encode(), hashlib.sha256(patched).hexdigest().encode())
        outputs[role] = patched
    swap_path.write_bytes(content)
    helper.write_bytes(replace_once(helper.read_bytes(), original_swap_hash, hashlib.sha256(content).hexdigest().encode()))
    dependency_path = Path(plugin) / 'dependency-policy.py'
    dependency = module('synthetic_dependency_policy_input', dependency_path)
    content = dependency_path.read_bytes()
    original_hash = hashlib.sha256(content).hexdigest().encode()
    for role, spec in dependency.SOURCES.items():
        canonical = outputs.get(role, contents[role])
        patched = dependency.patch(role, canonical)
        content = replace_once(content, spec['source_sha256'].encode(), hashlib.sha256(canonical).hexdigest().encode())
        content = replace_once(content, spec['patched_sha256'].encode(), hashlib.sha256(patched).hexdigest().encode())
        outputs[role] = patched
    dependency_path.write_bytes(content)
    helper.write_bytes(replace_once(helper.read_bytes(), original_hash, hashlib.sha256(content).hexdigest().encode()))
    data_path = Path(plugin) / 'data-policy.py'
    data_policy = module('synthetic_data_policy_input', data_path)
    content = data_path.read_bytes()
    original_hash = hashlib.sha256(content).hexdigest().encode()
    identities = {name: {'sha256': hashlib.sha256(contents[name]).hexdigest(), 'size': len(contents[name])}
                  for name in data_policy.DATA}
    content = assignment(content, 'DATA', identities)
    for role, spec in data_policy.SOURCES.items():
        canonical = outputs[role]
        patched = data_policy.patch(role, canonical, {name: contents[name] for name in data_policy.REQUESTS[role].values()})
        content = replace_once(content, spec['source_sha256'].encode(), hashlib.sha256(canonical).hexdigest().encode())
        content = replace_once(content, spec['patched_sha256'].encode(), hashlib.sha256(patched).hexdigest().encode())
        outputs[role] = patched
    data_path.write_bytes(content)
    helper.write_bytes(replace_once(helper.read_bytes(), original_hash, hashlib.sha256(content).hexdigest().encode()))
    return outputs
