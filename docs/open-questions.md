# 执行中的问题与选择

## 2026-09-30：验证环境

当前工作机是 macOS；使用已有 Rust stable 1.97.1 和独立的本地 PostgreSQL 16 运行测试。systemd 和 Debian 12 运行链路另由 Linux CI 和实机脚本验收，不能把本机测试视为实机完成。

## 内部依赖边界

“agent-core 只依赖 adapter-sdk 和 protocol”指 workspace 内部依赖；基础库仍使用规定技术栈。适配器内部只依赖 adapter-sdk。

## 本地文件读取

系统曾把刚创建的部分文件标记为 dataless，读取超时。仅按本次已知内容原子重建新文件；后续编辑优先原子替换，保护已有数据。

## 开源许可

文档未指定许可证；采用 AGPL-3.0-only，适用于自托管网络服务。发布时保留依赖原有许可证。

## 无可用授权时的订阅

links 格式返回空文本的 Base64（空字符串）；singbox 格式返回“无已应用节点”的错误，避免生成空 selector 或悄然走直连。跨服务器节点允许相同端口；同一服务器内禁止端口重复。

## G4：接入、制品和历史记录

- 安装尚未产生设备 session，增加 `/api/bootstrap/{version}/{arch}?token=...`，只允许有效未消费注册 token 下载 Agent，下载不消费 token。运行时制品与配置包仍要求设备 Bearer 会话。
- 制品保存为 `artifacts/{agent|sing-box}/{version}/{amd64|arm64}`，每个版本目录使用 SHA256SUMS 列出架构文件哈希。面板校验文件与清单匹配才提供下载。
- 同设备升级可用新 token 再次注册同一公钥；已有服务器不允许替换成另一公钥。安装脚本先用暂存二进制完成注册，再激活版本。
- 服务器、节点和用户采用软删除，保留流量与部署历史。增加 usage_batches 表保证同一批次编号不能以不同记录集合重复入账；usage_records 仍保留规定唯一约束。
- 管理员会话 24 小时、设备会话 1 小时。管理员密码只在首次创建时生效，密码校验并发限制为 4，避免匿名请求同时分配无上限 Argon2 资源。

## G4：本地 Rust 链接器兼容

macOS 27 的动态库加载器暴露了 Rust/LLVM 删除调试信息后的 LINKEDIT 对齐问题。dev/test 显式设置 strip="none" 后编译通过，保留 debug=0 控制磁盘使用。参考上游问题：https://github.com/rust-lang/rust/issues/157750 。Linux 部署不依赖此工作机环境。
