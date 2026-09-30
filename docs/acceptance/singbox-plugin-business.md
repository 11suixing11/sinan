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

实现阶段：Python 端到端验收驱动的 21 项回归通过，workspace fmt 通过，TypeScript/Vite 构建通过。整合 main `6a583af` 后实际 dist 的桌面（1280）/手机（390）浏览器场景均通过，未启用服务器不发业务请求、显式启用、只读来源、管理员/代理用户导航和横向溢出检查均通过，页面错误为零。在独立 Debian 12 构建容器（1.5 GiB / 2 CPU / pids512 / 禁止额外 swap）通过最终代码 `b22386f` 的 workspace fmt、全 targets Clippy（warnings 为错误）和完整 Rust/PostgreSQL 291 项成功 / 0 失败 / 8 项既有 root/systemd/外部运行时条件忽略。新增三项启用/真实迁移测试无忽略。容器 exit0、OOM=false。首次 Clippy 报告抽取残留 import/多余引用，修正后重新 touch 全部源和资源并完整复跑；先前失败不记为通过。

远端日志位于 `/home/lucius7/sinan-remediation-build/target/singbox-business-{fmt,clippy,test}.log`，共享 target 的验证快照在运行前重新更新时间，避免跨分支旧 rlib。容器已移除，编译槽已释放。本提交 CI、专用节点和生产迁移独立核对，不由普通构建容器推断实机迁移通过。

最终正常整合 main `229becc`（已合并取消 #51 和独立包装器夹具修复 #56），保留取消 API、报告章节与遥测字段，再构建 dist；业务桌面/手机及取消浏览器回归通过。上述 291 项仅对应 `b22386f` 的旧 base，本轮 Rust/CI 按最终 head 单独验证。

合并审查阶段已正常保留作者 `b22386f` 与 main `6b63f71`。插件启用判定在发布事务的服务器行锁内再次验证，避免扫描后能力消失仍创建空部署；关闭时保留待发布状态。新增私有发布竞态、真实 0012 故障原子回滚/重试/幂等和十张旧业务/设备凭据表完整快照回归，并验证旧用量批次哈希的原样重放及改 payload 拒绝。此阶段仅完成源码；Rust/PostgreSQL 执行结果待共享槽补充。合并后的 Bun 1.4.2 TypeScript/Vite、实际 dist 桌面/手机插件场景和 Python 驱动 21 项、core 门禁通过。
