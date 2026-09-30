# sing-box 业务搬迁独立验收（Issue #26）

## 验收合同

1. 添加纯监控服务器，不声明 singbox、没有旧节点/部署：插件元数据 enabled=false；服务器详情无代理部署/节点，也不请求业务 API。新建代理节点返回 409，说明需到系统插件设置明确启用；后台发布不生成代理配置。
2. 管理员从系统插件设置明确启用，再新建节点；重复启用只有一条启用记录。当前设备声明能力显示只读来源，历史节点/部署显示兼容只读来源；未知能力不默认启用。
3. 迁移前导入代理用户、原订阅令牌、设备公钥、Reality 密钥、授权 UUID、已健康部署和流量账本；应用 0012 后原 `/sub/{token}` 返回相同分享链接和 sing-box JSON。用户 ID、令牌、节点密钥、授权及历史数值均保留，epoch 未改变。
4. 代理用户和代理节点在 sing-box 插件导航；系统管理员在系统导航。服务器网卡累计流量仍显示在服务器详情。后端管理 API 和前端同时使用 `/api/plugins/sing-box/...`；旧 `/sub` 继续可用。
5. 运行既有发布、会话重连、SQLite 对账/丢 ACK/重启、历史计量、在线退役与订阅重置回归。此代码所有权搬迁不改变诊断资源或 IP 源行为；这些场景由本版本完整回归和各独立 PR 的验收记录验证。

## 可重复检查

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- 设置独立 PostgreSQL DATABASE_URL 与 TEST_ONLY 编译信任根后 `cargo test --locked`。
- `cargo test --locked -p sinan-panel --test plugin_business` 覆盖真实 0012 迁移及启用行为。
- `python3 scripts/test-e2e-driver.py`、`python3 tools/check-core-boundary.py`。
- Bun 1.4.2 构建实际 web/dist；使用外部 Playwright/Chrome 运行 `web/tests/singbox-business.mjs`，桌面与手机分别检查纯监控页面、启用后业务页面、导航及显式启用流程。

## 当前证据

实现阶段：Python 端到端验收驱动的 21 项回归通过，workspace fmt 通过，TypeScript/Vite 构建通过。整合 main `6a583af` 后实际 dist 的桌面（1280）/手机（390）浏览器场景均通过，未启用服务器不发业务请求、显式启用、只读来源、管理员/代理用户导航和横向溢出检查均通过，页面错误为零。完整 Rust/PostgreSQL及最终 CI 验收待执行；不以代码移动或旧版本 CI 代替本项完整通过。
