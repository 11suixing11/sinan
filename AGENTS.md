# 仓库协作规则

本仓库实现 Sinan MVP。任务说明中的架构决策与范围是实现约束；遇到歧义，采用最简单且符合既定架构的方案，将问题、选择和理由记录到 `docs/open-questions.md` 后继续执行。

## 临时 CI 暂停（2026-10-01 用户要求）

当前 GitHub Actions 工作流已暂停。所有进行中的任务完成前，只做相称的本地验证，不触发、重跑或重新启用 CI；不要因单个 PR 或单个任务完成而恢复。全部任务完成后再统一恢复并验证最终整合提交。暂停期间未执行或取消的 CI 必须记为未验证，不据此宣称 main 全绿。本临时安排覆盖下文逐阶段运行远端 CI 的要求，恢复时移除此节。

## 本轮交付方式（2026-10-01 用户最新要求）

用户已将“每项单独PR”改为“一次交付所有”。当前整改后续工作统一放在集成分支，最终整体交付，不再逐项新建PR；原有逐项验收场景和证据要求保持。已合并内容保留，现有未合并草稿的实现与记录在集成完成后统一整理。此要求覆盖下文及旧文档中的逐项PR规则，不改变完整验机门禁、许可条件、生产边界或临时CI暂停。

## 本轮执行节奏（2026-10-01 用户最新要求）

用户要求“中途不要再进行测试，修改完了一个大步骤提交”。每个完整大步骤先集中完成相关实现、界面、协议、测试代码及文档修改；修改期间不运行测试、构建或穿插验证命令。整步修改完成并冻结输入后，再统一执行该步骤需要的验收、记录结果并提交；不得拆成反复测试和提交的小循环。最终验收发现的问题修复后，仅补验失败或受修复影响的范围。已有证据保留，未执行的测试明确记为待验，不替代原验收要求；临时CI暂停和完整验机许可门禁继续有效。本节覆盖下文及旧文档中的中间验证节奏。

## 分层

- `protocol` 定义面板与 Agent 的公共消息；`compiler` 将模型确定性地编译成完整配置包。
- `agent-core` 的工作区依赖只能是 `adapter-sdk`、`protocol`；原生配置、身份、传输、对账、状态、遥测、流量和制品管理放在 core。
- `adapter-*` 的工作区依赖只能是 `adapter-sdk`。适配器只负责无状态翻译，不自行持久化，不连接面板。
- 只有 `agent` 二进制入口同时依赖 core 和具体适配器，并将适配器注册到 core。
- `agent-core` 目录内任何文件不得出现 `singbox` 或 `sing-box` 字样；使用模块标识、能力和统一接口。
- core 管服务器；sing-box 插件管代理用户、授权、订阅、用户流量、配额和周期，详见 [ADR 0023](docs/adr/0023-proxy-business-boundary.md) 与 [搬迁及启用兼容 ADR 0030](docs/adr/0030-singbox-plugin-business.md)。core 不得引用代理业务的 `user`、`subscription`、`quota`，含复数、蛇形和驼峰形式；CI 使用 `tools/check-core-boundary.py` 检查。系统账户与 SQLite 原生 API 仅允许检查器列出的具体表达式，不允许文件或整行豁免。
- sing-box 面板业务实现物理位于根 `plugins/singbox/panel/`；面板只保留薄的 Rust path 嵌入桥，不得移回 `crates/panel/src/plugins/`。
- 设备声明插件能力只表示支持，不自动启用新服务器的代理业务；管理员启用必须安排签名运行时安装，安装状态和设备应用确认分开，见 [ADR 0044](docs/adr/0044-singbox-plugin-lifecycle.md)。链路管理归代理节点，策略页面引用已有资源。
- 代理资源与原子链路创建按 [ADR 0054](docs/adr/0054-proxy-resource-batch-lifecycle.md) 使用 `kind` 与 ID 一起标识，专用入口不重复展示；批量入口／链路同事务创建，删除不清除幂等收据。旧、新节点删除共用策略及链路引用保护，完整链路资源删除清理入口并保留共享出口；旧链路解除关系接口语义保持。本步骤仍为受管两跳，不替代 ADR 0040 的订阅来源和完整混合路径实现。
- 订阅来源按 [ADR 0055](docs/adr/0055-subscription-source-lifecycle.md) 在插件内有界获取、解析及保存不可变版本；来源设置 revision、身份 epoch、解析器版本与 claim 同时限制迟到结果。失败保留成功批次，归档／删除保留历史及幂等收据；完整地址、认证、原文、配置和摘要不进入公开输出。只导入节点，不执行来源内全局配置或嵌套下载；来源预览不能冒称网络在线、完整混合路径或引用保护已验收。
- 精确运行确认与持久恢复屏障按 [ADR 0047](docs/adr/0047-runtime-checkpoints-and-recovery-barriers.md) 实现：实际配置/受控实例与稳定 activation 共同核对，结果先持久化再发送，未 ACK 不按 TTL 删除；屏障后禁止低于已承诺 revision 的 apply/rollback/recovery。旧 revision 心跳不替代新能力的精确收据，检查请求不得隐式重启旧业务；这些通用基础能力不代表混合路径或端到端探测已实现。
- 诊断自然结束、保护停止、取消及退役按 [ADR 0049](docs/adr/0049-confirmed-diagnostic-completion.md) 保留活动所有权，确认进程、挂载与排队 job 已清理后才提交终态；原始结果先固定，重启只重试清理。`cleaning` 为非终态，不因期限释放互斥；旧面板不能单独配新 Agent。
- 周期拨测按 [ADR 0050](docs/adr/0050-authorized-probe-leases.md) 明确记录来源、地区和自有/第三方同意依据；新设备只凭绑定身份、配置版本和单调期限的短租约执行，不恢复旧一天缓存。旧八字段 `ProbeSpec` 及已有历史保持，旧 Agent 的离线窗口不能冒称已修复；授权证据不得进入匿名看板。
- 节点出口 IPQuality 按 [ADR 0051](docs/adr/0051-independent-node-ipquality.md) 使用独立固定源和最小离线 rootfs，不依赖商业硬件工具；章节认证、版本和真实归档身份核对后同事务投影逐来源缓存，单调任务序号阻止旧回报倒灌。NAT 出口与网卡地址分别展示，部分结果、失败和取消不抹掉最近成功。外层签名的 notice 必须绑定完整配套对应源资产，不能用库存或 URLs 代替真实源包；源码准备不等于实机签收、许可审批或正式发布。
- 最小 IPQuality 输入按 [ADR 0052](docs/adr/0052-ipquality-derived-debian-inputs.md) 从重新认证的完整 Debian 缓存显式派生：只读借用原缓存、隔离离线求解精确子闭包，记录父收据、profile、正文身份及新增空间；不改旧 collection kind、不把候选或哈希相等当 builder 审批。签名索引的有界展开预算不能由旧采样替代，资源拒绝不降低管理预留。
- IPQuality 最小 profile 按 [ADR 0053](docs/adr/0053-ipquality-minimal-profile-chain.md) 贯穿 prepare、build、export 和制品核对：重新认证派生与绑定，精确包／源库存保持一致；只含必需包或工具集合匹配不能证明最小。公开证明与私有父身份上下文分开，共用工厂扩展默认关闭，不降低 NodeQuality 或 builder 审批门禁。
- 离线诊断工厂按 [ADR 0048](docs/adr/0048-nodequality-factory-capacity.md) 对 prepare/build/export 分阶段核算副本、树、临时文件和收据，准入与动态守卫均保留管理空间和 inode。容量计划不是来源认证或镜像审批，动态轮询不是内核硬配额；失败原日志与清理结果分别留存，不为取证重跑构建，也不删除旧材料来腾空间。
- 系统管理员与代理用户分别命名；服务器网卡总流量留在 core。计量 `epoch` 只标记计数器重置，不得用作套餐周期。业务搬迁保留用户 ID、令牌、旧订阅路径、节点凭据、授权和历史流量，数据库表先不改名。
- 诊断任务生命周期、资源预算、持久化、取消及历史由共用服务管理；插件只转换参数、执行和解析报告，见 [ADR 0028](docs/adr/0028-shared-diagnostic-job-service.md)。后续插件登记代码可以经独立审查和相称验证后合入准备；NodeQuality 迁移及前置阶段的实机验收通过后，才能签收、正式发布或部署后续新增诊断能力。
- 特权操作必须经过 `Privileged` trait，服务管理经过 `ServiceManager` trait；外部运行时是独立的系统服务（Linux systemd/OpenRC、macOS launchd、FreeBSD rc.d、Windows 计划任务）。
- core 按 `identity`、`transport`、`reconcile`、`state`、`telemetry`、`usage`、`artifacts`、`system` 拆分，先用单文件，超过约 400 行再按需拆目录。

## 实现与验证

- Rust stable、edition 2024；每个 crate 根文件（含二进制根、测试 crate 和构建脚本）使用 `#![forbid(unsafe_code)]`，禁止引入 `unsafe` 代码。
- 代码标识符、代码注释使用英文；`docs/`、README、PROGRESS 使用中文。界面只使用中文。
- 只使用任务说明列出的依赖；确需新增依赖时先在 `docs/adr/` 说明必要性和替代方案。
- 不修改 sing-box 上游源码；不得提交真实密钥、令牌、域名、IP 或其他环境凭证。示例使用占位符、保留示例域名以及明确要求的回环或监听地址。
- 不在核心路径留下仅有 TODO 的实现，不为范围外功能创建空目录或空模块。
- 按 `docs/PLAN.md` 中 G1–G9 顺序推进。每阶段结束必须通过 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`；G1 另需 `cargo build`。
- 测试必须覆盖协议兼容、编译确定性、账本事务与恢复、对账合并与回滚。需要 PostgreSQL 或真实运行时而环境不可用的测试使用带原因的 `#[ignore]`，在 `PROGRESS.md` 记录未验证范围与人工步骤，不得宣称已通过实机验收。
- 每阶段更新 `PROGRESS.md`，记录完成项、验证结果、已知问题、下一步，然后提交一个 Conventional Commits 提交。
- 保留现有未提交工作，不覆盖不属于当前任务的修改。

## 禁止扩大 MVP 范围

用户现已授权增加 Hysteria2、Shadowsocks 2022、TUIC v5、AnyTLS、Naive、Snell v6，以及 TLS 证书的自动申请和续期，见 [ADR 0034](docs/adr/0034-modern-protocols-and-certificates.md)，覆盖下述相应协议排除项。

不得实现链路、转发、链式代理、外部出口、出口池、用户分组、配额强制执行、计费、DDNS、WebSSH、frp、Shadowsocks、SSM API、VLESS + Reality 之外的协议、xray、多个运行时实例、独立特权 helper 进程、防火墙或 nftables、Clash 订阅、多管理员、权限体系、多语言界面或面板高可用。OpenRC 服务支持已按用户要求增加，详见 [ADR 0021](docs/adr/0021-openrc-services.md)。用户进一步确认补齐 Agent 高频监控、任务、自动更新及非 Linux 常驻部署，详见 [ADR 0022](docs/adr/0022-agent-capability-alignment.md)，覆盖原排除项。特权 helper 仅保留 trait 边界；保留重复安装升级。制品签名与编译时发布信任根按 [ADR 0017](docs/adr/0017-signed-release-artifacts.md) 执行。

当前整改额外授权服务器成本、续费到期、按账单日计算的网卡配额、可配置轻量周期拨测，以及 sing-box 插件的代理用户配额、重置周期和到期；按 [ADR 0023](docs/adr/0023-proxy-business-boundary.md) 分层，覆盖上述相关排除项。整改清单在同一集成分支整体交付，每项保留验收证据，专用测试机验证资源场景，不在生产机器上反复运行完整验机。

当前整改按 P0 保护服务器 → P0 IP 查询 → P1 sing-box 业务归位 → P1 共用诊断框架 → P2 TCP 接入的顺序签收实机能力。源码审查、源码合入、实机能力签收和正式发布/部署分别记录；前置阶段未通过时，后续实现可经独立审查及相称验证后合入作为准备，但不能签收、正式发布或部署新增诊断能力，不以其 CI 结果宣称前置阶段完成。逐项记录故障场景、证据对应的源码和未验证范围；NodeQuality 历史报告可读与完整执行能力须分别验收。执行条件和当前缺口见 [整改顺序与验收状态](docs/acceptance/ordered-remediation.md)。续费、配额和周期监控仍属于之后的独立工作。

2026-10-01 用户进一步授权 sing-box 插件的策略组、套餐组及可授权的两跳链路，覆盖上述对应排除项。按 [ADR 0035](docs/adr/0035-singbox-policy-package-groups.md) 实现：权限组与套餐分别分配，套餐使用不可变快照和原计量账本；独立入口到出口仅支持两台服务器，不扩大为任意拓扑或平台全局用户。不能把清空计量 epoch 当作重置套餐，也不能把订阅过滤当作运行时停用。保持旧用户/凭据兼容，不混跑新旧 publisher；生产迁移、发布与实机验收单独授权。

2026-10-01 用户进一步授权服务器展示隐藏、可选公开看板、离线站内告警及 Telegram 通知、Agent 下载加速与服务器网卡流量矫正，并明确 Agent 二进制必须从 GitHub 下载，不能由面板提供。按 [ADR 0038](docs/adr/0038-server-operations-and-public-dashboard.md) 实现：安装和自更新保留独立验签，镜像请求不携带设备凭据；运行时与配置仍经面板；公开看板使用只读白名单及统一隐藏校验，流量矫正不改代理业务账本。覆盖旧 ADR 的 Agent 二进制面板同源下载限制，CI 暂停安排不变。

2026-10-01 用户进一步确认统一延迟检测任务，以及资源超限、服务器到期、网卡流量和 Telegram 完整通知配置，见 [ADR 0039](docs/adr/0039-latency-tasks-and-notification-rules.md)。仍复用现有 Agent 拨测协议；通知仅提醒，不执行付款、停用或远程命令，诊断实机门禁和 CI 暂停安排不变。

2026-10-01 用户明确链路在代理节点页统一创建/管理，且中间段可来自机场等订阅配置。按 [ADR 0040](docs/adr/0040-mixed-chains-and-subscriptions.md) 规划有序混合链路，覆盖此前仅两台受管服务器及外部出口的对应排除项；中间/最终段可为受管节点或订阅中的具体节点。保持单运行时、独立公开入口、线性无环路径、入口单次计量及内部秘密不进入用户订阅；不扩展自动出口池。当前源码分别按 ADR 0054／0055 提供统一资源与订阅来源版本管理；完整有序路径编译、版本化发布及真实混合运行仍继续后续实施／验收，不以来源解析成功替代。

详细架构约束见 `docs/adr/0001-declarative-snapshots.md` 至 `docs/adr/0011-loopback-local-api.md`。
