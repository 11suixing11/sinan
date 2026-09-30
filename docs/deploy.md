# 部署与节点接入

所有命令在仓库根目录运行。

## 真机已观测的部署问题

- [Agent 版本选择](https://github.com/theLucius7/sinan/issues/3)：历史 0.1.0→0.2.0 真机专项曾需要覆盖旧安装脚本的版本与摘要。新的签名发布流程从已导入、协议兼容的 Release 选择 Agent，接入界面可以指定版本；面板产品版本不再决定 Agent 版本。
- [CDN 设备路径](https://github.com/theLucius7/sinan/issues/4)：实际部署中，经 CDN 的设备 HTTP 请求返回 403，直达 Caddy origin、保留原域名和 TLS 的请求返回 200。页面访问成功不代表安装、注册、制品下载和 WebSocket 路径均可用，应逐项验证；回环 CI 不覆盖 CDN。
- [节点监听端口](https://github.com/theLucius7/sinan/issues/5)：历史验收曾依靠容器端口映射。现在创建和编辑节点可直接指定 443 等可用端口；创建时留空才自动分配 20000–29999。主机已有服务占用端口、云防火墙与容器端口映射仍需单独检查。
- [公网请求超时](https://github.com/theLucius7/sinan/issues/6)：真实持续载荷中曾出现 15 秒请求超时，包含升级前的请求；90 秒预算的升级后双向载荷通过，但仍需继续诊断公网短超时。回环 CI 不认证公网请求均无错误。

## 用 Compose 启动面板

需要 Docker Engine / Docker Desktop 和带 `--wait` 支持的 Docker Compose v2。下面的命令在仓库根目录运行：

```bash
git clone https://github.com/theLucius7/sinan.git
cd sinan

# Create once; never overwrite credentials for an existing database.
python3 - <<'PY'
from pathlib import Path
import os
import secrets
os.umask(0o077)
content = '\n'.join([
    'SINAN_DB_PASSWORD=' + secrets.token_hex(32),
    'SINAN_ADMIN_PASSWORD=' + secrets.token_hex(32),
    'SINAN_RELEASE_PUBLIC_KEYS=[]',
    'SINAN_PUBLIC_URL=http://127.0.0.1:8080',
    'SINAN_BIND_ADDRESS=127.0.0.1',
    'SINAN_PORT=8080',
    'RUST_LOG=info',
    '',
])
with Path('.env').open('x') as output:
    output.write(content)
Path('.env').chmod(0o600)
print('已创建 .env；请在本地查看管理员密码。')
PY

docker compose --project-name sinan --env-file .env \
  -f deploy/docker-compose.yml up -d --build --wait
```

构建前将 `.env` 中的 `SINAN_RELEASE_PUBLIC_KEYS` 填为独立核对过的 minisign 公钥记录 JSON 数组，例如 `["公钥记录"]`。这是公开信息，但必须核对来源；缺失或空集合会拒绝签名制品操作。Compose 把它作为镜像构建参数传入 Rust 编译器，容器运行时环境不能更换根。修改后必须重新构建镜像。

访问 <http://127.0.0.1:8080>，使用 `.env` 中的 `SINAN_ADMIN_PASSWORD` 登录。也可从 [.env.example](../.env.example) 手动创建配置，两个密码分别生成，不要填写相同值。数据库密码放入连接 URL，示例使用不需要额外转义的十六进制值。

首次启动自动创建 PostgreSQL 数据库、执行迁移并保存管理员密码散列。已有数据库再次启动时，修改 `SINAN_ADMIN_PASSWORD` **不会重置** 已有密码；修改数据库容器的密码环境变量也不会重置已有数据库用户密码。

| 配置 | 用途 |
|---|---|
| `SINAN_DB_PASSWORD` | 必填，PostgreSQL 密码 |
| `SINAN_ADMIN_PASSWORD` | 必填，首次创建管理员的密码 |
| `SINAN_RELEASE_PUBLIC_KEYS` | 必填的构建时公开信任根 JSON 数组；缺根时制品操作拒绝 |
| `SINAN_PUBLIC_URL` | 必填，Agent 与用户实际访问的 HTTP(S) origin，不包含路径 |
| `SINAN_BIND_ADDRESS` | 默认 `127.0.0.1`，宿主机监听地址 |
| `SINAN_PORT` | 默认 `8080`，宿主机端口 |
| `RUST_LOG` | 默认 `info` |

接入远端 Agent **之前**，将 `SINAN_PUBLIC_URL` 改为远端可访问的地址，例如自己的 HTTPS 域名，再执行 `docker compose … up -d --build --wait` 重建面板容器以应用环境变量；仅执行 `restart` 不会更新容器环境。公网部署应通过 HTTPS 反向代理；代理需支持 `/api/agent/v1/ws` 的 WebSocket 升级和长连接。若反向代理位于同一宿主机，可继续只监听回环地址；确需直接暴露端口时显式设置 `SINAN_BIND_ADDRESS`。这里不配置防火墙，管理员自行保证面板和节点端口可达。

例如在已有 Caddy 配置中追加站点，使用保留示例域名并替换为部署域名，不覆盖其他站点：

```caddyfile
panel.example.com {
    reverse_proxy 127.0.0.1:8080
}
```

端口应与 `SINAN_PORT` 一致。先校验 Caddy 配置再重载，检查 HTTPS 健康接口以及实际设备在线状态。Caddy 支持 WebSocket；若前面还有 CDN，应确认设备 API、安装脚本、订阅和长连接都能到达 origin，且不会被缓存或访问策略拒绝。

面板镜像默认使用两个 Rust 编译任务，降低 LTO 构建时的内存压力。资源充足时可先执行 `docker compose --env-file .env -f deploy/docker-compose.yml build --build-arg CARGO_BUILD_JOBS=4 panel`，再执行 `up -d --wait`；后一步不要增加 `--build` 以覆盖刚才的参数。

PostgreSQL 没有映射到宿主机端口。`postgres-data` 保存数据库，`panel-data` 保存制品；面板进程以 UID/GID `10001:10001` 运行。普通更新保留命名卷：

```bash
docker compose --project-name sinan --env-file .env -f deploy/docker-compose.yml ps
docker compose --project-name sinan --env-file .env -f deploy/docker-compose.yml logs --tail=100 panel
docker compose --project-name sinan --env-file .env -f deploy/docker-compose.yml up -d --build --wait
```

停止使用 `docker compose … down` 即可；`down --volumes` 会删除数据库和制品数据，只用于明确要清空的测试环境。更新和迁移前应备份数据库、制品目录以及 Agent 的身份与状态目录。

## 管理员登录与二步验证

登录后进入“账户安全”，输入管理员密码开始设置，将显示的秘密或 `otpauth://` 导入地址添加到验证器，再在 5 分钟内输入六位验证码确认。秘密只在本次设置页面显示，离开后需重新生成；确认之前仍按原方式登录。启用成功会退出其他管理员会话，保留当前会话。后续登录同时提交密码和验证码，关闭二步验证也需要密码与尚未使用的验证码。

验证码每 30 秒更新，允许前后各一个时间步，但成功消费的步号不能再次使用。确认启用、登录或关闭后如需立即进行下一次验证，请等待验证器更新；保持手机和面板主机时间同步。

登录与二步验证写操作共用限速：每个实际 TCP 来源 IP 在固定 60 秒窗口内最多接受 8 次，全局最多 64 次，并发验证最多 4 个。通过额度检查后的认证成功和失败均计数，因额度或并发限制返回的 429 不消耗额度；重启面板不会清空窗口，收到 429 时稍后重试。反向代理后的用户共享代理地址额度，面板不信任 `X-Forwarded-For` 或 `Forwarded` 来绕过限制，本地验收也不豁免。

未提供恢复码。丢失验证器时，由部署所有者经受保护的数据库管理通道恢复；普通管理员密码不能单独关闭 TOTP。确认拥有主机与数据库管理权、备份数据库并记录操作者和时间后，可运行以下事务，清除二步验证状态并使全部管理员会话失效：

```bash
docker compose --project-name sinan --env-file .env -f deploy/docker-compose.yml \
  exec -T postgres psql -U sinan -d sinan -v ON_ERROR_STOP=1 <<'SQL'
BEGIN;
SELECT id FROM admins WHERE id = 1 FOR UPDATE;
UPDATE admins SET
  totp_secret = NULL, totp_last_step = NULL,
  totp_pending_secret = NULL, totp_pending_expires = NULL,
  totp_pending_session = NULL
WHERE id = 1;
DELETE FROM sessions WHERE admin_id = 1;
COMMIT;
SQL
```

恢复不更改管理员密码；重新登录后立即设置新的验证器。TOTP 秘密保存在 PostgreSQL，数据卷和备份应按身份凭据保管。接口与状态码见 [API 文档](api.md#二步验证)。

## 导入签名 Release

正常部署无需在服务器编译运行时或 `docker cp` 制品。进入面板“制品”，输入官方仓库已发布的 `agent-v…` 标签，点击“导入制品”。只有已正式发布、签名通过且所有资产齐全的 Release 可导入；草稿、缺签名、错误版本、归档或摘要不一致均拒绝，现有集合保持不变。完整导入后才能用于设备接入和配置发布。

每个 Release 包含 Agent、固定版本运行时、NodeQuality r2 的 amd64/arm64 制品、固定安装器、`release.json`、`SHA256SUMS` 与 `SHA256SUMS.minisig`。Agent 是 musl 静态二进制；保留上游完整默认标签的运行时仍要求 glibc ≥ 2.31。只支持 Linux/systemd 节点，保留的其他平台构建脚本不代表可在这些平台部署。

面板核对签名、仓库/tag、架构、版本、归档内容和安装后二进制摘要，完成后一次发布整个目录。相同组件版本不能用不同内容覆盖。Agent 下载后独立以自身内嵌公钥再次验证，运行时 systemd 启动前也复验本地缓存。

签名发布和自建公钥的完整步骤见 [发布与信任根](release.md)；编译规范见 [开发文档](dev.md)。CI 的测试公钥是公开测试夹具，禁止用于正式节点、镜像或发布。

## 准备可信 bootstrap

首次安装的验证器和根公钥必须先从面板以外的可信渠道获得，并由操作者独立核对。不能执行面板提供的 `curl … | sh` 来建立信任，也不能先执行未知的新 Agent 让它验证自己。

在已核对来源的本地仓库中，检查 `tools/bootstrap.py`、`tools/release.py` 和发布公钥，再在目标 Linux 节点安装可信 bootstrap。以下命令仅安装已经由操作者核对的本地文件：

```bash
sudo apt-get update
sudo apt-get install -y python3 minisign ca-certificates curl coreutils passwd
sudo install -d -m 755 /usr/local/lib/sinan /etc/sinan/trust
sudo install -m 755 tools/bootstrap.py /usr/local/lib/sinan/bootstrap.py
sudo install -m 644 tools/release.py /usr/local/lib/sinan/release.py
sudo install -m 644 /已独立核对的路径/public-keys.json /etc/sinan/trust/public-keys.json
sudo tee /usr/local/bin/sinan-bootstrap >/dev/null <<'SH'
#!/bin/sh
exec python3 /usr/local/lib/sinan/bootstrap.py "$@"
SH
sudo chmod 755 /usr/local/bin/sinan-bootstrap
```

bootstrap 的公钥文件仅用于独立确认安装起点；它不会写进 Agent 的运行时配置，也不能替换 Agent 编译时的公钥。后续版本或根轮换须由已信任的旧根批准，私钥泄漏恢复须走独立可信渠道，见发布文档。

## 接入 Debian 12 并使用节点

1. 在全新 Debian 12 amd64/arm64 服务器安装基础工具：

   ```bash
   sudo apt-get update
   sudo apt-get install -y python3 minisign ca-certificates curl coreutils passwd
   ```

2. 先导入签名 Release 并准备可信 bootstrap。面板添加服务器，选择 Agent 版本并复制生成的安装命令，在目标服务器上以 root 执行。安装依赖 Linux + systemd，不支持 OpenRC、容器内缺失 systemd 的环境或非 Linux 平台。令牌 24 小时有效且只可消费一次。
3. 30 秒内检查服务器是否在线，并出现系统信息与最新指标。也可在设备上执行：

   ```bash
   sudo sinan-agent status
   sudo systemctl status sinan-agent.service --no-pager
   sudo journalctl -u sinan-agent.service -n 80 --no-pager
   ```

4. 在此服务器上创建节点，填写客户端可达的公开地址及可访问、支持 TLS 的伪装域名。监听端口留空时自动分配 20000–29999 中的空闲端口，也可指定 443 等 1–65535 内的端口；18085 保留给统计接口。面板拒绝同一服务器上其他有效节点占用的端口；Caddy 等主机程序是否占用、云防火墙是否放行仍需自行检查。面板自动生成节点密钥，无用户授权的节点不开放入站端口。
5. 创建用户并授权节点；连续变更合并 5 秒后自动发布。服务器详情应显示目标版本等于已应用版本、当前配置健康。失败后的健康回滚和最近错误会分别显示，不应把旧配置仍健康视为新版本已成功。
6. 在用户页复制订阅，导入支持该配置的客户端。通用格式为 base64 多行 VLESS 分享链接；`singbox` 格式为 sing-box JSON，提供 `127.0.0.1:2080` 混合代理入口。订阅只包含此用户当前有效、已应用且健康的节点。
7. 通过代理产生可控下载/上传，1–2 分钟后检查用户页“该用户的节点流量”。服务器网卡计数与代理用户流量是不同口径。

设备的关键路径：

| 路径 | 内容 |
|---|---|
| `/etc/sinan/agent.toml` | Agent 配置 |
| `/etc/sinan/identity/` | 设备身份和绑定的面板 origin，需保留 |
| `/var/lib/sinan/core/state.db` | SQLite 账本、意图、已应用状态与待确认批次 |
| `/run/sinan/agent.sock` | 权限 0600 的本地 status socket |
| `/opt/sinan/core/current/` | 当前 Agent 二进制 |
| `/opt/sinan/plugins/sing-box/current/` | 当前运行时二进制 |
| `/var/lib/sinan/plugins/sing-box@main/current/` | 当前原生配置 |

代理服务为 `sinan-singbox@main.service`，使用独立非特权用户，systemd 授予绑定 443 等低端口所需的 `CAP_NET_BIND_SERVICE`；统计 API 仅监听 `127.0.0.1:18085`。Agent 以 root 运行，特权操作通过内部 trait 边界执行。

编辑节点可修改监听端口；新配置应用成功后，客户端需更新订阅。省略端口的 API 更新保留现值，未实际改变字段的节点更新不会重复安排发布。

### 重置订阅链接

在用户页打开订阅窗口，点击“重置链接”并确认。旧地址立即失效，新地址仍对应原用户及授权；需将新地址交给对应用户并更新客户端订阅。此操作不改变代理 UUID、已应用配置或流量记录，已下载的配置仍可连接。需要终止代理访问时，应撤销该用户的节点授权。

### 升级设备

MVP 不做自动更新。导入协议兼容的新签名 Release 后，到原服务器详情点击“接入 / 升级”，可指定已导入的 Agent 版本，签发**新的**一次性令牌，再执行可信 bootstrap 命令。面板和 Agent 产品版本无需相同；首次安装使用独立预置验证器，重复安装可以明确指定已信任的旧 Agent 验证下一版。

安装器先核对签名和实际内容，预检既有运行时、诊断和未完成回滚引用，再注册暂存 Agent、切换当前版本并重启 Agent。身份、配置与状态库继续保留；同一服务器只允许原公钥再次注册，不能复制其他设备的身份目录。旧命令消费后不可复用。

历史未签名缓存不会被自动认可；预检失败保持旧安装，明确报错。需按发布文档提供与实际二进制完全匹配、经过独立验证的签名证明，或安排迁移维护窗口。历史 0.1.0→0.2.0 连续升级验收不等于已经完成旧未签版本到新签名版本的迁移。

### 删除与退役设备

删除在线且支持退役的 Agent 时，面板先发送退役请求。Agent 阻止新操作、等待正在执行的受管操作，采集可观测的最后用量并停止运行时和诊断；已持久用量全部确认后，清除设备凭据和运行配置，再提交签名完成回执。面板验证后删除记录、撤销设备会话和接入令牌，移出节点列表和订阅。已确认历史用量保留，退役 Agent 停止且重启不会恢复代理。

在线删除最多等待 20 秒；失败或超时保留记录与原退役请求，可重试。在线旧 Agent 不支持退役时会提示先升级。判断以实际 WebSocket 连接为准，列表“在线”按最近消息显示，可能稍有延迟。

**离线删除没有设备清理确认**：只软删除面板记录、撤销面板接入及未完成诊断，离线机器仍可能运行最后配置。需在设备本地停止相关服务、清理身份和运行配置。在线请求失败后再次主动删除，若设备已离线，也会走此分支；尚有用量未确认时撤销认证会阻止自动完成清理。只有已清理且保存签名回执的设备才能稍后补交确认。退役状态不会被普通重复安装清除，重新使用该机器需先明确完成本地清理。

清理只涉及 Agent 托管的身份和配置，保留历史计量、退役记录和公开制品，不承诺物理介质安全擦除。故障恢复与确认边界见 [ADR 0019](adr/0019-server-retirement.md)。

## IP 质量与 NodeQuality 报告

服务器详情页显示 Agent 上报的 IPv4/IPv6。网卡地址在连接建立时和运行中更新；NAT 服务器可在 `/etc/sinan/agent.toml` 增加公网地址，随后重启 Agent：

```toml
# Documentation addresses only; replace with this server's actual public IPs.
public_ips = ["192.0.2.10", "2001:db8::10"]
```

点击“刷新 IP 质量”时，由面板访问 NodeQuality 使用的 [IPQuality](https://github.com/xykt/IPQuality) 数据库接口，查询位置、ASN、用途、风险及代理等信息。各数据库独立展示，包含更新时间、原始字段和错误；第三方数据可能缺失或互相矛盾，不合成为一个无依据的总分。私网和回环地址不向外部接口查询。本次开发环境对该接口的实际请求返回 403，因此记录了服务错误；成功字段解析和失败处理通过受控 HTTP 夹具验证，不能据此宣称线上数据库服务当前可用。

完整报告使用已导入签名 Release 中的 NodeQuality 外插，按前述流程导入即可；单独编译或拷贝未签名目录不能代替验签导入。外插固定 [NodeQuality 上游提交](https://github.com/LloydAsp/NodeQuality/tree/a92fca6c0067df29ddd03fdc2fee6f3000f64545)，保留原样源码和许可证，版本为 `a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2`。旧制品不能覆盖；默认关闭公开上传需要面板、支持该选项的 Agent 与 `-r2` 外插共同支持，旧的已排队或运行任务继续使用创建时的选项。

上游固定下载 amd64 版 NextTrace；包装器在 ARM64 节点仅将这条下载命令映射到官方 arm64 资产。外插工作路径不能包含空白或 shell 通配符，使用默认目录即可。

目标服务器需要 Linux systemd、root、Bash、curl 和 Python 3，并能访问上游 BenchOS、测试和报告服务；最小 Debian 系统可先安装：

```bash
sudo apt-get update
sudo apt-get install -y bash curl python3 ca-certificates
```

在服务器详情点击“一键获取报告”，选择双栈/IPv4/IPv6和低流量/普通网络测试。任务运行硬件、IP、网络和回程测试，会消耗真实 CPU、磁盘和带宽；默认关闭公开报告上传，并采用低流量网络模式。只有创建任务时勾选“上传报告并生成公开链接”，才允许上传到 NodeQuality；报告可能包含节点网络和硬件信息。任务在该节点的独立 systemd 服务运行，Agent 重启后继续观察，不重复执行；每台服务器同时只允许一个任务。

界面显示排队、运行、成功或失败，并保留本地文本报告及可用的在线链接。在线上传失败时，本地报告仍可查看。任务有整体运行时限；未安装依赖、上游下载失败、报告缺失和超时均返回错误。上游 chroot 用于隔离测试文件，systemd 使用独立挂载命名空间处理清理，不提供针对不可信程序的安全沙箱。外插按用户选择运行，运行时外网访问是 [ADR 0016](adr/0016-nodequality-diagnostics.md) 明确记录的例外。

面板报告文本最多 256 KiB，超过时显示截断说明；原始 `report.zip` 默认保存在节点的 `/var/lib/sinan/plugins/diagnostics/<任务 UUID>/`，可由管理员在节点本地读取。

## 实机验收与已知边界

`scripts/e2e-real.sh guide` 给出完整人工步骤；`snapshot` 在 Debian 设备收集当前状态、服务 PID、统计监听和只读账本摘要，可选保存按用户、节点筛选的面板用量。它不会安装软件、修改业务配置、重启服务或更改账本；可选的面板查询会创建临时管理员会话并在结束时注销。

```bash
bash scripts/e2e-real.sh guide
# Run as root on an already enrolled Debian 12 node:
export SINAN_PANEL_URL=https://panel.example.com
export SINAN_USER_ID=1
export SINAN_NODE_ID=1
sudo --preserve-env=SINAN_PANEL_URL,SINAN_USER_ID,SINAN_NODE_ID \
  bash scripts/e2e-real.sh snapshot before ./evidence
```

可选的面板摘要会在终端隐藏输入管理员密码，不把密码或会话写入证据。已启用 TOTP 时，在上述 `sudo` 命令前 `export SINAN_E2E_TOTP=1`，并在 `--preserve-env` 中追加 `SINAN_E2E_TOTP`，脚本会隐藏输入新的六位验证码。`scripts/e2e-driver.py` 可使用全局参数 `--totp` 请求同样的输入；非交互调用可通过 `SINAN_E2E_TOTP_CODE` 提供本次验证码，脚本不保存秘密或验证码，每次新登录须使用尚未消费的验证码。不要把验证码写进命令历史、日志或证据文件。

证据目录仍可能包含主机名、用量和内部编号，应按私有运维数据保管。记录重启前、重启 Agent 后、重载运行时后以及恢复流量后的快照；在暂停客户端、等待连续两次采样稳定和待确认批次清零后比较，确认不会重复计入已确认流量。

Agent 在托管应用前读取终值，再打开新计量周期；外部强制重载或异常进程退出可能留下不可观测的短采样窗口，会记录告警。已经在本地 outbox 落盘的批次可重发，面板事务去重后确认；这不代表能恢复从未采集到的字节。脚本默认不修改 outbox 来伪造丢失确认，丢 ACK 重传由自动化集成测试覆盖。

未包含配额、计费、链式代理、其他代理协议、多管理员或权限体系。订阅 URL 是用户访问凭据，应仅交给对应用户。
