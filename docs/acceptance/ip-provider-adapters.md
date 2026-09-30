# IP 查询入口适配独立验收

对应 Issue #24，依赖成功缓存和 IP 独立视图。该 PR 接入一个正式凭据 API，并明确旧聚合入口归属；IPQuality 节点自查仍未启用，不能据此宣布所有来源或流媒体检测已完成。

## 官方契约与配置

依据 [AbuseIPDB API v2 CHECK 官方文档](https://docs.abuseipdb.com/#check-endpoint)。生产地址固定 HTTPS GET `/api/v2/check`，只传 URL 编码 `ipAddress` 和固定 `maxAgeInDays=30`，通过 sensitive 的 `Key` 请求头传运营者的私有凭据，Accept 为 JSON。没有 verbose、上传/报告/写入、UA 或重试，拒绝重定向。`SINAN_ABUSEIPDB_API_KEY` 缺失/空白/无效时不创建请求或尝试时间，并明确未启用原因。

目标 IP、isPublic=true 和 IP 版本均须确认。只展示可信用途、国家代码、ISP、Tor 和 0–100 JSON 整数的原始置信度；真实 0/false 保留，空值/错误类型/明确失败不补成默认事实。白名单不用于“干净”判断，流媒体解锁始终未知。

Provider 维度是 check-place 聚合入口、abuseipdb-api 正式接口与尚未启用的 ipquality-node；七种 database 仅是第一个入口的响应视图。缓存沿用 0008，无迁移或表名变化。禁用入口后，历史字段/成功时间/有效期保留并标明历史及原因；本地禁用不伪造网络失败。

## 可重复的 HTTP/PostgreSQL 场景

```sh
unset SINAN_ABUSEIPDB_API_KEY  # Tests use synthetic fixture credentials only.
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
cargo fmt --all --check
cargo test --locked -p sinan-panel --lib ip_quality
cargo test --locked -p sinan-panel --test diagnostics
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cd web
bun install --frozen-lockfile
bun run build
```

DATABASE_URL 使用独立测试库。`providers/tests.rs` 与回环 fixture 使用明确公开的 TEST_ONLY API key，不访问第三方或使用真实凭据。

| 场景 | 验证 |
|---|---|
| 空/非法凭据 | 入口未启用、有中文原因，正式接口零请求，Key 不进入描述或结果 JSON |
| IPv4/IPv6 成功 | 固定只读路径/方法、仅两个查询参数、IPv6 URL 编码，Key 仅发送给正式入口，响应视图不冒充独立来源 |
| 403、429、超时、HTML、超限、重定向 | 相应类别，单次请求数不增加，不访问重定向地址 |
| 缺失/null/错误类型、错误 IP/版本、公网状态、明确失败 | 信息未知；真实 0/false 保留，置信度超出 0–100 或非整数不采纳 |
| 多入口预算 | 共享最多四个在途请求和一个批次截止；未开始的正式查询无逐条时间/耗时，不累加入口超时 |
| 两个 IP，成功后正式接口失败 | check-place 可成功，正式失败仍保留自己的成功字段和时间，两入口互不覆盖 |
| 聚合入口失败但正式接口成功 | 各自更新自己的状态和历史，IP/provider/database 键数量及字段正确 |
| 关闭凭据、换连接池、只刷新其中一个 IP | 正式快照保留并标历史，旧 IP 缓存还在，没有新正式请求或编造时间 |
| 同时重复刷新 | 同一个服务器的持久锁/租约仍只允许一个请求，无新增迁移破坏去重 |

既有 DNS/TLS typed source 夹具、旧 payload/旧 DDL 迁移、重启读缓存、字段 unknown 与 IP/报告 API 隔离继续运行。测试面板显式使用无私有凭据的注册表，避免测试环境变量意外启用外部接口。

## 实际前端

Bun 1.4.2 冻结安装、TypeScript/Vite 构建 dist（含五项字段回归、711 断言），再用真实 Chromium 的桌面 1280 px/手机 390 px 页面与明确模拟 API 验证：一个聚合入口七种响应视图、官方凭据未配置原因、节点自查未启用/流媒体未知；正式 0/false、403历史、429未知、超时历史、关凭据后历史、非法百分比未知和 100/true 原值。不存在将七个响应计为七个来源的文案，不显示 Key，不发 NodeQuality 请求；无页面异常或手机横向溢出，检查截图。模拟 API 不是正式账户授权/额度或公网可达性证据。

## 结果与未覆盖项

- 本机 fmt、core 门禁及六项行为测试、差异检查、Bun 1.4.2 冻结安装/TypeScript/Vite、实际 dist 桌面/手机上述场景通过；五项字段回归/711 断言覆盖所有 60 个后端已知字段与正式百分比边界。
- Rust/Clippy 与真实回环 HTTP/PostgreSQL 场景由独立 CI 或隔离 Debian 12 编译槽验证，结果另行补充；本机没有因剩余磁盘不足从头编译。
- 未调用真实 AbuseIPDB 账户。实际权限、额度、DNS/TLS 和面板网络可达性在运营者配置密钥后另验；没有凭据仍可交付明确禁用行为。
- 没有执行或打包原版 IPQuality。固定 AGPL-3.0 源码的 UA、在线 main 引用、统计/上传、并发与宿主依赖修改须由后续独立 PR 实现，并做静态及网络请求验收，详见 [ADR 0027](../adr/0027-ip-provider-adapters.md)。
- 未跑完整 NodeQuality、Agent 重启/面板断连、取消或持续代理压力场景，不将本项通过等同整阶段通过。

- 合并审查最终验证：正常合入 main `6b63f71`（含确认式取消），保留作者 `cfea748` 祖先及双方全部记录；最终代码审查未发现需要改动的生产缺陷。全程移除真实 `SINAN_ABUSEIPDB_API_KEY`，只使用明确公开的合成 key 与回环 HTTP，未读取或调用真实账户。独立 PostgreSQL 下 IP/provider library 25 项、diagnostics API 7 项及章节/迁移 3 项，共 35 项通过、0 失败/忽略；覆盖敏感 Header、固定路径/参数/无 UA/重试/重定向、身份/类型、403/429/超时、禁用零请求、0/false 与双来源历史及新连接池/租约。Panel 全 targets Clippy（warnings 为错误）、workspace fmt、core 门禁及其 6 项行为回归、差异检查通过。Bun 1.4.2 冻结安装、5 项字段测试/711 断言及 TypeScript/Vite 构建通过，重建 JS `index-C1v7YTay.js`；最终 dist 在实际 Chromium 桌面/390px 手机的入口/缺凭据/0false/错误历史/禁用/百分比边界及确认取消组合场景全部通过、页面错误 0。本轮没有重复完整 workspace 或调用正式公网账户，不宣称配额/权限/节点自查或完整 NodeQuality 压力已验。

- 继续正常合入 main `229becc`（PR #56）：与已验证 `c2b01cc` 相比仅修改 `tools/test-nodequality.py`，所有 Rust/Cargo、生产工具、web 源与 dist 完全相同，未重复无交集 Cargo。包装器 Python 28 项中 23 项通过、5 项既有 Linux/root 条件跳过；core 门禁与差异检查再次通过。真实 Linux 正常退出的生产工具补修仍由独立任务验收，本项不将本机跳过计为通过。
