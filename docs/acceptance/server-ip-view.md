# 服务器 IP 信息与 NodeQuality 视图拆分验收

对应 Issue #25，依赖未知字段 PR #44。此项只拆接口和视图，不新增来源、不改缓存 schema/诊断制品/任务执行。

## 接口与历史兼容

IP 独立 GET `/api/servers/{id}/ip-quality` 返回 ip_addresses/quality，POST `/ip-quality/refresh` 与原刷新使用同一逻辑。NodeQuality 独立 GET `/api/servers/{id}/node-quality/reports` 只返回 plugin_ready/plugin_reason/reports，与现有 POST 创建接口共路径。NodeQualityView 无 IP 查询字段，IP 读取不查询诊断准备，报告读取不访问 IP 缓存。

旧 GET `/node-quality` 由 LegacyNodeQualityView 保留全部原字段，旧 POST `/node-quality/refresh` 保留原刷新别名。没有数据库迁移、表名修改、历史报告/缓存清理或旧路由撤销。当前 r2 制品引用保持原样，后续章节制品与工具链分别处理。

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
