# 共用诊断任务服务验收

对应 Issue #27。仅抽取共用服务并迁移 NodeQuality，其他插件单独 PR 登记。

- 登记插件在 `diagnostic_plugins`，NodeQuality 面板参数/报告规则在 `plugins/nodequality/panel/`。插件文件不写 `diagnostic_jobs`；面板共用代码不含 NodeQuality 的执行参数、报告章节、工具版本或链接来源规则。
- 服务器行锁与统一活动任务查询约束所有插件和所有创建入口，取消确认前保留互斥。既有结果、章节、设备范围权限和终态幂等沿用同一服务。
- 新协议预算只由登记计划设置，需设备声明能力，Agent 只收紧适配器限制。旧 JSON 无预算时保持兼容，历史报告不改名、不迁移。
- 新 PostgreSQL/HTTP 夹具覆盖旧/新路由同时提交、预算持久化、部分报告、旧 r2 原文、不同插件历史、不同插件活动互斥、设备凭据不能使用管理员入口、缺失能力与预算注入拒绝。
- 原 NodeQuality Agent/Panel 恢复、确认式取消、资源不足、失联、章节重传和历史测试继续执行。

代码提交 `bfae951` 在专用 Debian 12 构建容器通过 `cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings` 和完整 `cargo test --locked`：318 项通过、0 失败、9 项既有条件忽略，包含真实 PostgreSQL/HTTP。容器限制为 2 CPU、1536 MiB 内存与 512 任务，退出码 0、OOMKilled=false；该构建不运行硬件压测。Linux 包装器 29 项、日常探测辅助器 6 项、分层行为 6 项也通过。共享构建缓存前刷新完整快照文件时间，日志保存于独立 `framework-*.log`。

回归曾发现历史 `job={}` 上传正文被错误当作未登记插件；已修复并重新运行上述完整验证。缺少插件字段的历史任务归入原 NodeQuality 历史，正文继续接收，链接仍受 NodeQuality 的允许来源约束；不修改历史 JSON。

当前提交的 GitHub CI、真实 root/systemd 6 项与双架构构建另行核对，不把构建容器内忽略项算作通过。专用 Debian 节点 SSH 当前不可达，完整验机及持续流量下心跳/取消实机总验收待恢复后补齐。

最终合并审查正常保留作者 `924a8ff` 和主线 `2c3c1e5`。NodeQuality 仍为 r5；迁移后的活动判断通过中性运行时证据接口读取，不查询代理业务表。历史夹具实际删除 plugin 和 resource_budget，验证原始 JSON、r2 原文与 legacy 完整度精确保留；queued、running 和过期但未确认的 cancel_requested 记录均阻止通用及旧入口创建，不因超时冒充取消确认。Agent 预算回归覆盖所有五种限制的不可放宽，以及拒绝时服务配置完整不变；协议拒绝未知预算命令字段。

最终相关 Rust/PostgreSQL 专项 68 项通过、0 失败/忽略：协议 12、Agent 诊断/预算 39、共用服务 2、诊断 API 10、章节 3、真实 Agent WS/HTTP/重启/取消 2。workspace 全 targets Clippy、fmt、core 门禁、actionlint 与差异检查通过。完整 workspace 测试尝试还通过另外 47 项、忽略 2 项既有条件测试，但 accounting 链接遇到磁盘耗尽而中止，不能视为完整 workspace 通过。包装器、daily/observer 和前端与已验证主线原字节相同，复用其既有证据；本轮未运行真实 root/systemd、完整硬件压力或生产节点验机，最终提交的 CI 单独核对。
