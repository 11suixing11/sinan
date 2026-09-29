# 执行进度

## G1：已完成

- 建立七个 crate 的 Rust 2021 workspace，每个 crate 根禁止 unsafe。
- 完成 AGENTS 分层与禁止事项、术语表、11 条既定架构 ADR、CI、执行计划。
- 验证：Rust 1.97.1 下 `cargo build`、`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 全部通过。
- 环境问题：系统网络直连不稳定，使用系统现有代理的进程级配置重试；项目文件曾被 macOS 标记为 dataless，重写本阶段新建文件后恢复。PostgreSQL 16 正在准备。
- 下一步：G2 协议与兼容测试。

## G2：已完成

- 完成全部 WebSocket 消息与 HTTP 注册、清单、配置包的类型；信封可保留并忽略未知消息，未知字段保持兼容。
- 完成协议规范，明确 nonce 签名编码、会话范围、哈希原始字节、重发及确认语义。保存原始任务说明便于验收追踪。
- 验证：全 workspace fmt、clippy、test 通过；协议 9 项测试覆盖全部消息往返、未知字段/类型、必需字段错误及缺失指标省略。
- 环境恢复：项目已在 Finder 标记保留下载，编译产物与测试数据库移出同步目录；文件读取已恢复。托管 worktree 创建失败，继续使用原仓库。
- 下一步：G3 确定性编译器与黄金测试。
