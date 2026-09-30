# 部署与节点接入

所有命令在仓库根目录运行。

## 真机已观测的部署问题

- [Agent 版本选择](https://github.com/theLucius7/sinan/issues/3)：安装脚本目前固定使用面板版本对应的 Agent。0.1.0 初装与 0.2.0 升级专项使用了记录在私有证据中的旧版本/摘要覆盖；正式 0.2.0 安装脚本保持原样。不能据此把任意 Agent 版本选择视为已实现。
- [CDN 设备路径](https://github.com/theLucius7/sinan/issues/4)：实际部署中，经 CDN 的设备 HTTP 请求返回 403，直达 Caddy origin、保留原域名和 TLS 的请求返回 200。页面访问成功不代表安装、注册、制品下载和 WebSocket 路径均可用，应逐项验证；回环 CI 不覆盖 CDN。
- [节点监听端口](https://github.com/theLucius7/sinan/issues/5)：当前节点端口仍自动分配 20000–29999。容器验收可以映射宿主端口并仅在私有客户端 JSON 覆盖目标端口，原生监听端口不变；产品内手动指定 443 属于后续安全阶段。
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

访问 <http://127.0.0.1:8080>，使用 `.env` 中的 `SINAN_ADMIN_PASSWORD` 登录。也可从 [.env.example](../.env.example) 手动创建配置，两个密码分别生成，不要填写相同值。数据库密码放入连接 URL，示例使用不需要额外转义的十六进制值。

首次启动自动创建 PostgreSQL 数据库、执行迁移并保存管理员密码散列。已有数据库再次启动时，修改 `SINAN_ADMIN_PASSWORD` **不会重置** 已有密码；修改数据库容器的密码环境变量也不会重置已有数据库用户密码。

| 配置 | 用途 |
|---|---|
| `SINAN_DB_PASSWORD` | 必填，PostgreSQL 密码 |
| `SINAN_ADMIN_PASSWORD` | 必填，首次创建管理员的密码 |
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

## 准备设备制品

面板容器只提供控制面板，不代替设备制品构建。按实际服务器架构提供 Agent 和运行时，目录形状如下：

```text
data/artifacts/
├── agent/
│   └── 0.2.0/
│       ├── amd64          # Raw, statically linked Linux Agent ELF
│       ├── arm64          # Optional; provide only deployed architectures
│       └── SHA256SUMS
└── sing-box/
    └── 1.14.2/
        ├── amd64          # tar.gz containing the sing-box executable
        ├── arm64          # Optional
        └── SHA256SUMS
```

每个 `SHA256SUMS` 中的文件名为 `amd64` / `arm64`，不是下载 URL。构建脚本会生成清单，并在添加另一架构时验证和保留已有条目。同一版本、同一架构的制品不可覆盖；不要用不同内容复用运行时版本。Agent 制品版本必须与面板编译时的 workspace 版本一致，当前为 `0.2.0`。

### 构建 Agent

在**对应架构的 Linux 主机**上运行。amd64 和 arm64 各自使用原生 Rust musl 工具链，不支持直接在 macOS 上生成可部署 Agent：

```bash
sudo apt-get update
sudo apt-get install -y build-essential musl-tools binutils python3 ca-certificates
# After installing rustup using the official Rust instructions:
rustup toolchain install stable
rustup default stable
rustup target add x86_64-unknown-linux-musl
bash tools/build-agent.sh amd64 "$PWD/data/artifacts"
```

arm64 主机将目标改为 `aarch64-unknown-linux-musl`，脚本参数改为 `arm64`。脚本验证 ELF 架构、无动态解释器/动态库依赖，以及原生 `--version` / `--help` 运行结果，再发布制品。安装脚本的 curl 需要系统 CA 证书及基础安装工具；Agent 的 HTTPS/WebSocket 使用公共 WebPKI 根证书，目前没有自定义 CA 配置项。运行平台需要 systemd。

### CI 可下载的 Agent 编译产物

每次 push 或 PR 的 CI 只自动构建 Linux musl 静态 amd64/arm64，分别使用 Ubuntu 24.04 和 Ubuntu 24.04 arm runner。成功执行后，可在对应 Actions 运行页面下载 `sinan-agent-linux-musl-amd64`、`sinan-agent-linux-musl-arm64`，保留七天。

下载内容保持上方 `agent/<version>/<arch>` 结构，并附 `SHA256SUMS`。Actions ZIP 不保留 Unix 执行权限，直接运行下载文件前执行 `chmod +x <文件路径>`。设备注册、常驻运行、状态查询及安装脚本要求 Linux/systemd。

其他平台的历史编译脚本仍保留，已从自动 CI 移除；不将历史构建结果视为当前提交已通过。收敛原因及历史决策见 [ADR 0015](adr/0015-agent-build-platforms.md)。

### 构建运行时

在 **Linux/amd64** 构建机上运行，支持输出 amd64 或交叉编译 arm64。要求 **Go 1.26.8** 和以下工具：

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates git curl python3 python3-requests \
  gnupg dirmngr xz-utils unzip bzip2 zstd file binutils binutils-aarch64-linux-gnu \
  build-essential pkg-config coreutils

go version  # Must report go1.26.8
bash tools/build-singbox.sh amd64 "$PWD/data/artifacts"
# Build the other architecture sequentially, never concurrently:
bash tools/build-singbox.sh arm64 "$PWD/data/artifacts"
```

构建脚本核对上游 tag、提交和工具链版本；使用上游默认标签加 `with_v2ray_api`。官方默认标签包含需要 Chromium 工具链的组件，因此脚本按上游方式获取对应 clang / sysroot，不能用简单的 `CGO_ENABLED=0` 构建替代。构建机需能访问上游源码及工具链下载站点，并准备足够的时间、磁盘和内存。Agent 原生传输与制品下载只访问面板；按需运行的 NodeQuality 测试外插需要访问上游测试和报告站点。

也可使用仓库中的工具链容器，避免手动安装 Go 和构建依赖；以下两次运行顺序执行：

```bash
docker build --platform=linux/amd64 -f tools/singbox-builder.Dockerfile \
  -t sinan-runtime-builder:1.14.2 .
mkdir -p data/artifacts
docker run --rm --platform=linux/amd64 -v "$PWD/data/artifacts:/artifacts" \
  sinan-runtime-builder:1.14.2 amd64 /artifacts
# Optional second architecture.
docker run --rm --platform=linux/amd64 -v "$PWD/data/artifacts:/artifacts" \
  sinan-runtime-builder:1.14.2 arm64 /artifacts
```

该容器仍需访问上游工具链；原生 Linux/amd64 是构建脚本的目标环境，本地未验证 macOS 上的容器模拟构建。

生成运行时要求 glibc ≥ 2.31，Debian 12 满足该条件。amd64 输出可在构建机执行版本验证；交叉编译 arm64 输出只验证 ELF 和构建信息，仍须在 arm64 Linux 主机执行 `sing-box version` 并检查 `with_v2ray_api` 后部署。详见脚本的 `--help`。

### 导入 Compose 命名卷

构建完成后，把整个 `data/artifacts` 目录复制到面板命名卷：

```bash
# Apply to artifact copies only; this directory contains no identity keys.
find data/artifacts -type d -exec chmod 755 {} +
find data/artifacts -type f -exec chmod 644 {} +
docker compose --project-name sinan --env-file .env -f deploy/docker-compose.yml \
  cp ./data/artifacts/. panel:/data/artifacts/
```

面板只需读取这些文件，复制后属于 root 也可以，只要目录可遍历、文件可读。`/data` 本身由 UID 10001 拥有，不要把整个数据卷改为不可写。打开“制品”页面并刷新，确认实际使用架构的 Agent 和运行时均出现；校验清单缺失或哈希不符的制品不会被提供给设备。

## 接入 Debian 12 并使用节点

1. 在全新 Debian 12 amd64/arm64 服务器安装基础工具：

   ```bash
   sudo apt-get update
   sudo apt-get install -y ca-certificates curl coreutils passwd
   ```

2. 面板添加服务器，复制生成的安装命令，在目标服务器上以 root 执行。安装依赖 Linux + systemd，不支持 OpenRC、容器内缺失 systemd 的环境或非 Linux 平台。令牌 24 小时有效且只可消费一次。
3. 30 秒内检查服务器是否在线，并出现系统信息与最新指标。也可在设备上执行：

   ```bash
   sudo sinan-agent status
   sudo systemctl status sinan-agent.service --no-pager
   sudo journalctl -u sinan-agent.service -n 80 --no-pager
   ```

4. 在此服务器上创建节点，填写客户端可达的公开地址及可访问、支持 TLS 的伪装域名。面板分配 20000–29999 中的空闲端口并生成密钥。无用户授权的节点不开放入站端口。
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

代理服务为 `sinan-singbox@main.service`，使用独立非特权用户；统计 API 仅监听 `127.0.0.1:18085`。Agent 以 root 运行，特权操作通过内部 trait 边界执行。

### 升级设备

MVP 不做自动更新。在面板发布与新面板版本一致的 Agent 制品后，到原服务器详情点击“接入 / 升级”，签发**新的**一次性令牌，重新执行安装命令。保留原设备身份、面板地址和状态库；同一服务器只允许原公钥再次注册，不能复制其他设备的身份目录。已消费的旧命令不能再次使用。安装脚本先校验并注册暂存二进制，再切换当前版本并重启 Agent。

删除面板服务器会撤销面板会话并移出订阅，**不会远程停止** 该设备最后一份可用配置。停用设备时由管理员在本地停止相关服务。

## IP 质量与 NodeQuality 报告

升级到 `0.2.0` Agent 后，服务器详情页显示上报的 IPv4/IPv6。网卡地址在连接建立时和运行中更新；NAT 服务器可在 `/etc/sinan/agent.toml` 增加公网地址，随后重启 Agent：

```toml
# Documentation addresses only; replace with this server's actual public IPs.
public_ips = ["192.0.2.10", "2001:db8::10"]
```

点击“刷新 IP 质量”时，由面板访问 NodeQuality 使用的 [IPQuality](https://github.com/xykt/IPQuality) 数据库接口，查询位置、ASN、用途、风险及代理等信息。各数据库独立展示，包含更新时间、原始字段和错误；第三方数据可能缺失或互相矛盾，不合成为一个无依据的总分。私网和回环地址不向外部接口查询。本次开发环境对该接口的实际请求返回 403，因此记录了服务错误；成功字段解析和失败处理通过受控 HTTP 夹具验证，不能据此宣称线上数据库服务当前可用。

完整报告需要先导入 NodeQuality 外插制品：

```bash
# Build on an operator's machine; this does not run benchmark tests.
bash tools/build-nodequality.sh amd64 "$PWD/data/artifacts"
bash tools/build-nodequality.sh arm64 "$PWD/data/artifacts"
```

制品位于 `nodequality/a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2/`，按前述导入方法放入面板的 `artifacts` 目录。脚本固定 [NodeQuality 上游提交](https://github.com/LloydAsp/NodeQuality/tree/a92fca6c0067df29ddd03fdc2fee6f3000f64545)，保留原样源码和许可证，并生成同源下载的 SHA256 清单。升级 Agent 需重新构建 `agent/0.2.0`，已有 `0.1.0` 制品继续保留。本次隐私默认需要同步升级面板、Agent 和 `-r2` 外插；旧制品不能覆盖，旧 Agent 会拒绝新制品任务，旧的已排队或运行任务仍使用原选项。

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

可选的面板摘要会在终端隐藏输入管理员密码，不把密码或会话写入证据。证据目录仍可能包含主机名、用量和内部编号，应按私有运维数据保管。记录重启前、重启 Agent 后、重载运行时后以及恢复流量后的快照；在暂停客户端、等待连续两次采样稳定和待确认批次清零后比较，确认不会重复计入已确认流量。

Agent 在托管应用前读取终值，再打开新计量周期；外部强制重载或异常进程退出可能留下不可观测的短采样窗口，会记录告警。已经在本地 outbox 落盘的批次可重发，面板事务去重后确认；这不代表能恢复从未采集到的字节。脚本默认不修改 outbox 来伪造丢失确认，丢 ACK 重传由自动化集成测试覆盖。

未包含配额、计费、链式代理、其他代理协议、多管理员或权限体系。订阅 URL 是用户访问凭据，应仅交给对应用户。
