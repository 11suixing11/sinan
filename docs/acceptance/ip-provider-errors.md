# IP 查询错误分类独立验收

对应 Issue #22。此项只增加逐条查询分类和元数据；按 IP / provider 保存历史成功结果由后续缓存 PR 验收。

## 可复现的检查

使用独立 PostgreSQL 测试数据库，不指向现有面板库。数据库测试地址通过 `DATABASE_URL` 提供；签名夹具信任根只使用仓库公开的 TEST_ONLY 公钥：

```sh
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
cargo fmt --all --check
cargo clippy --locked -p sinan-panel --all-targets -- -D warnings
cargo test --locked -p sinan-panel --lib ip_quality
cargo test --locked -p sinan-panel --test diagnostics --test foundation
cd web
bun install --frozen-lockfile
bun run build
```

自动夹具仅访问回环源和可注入的 resolver，不向 check-place 或其他第三方发起请求。文档保留 IP 只用作查询参数。

| 场景 | 断言与实现位置 |
|---|---|
| 403 / 429 / 503 | 真实 Axum HTTP 源分别记录 `http_403` / `http_429` / `http_other` 和具体 HTTP 状态；目标 IP、`check-place`、时间及耗时完整 |
| HTML / 未知 JSON 字段 | 分别记录 `non_json` / `schema_mismatch`；失败字段列表为空，界面显示未知 |
| DNS / 连接 | resolver 返回具有专用 source 类型的 DNS 错误，实际 reqwest 请求分类为 `dns`；关闭的回环端口分类为 `connect`；系统 resolver 的 localhost 结果保持回环地址 |
| TLS | 回环 TCP 源收到真实 ClientHello 后返回非法 TLS 记录，从 rustls source 类型分类为 `tls` |
| 超时 / 截断响应 / 超大响应 | 分别记录 `timeout` / `body_error` / `response_limit`；既有每请求限时和 64 KiB 上限保持有效 |
| 批次总超时 | 四个已开始请求保留实际尝试时间和耗时，三个尚未开始的请求为 `not_attempted`，时间与耗时为空 |
| 旧 payload / 零值 | 旧 JSON 可反序列化；已有数字 0、布尔 false 和旧错误保持原值，不补造分类或逐条时间 |
| 认证、重复刷新与持久化 | 原管理接口继续拒绝未认证和一分钟内重复刷新；保留地址在本地拒绝后，分类、目标及元数据可从 PostgreSQL 读回 |

上述传输与兼容夹具位于 `crates/panel/src/ip_quality/structured_error_tests.rs` 和既有 IP 查询测试；数据库路径位于 `crates/panel/tests/diagnostics.rs`。

## 页面检查

用构建后的实际 `web/dist` 和明确标记为模拟数据的 API 夹具检查桌面 1280 px、手机 390 px：逐条展开后显示中文分类、check-place 入口、目标 IP、时间和耗时（包括 0 毫秒）；HTTP 状态可见。旧记录显示“旧记录未分类”“旧记录未保存逐源查询时间”“耗时未知”，真实返回的 0 分和 false 显示为 0 和“否”。十四种分类及旧记录均无页面异常，手机布局无横向溢出；入口地址无效的分类映射另经 TypeScript 构建检查。

## 本次结果与边界

- 对齐 main `45df3b1` 后，IP 查询 11 项、诊断 PostgreSQL 3 项、foundation 6 项专项测试全部通过（共 20 项，无忽略）。冻结锁文件安装及 TypeScript/Vite 构建通过；重建实际 dist 的桌面/手机夹具通过，并检查手机截图，主分支新增 Agent 设置、持续拨测和命令界面保留。
- 初次全 workspace 测试已通过 IP 查询、诊断 PostgreSQL 与端到端组，随后 foundation 的六项测试在创建临时数据库时遇到 `No space left on device`。释放测试空间后，上述六项已补跑成功；不把该次运行计为完整通过，最新提交的全 workspace 与平台验证由 CI 执行。
- 最终整合 main `21e6a01`，保留诊断资源预算、有限流量补报、监控模式、任务页面及退役保护。使用独立 PostgreSQL 在此基线上重跑 IP 查询 11 项、诊断 3 项、foundation 6 项，20 项全部通过且无忽略；panel 全 targets Clippy（warnings 为错误）、workspace fmt 和差异空白检查通过。Bun 1.4.2 冻结锁文件安装及 TypeScript/Vite 构建通过，合并后的 dist 已同步重建；完整 workspace 和各平台 CI 仍须以最终提交的实际执行为准。
- Rust 新增直接依赖只引用已锁定 rustls 类型，详见 [ADR 0024](../adr/0024-ip-provider-error-classification.md)。没有调整 UA、重试、重定向、查询并发、时限或响应上限。
- 此项没有执行完整 NodeQuality、Agent 重启、面板断连、取消、持续代理流量或磁盘压力场景；这些属于相应诊断生命周期/资源保护 PR 的独立验收，不能用查询夹具代替。
- 真实外部源可用性仍以面板容器网络命名空间的基线为准。此项尚未保留上次成功结果；失败后历史结果继续存在的验收属于后续缓存 PR。
