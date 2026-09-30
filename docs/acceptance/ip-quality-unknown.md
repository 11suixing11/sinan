# IP 未知字段独立验收

对应 Issue #42，依赖成功缓存 PR #41，与查询源适配层 Issue #24 分开。

## 已复现缺口与契约

旧实现将任何已知路径的 string/number/bool 当作有效字段：空白字符串、proxy=0、score=true 都可成为成功；success=false 仍会采纳默认 0/false。此次为每个已知字段定义语义类型，拒绝空值、占位、错误类型和不能确认的响应状态。不推导总评分或“干净”；真实 0 与 JSON false 原值保留。IPAPI 原始带评级的数字字符串仍保留，无法确认的字符串不是评分。字段 kind 可选，旧 payload 兼容。

已知缓存字段读取时再次验证并补 kind，磁盘 payload/成功快照不修改。未知旧标签保留有效原始标量，不推断字段含义；旧缓存未保存完整源响应，无法追溯补证当时源成功标志。来源失败且已有可信成功字段时继续标为历史；没有可信字段时未知。过期或未知状态不计入当前成功。

## 自动和页面场景

```sh
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
cargo fmt --all --check
cargo test --locked -p sinan-panel --lib ip_quality
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cd web
bun install --frozen-lockfile
bun run build
```

DATABASE_URL 使用独立测试 PostgreSQL。字段契约测试覆盖七种响应的所有已知字段，测试缺失/null/对象/数组/空串/空白/未知占位与错误类型，并验证评分 0、布尔 false、原始数字字符串、坐标与 ASN 的合法范围。success=false/null/字符串、失败或 pending 状态、error 响应不会采纳默认字段；未注册响应形状不会产生数据。

真实回环 HTTP 与 PostgreSQL 测试先保存成功，再返回明确失败、JSON null、空对象、错误类型、403、429 和超时；本轮失败和历史快照分离，成功时间与有效期不变。新 IP 从未成功时仍未知；换连接池读历史结果。模拟旧非法缓存的字段不会返回为事实，原磁盘快照仍一致。

实际 dist 的 API 明确标为模拟数据：桌面 1280 px 与手机 390 px 验证失败后历史 0/false、无成功数据、部分成功、过期、双入口与旧 payload；额外验证 null/空白/对象、score=false/proxy=0 不能计为成功，合法 0/false 显示为 0/否，缺失和不可信布尔显示未知，未启用/尚未获得许可的模拟来源显示未知和原因。该夹具不接入或执行任何未经许可来源。无页面异常、手机无横向溢出，并检查手机截图。

## 本次结果与边界

- fmt、core 分层门禁、差异检查、Bun 1.4.2 冻结安装与 TypeScript/Vite、最终 dist 的桌面/手机上述夹具通过。
- 首轮 CI 在 Clippy 编译测试时发现路径夹具的反向迭代器要求不满足，已改用 rsplit；该轮 Rust 测试尚未执行，不记为通过。字段契约、实际 HTTP/数据库场景、Clippy 与完整回归仍待隔离 Debian 12 编译槽或修复后的 CI；不会用格式/浏览器验收代替。
- 不修改查询入口、UA、重试、诊断工具链或数据库 schema；来源许可和正式凭证/节点自查适配由对应独立项处理。未执行完整 NodeQuality、Agent 重启/面板断连、诊断取消或持续代理流量压力场景。
