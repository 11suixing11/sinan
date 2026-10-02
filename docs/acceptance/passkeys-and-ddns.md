# Passkey 与 DDNS 双栈本地验证

日期：2026-10-02。对应 [ADR 0059](../adr/0059-passkeys-and-proxy-user-access.md)、[ADR 0060](../adr/0060-ddns-dual-stack-creation.md)。在当前 `main` 上开发，CI 继续暂停；未执行生产迁移、部署、真实 DNS 写入或原生认证器验收。

## 实现边界

- 共用验证、服务端挑战及凭据：`crates/panel/src/passkeys/`；管理员绑定/删除/登录及新鲜密码/TOTP 验证：`crates/panel/src/auth/{passkeys,proof}.rs`。
- 代理用户邀请、独立会话、自身订阅和用量：`plugins/singbox/panel/portal/`；删除原代理用户同时清理认证账户。旧用户 ID、授权、订阅令牌、代理流量不迁移或重置。
- 数据库只新增 `0039_passkeys.sql`，不修改历史迁移。默认源/精确端口、会话和用途绑定、一次性挑战、账户撤销版本和签名计数更新在服务端检查。
- DDNS 新增 `POST /api/plugins/ddns/rules/dual-stack`，现有单家族数据模型/同步器继续使用；一个事务创建 A 和 AAAA，任何冲突或容量不足时整体回滚，创建后独立运行。
- 原生 WebAuthn 前端不增加 SDK；新增 Rust 库的依据、替代方案及 OpenSSL 部署要求见 ADR 0059 和 [使用说明](../passkeys.md)。

## 已通过的本地检查

| 检查 | 结果与范围 |
| --- | --- |
| `cargo fmt --all --check` | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过，含新测试与迁移引用 |
| `cargo test --workspace --no-fail-fast -- --test-threads=4` | 98 组，780 通过、0 失败、21 默认忽略；新浏览器条件用例随后显式运行通过，其他 20 项原有条件用例仍未验证 |
| Passkey 虚拟认证器 | 真实面板路由、隔离 PostgreSQL、构建产物和 Chromium CTAP2 虚拟认证器，19 组端到端场景通过 |
| DDNS 数据库测试 | 四家双栈创建、凭据脱敏、第二家族冲突回滚、31 条拒绝增加两条、30 条允许增加两条，以及旧 CRUD/迁移/IP 回归通过 |
| Bun | 63 项、1345 个断言通过，包含用户路由边界与用户认证失败不退出管理员会话 |
| TypeScript/Vite | 构建通过，已更新提交的 `web/dist`；保留既有主包超过 500 kB 提示 |
| 构建页面回归 | DDNS、订阅、授权草稿、快照写保护、插件业务和原登录/服务器运营六组通过；新增列表刷新后重跑受影响的订阅与快照写保护 |
| 分层 | `tools/check-core-boundary.py` 通过，Agent 不新增认证库或代理业务依赖 |

虚拟认证器场景包括：管理员注册/登录、同挑战并发提交仅一次成功、篡改签名/Origin/RP hash/UV 标志被拒绝、过期/重复/错误浏览器挑战被拒绝、注册绑定发起管理员会话、用户一次性开通、订阅及流量仅本人可读、跨账户和跨角色的有效签名不能登录、用户备用密钥管理、过期验证不能加密钥、删除最后一把被拒绝、删除密钥撤销其他会话、管理员明确重置撤销旧密钥/挑战/会话、并发开通单赢家、用户删除后立即失效、已启用 TOTP 时绑定/删除/恢复必须验证且拒绝复用验证码、管理员删除 Passkey 后密码入口仍有效。

测试驱动为 `crates/panel/tests/passkeys.rs` 和 `web/tests/passkeys.mjs`，显式执行方式见 [开发说明](../dev.md)。它只允许 SQLx 的隔离测试数据库及回环服务；为兼容 Bun 驱动移除 SQLx 私有连接参数，对隔离回环 PostgreSQL 使用明文测试连接，不修改产品连接方式。初期检查发现新增迁移使用了 PostgreSQL 保留列名，已修正；浏览器驱动中的选择器、文本错误响应解析和数据库连接参数也已修正后复跑通过。

## 未验证范围

没有使用真实 Touch ID、Windows Hello、手机扫码、第三方同步 Passkey、安全密钥或实际 HTTPS 反向代理；没有进行生产数据库升级/回滚或 Docker 镜像运行验证。新库在本机 OpenSSL 上已编译并完成密码学交互，容器安装依赖已声明，但不能将本机结果等同于全部发布平台验证。没有调用四家真实 DNS 写接口或验证传播、长期地址变更。CI 未运行，不能据此宣称主分支 CI 全绿；原诊断实机签收门禁保持。
