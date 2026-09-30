#!/usr/bin/env python3
"""Verify the installer and real OpenRC supervision with disposable process fixtures."""

import hashlib
import http.server
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import threading
import time


ROOT = Path(__file__).resolve().parents[1]
RUNTIME_SERVICE = "sinan-singbox@main"
AGENT_STATE = Path("/var/lib/sinan/core/test-agent.pid")
RUNTIME_STATE = Path("/var/lib/sinan/plugins/sing-box@main/data/test-runtime.pid")
RELOAD_STATE = RUNTIME_STATE.with_suffix(".reloads")
TOKEN = "openrc-smoke-fixture-token"

AGENT = b'''#!/usr/bin/python3
from pathlib import Path
import os
import sys
import time

if sys.argv[1] == "--version":
    print("sinan-agent 0.2.0")
elif sys.argv[1] == "status":
    if os.environ.get("SINAN_TEST_STATUS_FAIL") == "1":
        raise SystemExit("fixture startup check failed")
    print('{"fixture":true,"connected":true,"pending_batches":0}')
elif sys.argv[1] == "enroll":
    if os.environ.get("SINAN_TEST_ENROLL_FAIL") == "1":
        raise SystemExit("fixture enrollment rejected")
    config = Path("/etc/sinan/agent.toml")
    if not config.exists():
        config.write_text('panel_url = "http://panel.example.com"\\n')
    print("fixture enrollment accepted", flush=True)
else:
    Path("/var/lib/sinan/core/test-agent.pid").write_text(str(os.getpid()))
    print("fixture agent started", flush=True)
    while True:
        time.sleep(0.1)
'''

RUNTIME = b'''#!/usr/bin/python3
from pathlib import Path
import os
import signal
import socket
import sys
import time

if sys.argv[1] == "version":
    print("sing-box version 1.14.2 (OpenRC process fixture)")
    raise SystemExit(0)
root = Path("/var/lib/sinan/plugins/sing-box@main/data")
listener = socket.socket()
listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
listener.bind(("127.0.0.1", 80))
listener.listen()
reloads = 0
def reload(signum, frame):
    global reloads
    reloads += 1
    (root / "test-runtime.reloads").write_text(str(reloads))
    print("fixture runtime reloaded", flush=True)
signal.signal(signal.SIGHUP, reload)
(root / "test-runtime.reloads").write_text("0")
(root / "test-runtime.pid").write_text(str(os.getpid()))
print("fixture runtime started", flush=True)
while True:
    time.sleep(0.1)
'''


def run(*args, **kwargs):
    result = subprocess.run(args, capture_output=True, text=True, timeout=30, **kwargs)
    if result.returncode:
        raise RuntimeError(f"{args!r}:\n{result.stdout}\n{result.stderr}")
    return result.stdout.strip()


def wait_for(check, description):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        try:
            value = check()
            if value:
                return value
        except (OSError, ValueError, RuntimeError):
            pass
        time.sleep(0.1)
    raise AssertionError(f"Timed out waiting for {description}")


def pid(path):
    number = int(path.read_text())
    return number if Path(f"/proc/{number}").exists() else None


def service(name, action):
    return run("rc-service", "--", name, action)


def install_script(origin):
    values = {
        "PANEL": f"'{origin}'",
        "TOKEN": f"'{TOKEN}'",
        "VERSION": "'0.2.0'",
        "AMD64_HASH": f"'{hashlib.sha256(AGENT).hexdigest()}'",
        "ARM64_HASH": f"'{hashlib.sha256(AGENT).hexdigest()}'",
        "AGENT_UNIT": (ROOT / "deploy/sinan-agent.service").read_text().rstrip(),
        "RUNTIME_UNIT": (ROOT / "plugins/sing-box/sinan-singbox@.service").read_text().rstrip(),
        "AGENT_OPENRC": (ROOT / "deploy/sinan-agent.openrc").read_text().rstrip(),
        "RUNTIME_OPENRC": (ROOT / "plugins/sing-box/sinan-singbox.openrc").read_text().rstrip(),
    }
    script = (ROOT / "deploy/install.sh.tmpl").read_text()
    for key, value in values.items():
        script = script.replace(f"@@{key}@@", value)
    assert "@@" not in script
    return script


def main():
    uid_map = Path("/proc/self/uid_map").read_text().split()
    isolated = Path("/.dockerenv").is_file() or uid_map[:2] != ["0", "0"]
    if os.geteuid() != 0 or os.environ.get("SINAN_OPENRC_SMOKE") != "1" or not isolated:
        raise SystemExit("Run this test in its disposable container or isolated root filesystem.")

    Path("/run/openrc").mkdir(parents=True, exist_ok=True)
    Path("/var/lib").mkdir(mode=0o755, exist_ok=True)
    Path("/run/openrc/softlevel").write_text("default\n")
    with Path("/etc/rc.conf").open("a") as config:
        config.write('\nrc_sys="docker"\nrc_cgroup_mode="none"\nrc_logger="NO"\n')
    net = Path("/etc/init.d/net")
    net.write_text("#!/sbin/openrc-run\nstart() { return 0; }\nstop() { return 0; }\n")
    net.chmod(0o755)
    service("net", "start")
    dependencies = Path("/run/openrc/deptree")
    # Keep the old cache newer than new services to reproduce missed invalidation
    # deterministically, including on slower CI runners.
    cache_time = time.time() + 60
    os.utime(dependencies, (cache_time, cache_time))

    class Download(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            assert self.path.startswith("/api/bootstrap/0.2.0/")
            assert self.path.endswith(f"?token={TOKEN}")
            self.send_response(200)
            self.send_header("Content-Length", str(len(AGENT)))
            self.end_headers()
            self.wfile.write(AGENT)

        def log_message(self, format, *args):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Download)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    script = install_script(f"http://127.0.0.1:{server.server_port}")
    environment = {**os.environ, "NO_PROXY": "127.0.0.1", "no_proxy": "127.0.0.1"}
    try:
        run("sh", input=script, env=environment)
        agent_pid = wait_for(lambda: pid(AGENT_STATE), "Agent startup after installation")
        for name in ["sinan-agent", RUNTIME_SERVICE]:
            assert f"_service='{name}'" in dependencies.read_text(), name
            assert Path(f"/etc/runlevels/default/{name}").is_symlink()
        assert not Path("/etc/systemd/system/sinan-agent.service").exists()

        runtime = Path("/opt/sinan/plugins/sing-box/current/sing-box")
        runtime.parent.mkdir(parents=True)
        runtime.parent.chmod(0o755)
        runtime.write_bytes(RUNTIME)
        runtime.chmod(0o755)
        current = Path("/var/lib/sinan/plugins/sing-box@main/current")
        current.mkdir(mode=0o755)
        current.chmod(0o755)
        configuration = current / "config.json"
        configuration.write_text("{}\n")
        configuration.chmod(0o644)
        service(RUNTIME_SERVICE, "restart")
        runtime_pid = wait_for(lambda: pid(RUNTIME_STATE), "runtime startup")
        status = Path(f"/proc/{runtime_pid}/status").read_text().splitlines()
        fields = dict(line.split(":", 1) for line in status if ":" in line)
        assert fields["Uid"].split()[0] != "0", fields["Uid"]
        assert int(fields["CapEff"].strip(), 16) == 1 << 10, fields["CapEff"]
        assert int(fields["CapAmb"].strip(), 16) == 1 << 10, fields["CapAmb"]
        assert fields["NoNewPrivs"].strip() == "1"
        assert int(Path(f"/run/{RUNTIME_SERVICE}.pid").read_text()) != runtime_pid

        identity = Path("/etc/sinan/identity/fixture-existing-key")
        identity.write_text("fixture identity must survive installation\n")
        ledger = Path("/var/lib/sinan/core/fixture-existing-ledger")
        ledger.write_text("fixture persisted accounting\n")
        run("sh", input=script, env=environment)
        agent_pid = wait_for(lambda: (number := pid(AGENT_STATE)) != agent_pid and number,
                             "Agent restart after repeated installation")
        assert pid(RUNTIME_STATE) == runtime_pid
        assert identity.read_text() == "fixture identity must survive installation\n"
        assert ledger.read_text() == "fixture persisted accounting\n"

        failed = subprocess.run(["sh"], input=script, text=True, capture_output=True, timeout=30,
                                env={**environment, "SINAN_TEST_ENROLL_FAIL": "1"})
        assert failed.returncode != 0
        assert pid(AGENT_STATE) == agent_pid
        assert pid(RUNTIME_STATE) == runtime_pid
        assert Path("/opt/sinan/core/current/sinan-agent").read_bytes() == AGENT

        service(RUNTIME_SERVICE, "reload")
        wait_for(lambda: int(RELOAD_STATE.read_text()) == 1, "HUP reaching the runtime child")
        assert pid(RUNTIME_STATE) == runtime_pid
        service("sinan-agent", "stop")
        assert pid(RUNTIME_STATE) == runtime_pid
        service(RUNTIME_SERVICE, "status")
        previous = Path("/opt/sinan/core/current").readlink()
        before_recovery = pid(AGENT_STATE)
        failed = subprocess.run(
            ["sh"], input=script, capture_output=True, text=True, timeout=30,
            env={**environment, "SINAN_TEST_STATUS_FAIL": "1"},
        )
        assert failed.returncode != 0 and "恢复上一版本" in failed.stderr
        assert Path("/opt/sinan/core/current").readlink() == previous
        wait_for(lambda: pid(AGENT_STATE) != before_recovery, "Agent recovery after failed startup")
        assert pid(RUNTIME_STATE) == runtime_pid
        service("sinan-agent", "stop")
        service("sinan-agent", "start")
        agent_pid = wait_for(lambda: (number := pid(AGENT_STATE)) != agent_pid and number,
                             "Agent starting independently")

        os.kill(runtime_pid, signal.SIGKILL)
        runtime_pid = wait_for(lambda: (number := pid(RUNTIME_STATE)) != runtime_pid and number,
                               "runtime automatic recovery")
        os.kill(agent_pid, signal.SIGKILL)
        wait_for(lambda: (number := pid(AGENT_STATE)) != agent_pid and number,
                 "Agent automatic recovery")
        assert pid(RUNTIME_STATE) == runtime_pid
        service("sinan-agent", "status")
        service(RUNTIME_SERVICE, "status")
        assert "fixture agent started" in Path("/var/log/sinan/agent.log").read_text()
        assert "fixture runtime reloaded" in Path("/var/log/sinan/runtime.log").read_text()

        with sqlite3.connect("/var/lib/sinan/core/state.db") as database:
            database.executescript('''
                CREATE TABLE kv (key TEXT, value TEXT);
                CREATE TABLE usage_outbox (epoch TEXT, seq TEXT, batch TEXT, acknowledged INTEGER);
                CREATE TABLE usage_baselines (module TEXT, stat_name TEXT, epoch TEXT,
                                             uplink TEXT, downlink TEXT, observed_at INTEGER);
                CREATE TABLE intents (completed INTEGER);
            ''')
        run("bash", "scripts/e2e-real.sh", "snapshot", "openrc-fixture", "/tmp/evidence")
        snapshot = next(Path("/tmp/evidence").iterdir())
        assert (snapshot / "init-system.txt").read_text() == "openrc\n"
        assert f"ChildPIDs={runtime_pid}" in (snapshot / "runtime-service.txt").read_text()
        assert f"SupervisorPID={Path(f'/run/{RUNTIME_SERVICE}.pid').read_text().strip()}" in (snapshot / "runtime-service.txt").read_text()

        service("sinan-agent", "stop")
        service(RUNTIME_SERVICE, "stop")
        agent_pid = int(AGENT_STATE.read_text())
        runtime_pid = int(RUNTIME_STATE.read_text())
        print(run("openrc", "default"), flush=True)
        wait_for(lambda: (number := pid(AGENT_STATE)) != agent_pid and number,
                 "Agent starting from the default runlevel")
        wait_for(lambda: (number := pid(RUNTIME_STATE)) != runtime_pid and number,
                 "runtime starting from its last configuration")

        if os.environ.get("SINAN_TEST_AGENT"):
            print(run("python3", "tools/openrc-job-smoke.py", os.environ["SINAN_TEST_AGENT"]), flush=True)

        systemctl = Path("/usr/local/bin/systemctl")
        systemctl.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> /tmp/systemctl-calls\n')
        systemctl.chmod(0o755)
        Path("/run/systemd/system").mkdir(parents=True)
        Path("/etc/systemd/system").mkdir(parents=True, exist_ok=True)
        try:
            run("sh", input=script, env=environment)
            assert Path("/tmp/systemctl-calls").read_text().splitlines() == [
                "daemon-reload", "enable sinan-singbox@main.service",
                "enable sinan-agent.service", "restart sinan-agent.service",
            ]
            assert Path("/etc/systemd/system/sinan-agent.service").read_text() == (ROOT / "deploy/sinan-agent.service").read_text()
            assert Path("/etc/systemd/system/sinan-singbox@.service").read_text() == (ROOT / "plugins/sing-box/sinan-singbox@.service").read_text()
        finally:
            systemctl.unlink()
            Path("/etc/systemd/system/sinan-agent.service").unlink(missing_ok=True)
            Path("/etc/systemd/system/sinan-singbox@.service").unlink(missing_ok=True)
        print("OpenRC installer, repeated upgrade, failed enrollment, isolated runtime, HUP, "
              "unprivileged capabilities, logs, automatic recovery, default runlevel, "
              "read-only snapshot, and systemd installer command contract: passed")
    except Exception:
        for command in [("rc-status", "--all"), ("rc-update", "show")]:
            result = subprocess.run(command, capture_output=True, text=True, timeout=30)
            print(f"{command}:\n{result.stdout}\n{result.stderr}", flush=True)
        for name in ["agent", "runtime"]:
            log = Path(f"/var/log/sinan/{name}.log")
            if log.exists():
                print(f"{log}:\n{log.read_text()}", flush=True)
        raise
    finally:
        for name in ["sinan-agent", RUNTIME_SERVICE, "net"]:
            subprocess.run(["rc-service", "--", name, "stop"], capture_output=True, timeout=30)
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
