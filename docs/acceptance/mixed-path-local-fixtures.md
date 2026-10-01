# 混合链路本地隔离夹具记录

日期：2026-10-01。源码位于 `crates/compiler/tests/paths.rs` 与 `crates/panel/tests/mixed_path_runtime.rs`。本记录证明固定官方运行时上的局部网络行为，不替代 Agent 整包发布、候选探测、恢复屏障、生产迁移和正式签收。

## 编译器及原生校验

`cargo test -p sinan-compiler --test paths`：7 项通过，1 项原生检查测试默认忽略。覆盖一至八个有序代理跳、tag/detour 顺序、HTTP 中间段承载 Reality 的 TCP/UDP、HTTP 不能承载下游 QUIC、SS 原生 UDP 与 UoT 区别、TCP 出口的显式 UDP 拒绝、重复资源/端点及入口回环拒绝、候选入口不开放、内部身份不进入用户订阅、仅入口记录终端用户统计、输入顺序确定性及旧两跳字节不变。

将 `SINAN_TEST_UPSTREAM` 指向官方 v1.14.2，并显式运行 `official_runtime_checks_imported_protocols_and_nested_paths -- --ignored`：1 项通过，其中逐一检查 12 个配置。包括三/四段混合结构、八个代理跳、SS、VMess、VLESS、Trojan、Hysteria2、TUIC、AnyTLS、SOCKS 和 HTTP。原生 `check` 只证明配置结构被此构建接受；图和承载关系仍由本仓库编译器校验。

## 实际代理路径

显式运行 `cargo test -p sinan-panel --test mixed_path_runtime external_middle_hop -- --ignored --nocapture`：1 项通过，运行 16.95 秒。运行时为未修改的官方 v1.14.2；测试仅使用随机回环监听和自行生成的一日证书，不访问真实机场或公网目标。

| 场景 | 证据 |
| --- | --- |
| A → HTTP 订阅 X → 受管 B | 经过 A 的真实 Reality 用户入口；4096 字节 TCP 与 1024 字节 UDP 回显均一致。 |
| A → 受管 M → HTTP 订阅 X → 受管 B | 同样的 TCP/UDP 回显通过；A、M、X、B 对应的转接夹具均观察到超过 4096 字节。 |
| X 必须位于中间 | X 验证 HTTP 代理认证，并且仅允许 CONNECT 到明确的 B 监听；无法作为任意开放出口。 |
| 最终出口 | B 的夹具 direct 出站绑定独立回环地址 `127.0.0.4`，TCP/UDP 目标都确认来源为此地址。 |
| 禁止绕过故障段 | 停止 X 并关闭其全部已建立连接后，两种拓扑的新 TCP、UDP 请求都失败，没有改为直连或跳过 X。 |

为隔离环境，测试对编译输出仅作这些夹具调整：监听限制到回环；Reality 的伪装握手目标改为回环 OpenSSL TLS 服务；各进程控制接口改为独立随机回环端口；最终 direct 使用独立回环来源地址。链路的 outbounds、detour、内部身份、用户入口认证和转发路由仍来自实际编译函数。测试通过 `public_port` 将各受管端点连接经过字节计数转接夹具，不修改上游运行时源码。

官方归档缺少可用的 V2Ray 计量扩展，因此夹具删除 `experimental.v2ray_api` 后启动。真实账本扣量、用户套餐、持续用量及多代发布/恢复不属于这项通过结果。入口单次统计仅有编译配置断言，不能据此签收真实计量。订阅真实供应商认证、平台差异、断网恢复、Agent 应用确认及生产路径仍需相应独立证据。

## 原生 Clash 路径探测

另行显式运行 `cargo test -p sinan-panel --test mixed_path_runtime clash_delay -- --ignored --nocapture`：1 项通过，运行 0.45 秒。该项复用三段路径夹具，不重复前述 TCP/UDP 负载。入口使用编译器生成的 `127.0.0.1:18086` Clash 控制接口、认证秘密及 `PathCheck.outbound`，并从真实 `/version` 确认运行时为 `sing-box 1.14.2`。

| 场景 | 结果 |
| --- | --- |
| 不带认证访问 `/proxies/{tag}/delay` | 原生接口返回 401，外部段没有打开连接。 |
| 携带编译秘密与 `url=https://127.0.0.1:…/ready&timeout=10000` | 返回 200 与正数 `delay`；X 和 B 的转接夹具均观察到数据，HTTPS 目标记录 `127.0.0.4 HEAD /ready`，证明通过指定最终出口。 |
| 停止外部中间段 X | 原生探测返回 503/504 失败；随后独立可信 HTTPS 请求仍得到目标的 204，排除目标本身停止导致的假阳性。 |

目标使用 Python 标准库的回环 HTTPS HEAD 服务及一日自签终端证书。普通 OpenSSL `s_server -www` 只适合本测试的 Reality 握手夹具，不提供此处需要的可靠 HEAD 响应。测试仅给入口子进程设置 `SSL_CERT_FILE` 和独立空 `SSL_CERT_DIR`，不修改宿主信任库，不禁用证书校验；独立目标存活检查也显式信任同一证书。

这项选择参考固定版的 [Clash delay 实现](https://github.com/SagerNet/sing-box/blob/v1.14.2/experimental/clashapi/proxies.go)、[URLTest 实现](https://github.com/SagerNet/sing-box/blob/v1.14.2/common/urltest/urltest.go)及 [Go 的 Unix 证书加载](https://github.com/golang/go/blob/go1.26.1/src/crypto/x509/root_unix.go)。Clash handler 从 `context.Background()` 创建探测上下文；为避免将配置证书 store 继承视为已证明事实，夹具明确限定子进程信任。URLTest 发出 HTTPS HEAD，但不检查 HTTP 状态码，因此通过只证明所选出站到此目标的 TLS/HTTP 往返，不能声称目标应用健康，更不能据此签收全 Agent 分布式候选、切换、恢复或真实计量。

GitHub Actions 继续暂停，未执行远端 CI。未签署、发布、部署或迁移生产数据。
