# 仓库协作规则

本仓库实现 Sinan MVP。任务说明中的架构决策与范围是实现约束；遇到歧义，采用最简单且符合既定架构的方案，将问题、选择和理由记录到 `docs/open-questions.md` 后继续执行。

## 分层

- `protocol` 定义面板与 Agent 的公共消息；`compiler` 将模型确定性地编译成完整配置包。
- `agent-core` 的工作区依赖只能是 `adapter-sdk`、`protocol`；原生配置、身份、传输、对账、状态、遥测、流量和制品管理放在 core。
- `adapter-*` 的工作区依赖只能是 `adapter-sdk`。适配器只负责无状态翻译，不自行持久化，不连接面板。
- 只有 `agent` 二进制入口同时依赖 core 和具体适配器，并将适配器注册到 core。
- `agent-core` 目录内任何文件不得出现 `singbox` 或 `sing-box` 字样；使用模块标识、能力和统一接口。
- 特权操作必须经过 `Privileged` trait，服务管理经过 `ServiceManager` trait；外部运行时是独立的 systemd 或 OpenRC 服务。
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

不得实现链路、转发、链式代理、外部出口、出口池、用户分组、配额强制执行、计费、DDNS、WebSSH、frp、Shadowsocks、SSM API、VLESS + Reality 之外的协议、xray、多个运行时实例、独立特权 helper 进程、防火墙或 nftables、正式自更新、非 Linux 运行平台、Clash 订阅、多管理员、权限体系、多语言界面或面板高可用。OpenRC 服务支持已按用户要求增加，详见 ADR 0016。特权 helper 仅保留 trait 边界；升级通过重复执行安装脚本完成。制品签名和高频指标模式只有 G1–G9 全部完成后才可考虑。

详细架构约束见 `docs/adr/0001-declarative-snapshots.md` 至 `docs/adr/0011-loopback-local-api.md`。
