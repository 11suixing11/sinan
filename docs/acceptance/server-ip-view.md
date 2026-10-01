# 服务器 IP 信息与 NodeQuality 视图拆分验收

对应 Issue #25，依赖未知字段 PR #44。此项只拆接口和视图，不新增来源、不改缓存 schema/诊断制品/任务执行。

## 接口与历史兼容

IP 独立 GET `/api/servers/{id}/ip-quality` 返回 ip_addresses/quality，另提供 public_ip_addresses/private_ip_addresses 分组，POST `/ip-quality/refresh` 与原刷新使用同一逻辑。公网分类复用后端现有查询规则，不在浏览器另写一套地址判断；原地址列表、去重排序、八地址上限与缓存语义保留。页面仅为公网 IPv4/IPv6 展开质量卡片，内网、Docker 等虚拟网卡及其他非公网地址合并到默认关闭的“内网地址”中；仅内网或空列表时禁用公网质量刷新。NodeQuality 独立 GET `/api/servers/{id}/node-quality/reports` 只返回 plugin_ready/plugin_reason/reports，与现有 POST 创建接口共路径。NodeQualityView 无 IP 查询字段，IP 读取不查询诊断准备，报告读取不访问 IP 缓存。

本地回归：`cargo test --locked -p sinan-panel --test ip_addresses` 验证混合 IPv4/IPv6、Docker 私网、ULA、映射私网、共享地址、链路本地、重复/无效输入和空列表；既有独立/兼容路由测试继续检查原字段与质量缓存。`SINAN_PLAYWRIGHT_MODULE=<playwright/index.mjs> node web/tests/ip-addresses.mjs` 使用构建后的 dist 和回环模拟 API，在 1280/390 宽度验证公网可见、内网默认折叠/键盘展开、刷新后展开状态、仅内网/仅公网/空列表及页面无横向溢出。测试不查询外部 IP 服务，不涉及 Agent 升级或生产部署。

旧 GET `/node-quality` 由 LegacyNodeQualityView 保留全部原字段，旧 POST `/node-quality/refresh` 保留原刷新别名。没有数据库迁移、表名修改、历史报告/缓存清理或旧路由撤销。保留主线当前 r3 签名制品、章节组件与 r2 历史兼容，本项不修改制品或执行。

`crates/panel/tests/diagnostics.rs` 中的独立 PostgreSQL/实际 HTTP 测试验证：

- 新旧 GET 和刷新都需要管理员会话；已删除服务器返回 404。
- 不具备 NodeQuality 能力的设备仍能读 IP，保留未知和原 provider/database 语义。
- 新 IP 响应不含 report/readiness 字段，新 NodeQuality 响应不含 IP/cache 字段。
- 拆分前已保存的历史报告 ID、job 和 report 原值不变；旧组合响应与两个新视图一致；旧刷新仍受同一冷却检查。
- 模拟损坏 IP 缓存使 IP API 返回 500，但 NodeQuality API 仍返回历史报告。

```sh
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
cargo fmt --all --check
cargo test --locked -p sinan-panel --test diagnostics
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cd web
bun install --frozen-lockfile
bun run build
```

## 页面

服务器子导航为“服务器概况 / IP 信息 / NodeQuality 验机”，各有独立 hash 路径。概况不挂载 IP/诊断视图。ServerIpInfo 显示入口与逐数据库响应，七个数据库仍明确属于同一 check-place；NodeQuality 只显示准备状态、参数与历史报告。切换页面不刷新来源或创建任务。

实际 dist + 明确模拟 API：IP 页检查成功/403历史/429未知/部分/过期/双入口/空值与错类型；只请求独立 IP API。点击 NodeQuality 页只请求独立 reports API，已保存报告仍可查看，IP 区块不存在。模拟 IP API 500 后仍可切换报告页查看旧报告，概况页不再包含两个区块，新 UI 从未请求旧组合接口。桌面 1280 px 和手机 390 px 无页面异常或横向溢出，检查报告页手机截图。

## 本次结果与边界

- fmt、core 分层门禁、差异检查、Bun 1.4.2 冻结安装/TypeScript/Vite、最终 dist 的上述真实桌面/手机浏览器夹具通过。
- 提交 `177bfc9` 的 CI `36769525534`：check 中 Rust/PostgreSQL、全 targets Clippy、systemd 与 dist 通过，Compose 和两项 musl 通过；旧基线 Reality 失败，不称全矩阵通过。整合最新 main 和未知字段修复后，由最终提交 CI 分别复验。
- 不执行完整 NodeQuality、Agent 重启/面板断连、诊断取消或持续代理流量压力；相关生命周期/资源场景由对应独立 PR 验收。

本次最终视图源码对齐 main `6a583af`，保留 DiagnosticSections 组件、r3 制品与 r2 历史、严格旧字段兼容和遥测心跳/指标过期。前次 CI 仅认证当时源码，整合后最终 CI 单独核对。

- 合并审查最终保留 main `6a583af` 与作者最新 `84938ba` 正常祖先；作者新提交与已验证 `e0d6bda` 的全部 Rust/Cargo/CI/工具脚本及 web 源和 dist 完全相同，仅更新两项文档，不重复相同源码构建。完整 locked workspace/all-targets Rust/PostgreSQL 回归 289 项通过、0 失败、8 项既有 Linux/root/systemd 或外部运行时条件忽略；workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁、actionlint 与差异检查通过。Python discovery 83 项通过/5 跳过，NodeQuality 包装器 23 项通过/5 跳过；Bun 4 项/637 断言与 TypeScript/Vite 通过。最终 dist 实际 Chromium 桌面 1280×900/手机 390×844 共 9 组场景通过、页面错误 0，包含严格未知/有效0和false、IP失败隔离、旧报告与独立章节、指标过期与三类时间、独立导航；本轮未测试取消或实机完整诊断，最终 Linux CI 单独核对。
