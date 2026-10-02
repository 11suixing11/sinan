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

只读复核发现候选筛选与发布事务间设备切回纯监控的竞态：发布阶段再次在锁定服务器的查询内检查启用证据。新增 PostgreSQL 回归先选旧候选，再清除能力，验证零部署、保留待发布标记，管理员明确启用后才允许发布；该新项由新提交 CI 验证，不包含在旧291记录中。

## 最终代码 CI 证据

代码 `29c4df4`，基于 main `229becc`，CI [36780128478](https://github.com/theLucius7/sinan/actions/runs/36780128478) 实际成功：workspace fmt、全 targets Clippy、完整 Rust/PostgreSQL **307 成功 / 0 失败 / 9 既有环境条件忽略**；新增三项迁移/启用场景与发布竞态 PostgreSQL 场景均实际通过。随后独立真实 systemd 诊断夹具 **6 成功 / 0 失败 / 0 忽略**，包含内存/任务数限制、排队启动、超时恢复、预检互斥、锁权限以及取消后的进程和私有挂载确认。

Compose 与 musl amd64/arm64 成功，该 PR 事件的 Reality 安装计量任务按草稿策略跳过。旧代码 `1fe1cc5` 的另一流水线仍可能独立运行 Reality，不把其结果用于认证最后新增的并发修复。专用小内存节点和生产迁移仍属于单独实机验收。本段追加仅为验收文档，不改变已验证代码。

## 最终 main 整合验收

正常保留作者 `9cc507e` 与 main `8254055`（确认取消、IP 入口、日常/完整模式及 R5 兼容），源码验证基线为 `276bdea`。模式页通过业务中性 `plugins::runtime_activity_on` 读取启用和近期正向计量证据，业务 SQL 留在 sing-box 插件；真实 PostgreSQL 断言纯监控日常检查为 `not_enabled`，且不会生成部署。发布同时保留作者锁内候选筛选和独立语句启用重检，两个并发回归均通过。

本机磁盘耗尽曾中断整套链接，未将中断计为成功。随后按 Cargo target 清单串行完成全部八个 workspace 包的 all-targets 覆盖，并在统一 workspace 依赖图补跑所有 library 与 sing-box adapter/runtime：去重后 **325 成功 / 0 失败 / 9 既有环境条件忽略**（Panel100、protocol19、compiler7、NodeQuality16、SDK1、Agent core168、Agent2、sing-box12）。真实 PostgreSQL 包含 0012 故障回滚/重试/幂等、十张旧表完整快照、旧订阅两种格式、旧账本重放与改 payload 拒绝，以及取消、章节、模式和 provider 交互。统一 workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁及六项行为回归、actionlint、差异检查全部通过。

Bun 1.4.2 冻结依赖、五项字段测试/711 断言及 TypeScript/Vite 构建通过；最终 dist 为 `index-Hx7wA0D0.js`。实际 Chromium 桌面和 390px 手机运行插件启用/导航/零业务请求、确认取消、IP 来源展示与每种尺寸七个模式场景，页面错误为零；浏览器 API 使用明确夹具，与上述真实 PostgreSQL 后端验证分别记录。Python discovery83通过/5条件跳过、R5包装器34通过、daily helper7通过。测试显式移除真实 `SINAN_ABUSEIPDB_API_KEY`，仅用合成凭据和回环 HTTP。本轮 macOS 未执行真实 Linux/root/systemd、正式 API 账户/配额、完整上游负载或生产迁移；最终 head CI 与专用节点继续独立核对。

## 根插件目录恢复（独立后续 PR）

业务代码由 crates/panel/src/plugins/singbox/ 恢复至根 plugins/singbox/panel/，落实用户要求与 ADR0023。面板 plugins/mod.rs 只通过明确的 path 属性嵌入同一模块；13个文件使用 git mv，逐一对比迁移前后 Git blob SHA，所有业务与测试字节完全相同。原 API、旧 /sub/{token} 路径、设备凭据、用户ID/令牌、授权、账本及 epoch 语义未修改；本项不新增数据库迁移、不改前端或套餐功能。

现有 Dockerfile 已 COPY plugins/，无需改变镜像构建上下文。验证分为：直接 rustfmt/core 门禁与 Git差异检查；最新 PR 的 Rust编译、Clippy和既有插件/订阅/账本测试由独立CI核对。物理路径变更不冒用业务首次实机流量证据，新的CI尚未完成前仅记为待验。

本轮冻结源码 `7d1bda4` 在 macOS 独立回环 PostgreSQL 完成19项专项、0失败/忽略：搬迁 publisher2、插件启用/旧数据迁移4、账本4、业务/旧订阅4、订阅重置2、端口2、真实 Agent 的配置发布/流量/丢失ACK/重启1。workspace 全 targets Clippy（warnings为错误）、fmt、core 门禁/六项行为、build-script5、runtime-cache3与差异检查通过。逐一 Git blob 对比及物理模块树确认13文件完全相同、pub(super)与公开 Rust 导出保持；独立数据库仅在127.0.0.1:55432启动并已停止。

正常合入正式main `5d908b9` 保留本项桥与13项已验业务原字节；后续差异仅来自独立TCP/制品构建流程及验收文档，本项不将其他冻结源码证据转记为最终HEAD完整workspace或真实Docker/systemd验收。最终主线CI继续独立核对。

## 整体交付补验：旧订阅实际连接

源码979b998在专用Debian12，以搬迁前固定订阅合同和真实sing-box1.14.2通过迁移前后同客户端/PID、授权计数、历史保留与重复上报验收。详见[真实客户端验收](imported-subscription-runtime.md)。仅有自有回环TLS/Reality/echo和真实PostgreSQL，不冒充生产迁移或Agent托管再发布；前置P0门禁与CI暂停保持。
