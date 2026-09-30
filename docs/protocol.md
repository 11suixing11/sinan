# 面板与 Agent 协议 v1

## 传输与兼容

生产环境必须通过 HTTPS/WSS 暴露面板；本地测试可使用回环 HTTP。Agent 原生传输只访问已配置面板的同源地址，拒绝外站制品、重定向及路径穿越。按需执行的 NodeQuality 外插需要访问上游测试服务，见 ADR 0016。WebSocket 入口为 `GET /api/agent/v1/ws`。全部业务消息为 UTF-8 JSON 文本；单条消息应小于 1 MiB。

Agent 与面板的产品版本独立；面板当前声明支持协议范围 `1..=1`，按协议版本和能力判断兼容，不要求产品版本相等。hello 和静态遥测报告 Agent 二进制自己的版本。

信封：`{"v":1,"type":"heartbeat","id":"UUID","ts":1790000000,"payload":{}}`。

`v` 为协议主版本，`id` 为消息 UUID，`ts` 为 UTC Unix 秒，`payload` 为对应类型对象。字段只能增加，接收方忽略未知字段；未知 `type` 记录后忽略，不断开已认证连接。无法解析的已知消息拒绝处理。消息 ID 不承担流量去重；流量使用自己的 epoch 和 seq。

## 注册与身份

管理员为一个服务器产生一次性随机 token，24 小时过期。数据库只保存 token 摘要；服务器标识使用正整数。`POST /api/agent/v1/enroll` 请求 `{token,device_public_key,static_info}`，成功返回 `{server_id}`。设备公钥为 Ed25519 的 32 字节无填充 Base64 URL-safe 文本。

令牌校验、绑定设备与消费必须在同一数据库事务中完成，重复或过期 token 不可注册。Agent 私钥只保存在本地权限 0600 的身份目录中，永不上传。

## WebSocket 认证时序

1. 面板生成随机、连接专用、一次性的 nonce，发送 `auth.challenge`，内容 `{nonce,server_time}`。nonce 是无填充 Base64 URL-safe 文本。
2. Agent 用 Ed25519 签名 **nonce 字符串的 UTF-8 字节**，发送 `auth.response`：`{server_id,signature}`，signature 同样使用无填充 Base64 URL-safe。
3. 面板验证设备公钥和当前挑战，通过后发送 `hello.ack`：`{server_time,session_token,session_expires_at}`。HTTP Bearer token 有效 3600 秒，重新连接重新签发，绑定服务器身份。认证前不可获取清单或发送计量。
4. Agent 发送 `hello`：`{agent_version,protocol_version,capabilities:[],applied:{"module":rev}}`，再发送静态遥测。

一次挑战不能用于另一条连接。认证阶段有超时；超过 60 秒未收到任何消息判定离线。Agent 每 20 秒心跳，断线指数退避（上限 60 秒，另加 0–30% 抖动）重新认证。在会话过期前主动重连，避免 HTTP 凭证过期导致对账持续失败。

## 消息目录

| 方向 | type | payload |
|---|---|---|
| 面板 → Agent | `auth.challenge` | `{nonce,server_time}` |
| Agent → 面板 | `auth.response` | `{server_id,signature}` |
| 面板 → Agent | `hello.ack` | `{server_time,session_token,session_expires_at}` |
| Agent → 面板 | `hello` | `{agent_version,protocol_version,capabilities,applied}` |
| Agent → 面板 | `heartbeat` | `{applied,uptime_secs}` |
| Agent → 面板 | `telemetry.static` | 系统、内核、架构、Agent 编译 ABI `libc`、Linux 宿主运行时 ABI `runtime_libc`、CPU 型号与核数、内存与磁盘总量、虚拟化、主机名、Agent 与模块版本、`ip_addresses`（IPv4/IPv6 字符串数组） |
| Agent → 面板 | `telemetry.metrics` | CPU 百分比、内存使用、load 1/5/15、磁盘使用、网卡累计和速率、TCP/UDP 连接数、运行时间 |
| 面板 → Agent | `manifest.changed` | `{rev}`，提示重新读取全量清单 |
| Agent → 面板 | `apply.result` | `{module,rev,op_id,status,healthy,error?}`，status 为 `applied` 或 `failed` |
| Agent → 面板 | `usage.batch` | `{epoch,seq,period_start,period_end,records:[{stat_name,uplink,downlink}]}` |
| 面板 → Agent | `usage.ack` | `{epoch,seq}` |
| 面板 → Agent | `retirement.request` | `{request_id}`，持久退役请求 UUID |
| Agent → 面板 | `retirement.result` | `{request_id,success,error?,receipt?}`，成功须携带匹配的签名回执 |

指标每 10 秒发送，采集失败字段省略，不用 0 代表未知。流量每 30 秒采集；上下载单位是字节，负数无效。epoch 为 UUID；seq 在本地持久递增。所有时间戳使用 UTC Unix 秒。

Linux 静态信息区分两种 ABI：`libc` 保留 Agent 自身的编译 ABI，`runtime_libc` 是独立探测的宿主运行时 ABI。静态 musl Agent 在 glibc 主机上报告 `libc:"musl",runtime_libc:"gnu"`。Agent 自动更新只依据 `os`、`arch` 与 `libc`，运行时清单结合宿主 ABI 选择兼容候选；两个字段互不覆盖。

`runtime_libc` 为 Linux 可选新增字段，识别成功取 `gnu` 或 `musl`；新 Agent 无法可靠识别宿主时兼容沿用自身编译 ABI。面板接受 `glibc` 作为 `gnu` 别名，非 Linux 设备不发送此字段。旧设备缺少字段时保留原 `libc` 选择路径；显式 null 或非字符串在消息解析时拒绝，显式 `unknown`、空字符串或未支持值的运行时清单返回 400，不将这些值当作字段缺失。

GNU 宿主上的 musl Agent 按 `linux-musl-{arch}`、旧 `{arch}`、`linux-gnu-{arch}` 依次选择运行时，保留旧版本在同一签名证明中选择 musl 或 legacy 的优先级；GNU Agent 则从 GNU 完整标识开始，再兼容旧目录。只有制品不存在时才尝试下一候选，校验失败不能降级。真正 musl 宿主不使用 GNU 完整标识或 GNU 兼容目录。Agent 自身升级继续只用编译 ABI，不随 `runtime_libc` 改变。

## HTTP 期望状态

下列接口使用 `Authorization: Bearer <session_token>`，凭证只能访问绑定服务器的资源。

- `GET /api/agent/v1/manifest` → `{rev,modules:{module:{kernel_version,artifact:{url,sha256,proof},config_rev,bundle_url,bundle_sha256,stats_listen}}}`。
- 配置包 URL → `{files:{"config.json":"配置文件文本"}}`。sha256 是 HTTP 响应原始 UTF-8 字节的 SHA-256 小写十六进制，不是重新序列化的摘要。
- `GET /api/agent/v1/artifacts/{name}/{version}/{arch}` → 制品原始字节。路径段限定安全字符；`arch` 是完整平台标识（例如 `linux-gnu-amd64`、`linux-musl-arm64`），或旧发布的 `amd64` / `arm64` 兼容键。所有下载均校验已签清单中的 SHA-256。

`proof` 为 `{metadata_json,checksums,signature}`；`signature` 保留完整四行 `SHA256SUMS.minisig`，正文是 `checksums` 原始 UTF-8 字节。已签清单绑定 metadata 原始摘要、制品路径与压缩包摘要，metadata 进一步绑定仓库、发布 tag、协议范围、版本、架构、格式、安装后二进制摘要和大小。新 Agent 在应用、缓存命中、恢复、回滚及诊断执行前都以构建时固定的多个公钥验证证明与实际内容，不能把本地 marker 中的未签摘要当作可信值。格式与信任根轮换见 [ADR 0017](adr/0017-signed-release-artifacts.md)。

字段缺省时旧消息仍可解析，但新 Agent 拒绝执行无 proof 的制品。签名能力为 `artifact:minisign-v1`；面板拒绝向未声明该能力的旧设备提供新 manifest、制品或新诊断任务，继续接受旧设备的状态、流量和确认，保留已运行配置。缺根或缺能力都不降级到仅 SHA256 校验。

清单 rev 单调增加，模块 config_rev 表示配置包版本。无部署时清单可以是 rev 0、空 modules。Agent 每 60 秒拉取全量清单，并响应变更通知；心跳版本不一致时面板补发通知。多个通知可合并，以最终读取的全量状态为准。

## 应用与计量语义

`apply.result` 中 op_id 对应本地意图。只有校验、原子切换、服务动作、健康检查都成功后才能报告 applied；失败应回滚并提供错误。面板只接受已经为该服务器发布的版本，不接受未来版本，旧回报不能覆盖较新已应用状态。

累计计数读取必须 reset=false。Agent 在同一 SQLite 事务内写基线和待发送差值。确认前持续重发；面板事务去重键为 `(server_id,epoch,seq,stat_name)`，持久化成功后回复 ack，重复批次仍回复 ack。Agent 收到相同 ack 多次是幂等操作。

运行时重载前采集终值，再换 epoch。无法读取终值时应阻止主动破坏旧计数；外部重启导致计数下降则开启新 epoch，并记录可能丢失窗口。Agent 自身重启不换 epoch，保留已写入本地数据库的基线和未确认批次。

统计用户名格式为 `u{user_id}_n{node_id}`，用于唯一定位用户与节点，不应从入站标签猜测归属。

## 一次性诊断外插

新增内容保持协议主版本 1。旧 Agent 不发送 `ip_addresses` 时按空数组处理，不声明诊断能力时面板不下发新任务。新 Agent 在 hello 的 capabilities 中声明 `diagnostic:nodequality`，仍独立声明已有运行时模块。

设备接口继续使用绑定服务器身份的 Bearer session：

- `GET /api/agent/v1/diagnostics` 返回该设备尚未终止的任务数组，每个为 `{id,plugin,version,artifact:{url,sha256,proof},timeout_secs,expires_at?,options}`。
- `POST /api/agent/v1/diagnostics/{id}` 提交 `{id,status,report?,error?}`。设备可提交的 status 是 `running`、`succeeded`、`failed`，最终报告为 `{text,report_url?}`。数据库持久化后返回 204；其他设备不能更新该任务，过期会话不能取回任务。

NodeQuality 的 plugin 标识为 `nodequality`，version 为固定上游提交加包装器版本（当前为 `a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2`），制品同源、校验后安装。options 仅允许 `ip_version=both|ipv4|ipv6`、`network_mode=low|normal` 和 `upload_report=true|false`。`upload_report` 在管理员创建任务的 HTTP 请求中为布尔值，缺省 `false`；在公共任务中为固定字符串，缺少时新 Agent 按关闭处理。旧 Agent 拒绝新版本和未知选项，不通过忽略隐私选项继续运行旧包。升级必须准备 r2 包；已排队或运行的旧任务不受新缺省值影响，应先结束旧任务再升级。任务不携带任意命令、程序地址或自由 shell 参数。

任务 ID 同时用于设备持久 checkpoint、独立服务及面板去重。先记录启动意图再创建 systemd 服务；Agent 重启检查已有服务并继续观察，不自动重复运行。启动边界状态不明或服务消失时回报失败，管理员可另发新任务。结果确认前保存并重传；终态不能被晚到的 running 覆盖。每台设备最多一个活跃任务。代理配置版本与用户流量周期不会因诊断任务变化。

IP 地址来自网卡和可选的 Agent `public_ips` 配置。面板仅向固定的 IPQuality 查询域名请求公网地址，私网、回环、链路本地等地址可展示但不参与外部查询。每个数据库分别保存结果或错误；未知风险不能填成零风险。

## 在线退役

设备在 hello 声明 `server:retire-v1`。面板删除在线服务器时先保存并发送请求，收到成功回执后才完成软删除；没有能力的在线旧设备返回升级提示。离线软删除不证明设备清理成功。面板认证注册、删除与回执提交共同串行，在签发会话前再次检查服务器和公钥。

Agent 收到请求后持久阻止新的受管操作，停止运行时与诊断，提交所有已持久用量，再清除本机身份、会话与受管运行配置；保留历史账本。相同请求可重试，不同请求不能覆盖未结束的退役。清理完成后进入终态，不再自动注册或恢复代理。

回执为 `{server_id,request_id,signature}`；签名对象按顺序拼接 UTF-8 `sinan-retirement-v1`、一个零字节、8 字节大端有符号 server ID、16 字节请求 UUID。使用原设备 Ed25519 密钥，签名为 URL-safe base64，无填充。Agent 在删除私钥前持久保存回执，但只在清理完成后发送。面板使用保留的公钥和已存在请求验签。

完成后的 Agent 也可向原绑定面板的 `POST /api/agent/v1/retirement/receipt` 发送该回执。此接口不需要 Bearer session；签名本身只授权对应退役确认，不能恢复设备会话。重复合法回执返回 204，错误回执拒绝。确认响应丢失时仍可恢复；未清理完成就被离线软删除的设备不具备此保证，需人工处理。详细崩溃与离线边界见 [ADR 0019](adr/0019-server-retirement.md)。
