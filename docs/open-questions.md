# 执行中的问题与选择

## 2026-09-30：测试环境

当前工作机为 macOS，缺少 Rust、Docker 和 PostgreSQL。安装独立 Rust stable 工具链及本地 PostgreSQL 16，用真实数据库完成集成测试；systemd 和 Debian 12 运行链路以 Linux CI 和真实服务器验收脚本验证，不能把 macOS 测试视为 Debian 实机验收。

## 2026-09-30：依赖边界

“agent-core 只依赖 adapter-sdk 和 protocol”解释为 workspace 内部依赖边界；其实现仍使用技术栈明确允许的 tokio、SQLite 等第三方基础库。适配器的内部依赖只指向 adapter-sdk。
