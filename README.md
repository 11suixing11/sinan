# 司南 Sinan

自托管的服务器与代理节点控制面板。面板保存期望配置；跨平台 Agent 主动连接面板，负责配置对账、应用恢复、系统遥测和按用户计量。代理运行时作为独立系统服务运行，面板或 Agent 暂时离线时，最后一份可用配置继续工作。

MVP 提供中文管理界面、单管理员登录、服务器接入、VLESS + Reality 节点、用户授权、两种订阅格式、部署状态与流量汇总。运行时固定为上游 **sing-box 1.14.2**，保留官方默认构建标签，额外启用 `with_v2ray_api`，不修改上游源码。后续新增 **NodeQuality 外插**：Agent 上报 IP，面板查询 IP 质量，管理员可一键在该服务器运行测试并获取报告。

本地测试覆盖真实 PostgreSQL、协议、编译、应用回滚、持久化计量和面板—Agent 通信；真实上游二进制已用于配置、密钥、统计接口等专项验证，浏览器已验证主要管理操作。当前工作机尚未实际运行 Docker Compose 和 Linux musl 制品构建，相关 CI 配置的存在不代表远端运行已经通过。**这不等于已经完成全新 Debian 12、systemd 与公网 Reality 客户端的完整实机验收。** 实际完成范围和限制见 [PROGRESS.md](PROGRESS.md)，实机步骤见后文及 [scripts/e2e-real.sh](scripts/e2e-real.sh)。

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

访问 <http://127.0.0.1:8080>，使用 `.env` 中的 `SINAN_ADMIN_PASSWORD` 登录。也可从 [.env.example](.env.example) 手动创建配置，两个密码分别生成，不要填写相同值。数据库密码放入连接 URL，示例使用不需要额外转义的十六进制值。

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

arm64 主机将目标改为 `aarch64-unknown-linux-musl`，脚本参数改为 `arm64`。脚本验证 ELF 架构、无动态解释器/动态库依赖，以及原生 `--version` / `--help` 运行结果，再发布制品。安装脚本的 curl 需要系统 CA 证书及基础安装工具；Agent 的 HTTPS/WebSocket 使用公共 WebPKI 根证书，目前没有自定义 CA 配置项。Linux 服务支持 systemd 和 OpenRC；原生 macOS、FreeBSD、Windows 安装见下文。

### CI 可下载的 Agent 编译产物

每次 push 或 PR 都由同一个 [CI 工作流](.github/workflows/ci.yml) 构建下方九个 Agent 目标，也可在 Actions 页面手动运行。Linux 按 libc 与架构提供四个目标；OpenRC 安装与服务检查作为 musl 构建步骤，systemd 检查保留在主检查任务。成功执行后，可在对应 Actions 运行页面的 Artifacts 下载，保留七天；当前执行结果以 Actions 为准。

| 系统与链接方式 | 架构 | Artifact 名称 | 构建环境 |
| --- | --- | --- | --- |
| Linux musl 静态 | amd64、arm64 | `sinan-agent-linux-musl-<arch>` | Ubuntu 24.04 对应架构 |
| Linux glibc 动态 | amd64、arm64 | `sinan-agent-linux-gnu-<arch>` | Ubuntu 24.04 对应架构及系统动态库 |
| macOS | arm64 | `sinan-agent-macos-arm64` | GitHub 最新 macOS arm64 runner |
| FreeBSD | amd64、arm64 | `sinan-agent-freebsd-<arch>` | Linux cross + FreeBSD 13 sysroot，在 13.5 及最新 14/15 系列检查同一产物启动 |
| Windows MSVC | amd64、arm64 | `sinan-agent-windows-<arch>` | Visual Studio 2026 对应架构 runner，静态 CRT |

新增目标使用最新 Rust stable，通过 Python 标准库脚本构建并检查 ELF、Mach-O 或 PE 架构和实际 `--version`、`--help` 启动。glibc 另外检查动态解释器、`libc.so.6` 和共享库解析。FreeBSD 的兼容基线是 13.5，未验证更早 13 小版本或未来主版本。

Alpine/OpenRC 使用 musl 静态 Agent；Ubuntu 24.04/systemd 可使用 glibc 动态 Agent。OpenRC 与 systemd 是服务管理方式，不是额外的编译目标；选择二进制仍需匹配设备的 libc 和架构。musl 静态版也可用于 systemd 设备，glibc 动态版需与宿主共享库兼容。

所有 Agent 产物均提供面板可直接导入的 `<version>/<platform-target>` 和 `SHA256SUMS`，如 `linux-musl-amd64`、`macos-arm64`、`freebsd-arm64`、`windows-amd64`。musl 兼容旧 `amd64`/`arm64` 文件名，其他目标另附 Rust target 子目录方便直接运行。Actions ZIP 不保留 Unix 执行权限，直接运行前执行 `chmod +x sinan-agent`。不同任务的产物导入同一个版本目录时，需要合并各自 `SHA256SUMS` 的条目，不能覆盖其他平台的摘要。

各平台支持注册、常驻运行、状态查询、遥测、命令、拨测和配置对账。macOS 使用 launchd、FreeBSD 使用 rc.d 与 daemon、Windows 使用启动时计划任务；Agent 与运行时始终独立。Windows 运行时使用专用普通账户，Agent 使用 SYSTEM，身份与状态由 ACL 保护。NodeQuality 外插仍仅支持 Linux。

在对应系统及架构安装 Rust stable、Python 3.11 以上和本机 C 工具链后，可以本地构建新增目标：

```bash
# Native Ubuntu 24.04 amd64, dynamically linked glibc:
python3 tools/build-agent.py x86_64-unknown-linux-gnu "$PWD/artifacts"
# Native Apple Silicon macOS:
python3 tools/build-agent.py aarch64-apple-darwin "$PWD/artifacts"
```

FreeBSD CI 在 Linux 安装目标标准库，使用 `Cross.toml` 中固定摘要的交叉编译镜像，不在 ARM64 FreeBSD VM 内安装 rustup；13.5/14 VM 只执行二进制，15 VM 使用 Python 验证并打包，均在验证成功后上传。FreeBSD 本机原生构建仍需安装 Rust、protobuf 并设置 `PROTOC=/usr/local/bin/protoc`；Windows 使用 `python` 和对应 MSVC Rust toolchain。原生构建要求工具链与目标一致。新增下载包不能直接替代面板的原制品目录：Linux glibc 部署时，将对应二进制复制到 `agent/<version>/<arch>` 并重新生成 `SHA256SUMS`，确保同一版本、同一架构的已有制品不被覆盖。

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

缺省构建保留 glibc ≥ 2.31 的旧制品路径。新增显式 libc 选项：

```bash
bash tools/build-singbox.sh amd64 "$PWD/data/artifacts" --libc=musl
bash tools/build-singbox.sh arm64 "$PWD/data/artifacts" --libc=musl
bash tools/build-singbox.sh amd64 "$PWD/data/artifacts" --libc=gnu
bash tools/build-singbox.sh arm64 "$PWD/data/artifacts" --libc=gnu
```

musl 使用上游 Chromium musl 工具链、完整默认标签、`with_musl` 和 `with_v2ray_api`，验证 ELF 没有动态解释器和共享库依赖。新增文件名为 `linux-musl-amd64`、`linux-musl-arm64` 或对应的 `linux-gnu-*`，同目录清单同时保留已有文件。新版 Agent 上报 OS/libc，面板选择匹配制品；musl 缺失时不会退回 glibc。旧 Agent 和 glibc 设备保留旧路径兼容。

amd64 输出可在构建机执行版本验证；交叉编译 arm64 输出只验证 ELF 和构建信息，仍须在 arm64 Linux 主机执行 `sing-box version` 并检查标签后部署。详见脚本的 `--help`。

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

2. 面板添加服务器，复制生成的安装命令，在目标服务器上以 root 执行。安装时选择 Linux/macOS、FreeBSD 或 Windows 命令；Unix 以 root、Windows 以管理员 PowerShell 运行。Linux 自动识别正在运行的 systemd/OpenRC，普通容器中只安装 init 工具不满足条件。令牌 24 小时有效且只可消费一次。
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

systemd 代理服务为 `sinan-singbox@main.service`，OpenRC 为 `sinan-singbox@main`，均使用独立非特权用户；统计 API 仅监听 `127.0.0.1:18085`。Agent 以 root 运行，特权操作通过内部 trait 边界执行。

### 接入 OpenRC 设备

使用同一面板安装命令，设备须运行 OpenRC，并安装 CA 证书、curl、SHA-256 与基础文件工具、getent，以及 groupadd/useradd 或 BusyBox addgroup/adduser。OpenRC 的 supervise-daemon 必须支持 `--capabilities` 和 `--no-new-privs`；安装脚本会提前检查，不会将运行时改为 root 运行。

安装生成 `/etc/init.d/sinan-agent` 和 `/etc/init.d/sinan-singbox@main`，并加入 default runlevel。首次发布前运行时尚无配置，安装只启动 Agent；已有配置的运行时可在设备启动时恢复。Agent 与安装升级自动识别 init，设备 TOML 无需增加设置。

```sh
sudo sinan-agent status
sudo rc-service sinan-agent status
sudo rc-service sinan-singbox@main status
sudo tail -n 80 /var/log/sinan/agent.log
sudo tail -n 80 /var/log/sinan/runtime.log
sudo rc-service sinan-agent restart
sudo rc-service sinan-singbox@main reload
```

日志权限为 0640，应按设备现有日志轮转规则管理。运行时 reload 通过 supervisor 向实际代理进程发送 HUP，Agent 重启不会停止代理；运行时文件与账本路径和 systemd 相同。停用设备时分别停止这两个服务，取消开机启动使用 `rc-update del <服务名> default`。

OpenRC 设备支持完整遥测、IP 质量、代理配置、流量计量和 NodeQuality 一次性诊断；需要支持挂载命名空间的 `unshare`。

Agent 与运行时均提供 musl 静态产物，可在 Alpine 使用；面板依据 OS/libc/架构选择制品，缺失 musl 运行时不会误用 glibc。OpenRC 支持与实际代理、公网 Reality 和整机重启验收分别记录；CI 的进程夹具检查不代替完整实机验收。

### 升级设备

自动更新默认关闭，可在服务器详情的 Agent 设置启用。面板仅提供已导入、摘要有效、平台匹配且更高的稳定版（数字 `major.minor.patch`）；设备只从绑定面板下载，每六小时加随机延迟检查，网络失败五分钟后重试。Supervisor 校验 SHA-256 与实际版本后试启动，通过本地版本/PID 连续检查才确认；启动失败或未确认期间监督进程重启会恢复旧 Agent，并记住最近 32 个失败版本。更新保留身份、SQLite 账本和独立代理服务；`sinan-agent status` 的 `update` 字段可查看结果。新安装通过监督进程运行，旧安装需先重复执行安装脚本接入。

仍可在“接入 / 升级”签发**新的**一次性令牌，重新执行安装命令。保留原身份和状态，同一服务器只接受原公钥；已消费命令不能再次使用。安装启动检查失败时恢复旧 Agent，Linux 还恢复旧服务定义与配置。Windows 以原子替换的受保护引用文件切换目录，文件内容写盘后切换；当前标准库方案不提供 Windows 断电时目录元数据刷盘保证。

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

目标服务器需要 Linux systemd 或 OpenRC、root、Bash、curl 和 Python 3，并能访问上游 BenchOS、测试和报告服务；最小 Debian 系统可先安装：

```bash
sudo apt-get update
sudo apt-get install -y bash curl python3 ca-certificates
```

在服务器详情点击“一键获取报告”，选择双栈/IPv4/IPv6和低流量/普通网络测试。任务运行硬件、IP、网络和回程测试，会消耗真实 CPU、磁盘和带宽；默认关闭公开报告上传，并采用低流量网络模式。只有创建任务时勾选“上传报告并生成公开链接”，才允许上传到 NodeQuality；报告可能包含节点网络和硬件信息。任务在该节点的独立 systemd 服务运行，Agent 重启后继续观察，不重复执行；每台服务器同时只允许一个任务。

界面显示排队、运行、成功或失败，并保留本地文本报告及可用的在线链接。在线上传失败时，本地报告仍可查看。任务有整体运行时限；未安装依赖、上游下载失败、报告缺失和超时均返回错误。上游 chroot 用于隔离测试文件，systemd 使用独立挂载命名空间处理清理，不提供针对不可信程序的安全沙箱。外插按用户选择运行，运行时外网访问是 [ADR 0016](docs/adr/0016-nodequality-diagnostics.md) 明确记录的例外。

面板报告文本最多 256 KiB，超过时显示截断说明；原始 `report.zip` 默认保存在节点的 `/var/lib/sinan/plugins/diagnostics/<任务 UUID>/`，可由管理员在节点本地读取。

## 本地开发与测试

需要 Rust stable、PostgreSQL 16。仓库提交了 `web/dist`，不修改前端时可直接编译面板；修改前端时使用固定版本 **Bun 1.4.2** 管理依赖和运行构建，并同步构建产物。CI 与 Docker 前端构建使用相同版本。

尚未安装 Bun 时，按[官方安装说明](https://bun.com/docs/installation)安装指定版本，并确认当前终端可调用：

```bash
curl -fsSL https://bun.com/install | bash -s "bun-v1.4.2"
export PATH="$HOME/.bun/bin:$PATH"
bun --version  # Must report 1.4.2.
```

如果不使用 Compose 面板，可以单独启动一个开发数据库。在前面的 `.env` 已生成、且没有占用 5432 端口的环境中运行：

```bash
# Source only the generated configuration with URL-safe hexadecimal passwords.
set -a
. ./.env
set +a

docker run -d --name sinan-dev-postgres \
  -p 127.0.0.1:5432:5432 \
  -e POSTGRES_DB=sinan -e POSTGRES_USER=sinan \
  -e POSTGRES_PASSWORD="$SINAN_DB_PASSWORD" \
  -v sinan-dev-postgres-data:/var/lib/postgresql/data postgres:16
until docker exec sinan-dev-postgres pg_isready -U sinan -d sinan; do sleep 1; done

export SINAN_DATABASE_URL="postgres://sinan:$SINAN_DB_PASSWORD@127.0.0.1:5432/sinan"
export DATABASE_URL="$SINAN_DATABASE_URL"
export SINAN_LISTEN=127.0.0.1:8080
export SINAN_DATA_DIR="$PWD/data"
cargo run --locked -p sinan-panel
```

数据库名、用户和卷在首次执行时创建，面板启动时自动迁移。重用该容器时运行 `docker start sinan-dev-postgres`，继续使用最初的数据库密码。只使用本地原生 PostgreSQL 时，也可以先创建具有建库权限的开发角色和数据库，再把 `SINAN_DATABASE_URL` 与测试用 `DATABASE_URL` 指向它；sqlx 集成测试会创建独立测试数据库。

另开终端开发前端，Vite 将 API 请求代理到 `127.0.0.1:8080`：

```bash
cd web
bun install --frozen-lockfile
bun run dev
# After editing the frontend:
bun run build
```

提交前运行：

```bash
# DATABASE_URL must allow creating isolated test databases on PostgreSQL 16.
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
(cd web && bun install --frozen-lockfile && bun run build)
git diff --exit-code -- web/dist
```

依赖真实上游二进制的测试默认标记为 ignored，设置 `SINAN_TEST_SINGBOX` 后显式执行，不能把默认 `cargo test` 当作这些专项检查已经通过：

```bash
export SINAN_TEST_SINGBOX=/绝对路径/sing-box
cargo test -p sinan-compiler --test compilation -- --ignored
cargo test -p sinan-adapter-singbox --test runtime -- --ignored
cargo test -p sinan-panel --test reality -- --ignored
```

CI 另外检查分层禁用词、Linux 构建、Compose 启动持久化，以及双架构的真实 OpenRC 进程夹具。OpenRC 检查可运行 `bash scripts/ci-openrc-smoke.sh`，需要 Docker；它在临时容器内测试安装、升级、HUP、权限与异常退出恢复，不安装宿主服务。CI 的通过状态需要以远端实际运行结果为准。

## 实机验收与已知边界

`scripts/e2e-real.sh guide` 给出 systemd/OpenRC 的完整人工步骤；`snapshot` 在 Linux 设备自动识别 init，收集当前状态、服务 PID、统计监听和只读账本摘要，可选保存按用户、节点筛选的面板用量。OpenRC 快照区分 supervisor PID 与实际子进程 PID。它不会安装软件、修改业务配置、重启服务或更改账本；可选的面板查询会创建临时管理员会话并在结束时注销。

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

## 文档

- [接口与前端契约](docs/api.md)
- [设备协议](docs/protocol.md)
- [架构决策](docs/adr/0001-declarative-snapshots.md)
- [术语表](docs/glossary.md)
- [实现中的问题与选择](docs/open-questions.md)
- [阶段计划](docs/PLAN.md) / [验收进度](PROGRESS.md)

许可证：AGPL-3.0-only。
# Agent 监控与任务

服务器详情页可配置采样和上传间隔。默认一秒采样、三秒压缩上传，新增 SWAP、进程数、逐盘 I/O 和可用的 GPU 指标。普通指标在本地 SQLite 保留最多两小时、7200 个样本及 64 MiB，面板提交后才确认清理；重复补报不会覆盖较新的指标。GPU 利用率依赖设备提供的工具，无法采集时保持缺失。

可配置 TCP/ICMP 持续拨测及线路备注，每轮四次测量，显示延迟、丢包和历史。拨测最多 32 项，间隔 10–3600 秒；离线期间最多继续使用一天前同步的配置，结果在本地保留两小时、4096 条。公网 IPv4/IPv6 自动识别可以在面板关闭，设备本地关闭时面板不能覆盖。

远程命令只允许已登录管理员下发，使用设备服务账号，在 Unix 上运行 `/bin/sh`，Windows 上运行 PowerShell。单条命令最多执行 600 秒，领取期限最长一天，标准输出和错误输出各保留 256 KiB。Agent 先持久记录再执行，重启后将状态不明的命令标记中断，不重复执行；执行结果确认后才清理待上传状态。

Linux OpenRC 也支持 NodeQuality 独立一次性服务，使用独立挂载命名空间、超时和持久完成记录。安装时需要 `unshare`（BusyBox 或 util-linux）；Agent 重启不重启已经开始的诊断。实际双架构结果以 CI 的 OpenRC 任务检查为准。


## 原生平台运行时与服务

`tools/build-runtime-native.py <target> <ARTIFACT_ROOT>` 固定 Go 1.26.8、上游 sing-box 1.14.2 及 cronet 提交，不修改源码。目标包括 `macos-arm64`、`freebsd-amd64`、`freebsd-arm64`、`windows-amd64`、`windows-arm64`。macOS 和 Windows 在对应原生 runner 构建；FreeBSD 使用官方纯 Go 标签交叉编译。Windows 包含对应架构的 `libcronet.dll`，Agent 检查整个文件集合和缓存摘要。CI 同时上传 `sinan-runtime-<target>`；导入 `data/artifacts/sing-box/1.14.2/` 并合并摘要即可由面板分发。

Unix 默认配置 `/etc/sinan/agent.toml`、Agent `/opt/sinan/core`、状态 `/var/lib/sinan/core`；macOS/FreeBSD 状态套接字 `/var/run/sinan/agent.sock`。Windows 默认根目录 `%ProgramData%\Sinan`，使用受保护命名管道查询状态。`agent_root` 可单独指定 Agent 安装位置，不依赖代理 `install_root`。`run --monitor-only` 用于不管理代理服务的监控场景。

同一个 CI 工作流在各原生平台运行注册、压缩遥测、补报、命令去重、拨测和升级回退检查；macOS、Windows、FreeBSD 15 额外验证真实服务安装、运行时配置及回环流量，FreeBSD 13.5/14 验证同一 Agent 与运行时二进制的启动兼容。成功状态以对应提交的 Actions 为准。GPU 实际负载、公网 Reality 客户端与整机断电/重启仍须在专用设备验收。
