"""Prepare private synthetic policy identities; never change production files."""
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
    return source + ("printf '%s' '" + name + "' > \"$NQ_SOURCE_EXECUTED\"\n").encode()


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
    return outputs
