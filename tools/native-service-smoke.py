#!/usr/bin/env python3
"""Verify native services and real runtime reconciliation on disposable CI hosts only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import runpy
import shutil
import socket
import subprocess
import sys
import time
import uuid

helpers = runpy.run_path(str(Path(__file__).with_name('agent-smoke.py')))
Panel, invoke, wait_for, status = (helpers[name] for name in ('Panel', 'invoke', 'wait_for', 'status'))
SYSTEM = platform.system()


def command(args, check=True):
    result = subprocess.run(args, capture_output=True, text=True, encoding='utf-8', errors='replace', timeout=90)
    if check and result.returncode:
        raise RuntimeError(f'{args[0]} failed: {result.stdout}\n{result.stderr}')
    return result


def powershell(script, check=True):
    return command(['powershell.exe', '-NoProfile', '-NonInteractive', '-Command', "$ErrorActionPreference='Stop'; " + script], check)


def service(name, action):
    if SYSTEM == 'Darwin':
        label = 'system/org.sinan.' + name.replace('@', '.')
        args = ['launchctl', 'kickstart', '-k', label] if action == 'restart' else ['launchctl', 'bootout', label]
    elif SYSTEM == 'FreeBSD':
        args = ['service', name.replace('-', '_').replace('@', '_'), action]
    else:
        script = f"Stop-ScheduledTask -TaskName '{name}'"
        if action == 'restart':
            script += f"; Start-Sleep -Seconds 1; Start-ScheduledTask -TaskName '{name}'"
        powershell(script)
        return
    command(args)


def port():
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', 0))
        return listener.getsockname()[1]


def transfer(port_number):
    # Plain loopback VLESS is only a transport fixture; production uses Reality.
    with socket.socket() as echo:
        echo.settimeout(5)
        echo.bind(('127.0.0.1', 0))
        echo.listen()
        with socket.create_connection(('127.0.0.1', port_number), timeout=5) as proxy:
            body = bytes([42]) * 1024
            proxy.sendall(b'\0' + uuid.UUID(int=1).bytes + b'\0\1' +
                          echo.getsockname()[1].to_bytes(2, 'big') + b'\1\x7f\0\0\1' + body)
            stream, _ = echo.accept()
            with stream:
                received = bytearray()
                while len(received) < len(body):
                    received.extend(stream.recv(len(body) - len(received)))
                assert bytes(received) == body
                stream.sendall(body)
            received = bytearray()
            while len(received) < 1026:
                chunk = proxy.recv(1026 - len(received))
                assert chunk
                received.extend(chunk)
            assert received == b'\0\0' + body


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('runtime_archive', type=Path)
    args = parser.parse_args()
    if os.environ.get('CI') != 'true' or SYSTEM not in ('Darwin', 'FreeBSD', 'Windows'):
        parser.error('requires a disposable native CI runner')
    binary = args.binary.resolve()
    archive = args.runtime_archive.read_bytes()
    root = (Path(os.environ['ProgramData']) if SYSTEM == 'Windows' else Path('/opt')) / ('sinan-test-' + uuid.uuid4().hex[:8])
    root.mkdir(mode=0o755)
    os.environ['NO_PROXY'] = os.environ['no_proxy'] = '127.0.0.1,localhost'
    panel = Panel()
    config = helpers['configure'](root, panel.origin)
    # Native permission updates and service operations involve multiple OS commands.
    config.write_text(config.read_text().replace('operation_timeout_secs = 5', 'operation_timeout_secs = 120').replace(json.dumps(str(root / 'state.db')), json.dumps(str(root / 'state/core/state.db'))).replace(json.dumps(str(root / 'status.sock')), json.dumps(str(root / 'state/core/status.sock'))))
    proxy_port, stats_port = port(), port()
    native = dict(log=dict(level='error'), inbounds=[dict(type='vless', tag='fixture', listen='127.0.0.1',
                  listen_port=proxy_port, users=[dict(name='u1_n1', uuid=str(uuid.UUID(int=1)))])],
                  outbounds=[dict(type='direct', tag='direct')], route=dict(final='direct'),
                  experimental=dict(v2ray_api=dict(listen=f'127.0.0.1:{stats_port}', stats=dict(enabled=True, users=['u1_n1']))))
    bundle = json.dumps(dict(files={'config.json': json.dumps(native)}), separators=(',', ':')).encode()
    panel.downloads = {'/fixture/runtime': archive, '/fixture/bundle': bundle}
    module = dict(kernel_version='1.14.2', artifact=dict(url=panel.origin + '/fixture/runtime', sha256=hashlib.sha256(archive).hexdigest()),
                  config_rev=1, bundle_url=panel.origin + '/fixture/bundle', bundle_sha256=hashlib.sha256(bundle).hexdigest(), stats_listen=f'127.0.0.1:{stats_port}')
    try:
        print(invoke(binary, config, 'enroll', '--panel', panel.origin, '--token', 'smoke-enrollment'))
        # Publish after services are registered so launchd's eager startup cannot race the fixture.
        print(invoke(binary, config, 'install-service'))
        panel.manifest = dict(rev=1, modules={'singbox': module})
        def applied():
            info = status(binary, config)
            return info and info['healthy'].get('singbox') and info['applied'].get('singbox') == 1
        wait_for(applied, 'native runtime reconciliation', 180)
        transfer(proxy_port)
        old = status(binary, config)['pid']
        service('sinan-agent', 'restart')
        wait_for(lambda: (s := status(binary, config)) and s['pid'] != old and s['connected'], 'Agent service restart', 90)
        transfer(proxy_port)
        # Reinstalling must preserve identity, service independence and immutable artifacts.
        identity = (root / 'identity/device.key').read_bytes()
        print(invoke(binary, config, 'install-service'))
        assert (root / 'identity/device.key').read_bytes() == identity
        transfer(proxy_port)
        service('sinan-agent', 'stop')
        wait_for(lambda: status(binary, config) is None, 'Agent service shutdown', 30)
        transfer(proxy_port)
        if SYSTEM == 'Darwin':
            data = command(['launchctl', 'print', 'system/org.sinan.sinan-singbox.main']).stdout
            assert 'sinan-singbox' in data
        elif SYSTEM == 'FreeBSD':
            assert command(['sysrc', '-n', 'sinan_agent_enable']).stdout.strip() == 'YES'
        else:
            powershell("$t=Get-ScheduledTask -TaskName 'sinan-singbox@main'; if ($t.Principal.UserId -notmatch 'sinan-singbox') { throw 'Runtime account mismatch' }; if ($t.Triggers.Count -ne 1) { throw 'Missing startup trigger' }")
        print('Native services: startup registration, privilege separation, runtime artifact/configuration, traffic, Agent restart/reinstall and independent runtime passed')
    except BaseException as error:
        if isinstance(error, subprocess.CalledProcessError):
            print(error.stdout, error.stderr)
        for path in [root / 'core/update-state.json', Path('/var/log/sinan-agent.log'), Path('/var/log/sinan-singbox@main.log')]:
            if path.exists():
                print(path, path.read_text(errors='replace')[-12000:])
        print('Apply results:', json.dumps([m for m in panel.messages if m['type'] == 'apply.result']))
        if SYSTEM == 'Darwin':
            print(command(['launchctl', 'print', 'system/org.sinan.sinan-singbox.main'], False).stdout)
            print(command(['id', 'sinan-singbox'], False).stdout)
            print(command(['ls', '-lde', str(root), str(root / 'runtime'), str(root / 'runtime/sing-box@main/data'), str(root / 'core/runtime-launcher.sh')], False).stdout)
        if SYSTEM == 'FreeBSD':
            print(command(['service', 'sinan_agent', 'onestatus'], False).stdout)
            print(command(['service', 'sinan_singbox_main', 'onestatus'], False).stdout)
            messages = Path('/var/log/messages')
            if messages.exists():
                print(messages.read_text(errors='replace')[-6000:])
        if SYSTEM == 'Windows':
            print(powershell("Get-ScheduledTaskInfo -TaskName 'sinan-agent'; Get-ScheduledTaskInfo -TaskName 'sinan-singbox@main'", False).stdout)
        raise
    finally:
        for name in ('sinan-agent', 'sinan-singbox@main'):
            try:
                service(name, 'stop')
            except (RuntimeError, subprocess.TimeoutExpired):
                pass
            if SYSTEM == 'Windows':
                powershell(f"Unregister-ScheduledTask -TaskName '{name}' -Confirm:$false", False)
            elif SYSTEM == 'Darwin':
                Path('/Library/LaunchDaemons', 'org.sinan.' + name.replace('@', '.') + '.plist').unlink(missing_ok=True)
            else:
                Path('/usr/local/etc/rc.d', name.replace('-', '_').replace('@', '_')).unlink(missing_ok=True)
        panel.shutdown()
        panel.server_close()
        shutil.rmtree(root)


if __name__ == '__main__':
    main()
