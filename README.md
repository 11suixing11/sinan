# 司南 Sinan

自托管的中文服务器与代理节点控制面板。面板保存期望配置；Agent 主动连接，负责对账、应用恢复、系统遥测和按用户计量。代理运行时由独立系统服务管理，Agent 重启时继续提供服务。

提供 VLESS + Reality、Hysteria2、Shadowsocks 2022、TUIC v5、AnyTLS、Naive 和 Snell v6，以及用户授权、订阅、部署状态与流量汇总。TLS 支持手动证书及自动申请、续期，见 [协议与证书配置](docs/proxy-protocols.md)。节点端口可指定为 443 等可用端口，订阅链接可一键重置；管理员登录支持限速和 TOTP 二步验证。删除在线设备时先停服务、清凭据，离线设备仅删除面板记录。

运行时固定上游 sing-box 1.14.2，保留官方默认构建标签并启用统计 API。NodeQuality 日常检查使用有限轻量探测；完整验机因在线执行依赖、内层上传和宿主 swap 风险暂停新任务，详见[安全门禁](docs/acceptance/nodequality-full-start-gate.md)。

先按[部署文档](docs/deploy.md)核对发布公钥、创建私有 `.env` 并配置构建时信任根，再在仓库根目录启动：

```sh
docker compose --project-name sinan --env-file .env \
  -f deploy/docker-compose.yml up -d --build --wait
```

面板默认在 `http://127.0.0.1:8080`。登录后可在 `/#/dashboard` 查看独立服务器看板，或从后台侧栏“服务器看板”、服务器管理页“打开服务器看板”跳转；看板右上角可返回后台，旧 `/#/overview` 链接继续可用。「看板与通知」可配置公开访问及 Telegram 离线通知，默认仍需登录。

远端接入需要可达的 HTTPS 地址。在制品页按服务器架构导入已签名的 Release，再复制面板生成的一次性安装命令到目标服务器执行。选择 Shell（Linux、macOS、FreeBSD）或 PowerShell（Windows）入口，自动匹配本机系统、CPU/ABI 与最新兼容的已签稳定版本，也可指定已签版本。一行命令自动下载官方独立安装入口，核对固定入口摘要、准备验证工具并核验制品签名，无需预装 `sinan-bootstrap`。

Agent 从 GitHub Release 或配置的独立 HTTPS 镜像下载，面板不提供 Agent 二进制。独立入口内嵌可信的 Linux 安装执行器，可在核对完整签名证明后安装旧 `agent-v0.3.0`，无需修改其已发布资产；其他平台仍需对应已签制品。Agent 与面板版本独立，设备只应用内嵌公钥认可的制品。

- [部署、制品导入、节点接入与升级](docs/deploy.md)
- [服务器看板、表格视图与历史曲线](docs/server-display.md)
- [服务器资产、续费记录与流量额度](docs/server-assets.md)
- [开发、测试和 CI](docs/dev.md)
- [离线签署、发布与公钥轮换](docs/release.md)
- [真实 Reality 验收与阶段证据](docs/e2e.md)
- [sing-box 策略组、套餐周期与两跳链路](docs/singbox-groups.md)
- [代理节点内创建与管理多条链路的设计（待实现）](docs/node-chain-design.md)
- [HTTP API](docs/api.md) / [设备协议](docs/protocol.md)
- [架构决策](docs/adr/0001-declarative-snapshots.md) / [问题与选择](docs/open-questions.md)
- [执行计划](docs/PLAN.md) / [验证进度](PROGRESS.md)

支持 Linux systemd/OpenRC、macOS launchd、FreeBSD rc.d 和 Windows 计划任务；各平台的服务、升级及验收边界见 [设备平台与能力](docs/platforms.md)。当前业务协议与单实例范围见架构决策。许可证：AGPL-3.0-only。
