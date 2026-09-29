# 执行中的问题与选择

## 2026-09-30：验证环境

当前工作机是 macOS；使用已有 Rust stable 1.97.1 和独立的本地 PostgreSQL 16 运行测试。systemd 和 Debian 12 运行链路另由 Linux CI 和实机脚本验收，不能把本机测试视为实机完成。

## 内部依赖边界

“agent-core 只依赖 adapter-sdk 和 protocol”指 workspace 内部依赖；基础库仍使用规定技术栈。适配器内部只依赖 adapter-sdk。

## 本地文件读取

系统曾把刚创建的部分文件标记为 dataless，读取超时。仅按本次已知内容原子重建新文件；后续编辑优先原子替换，保护已有数据。

## 开源许可

文档未指定许可证；采用 AGPL-3.0-only，适用于自托管网络服务。发布时保留依赖原有许可证。
