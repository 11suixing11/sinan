# 司南 Sinan

自托管的中文服务器与代理节点控制面板。面板保存期望配置；Agent 主动连接，负责对账、应用恢复、系统遥测和按用户计量。代理运行时由独立系统服务管理，Agent 重启时继续提供服务。

提供 VLESS + Reality、Hysteria2、Shadowsocks 2022、TUIC v5、AnyTLS、Naive 和 Snell v6，以及用户授权、订阅、部署状态与流量汇总。TLS 支持手动证书及自动申请、续期，见 [协议与证书配置](docs/proxy-protocols.md)。节点端口可指定为 443 等可用端口，订阅链接可一键重置；管理员登录支持限速和 TOTP 二步验证。删除在线设备时先停服务、清凭据，离线设备仅删除面板记录。

运行时固定上游 sing-box 1.14.2，保留官方默认构建标签并启用统计 API。NodeQuality 日常检查使用有限轻量探测；完整验机因在线执行依赖、内层上传和宿主 swap 风险暂停新任务，详见[安全门禁](docs/acceptance/nodequality-full-start-gate.md)。

准备 Docker Engine、Compose 插件与 Python 3，按[部署文档](docs/deploy.md)核对仓库内发布公钥，在仓库根目录安装：

```sh
python3 scripts/panel.py install --public-url https://panel.example.com
```

面板默认在 `http://127.0.0.1:8080`。登录后可在 `/#/dashboard` 查看独立服务器看板，或从后台侧栏“服务器看板”跳转；看板右上角可返回后台，旧 `/#/overview` 链接继续可用。看板可在「看板与通知」中配置公开访问，默认仍需登录。

后台“统计仪表盘”（`/#/statistics`）提供服务器状态、近 7/30 天网卡与代理流量趋势和排行，仅管理员可见。节点可配置监听/公开端点、启停与各协议高级参数；代理用户的订阅窗口支持可用状态、配置预览、复制与下载。面板管理工具提供状态、日志、自检和私有备份，升级会先备份，保留凭据与数据卷。

sing-box 运维入口提供状态、脱敏日志、重启和失败部署重试；服务器任务页展示命令执行过程并支持按设备能力取消。代理节点页统一管理直连与有序混合链路，可导入机场订阅并选择具体节点作为中间段或出口，支持固定版本及跟随同一节点更新。使用方法与平台限制见 [Agent 运维与混合链路](docs/agent-runtime-and-chains.md)。

服务器看板支持卡片与表格、实时状态、分层历史、跨币种成本及每日汇率缓存。Agent 的采样、实时上报与历史批量写入分别配置，监控历史默认保留 30 天。操作与数据语义见[服务器看板](docs/server-display.md)。

「延迟检测」可统一分配 TCP/ICMP 任务；「看板与通知」可配置离线、资源、到期、流量提醒以及 Telegram、Webhook 渠道，各渠道独立记录投递和重试状态。操作见[延迟检测与通知](docs/monitoring.md)。检测与通知均需管理员配置，不执行付款、停用或远程命令。

「动态域名解析」是按服务器启用的面板插件，使用 Agent 上报的 IP 更新 Cloudflare A/AAAA 记录，支持暂停、手动同步与失败重试。可从插件目录或服务器插件页进入，详见 [DDNS 使用说明](docs/ddns.md)。

远端接入需要可达的 HTTPS 地址。维护者先按[部署维护流程](docs/deploy.md#导入签名-release)使用管理员运维接口准备对应服务器架构的签名 Release，再到服务器接入页复制一次性安装命令，在目标服务器执行。选择 Shell（Linux、macOS、FreeBSD）或 PowerShell（Windows）入口，自动匹配本机系统、CPU/ABI 与最新兼容的已签稳定版本，也可指定已签版本。一行命令自动下载官方独立安装入口，核对固定入口摘要、准备验证工具并核验制品签名，无需预装 `sinan-bootstrap`。

Agent 从 GitHub Release 或配置的独立 HTTPS 镜像下载，面板不提供 Agent 二进制。独立入口内嵌可信的 Linux 安装执行器，可在核对完整签名证明后安装旧 `agent-v0.3.0`，无需修改其已发布资产；其他平台仍需对应已签制品。Agent 与面板版本独立，设备只应用内嵌公钥认可的制品。

「插件目录」展示插件介绍和分发版本，选择服务器后进入对应服务器的管理页；插件启用和诊断任务仍需明确操作。目录不提供 Release 导入或面板本机安装，详见[插件目录与服务器执行边界](docs/plugin-catalog.md)。

- [部署、制品导入、节点接入与升级](docs/deploy.md)
- [服务器看板、表格视图与历史曲线](docs/server-display.md)
- [后台统计仪表盘与流量口径](docs/statistics.md)
- [服务器资产、续费记录与流量额度](docs/server-assets.md)
- [开发、测试和 CI](docs/dev.md)
- [离线签署、发布与公钥轮换](docs/release.md)
- [真实 Reality 验收与阶段证据](docs/e2e.md)
- [sing-box 策略组与套餐周期](docs/singbox-groups.md)
- [Agent 运维、命令状态与机场混合链路](docs/agent-runtime-and-chains.md)
- [混合链路与机场订阅来源的设计和验收边界](docs/node-chain-design.md)
- [HTTP API](docs/api.md) / [设备协议](docs/protocol.md)
- [架构决策](docs/adr/0001-declarative-snapshots.md) / [问题与选择](docs/open-questions.md)
- [执行计划](docs/PLAN.md) / [验证进度](PROGRESS.md)

支持 Linux systemd/OpenRC、macOS launchd、FreeBSD rc.d 和 Windows 计划任务；各平台的服务、升级及验收边界见 [设备平台与能力](docs/platforms.md)。当前业务协议与单实例范围见架构决策。许可证：AGPL-3.0-only。
