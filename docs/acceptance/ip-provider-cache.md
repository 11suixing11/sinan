# IP 查询成功缓存独立验收

对应 Issue #23，基于 main `e2d898c`，包含已合入的 [错误分类 PR #34](https://github.com/theLucius7/sinan/pull/34)。此项验收失败不覆盖已保存成功结果、迁移和刷新恢复；来源授权与新增适配器由其他 PR 处理。

## 自动场景

在独立 PostgreSQL 数据库和回环源上运行，不使用生产面板数据库，不向外部 IP 查询入口发出请求：

```sh
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -p sinan-panel --lib ip_quality
cargo test --locked -p sinan-panel --test diagnostics
cargo test --locked
cd web
bun install --frozen-lockfile
bun run build
```

`DATABASE_URL` 指向专用测试库。五项缓存 PostgreSQL 测试位于 `crates/panel/src/ip_quality/cache/tests.rs`，HTTP 并发刷新测试位于 `crates/panel/tests/diagnostics.rs`：

| 场景 | 断言 |
|---|---|
| 成功后 403 / 429 / 超时 | 真实 HTTP 源先返回七种有效形状，再逐类失败；本轮状态与错误更新，成功字段、时间和有效期完全不变，0/false 保持原值 |
| 重启后读取 | 每次失败后用新的 PostgreSQL 连接池读取，仍能同时读回最新失败和历史成功 |
| 部分成功 | IPAPI 成功更新独立快照，其余六个数据库保留历史数据，历史成功不能充当当前成功 |
| 双 IP / 多入口 / 换 IP | IPv4、IPv6 与模拟的第二入口独立保存；新 IP 写入不会删除旧 IP，较早结果不能倒写覆盖较新批次 |
| 旧表迁移 | 实际执行旧 DDL、插入旧 payload，再执行 0008；旧 payload 原值、0/false 和错误保留，无成功的数据库不补造成功时间或字段，缺失逐条时间与类别保持未知 |
| 并发与退出恢复 | 两次 admission 只允许一次；退出后运行租约持久保留，过期后允许恢复，旧请求不能清理后续租约；最近失败仍触发一分钟刷新间隔 |
| HTTP 重复刷新 | 管理员同时 POST，两次请求分别返回 200 / 409，无成功数据仍显示未知 |
| 服务器删除 | 已删除服务器的后续结果不能替换保存的缓存 |

入口与明细快照在同一事务持久化，读取用 repeatable-read 事务组装。provider 是真实入口，database 是响应形状；模拟第二入口仅验证缓存维度，不是新查询适配器。旧 IP 记录保留在数据库，当前页面仍按设备当前 IP 选择。

## 页面场景

构建后的实际 dist 配合明确标为模拟数据的 API：点击刷新从成功变成 403 后，页面同时显示“当前失败类别”和“正在显示历史结果”、上次成功时间、原 0 分和“否”。没有成功数据的 IPv6/429 显示未知。部分成功显示 1 个当前成功和 6 个历史结果；过期成功显示“数据已过期”，不计入当前成功。双入口不合并；旧格式记录不补造时间。桌面 1280 px、手机 390 px 均无脚本异常，手机无横向溢出。

## 本次结果与边界

- 实际 PostgreSQL 旧 DDL + 旧 payload + 0008 迁移通过，检查成功字段、旧错误、逐条未知时间和组合键。
- Bun 1.4.2 冻结锁文件安装、TypeScript/Vite 构建及上述真实 dist 桌面/手机夹具通过，并检查历史结果手机截图。
- 最终 main `e2d898c` 源在独立 Debian 12 构建容器通过 workspace fmt、全 targets Clippy（warnings 为错误）、IP 专项 16 项（含五项缓存 PostgreSQL 场景）、诊断 HTTP 专项四项、完整 workspace 245 项成功 / 0 失败 / 六项已有条件忽略。六项忽略分别需要真实 systemd/cgroup 或指定版本上游运行时，不计为实机通过；本次新增缓存和并发测试没有忽略。
- 构建容器限制 1536 MiB 内存且同额内存加 swap、两 CPU、512 PID，源码只读挂载，使用独立测试数据库和公开 TEST_ONLY 信任根；容器正常退出 0，未改动生产 Agent 或运行时。日志为隔离构建目录下的 `target/ip-cache-{fmt,tests,clippy,full-test}.log`。core 分层门禁及六项门禁行为测试、文档链接、差异空白检查通过；本提交 CI 另行报告。
- 此项未执行完整 NodeQuality、Agent 重启、诊断取消或持续代理流量压力场景；这些由相应诊断 PR 独立验收。当前查询入口仍为 check-place，外部源可用性以面板网络命名空间的基线为准。
