#!/usr/bin/env bash
# Install on a disposable Ubuntu host; remove only resources created by this run.
set -euo pipefail
umask 077

usage() {
  printf '%s\n' 'Usage: scripts/ci-real-e2e.sh --agent-root DIR --runtime-root DIR --summary FILE'
}
die() { printf 'Real e2e failed: %s\n' "$*" >&2; exit 1; }
agent_root= runtime_root= summary=
while [[ $# -gt 0 ]]; do
  case "$1" in
    --agent-root) agent_root=$2; shift 2 ;;
    --runtime-root) runtime_root=$2; shift 2 ;;
    --summary) summary=$2; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
done
[[ -n $agent_root && -n $runtime_root && -n $summary ]] || { usage >&2; exit 2; }
[[ $(uname -s) == Linux && $(uname -m) == x86_64 && -d /run/systemd/system ]] || die 'requires Linux amd64 with real systemd'
[[ ${GITHUB_ACTIONS:-} == true || ${SINAN_E2E_DISPOSABLE_HOST:-} == 1 ]] || die 'use a disposable runner; manual reproduction requires SINAN_E2E_DISPOSABLE_HOST=1'
for tool in docker python3 sudo systemctl openssl curl readelf go sha256sum; do
  command -v "$tool" >/dev/null || die "missing tool: $tool"
done
sudo -n true || die 'noninteractive sudo is required'
for path in /etc/sinan /opt/sinan /var/lib/sinan /run/sinan /usr/local/bin/sinan-agent \
  /etc/systemd/system/sinan-agent.service /etc/systemd/system/sinan-singbox@.service; do
  if sudo test -e "$path" || sudo test -L "$path"; then die 'existing Sinan installation detected; leaving it untouched'; fi
done
if getent passwd sinan-singbox >/dev/null || getent group sinan-singbox >/dev/null; then
  die 'existing runtime identity detected; leaving it untouched'
fi
repository=$(cd "$(dirname "$0")/.." && pwd -P)
cd "$repository"
scratch=$(mktemp -d "${RUNNER_TEMP:-/tmp}/sinan-real-e2e.XXXXXX")
export COMPOSE_PROJECT_NAME="sinan-real-e2e-${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}-$$"
export SINAN_BIND_ADDRESS=127.0.0.1
export SINAN_PORT=18080
export SINAN_PUBLIC_URL=http://127.0.0.1:18080
export SINAN_DB_PASSWORD=$(python3 -c 'import secrets; print(secrets.token_hex(32))')
export SINAN_ADMIN_PASSWORD=$(python3 -c 'import secrets; print(secrets.token_hex(32))')
export SINAN_E2E_ADMIN_PASSWORD="$SINAN_ADMIN_PASSWORD"
export SINAN_E2E_STATUS_COMMAND='sudo /usr/local/bin/sinan-agent status'
export SINAN_E2E_ARTIFACT_ROOT=$scratch/artifacts
phase=preflight owned_installation=0 hosts_entry=0 fixture_pid= client_pid= tls_pid=
passed=0
marker=$COMPOSE_PROJECT_NAME
compose=(docker compose --env-file /dev/null -f deploy/docker-compose.yml -f "$scratch/compose.override.yml")

write_summary() {
  python3 - "$scratch/state.json" "$summary" "$passed" "$phase" "${result:-0}" <<'PY'
import json
import os
from pathlib import Path
import sys

state_path, output, passed, phase, exit_code = sys.argv[1:]
summary = {"passed": passed == "1", "last_phase": phase, "exit_code": int(exit_code), "runtime_version": "1.14.2"}
if Path(state_path).is_file():
    state = json.loads(Path(state_path).read_text())
    summary["agent_version"] = state.get("installation", {}).get("version")
    summary["usage"] = {
        label: {key: checkpoint["usage"][key] for key in ("uplink", "downlink", "total")}
        for label, checkpoint in state.get("checkpoints", {}).items()
        if label in ("ready", "first", "before", "after-agent", "after-runtime", "resumed", "before-reinstall", "after-reinstall")
    }
ledger = Path(state_path).with_name("ledger-summary.json")
if ledger.is_file():
    summary["ledger"] = {
        label: {key: counters[key] for key in ("uplink", "downlink", "total")}
        for label, counters in json.loads(ledger.read_text()).items()
        if label in ("before", "after-agent", "after-runtime", "before-reinstall", "after-reinstall")
    }
Path(output).parent.mkdir(parents=True, exist_ok=True)
Path(output).write_text(json.dumps(summary, indent=2) + "\n")
if os.environ.get('GITHUB_STEP_SUMMARY'):
    with Path(os.environ['GITHUB_STEP_SUMMARY']).open('a') as report:
        report.write('### 真实安装与 Reality 验收\n\n')
        report.write(f"结果：{'通过' if summary['passed'] else '失败'}；最后阶段：{phase}。\n\n")
        report.write('| 阶段 | 上传字节 | 下载字节 |\n| --- | ---: | ---: |\n')
        for label, usage in summary.get('usage', {}).items():
            report.write(f"| {label} | {usage['uplink']} | {usage['downlink']} |\n")
PY
}

cleanup() {
  result=$?
  trap - EXIT
  write_summary || true
  [[ -z $client_pid ]] || { kill "$client_pid" 2>/dev/null || true; wait "$client_pid" 2>/dev/null || true; }
  [[ -z $fixture_pid ]] || { kill "$fixture_pid" 2>/dev/null || true; wait "$fixture_pid" 2>/dev/null || true; }
  if [[ -f $scratch/tls.pid ]]; then
    tls_pid=$(sudo cat "$scratch/tls.pid")
    [[ $tls_pid =~ ^[0-9]+$ ]] && sudo kill "$tls_pid" 2>/dev/null || true
  fi
  if [[ $owned_installation == 1 ]]; then
    sudo systemctl disable --now sinan-agent.service sinan-singbox@main.service >/dev/null 2>&1 || true
    sudo rm -f /etc/systemd/system/sinan-agent.service /etc/systemd/system/sinan-singbox@.service /usr/local/bin/sinan-agent
    sudo rm -rf /etc/sinan /opt/sinan /var/lib/sinan /run/sinan
    sudo systemctl daemon-reload
    sudo systemctl reset-failed sinan-agent.service sinan-singbox@main.service >/dev/null 2>&1 || true
    if getent passwd sinan-singbox >/dev/null; then sudo userdel sinan-singbox; fi
    if getent group sinan-singbox >/dev/null; then sudo groupdel sinan-singbox; fi
  fi
  if [[ $hosts_entry == 1 ]]; then
    sudo python3 - "$marker" <<'PY'
from pathlib import Path
import sys
path = Path('/etc/hosts')
marker = '# ' + sys.argv[1]
path.write_text(''.join(line for line in path.read_text().splitlines(keepends=True) if not line.rstrip().endswith(marker)))
PY
  fi
  if [[ -f $scratch/compose.override.yml ]]; then
    "${compose[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
    docker image rm "$COMPOSE_PROJECT_NAME-panel:local" >/dev/null 2>&1 || true
  fi
  sudo rm -rf -- "$scratch"
  if [[ $result != 0 ]]; then printf 'Real e2e failed during %s; only the redacted summary is public.\n' "$phase" >&2; fi
  exit "$result"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

sudo python3 - <<'PY'
import socket
for port in (443, 2080, 18080, 18081, 18085, 20000):
    with socket.socket() as listener:
        listener.bind(('127.0.0.1', port))
PY

mkdir -m 0755 "$scratch/artifacts"
cp -R "$agent_root/agent" "$scratch/artifacts/agent"
cp -R "$runtime_root/sing-box" "$scratch/artifacts/sing-box"
chmod -R a+rX "$scratch/artifacts"
agent_version=$(python3 - "$scratch/artifacts/agent" <<'PY'
import hashlib
from pathlib import Path
import re
import subprocess
import sys
root = Path(sys.argv[1])
versions = [path for path in root.iterdir() if path.is_dir() and not path.is_symlink()]
if len(versions) != 1: raise SystemExit('need one same-run Agent version')
version = versions[0].name
if not re.fullmatch(r'\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?', version): raise SystemExit('invalid version')
binary = versions[0] / 'amd64'
with binary.open('rb') as source: actual = hashlib.file_digest(source, 'sha256').hexdigest()
matches = [line.split()[0].lower() for line in (versions[0] / 'SHA256SUMS').read_text().splitlines()
           if re.fullmatch(r'[0-9a-fA-F]{64} [ *]amd64', line)]
if matches != [actual]: raise SystemExit('Agent checksum mismatch')
binary.chmod(0o755)
header = subprocess.check_output(['readelf', '-h', str(binary)], text=True)
if 'Advanced Micro Devices X86-64' not in header: raise SystemExit('Agent architecture mismatch')
program = subprocess.check_output(['readelf', '-lW', str(binary)], text=True)
dynamic = subprocess.check_output(['readelf', '-dW', str(binary)], text=True)
if 'INTERP' in program or 'NEEDED' in dynamic: raise SystemExit('Agent must be static musl')
if subprocess.check_output([str(binary), '--version'], text=True).strip() != 'sinan-agent ' + version:
    raise SystemExit('Agent version mismatch')
print(version)
PY
)
python3 scripts/verify-e2e-runtime.py "$runtime_root" "$scratch/runtime"
runtime=$scratch/runtime/sing-box
cat > "$scratch/compose.override.yml" <<'YAML'
services:
  panel:
    image: ${COMPOSE_PROJECT_NAME}-panel:local
    volumes:
      - ${SINAN_E2E_ARTIFACT_ROOT}:/data/artifacts:ro
YAML

phase=compose
"${compose[@]}" up --detach --build --wait --wait-timeout 240 > "$scratch/compose.log" 2>&1

phase=fixtures
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes -days 1 \
  -keyout "$scratch/camouflage.key" -out "$scratch/camouflage.crt" \
  -subj '/CN=sinan-e2e.example.test' -addext 'subjectAltName=DNS:sinan-e2e.example.test' \
  > "$scratch/certificate.log" 2>&1
sudo sh -c 'printf "%s\n" "$$" > "$1"; exec openssl s_server -quiet -www -tls1_3 -groups X25519 -accept 127.0.0.1:443 -cert "$2" -key "$3"' \
  sh "$scratch/tls.pid" "$scratch/camouflage.crt" "$scratch/camouflage.key" > "$scratch/tls.log" 2>&1 &
sudo python3 - "$marker" <<'PY'
from pathlib import Path
import sys
with Path('/etc/hosts').open('a') as output:
    output.write('\n127.0.0.1 sinan-e2e.example.test # ' + sys.argv[1] + '\n')
PY
hosts_entry=1
python3 scripts/e2e-http-fixture.py --listen 127.0.0.1 --port 18081 > "$scratch/fixture.log" 2>&1 &
fixture_pid=$!

phase=prepare
driver=(python3 scripts/e2e-driver.py --state "$scratch/state.json")
"${driver[@]}" prepare --origin "$SINAN_PUBLIC_URL" --public-host 127.0.0.1 --sni sinan-e2e.example.test
phase=install
owned_installation=1
sudo sh "$scratch/install.sh" > "$scratch/install.log" 2>&1
"${driver[@]}" ready --agent-version "$agent_version" --timeout 600
runtime_pid=$(sudo systemctl show sinan-singbox@main.service -p MainPID --value)
[[ $runtime_pid =~ ^[1-9][0-9]*$ ]] || die 'independent runtime is not active'

wait_agent() {
  python3 - <<'PY'
import json
import subprocess
import time
deadline = time.monotonic() + 120
while time.monotonic() < deadline:
    try:
        process = subprocess.run(['sudo', '/usr/local/bin/sinan-agent', 'status'],
                                 check=True, capture_output=True, text=True, timeout=10)
        value = json.loads(process.stdout)
        if (value.get('connected') is True and value.get('healthy', {}).get('singbox') is True
                and value.get('applied', {}).get('singbox', 0) > 0):
            break
    except (subprocess.SubprocessError, ValueError):
        pass
    time.sleep(2)
else:
    raise SystemExit('Agent did not restore its live status connection')
PY
}

# Read only this run's configured user/node baseline, never identity material.
assert_ledger() {
  sudo python3 - "$scratch/state.json" "$1" "${2:-}" > "$scratch/ledger-next.json" <<'PY_LEDGER'
import json
from pathlib import Path
import sqlite3
import sys

state_path, label, previous = sys.argv[1:]
state = json.loads(Path(state_path).read_text())
name = f"u{state['user_id']}_n{state['node_id']}"
with sqlite3.connect('file:/var/lib/sinan/core/state.db?mode=ro', uri=True, timeout=5) as connection:
    connection.execute('BEGIN')
    rows = connection.execute('SELECT uplink,downlink FROM usage_baselines WHERE module=? AND stat_name=?', ('singbox', name)).fetchall()
    pending = connection.execute('SELECT COUNT(*) FROM usage_outbox WHERE acknowledged=0').fetchone()[0]
if len(rows) != 1 or pending != 0:
    raise SystemExit('Sampled ledger baseline missing or still awaiting acknowledgement')
current = tuple(int(value) for value in rows[0])
path = Path(state_path).with_name('ledger-summary.json')
proof = json.loads(path.read_text()) if path.is_file() else {}
checkpoint = state['checkpoints'][label]['usage']
actual = tuple(int(checkpoint[key]) for key in ('uplink', 'downlink'))
if previous == 'zero':
    if current != (0, 0):
        raise SystemExit('Runtime reload did not establish sampled zero counters')
elif previous:
    baseline = proof[previous]
    previous_usage = state['checkpoints'][previous]['usage']
    delta = tuple(actual[index] - int(previous_usage[key]) for index, key in enumerate(('uplink', 'downlink')))
    expected = tuple(current[index] - int(baseline[key]) for index, key in enumerate(('uplink', 'downlink')))
    if delta != expected or any(value < 0 for value in expected):
        raise SystemExit('Panel increment differs from sampled runtime ledger increment')
elif actual != current:
    raise SystemExit('Panel totals differ from sampled runtime ledger totals')
proof[label] = {key: str(value) for key, value in zip(('uplink', 'downlink', 'total'), (*current, sum(current)))}
print(json.dumps(proof))
PY_LEDGER
  mv "$scratch/ledger-next.json" "$scratch/ledger-summary.json"
  printf 'Panel and sampled runtime ledger counters agree: %s\n' "$1"
}

start_client() {
  "$runtime" run -c "$scratch/client.json" > "$scratch/client.log" 2>&1 &
  client_pid=$!
  for attempt in {1..30}; do
    kill -0 "$client_pid" 2>/dev/null || die 'Reality client exited'
    if python3 - <<'PY'
import socket
try:
    with socket.create_connection(('127.0.0.1', 2080), timeout=1): pass
except OSError:
    raise SystemExit(1)
PY
    then return; fi
    sleep 1
  done
  die 'Reality client did not listen'
}
stop_client() { kill "$client_pid"; wait "$client_pid" || true; client_pid=; }
traffic_batch() {
  curl --fail --silent --show-error --max-time 90 --noproxy '' \
    --proxy socks5h://127.0.0.1:2080 http://127.0.0.1:18081/download -o "$scratch/download.bin"
  python3 - "$scratch/download.bin" "$scratch/upload.bin" <<'PY'
from pathlib import Path
import sys
download = Path(sys.argv[1]).read_bytes()
if download != b's' * (2 * 1024 * 1024): raise SystemExit('proxy download mismatch')
Path(sys.argv[2]).write_bytes(b'u' * (1024 * 1024))
PY
  curl --fail --silent --show-error --max-time 90 --noproxy '' \
    --proxy socks5h://127.0.0.1:2080 -X POST --data-binary "@$scratch/upload.bin" \
    http://127.0.0.1:18081/upload -o "$scratch/upload-response.txt"
  [[ $(cat "$scratch/upload-response.txt") == 1048576 ]] || die 'proxy upload mismatch'
}

phase=first-traffic
start_client
traffic_batch
"${driver[@]}" traffic --label first --after ready --min-uplink 1048576 --min-downlink 2097152
stop_client
phase=stable-before
"${driver[@]}" verify --label before --interval 35 --timeout 240
assert_ledger before
phase=agent-restart
sudo systemctl restart sinan-agent.service
wait_agent
"${driver[@]}" verify --label after-agent --unchanged-from before --interval 35 --timeout 240
assert_ledger after-agent
[[ $(sudo systemctl show sinan-singbox@main.service -p MainPID --value) == "$runtime_pid" ]] || die 'Agent restart changed independent runtime PID'
phase=runtime-reload
sudo systemctl reload sinan-singbox@main.service
wait_agent
"${driver[@]}" verify --label after-runtime --unchanged-from before --interval 35 --timeout 240
assert_ledger after-runtime zero
phase=resumed-traffic
start_client
traffic_batch
"${driver[@]}" traffic --label resumed --after after-runtime --min-uplink 1048576 --min-downlink 2097152
stop_client
"${driver[@]}" verify --label before-reinstall --interval 35 --timeout 240
assert_ledger before-reinstall after-runtime
phase=reinstall
sudo find /etc/sinan/identity -maxdepth 1 -type f -exec sha256sum {} + | sort > "$scratch/identity-before.txt"
"${driver[@]}" install --refresh
sudo sh "$scratch/install.sh" > "$scratch/reinstall.log" 2>&1
wait_agent
"${driver[@]}" ready --agent-version "$agent_version" --timeout 600
"${driver[@]}" verify --label after-reinstall --unchanged-from before-reinstall --interval 35 --timeout 240
assert_ledger after-reinstall after-runtime
sudo find /etc/sinan/identity -maxdepth 1 -type f -exec sha256sum {} + | sort > "$scratch/identity-after.txt"
cmp "$scratch/identity-before.txt" "$scratch/identity-after.txt" || die 'reinstallation changed device identity files'
[[ $(sudo systemctl show sinan-singbox@main.service -p MainPID --value) == "$runtime_pid" ]] || die 'same-version reinstallation changed runtime PID'
phase=complete
passed=1
printf '%s\n' 'Real installation, Reality bidirectional traffic, accounting, restart, reload, and reinstallation passed.'
