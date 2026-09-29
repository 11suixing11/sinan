#!/usr/bin/env bash
# Print the manual acceptance guide or collect read-only device evidence.
set -euo pipefail
umask 077

usage() {
  cat <<'TEXT'
用法：
  bash scripts/e2e-real.sh guide
  sudo bash scripts/e2e-real.sh snapshot <标签> <证据目录>

guide 仅打印完整人工流程；snapshot 只读收集本机状态，不安装、重启或修改服务。

snapshot 可选环境变量（前三项须同时提供）：
  SINAN_PANEL_URL=https://panel.example.com
  SINAN_USER_ID=1
  SINAN_NODE_ID=1
  SINAN_STATE_DB=/var/lib/sinan/core/state.db

提供面板参数后会隐藏输入管理员密码，将按用户+节点筛选的用量和部署状态
保存到本次证据目录；不会保存密码、会话、私钥或完整配置包。
使用 sudo 时按需 --preserve-env=SINAN_PANEL_URL,SINAN_USER_ID,SINAN_NODE_ID。
TEXT
}

guide() {
  cat <<'TEXT'
司南 Debian 12 人工端到端验收
============================
本脚本不是自动验收结果。请记录每一步的时间、版本、结果及失败日志。
准备一台全新 Debian 12 amd64/arm64 服务器、一台独立客户端和可访问的面板。
不要把 macOS 单测、虚拟服务测试或上游 check 成功当作下面的实机流程已通过。

一、面板、制品与接入
1. 按 README 启动面板+PostgreSQL；公开地址必须从设备和客户端都可访问。
   登录后在制品页确认匹配设备架构的 Agent 和运行时均通过 SHA-256 校验。
2. 在 Debian 设备安装基础工具：
     apt-get update
     apt-get install -y ca-certificates curl coreutils passwd iproute2 python3
3. 在面板添加服务器，把新生成的安装命令复制到设备，以 root 执行。
   令牌只使用一次；不得把另一个设备的身份复制过来。
4. 30 秒内确认服务器在线，记录设备架构、Agent 版本、CPU/内存和更新时间。
   在设备运行：
     sinan-agent status
     systemctl status sinan-agent.service --no-pager
   此时尚未发布配置，代理运行时可以尚未启动。

二、节点、用户、客户端与流量
5. 新建 VLESS + Reality 节点，填写客户端可达的公开地址和可用的伪装域名。
   记录分配端口；确认服务器/服务商已有网络规则允许该端口访问。
6. 创建一个专用验收用户，仅授权此节点。等待 5 秒合并发布及设备应用完成。
   服务器详情：目标版本=已应用版本、健康=true、无本次部署错误。
   设备：
     systemctl is-active sinan-singbox@main.service
     /opt/sinan/plugins/sing-box/current/sing-box version
     ss -lnt 'sport = :18085'
   版本应为 1.14.2，包含 with_v2ray_api；18085 必须只在 127.0.0.1 监听。
7. 在独立客户端导入此用户的订阅，不要在服务器内使用直连代替代理链路。
   使用 sing-box JSON 时，本地混合入口是 127.0.0.1:2080。示例：
     curl --fail --proxy socks5h://127.0.0.1:2080 https://example.com/
   再使用自己控制的测试端点产生可识别的下载和上传；记录客户端实际完成量。
   在客户端确认请求确实经所选代理节点出站，并按需检查出站地址。
8. 等待 1 至 2 分钟，在用户页“该用户的节点流量”验证上传/下载均有合理增量。
   不要求统计字节与 HTTP 文件大小完全相等；记录代理统计口径和协议开销。

三、稳定基线与 Agent 重启
9. 暂停验收客户端，关闭其他使用本验收用户的连接。
   等待至少两次采样，确认面板该用户该节点用量稳定且 pending_batches=0。
   可先设置以下环境（将示例改为本次实际值）：
     export SINAN_PANEL_URL=https://panel.example.com
     export SINAN_USER_ID=1
     export SINAN_NODE_ID=1
     bash scripts/e2e-real.sh snapshot before ./evidence
10. 记录运行时 MainPID，再重启 Agent：
      systemctl show sinan-singbox@main.service -p MainPID
      systemctl restart sinan-agent.service
    等待 Agent 恢复连接后再次查询运行时 MainPID，应保持不变；代理为独立服务。
    等待两次采样并收集：
      bash scripts/e2e-real.sh snapshot after-agent ./evidence
    检查已确认用量未倒退、未因重启翻倍；无新增流量时稳定总数应不变。

四、运行时重载与再次使用
11. 保持客户端暂停并确认待确认批次清零后，执行一次显式运行时重载：
      systemctl reload sinan-singbox@main.service
    等待本地统计接口恢复；检查日志中有无重载失败、统计重置/缺失窗口告警：
      journalctl -u sinan-agent.service --since '-5 minutes' --no-pager
      journalctl -u sinan-singbox@main.service --since '-5 minutes' --no-pager
    再收集：
      bash scripts/e2e-real.sh snapshot after-runtime ./evidence
    外部 HUP 不经过 Agent 的终值采样；若重载时仍有流量，可能存在无法观测的
    短采样窗口，必须如实记录。已经确认的历史流量不能丢失或重复入账。
12. 恢复同一客户端，执行新的小批次上传/下载，等待 1 至 2 分钟：
      bash scripts/e2e-real.sh snapshot after-new-traffic ./evidence
    验证新流量继续增长，重载前的累计计数没有被再加一遍。
13. 通过面板修改节点 SNI 等原生配置后改回，分别等待健康应用，验证托管发布。
    托管应用会在重载前读取统计终值，再打开新计量周期。检查目标/已应用版本。
    不要为了测试向公网用户使用中的节点发布故意损坏的配置。

五、升级、身份和结论
14. 验证升级时在原服务器签发新的接入令牌，并重新执行安装命令。
    保留 /etc/sinan/identity 和 /var/lib/sinan/core/state.db；注册服务器 ID 不变，
    已消费的旧令牌不能再用。完成后再次核对版本、部署健康、流量与待确认数。
15. 整理 before、after-agent、after-runtime、after-new-traffic 中的：
    - Agent connected/applied/healthy/pending_batches；
    - 运行时 MainPID、监听地址及版本；
    - 面板按本用户+本节点过滤的精确十进制用量；
    - 本地账本的最后 seq、epoch、待确认批次及未完成意图；
    - 本次 SHA256SUMS、客户端测试结果、异常窗口和人工观察。
    只有全部相应步骤实际执行才写“实机验收通过”。未执行项目保持“待验证”。

脚本不修改 outbox 来模拟丢失 ACK；批次重放去重与恢复另由自动集成测试覆盖。
证据可能包含主机名、内部编号和用量，保存在私有目录，不要提交真实环境证据。
TEXT
}

snapshot() {
  [[ $# == 2 ]] || { usage >&2; exit 2; }
  local label=$1 output=$2 evidence
  [[ $label =~ ^[A-Za-z0-9_-]+$ ]] || { echo '标签只能包含字母、数字、下划线和连字符' >&2; exit 2; }
  [[ $(uname -s) == Linux && $(id -u) == 0 ]] || { echo '请在已安装 Agent 的 Linux 设备上以 root 收集快照' >&2; exit 1; }
  for tool in sinan-agent systemctl ss python3; do
    command -v "$tool" >/dev/null || { printf '缺少工具：%s\n' "$tool" >&2; exit 1; }
  done
  mkdir -p -- "$output"
  evidence=$(mktemp -d "$output/$label.XXXXXX")
  date -u +'%Y-%m-%dT%H:%M:%SZ' > "$evidence/time.txt"
  uname -srmo > "$evidence/system.txt"
  sinan-agent --version > "$evidence/agent-version.txt"
  sinan-agent status > "$evidence/agent-status.json"
  systemctl show sinan-agent.service -p ActiveState -p SubState -p MainPID -p NRestarts -p ActiveEnterTimestamp > "$evidence/agent-service.txt"
  systemctl show sinan-singbox@main.service -p ActiveState -p SubState -p MainPID -p NRestarts -p ActiveEnterTimestamp > "$evidence/runtime-service.txt"
  /opt/sinan/plugins/sing-box/current/sing-box version > "$evidence/runtime-version.txt"
  ss -lnt 'sport = :18085' > "$evidence/stats-listener.txt"
  python3 - "$evidence" <<'PY'
import getpass
import hashlib
import http.cookiejar
import json
import os
from pathlib import Path
import sqlite3
import sys
import urllib.parse
import urllib.request

output = Path(sys.argv[1])
database = Path(os.environ.get('SINAN_STATE_DB', '/var/lib/sinan/core/state.db'))
if not database.is_file():
    raise SystemExit('找不到账本；自定义路径请设置 SINAN_STATE_DB')
with sqlite3.connect(database.resolve().as_uri() + '?mode=ro', uri=True, timeout=5) as connection:
    connection.execute('BEGIN')
    sequence = connection.execute("SELECT value FROM kv WHERE key='usage:last_seq'").fetchone()
    pending = []
    for epoch, seq, batch in connection.execute('SELECT epoch,seq,batch FROM usage_outbox WHERE acknowledged=0'):
        pending.append({'epoch': epoch, 'seq': seq, 'payload_sha256': hashlib.sha256(batch.encode()).hexdigest()})
    baselines = [
        {'module': module, 'stat_name': name, 'epoch': epoch, 'uplink': up, 'downlink': down, 'observed_at': timestamp}
        for module, name, epoch, up, down, timestamp in connection.execute(
            'SELECT module,stat_name,epoch,uplink,downlink,observed_at FROM usage_baselines ORDER BY module,stat_name')
    ]
    ledger = {
        'last_seq': str(json.loads(sequence[0])) if sequence else '0',
        'pending_batches': sorted(pending, key=lambda row: int(row['seq'])),
        'pending_intents': connection.execute('SELECT COUNT(*) FROM intents WHERE completed=0').fetchone()[0],
        'baselines': baselines,
    }
(output / 'ledger.json').write_text(json.dumps(ledger, ensure_ascii=False, indent=2) + '\n')

names = ['SINAN_PANEL_URL', 'SINAN_USER_ID', 'SINAN_NODE_ID']
values = [os.environ.get(name) for name in names]
if any(values) and not all(values):
    raise SystemExit('面板摘要需要同时设置 ' + '、'.join(names))
if all(values):
    origin, user, node = values
    parsed = urllib.parse.urlsplit(origin)
    if parsed.scheme not in ('https', 'http') or not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment or parsed.path not in ('', '/'):
        raise SystemExit('SINAN_PANEL_URL 必须是 HTTP(S) origin')
    if not user.isdecimal() or not node.isdecimal() or int(user) <= 0 or int(node) <= 0:
        raise SystemExit('用户和节点编号必须是正整数')
    origin = origin.rstrip('/')
    password = getpass.getpass('管理员密码（不保存）：')
    client = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
    def request(path, data=None):
        body = None if data is None else json.dumps(data).encode()
        headers = {} if body is None else {'Content-Type': 'application/json'}
        with client.open(urllib.request.Request(origin + path, data=body, headers=headers), timeout=20) as response:
            return json.load(response)
    request('/api/login', {'password': password})
    del password
    try:
        usage = request('/api/usage?' + urllib.parse.urlencode({'user_id': user, 'node_id': node}))
        node_data = request('/api/nodes/' + node)
        deployment = request('/api/servers/' + str(node_data['server_id']) + '/deployments')
        (output / 'panel-usage.json').write_text(json.dumps(usage, ensure_ascii=False, indent=2) + '\n')
        (output / 'panel-deployment.json').write_text(json.dumps(deployment, ensure_ascii=False, indent=2) + '\n')
        (output / 'selection.json').write_text(json.dumps({'user_id': user, 'node_id': node, 'server_id': node_data['server_id']}, indent=2) + '\n')
    finally:
        request('/api/logout', {})
PY
  printf '只读快照已保存：%s\n' "$evidence"
  printf '%s\n' '请人工比较各阶段数据；快照完成不等于完整实机验收通过。'
}

case ${1:-guide} in
  guide) [[ $# -le 1 ]] || { usage >&2; exit 2; }; guide ;;
  snapshot) shift; snapshot "$@" ;;
  --help|-h) usage ;;
  *) usage >&2; exit 2 ;;
esac
