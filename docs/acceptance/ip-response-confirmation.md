# IP 查询显式错误响应确认独立验收

对应 [Issue #42](https://github.com/theLucius7/sinan/issues/42) 的窄项补修，基线为 `3f65b42cc217217c9b75bae519d6467902dfbf70`。本项独立记录源码修复与本地验收；不关闭 Issue #42 的其他验收范围，也不替代 [整改顺序与阶段门禁](ordered-remediation.md)。

## 缺口与修复契约

HTTP 200 响应中的根 `errors` 非空，或 AbuseIPDB 的实际 `data` 响应容器明确 `success=false`，此前仍可能将默认 `abuseConfidenceScore=0` 解析为当前成功。本项在取字段前确认根响应，以及 `abuseipdb` / `abuseipdb-v2` 的 `data` 容器：非空 `errors`、畸形错误标志与既有不能确认的 success/status/error 返回空字段，查询层沿用 `SchemaMismatch`（字段不匹配），不创造零分成功。

缺失、null、空数组或空对象的 `errors` 兼容；真实评分 0、布尔 false 以及没有状态标志的旧成功格式仍保留。仅按已知响应容器校验，不递归扫描 ASN、company 等资料。正式 AbuseIPDB 继续验证目标 IP、公网标志和 IP 版本、整数 0–100 评分及已知字段白名单；共享确认规则取代其重复的 errors 检查，不增加来源、凭据、重试或查询请求。

## 本地正向与负对照

测试环境为 macOS arm64、Rust 1.97.1、专属 PostgreSQL 16.15 临时实例与随机端口回环 HTTP 服务。测试数据库和 Cargo target 位于同步目录之外，未连接生产数据库或第三方 API；签名编译根来自 `scripts/ci-test-trust.py` 的公开 TEST_ONLY 夹具。临时 PostgreSQL 已正常停止，既有并行任务的数据库未触碰。

| 检查 | 实际结果 |
| --- | --- |
| `cargo fmt --all --check` | 通过 |
| `cargo test --locked -p sinan-panel --lib ip_quality -- --test-threads=1` | 30 通过、0 失败、0 忽略 |
| `cargo clippy --locked -p sinan-panel --all-targets -- -D warnings` | 通过 |
| `git diff --check` | 通过 |
| 原解析器字段负对照 | 4 通过、2 预期失败；根 errors 与 data.success=false 均错误返回 0 分当前字段，真实 0 控制通过 |
| 原解析器真实回环/数据库负对照 | 新历史回归实际运行 1 项、预期失败；错误响应被记为 succeeded，断言要求 partial |
| 恢复修复后的同一历史回归 | 1 通过、0 失败；属于上述 30 项之一，不能累加为独立场景数 |

字段负对照使用原 `356350e17d9106dffdc0c17a4c0e6d068081d170` 的真实 fields 源文件；该文件与本项基线 fields 字节相同。数据库负对照仅将此 fields 文件恢复为基线字节，其他真实查询/缓存代码及新增回归保持，失败后立即恢复修复字节。私有日志保存正向、两个字段负例和数据库负例的具体断言及退出状态。

新增真实回环与 PostgreSQL 回归先存 AbuseIPDB 73 分，再依次返回根 errors、data.success=false、data.errors；每次查询为 partial，该数据库字段不匹配且没有本轮字段。换连接池读取后，73 分仍可读并标为历史，成功时间及有效期未改，其余六库仍是当前成功。没有历史的新 IP 保持未知、无成功时间。既有缓存回归同时覆盖根错误让整批失败、真实 0/false、403/429/超时与未知字段；本轮全部执行通过。

正式接口新增回归验证空 errors 兼容、根/容器的非空或畸形 errors 拒绝及不能确认的状态标志拒绝；既有目标 IP、公网标志、版本、评分类型范围和请求契约回归同批执行。ASN/company 中资料级错误标志不会使有效字段丢失。

## 被验证的文件身份

以下 SHA256 为上述正向验收和恢复后实际源码，便于核对最终提交没有改变受验字节。

| 文件 | SHA256 |
| --- | --- |
| `crates/panel/src/ip_quality/fields.rs` | `edab4a816e15c3718ec2efbeeb96889c12ad613d535a3c6b240df87d4dd03f35` |
| `crates/panel/src/ip_quality/fields/tests.rs` | `1656109d663866506f6145e8e9ffcefdefef17dca94d424506626e7111d7bec2` |
| `crates/panel/src/ip_quality/providers.rs` | `f987ce325b16c9145c11e4eaace99f8601bc1614c0172ea9563989151af539ee` |
| `crates/panel/src/ip_quality/providers/tests.rs` | `39337f6c559654379439ab3e506aa1a438d8e4000a6acc208ec2cd70de14294d` |
| `crates/panel/src/ip_quality/cache/tests.rs` | `d9213fbe8c02f809cd37d1999cc555386561b4031e59d4dac6334ca0149ff5e7` |

## 未执行范围

- 四个 GitHub Actions 工作流只读确认均为 `disabled_manually`；按仓库临时 CI 暂停规则，未触发、重跑或恢复任何远端 CI，不宣称最终 main 或平台全绿。
- 本项未修改前端、dist 或数据库 schema，未重复桌面/手机浏览器夹具和无关完整 workspace 回归。
- 正式账户权限、额度、公网 DNS/TLS、节点自查和流媒体查询仍未验；没有使用真实第三方凭据或执行生产压测。
- 专用节点完整负载、持续代理业务、NodeQuality 完整执行与后续 TCP 能力验收仍依原阶段门禁待验；本项没有签收、发布、部署或合并能力。
