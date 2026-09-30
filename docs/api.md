# 面板 HTTP API

本页记录面板管理接口，供中文前端和集成测试使用。所有路径都相对于 `SINAN_PUBLIC_URL`。管理接口使用同源 Cookie；请求 JSON 时发送 `Content-Type: application/json`。应用错误返回 `{"error":"中文说明"}`，常见状态码为 400（输入无效）、401（未登录）、404（资源不存在）、409（冲突）、429（请求过多）、500（内部错误）。框架对无法解析的 JSON 或路径参数也可能返回文本错误。

## 登录

| 方法与路径 | 请求 | 成功响应 |
|---|---|---|
| `POST /api/login` | `{"password":"管理员密码","totp_code":"六位验证码"}`；未启用 TOTP 时可省略 `totp_code` | `200 {"id":1}`，设置会话 Cookie |
| `GET /api/me` | 无 | `200 {"id":1}` |
| `POST /api/logout` | 无 | `200 {"ok":true}`，清除会话 |
| `GET /healthz` | 无 | `200 ok`，无需登录 |

会话 Cookie 名为 `sinan_session`，带 `HttpOnly; SameSite=Strict; Path=/`，有效期 24 小时；公开地址使用 HTTPS 时同时带 `Secure`。管理员密码只在首次初始化时设定，后续启动不会用环境变量覆盖已有密码。

已启用 TOTP 时，密码和验证码必须在同一次请求中验证成功才会创建会话；缺失、错误、过期或重放的验证码统一返回 401。验证码为 6 位 ASCII 数字，30 秒更新一次，允许前后各一个时间步；已成功消费的步号不能再次使用。启用确认也消费验证码，随后登录或关闭需等待新的验证码。

登录和下述三个二步验证写操作共用限速：每个 TCP 实际来源 IP 的固定 60 秒窗口最多接受 8 次，全局窗口最多 64 次，通过额度检查后认证成功与失败均计数；并发验证最多 4 个。额度或并发许可耗尽返回 429，被此限制拒绝的请求不消耗任一级额度。计数保存在 PostgreSQL，重启面板不重置窗口。反向代理后的请求按代理地址共享额度，`X-Forwarded-For`、`Forwarded` 不参与放行或分类。

### 二步验证

这些接口要求有效管理员 Cookie，成功响应带 `Cache-Control: no-store`。

| 方法与路径 | 请求 | 成功响应 |
|---|---|---|
| `GET /api/security/totp` | 无 | `200 {"enabled":false,"pending_expires_at":null}` |
| `POST /api/security/totp/setup` | `{"password":"管理员密码"}` | `200 {"secret":"Base32 秘密","otpauth_uri":"验证器导入 URI","expires_at":1790000000}` |
| `POST /api/security/totp/confirm` | `{"code":"六位验证码"}` | `200 {"enabled":true}` |
| `POST /api/security/totp/disable` | `{"password":"管理员密码","code":"尚未使用的六位验证码"}` | `200 {"enabled":false}` |

设置秘密为独立随机 20 字节，编码为无填充 Base32，仅在生成响应返回，绑定发起设置的当前会话，5 分钟内确认有效。重新 setup 替换旧的待确认秘密；启用前仍使用原登录方式。已启用时 setup 返回 409，需先验证并关闭。确认不存在、过期或其他会话发起的设置返回 409；设置/确认/关闭的密码或验证码错误返回 400，缺失或失效会话返回 401。

启用和关闭都在事务内更新状态并撤销其他管理员会话，保留当前完成操作的会话。状态接口不返回秘密。丢失验证器时需由部署所有者经受保护的数据库管理通道恢复，见[部署文档](deploy.md#管理员登录与二步验证)；普通密码不能单独关闭 TOTP。

除订阅、健康检查及另行说明的设备接入端点外，下述接口全部要求管理员登录。列表都是按编号排序的 JSON 数组，无分页。创建返回 201，普通读取和修改返回 200，删除返回 204。

## 服务器与接入

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/servers` | 服务器列表 |
| `POST /api/servers` | `{"name":"服务器名称"}` |
| `GET /api/servers/{id}` | 服务器详情 |
| `PATCH /api/servers/{id}` | `{"name":"新名称"}` |
| `DELETE /api/servers/{id}` | 在线时先退役并等待回执，离线时软删除；成功返回 204 |
| `POST /api/servers/{id}/enrollment` | 签发一次性接入令牌，无请求体；可选查询 `agent_version=0.3.0` 指定已导入版本 |

服务器对象：

```json
{
  "id": 1,
  "name": "示例服务器",
  "device_public_key": null,
  "static_info": {},
  "last_seen": null,
  "last_heartbeat_at": null,
  "metrics_sampled_at": null,
  "metrics_stale": false,
  "latest_metrics": {},
  "manifest_rev": 0,
  "online": false
}
```

`last_seen` 为 Unix 秒，表示最近设备消息，距最后消息不超过 60 秒视为在线。`last_heartbeat_at` 为 Unix 秒，仅 heartbeat 消息更新，旧数据或尚无心跳时为 null。`metrics_sampled_at` 是既有遥测采样时间，单位毫秒；尚无指标或旧 telemetry.metrics 不含采样时间时为 null。`metrics_stale` 按 Agent 采样和上传设置计算；过期不清空最近指标，在线也可能指标过期。静态信息和指标字段见 [协议文档](protocol.md)。未采集到的指标缺省，前端显示“暂无数据”；不得把缺失值显示为测得的零。

接入令牌响应为 `{token,expires_at,install_command,installation,warning}`。有兼容的签名 Agent 时，`installation={version,tag}`、`install_command` 为可信 `sinan-bootstrap` 的接入命令；缺少制品或指定版本不可用时，命令与版本为 null，并返回中文 warning。未指定版本时按已签 metadata 选择最新协议兼容版本，不使用面板产品版本。令牌 24 小时有效、成功注册后只能消费一次。操作者先按部署文档准备独立可信 bootstrap，再复制命令。重新签发令牌可用于原设备升级，已经注册的服务器只接受同一设备公钥。设备注册、WebSocket、制品下载的鉴权方式见协议文档。旧 `/install.sh` 不再提供可执行面板脚本，返回 409 提示可信 bootstrap。

删除服务器使用面板实际持有的 WebSocket 连接判定在线，与列表按最近 60 秒消息显示的 `online` 不同：

- 在线且声明 `server:retire-v1`：先持久保存退役请求并发送指令。Agent 阻止新对账/诊断，等待正在执行的操作，采集可观测终值并停止运行时和诊断服务；已持久用量全部收到确认后清理设备凭据及运行配置，再返回签名完成回执。面板核对后软删除、撤销设备会话和接入令牌，返回 204。
- 在线旧 Agent 缺少退役能力：返回 409，提示先升级；不将未知指令当成成功。
- 当前无连接：直接软删除并撤销面板会话、接入令牌和未完成诊断，返回 204；此分支没有设备清理确认，本机服务和凭据需由操作者处理。

在线分支发送指令后最多等待 20 秒回执；发送失败、设备报告失败或等待超时返回 409，保留面板记录与原请求 ID，操作者可重试。不会在同一次请求内因掉线自动当作离线删除。后续主动重试时若确实离线，可以执行离线分支，但内部状态标记为未确认，不伪造成功回执；尚有用量未确认的设备此时不能再自动完成清理。删除后节点和订阅不再包含该服务器，已确认历史用量保留。

`POST /api/agent/v1/retirement/receipt` 是无需 Cookie/Bearer 的设备恢复端点，请求 `{server_id,request_id,signature}`。它仅接受与持久退役请求、原注册公钥匹配的 Ed25519 签名，不能用任意请求触发删除；有效重复回执返回 204，错请求或签名返回 401，不存在的服务器返回 404。清凭据后的 Agent 可据本地保存的回执恢复提交，避免响应丢失后必须重新注册。签名格式与恢复边界见 [ADR 0019](adr/0019-server-retirement.md)。

## 节点

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/nodes` | 有效服务器下的节点列表 |
| `POST /api/nodes` | `{"name":"节点名称","server_id":1,"public_host":"node.example.com","sni":"www.example.com","port":443}`；`port` 可省略 |
| `GET /api/nodes/{id}` | 节点详情 |
| `PATCH /api/nodes/{id}` | 可选 `name`、`public_host`、`sni`、`port`，至少一个字段；省略 `port` 保留现值 |
| `DELETE /api/nodes/{id}` | 删除节点及现有授权，并安排重新发布 |

节点对象：

```json
{
  "id": 1,
  "name": "示例节点",
  "server_id": 1,
  "protocol": "vless-reality",
  "port": 20000,
  "public_host": "node.example.com",
  "sni": "www.example.com",
  "public_key": "URL-safe 无填充 base64 公钥",
  "short_id": "0123abcd"
}
```

创建时省略 `port` 才会自动分配 20000–29999 中最小空闲端口；显式端口须为 1–65535 的整数，支持 443，18085 保留给本地统计接口。零、越界或保留端口返回 400，同一服务器被其他有效节点占用的端口返回 409 中文说明；不同服务器可使用相同端口。分配和修改均持有服务器行锁，并由数据库唯一约束兜底。面板只能判断托管节点的端口冲突；主机其他程序占用造成的运行时启动失败在部署结果中报告。

修改端口会安排重新发布；字段实际未变的节点 PATCH 不重新标记 dirty，省略端口不会自动重新分配。设备应用新配置后，客户端需更新订阅。创建时仍自动生成 X25519 密钥对和 8 位十六进制 short ID；私钥仅保存在数据库及对应设备配置中，管理 API 不返回。不能迁移所属服务器或手动提供密钥。

名称限制为去除首尾空格后的 1–128 个字符，不接受控制字符。`public_host` 为合法 DNS 名或 IP 地址，不包含协议、端口或路径；IPv6 输入原始地址。`sni` 必须为有效 DNS 名。创建和修改会先经过与原生编译相同的字段验证。

没有用户授权的节点保留在业务模型中，原生配置不生成它的入站监听。删除节点保留用量历史；运行中的节点访问权限要等设备应用新版本才从运行时移除。

## 用户与节点授权

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/users` | 用户列表 |
| `POST /api/users` | `{"name":"用户名称"}` |
| `GET /api/users/{id}` | 用户详情 |
| `PATCH /api/users/{id}` | `{"name":"新名称"}` |
| `DELETE /api/users/{id}` | 删除用户和全部现有授权；历史用量保留 |
| `POST /api/users/{id}/subscription/reset` | 无请求体，原子替换订阅令牌，返回 200 和更新后的完整用户对象 |
| `GET /api/users/{id}/accesses` | 用户的有效节点授权列表 |
| `POST /api/users/{id}/accesses` | `{"node_id":1}`，授权成功返回 200 |
| `DELETE /api/users/{user_id}/accesses/{node_id}` | 撤销授权，重复撤销仍返回 204 |

用户对象为 `{"id":1,"name":"示例用户","subscription_token":"…","subscription_url":"https://panel.example.com/sub/…"}`。订阅令牌可直接获得该用户的代理凭据，界面应把订阅地址作为敏感内容按需展示或复制。

重置生成新的随机 32 字节订阅令牌并立即使旧链接返回 404；新链接指向相同用户，重复重置会再次替换令牌。用户不存在或已删除时返回 404，未登录返回 401。重置不修改节点 UUID、授权、配置版本或流量账本，也不安排重新发布；已经下载的客户端配置仍可连接。需要撤销代理访问时，应撤销相应节点授权。

授权对象为 `{"user_id":1,"node_id":1,"uuid":"UUID","stat_name":"u1_n1"}`。每个“用户 + 节点”具有独立 UUID。重复授权保持原 UUID；撤销后重新授权生成新 UUID，但统计名称仍由用户与节点编号决定。用户和节点列表均不包含已删除对象。

## 自动发布与部署状态

影响配置或订阅投影的节点、用户或授权变更在同一个数据库事务中更新对应服务器的 `dirty_at`；仅重置订阅令牌和字段未变的节点 PATCH 不触发发布。发布任务每秒检查一次，在最后一次变更后等待完整 5 秒，然后编译该服务器的完整快照。`dirty_at` 内部使用 Unix 毫秒，重启面板不会丢失待发布状态。新变更会重新开始合并窗口。

原生配置及包序列化结果确定；包 SHA-256 与最新发布版本相同则不增加版本号。节点名称、公开地址等仅影响客户端的元数据，在这一情况下更新原版本的订阅快照。配置内容变化时增加服务器版本、保存完整包和模型快照，再通知 Agent 拉取。只有事务提交后才发送通知；通知丢失由设备心跳和重新对账恢复。

`GET /api/servers/{id}/deployments` 返回：

```json
{
  "status": {
    "module": "singbox",
    "target_rev": 3,
    "applied_rev": 2,
    "last_result_rev": 3,
    "healthy": true,
    "last_error": "新版本应用失败，旧版本仍健康",
    "updated_at": 1790000000
  },
  "history": [
    {"module":"singbox","rev":3,"bundle_sha256":"64 位十六进制摘要","created_at":1790000000}
  ]
}
```

从未发布时 `status` 为 `null`、`history` 为空。历史按版本倒序，最多 100 条，仅包含元数据。`target_rev` 是最新期望版本；`applied_rev` 是已知成功应用的版本；`last_result_rev` 是最后接受的应用结果版本；`healthy` 表示设备报告当前配置是否健康。失败后成功回滚时可以同时出现 `healthy=true` 和 `last_error`，界面应保留失败提示及当前实际版本。

较旧的应用结果不能覆盖较新结果。认证设备在 hello/heartbeat 中报告已发布且高于面板记录的已应用版本时，面板补齐成功状态，以恢复应用成功但回报丢失的场景；最近目标版本的错误说明仍保留。

## 订阅

订阅不需要管理员 Cookie，以 URL 中的令牌授权：

- `GET /sub/{token}` 或 `?format=links`：标准 base64 编码的多行 `vless://` 分享链接，`text/plain`。
- `GET /sub/{token}?format=singbox`：可导入的 sing-box JSON，包含本地混合代理入口、选择器及用户有权使用的节点。

响应带 `Cache-Control: no-store`。订阅从服务器已应用且健康的版本快照生成，再与当前有效用户、节点和授权 UUID 取交集。待应用的新配置、其他用户 UUID、服务端私钥不会出现在订阅中。撤销授权立即从订阅移除；重新授权的新 UUID 要等相应版本成功应用后出现。只修改名称或公开地址且原生包不变时，合并窗口后无需等待一次空部署即可更新订阅元数据。

无可用节点时 links 返回空文本，singbox 返回 409 和中文说明；删除用户或重置订阅链接后，旧令牌返回 404。不支持的格式返回 400。

## 流量与制品

`GET /api/usage` 返回所有已确认流量，也可传入正整数 `user_id` 和/或 `node_id` 筛选：

```json
{
  "uplink": "1024",
  "downlink": "2048",
  "total": "3072",
  "by_user": [{"user_id":1,"name":"示例用户","deleted":false,"uplink":"1024","downlink":"2048"}],
  "by_node": [{"node_id":1,"name":"示例节点","deleted":false,"uplink":"1024","downlink":"2048"}]
}
```

所有字节总量都是精确十进制字符串，避免浏览器整数精度损失。分组数组只包含有流量的项目；`deleted` 表示对应对象已删除，用于显示历史记录。按 `(server_id, epoch, seq)` 在事务中去重，持久化成功才确认，设备重传不会重复计费。这里的流量仅来自代理统计，与服务器网卡指标分开显示。

`GET /api/artifacts` 返回签名与实际内容均验证通过的制品数组，每项为 `{"name":"agent、sing-box 或 nodequality","version":"版本","arch":"amd64 或 arm64","sha256":"摘要","bytes":123}`。

`POST /api/artifacts/import-release` 请求 `{"tag":"agent-v0.3.0"}`，只接受固定官方仓库的规范 tag，不接受 URL 或其他字段。成功返回 `{tag,artifacts,signature_verified:true}`。先验证签名再下载全部资产，在同文件系统 staging 完成核对后整体发布；失败保留原集合，相同签名集合幂等，相同版本不同内容返回 409，并发导入返回 429。草稿、缺签名、非法根、软链路径或内容篡改均拒绝；面板镜像缺少编译时公钥时也返回 409。目录布局、独立 bootstrap 和轮换步骤见部署文档与 ADR 0017。

## IP 质量与节点报告

| 方法与路径 | 请求与用途 |
|---|---|
| `GET /api/servers/{id}/node-quality` | 返回 IP、质量缓存、插件准备状态和最近十条报告 |
| `POST /api/servers/{id}/node-quality/refresh` | 无请求体；查询并保存质量结果，返回质量数组 |
| `POST /api/servers/{id}/node-quality/reports` | `{ip_version:"both",network_mode:"low"}`；创建一次性报告，返回 201 和任务记录 |

详情响应为 `{ip_addresses,quality,plugin_ready,plugin_reason,reports}`。`plugin_ready` 需要设备在线、声明 `diagnostic:nodequality` 和 `artifact:minisign-v1` 能力、支持的架构、有效对应签名制品；未就绪时 `plugin_reason` 提供原因。仅声明旧运行时能力的 Agent 不能领取诊断任务。

每个质量对象是 `{ip,checked_at,expires_at,status,databases,provider,last_attempt_at,last_success_at,fresh_until,last_error}`，status 为最近查询批次的 `succeeded`、`partial`、`failed`。每个数据库是 `{database,label,status,fields:[{label,value}],error,provider,target_ip,attempted_at,elapsed_ms,error_kind,http_status,last_attempt_at,last_success_at,fresh_until,last_error,historical}`；数字零和布尔 false 保持原值，缺失字段省略。`provider` 为真实查询入口 `check-place`，七个 `database` 是同一入口的响应形状（MaxMind 地理/ASN、IPAPI、Scamalytics、AbuseIPDB、IP2Location、IPData、IPQualityScore）。接口参数依据上游 IPQuality 源码，不假造 NodeQuality 的按 IP 查询接口。每种响应分别展示，不推导统一评分。

`target_ip` 是查询目标；`attempted_at` 为本条尝试开始的 Unix 秒，`elapsed_ms` 为包含解析、连接和响应读取的耗时毫秒。成功时 `error_kind` 和 `http_status` 为空；失败类别为 `dns`、`connect`、`tls`、`timeout`、`http_403`、`http_429`、`http_other`、`non_json`、`schema_mismatch`、`body_error`、`response_limit`、`request_error`。本地拒绝非公网 IP 为 `not_public`，入口地址无效为 `invalid_origin`，批次总超时前未开始的请求为 `not_attempted`，后者不编造尝试时间或耗时。HTTP 失败另保存 `http_status`；旧记录没有逐条时间、耗时或分类时这些字段为空，保留原错误和数据。

缓存按 `(server_id,ip,provider)` 保存，每个数据库独立保留最后成功快照。数据库的 status/error 和逐条尝试元数据始终描述最近尝试，fields 可以同时包含之前成功的字段；此时 `historical=true`。成功快照超过原有效期也标为历史；从未保存成功数据时 fields 为空，信息未知。`last_success_at` 只在该数据库成功时更新，`fresh_until` 为该成功时间加一天；失败不会覆盖成功数据或延长其有效期。`last_error` 为最近尝试的 `{kind,message,http_status,attempted_at,elapsed_ms}`，成功后为空，旧错误的 kind 可以为空。

入口 `last_attempt_at` 为查询批次时间，`last_success_at` 为最近一个数据库成功时间；仅所有已知数据库都有成功快照时入口 fresh_until 有值，取各数据库有效期的最早值，不能据此推断本轮全成功。入口 `last_error` 按 database 索引本轮错误。兼容字段 expires_at 取入口 fresh_until，缺少时为 0；它不再随着失败刷新向后延长。旧 payload 原样保留并迁移明确成功的字段，旧成功时间精度只到原查询批次，未知逐条时间与分类不补造。

页面读取不自动刷新；管理员手工刷新至少间隔一分钟，同机并发刷新在服务器行锁事务内去重。异常退出的运行租约过期后允许恢复。最多处理八个地址，每个源有限时及 64 KiB 响应上限，整体限时并限制并发。非公网地址不向第三方发送，并明确说明原因；外部服务 403、429、超时、非 JSON 或未知响应形状都作为相应源的失败保存，不是零风险。换 IP 不删除旧记录；页面仍只显示当前 IP，旧 IP 再出现时可读取原成功数据。

任务记录为 `{id,status,job,report,error,created_at,updated_at,expires_at}`。status 为 `queued`、`running`、`succeeded`、`failed`；job 的协议结构见 [设备协议](protocol.md)。选项仅允许 `ip_version=both|ipv4|ipv6`、`network_mode=low|normal`，默认 both/low。每台设备同时最多一个活跃任务；并发点击由事务锁与数据库唯一约束去重，返回 409。整体执行时限为 30 分钟，面板另留五分钟传输窗口。

report 为 `{text,report_url?}`，文本以纯文本呈现，协议接受上限 512 KiB；NodeQuality 适配器输出最多 256 KiB，超过时标注截断，原始 ZIP 保留在节点本地。可选链接限定 NodeQuality 官方 HTTPS origin。在线上传失败仍可保存本地报告。报告会执行节点上的资源和带宽测试，上游可能生成公开链接；只有管理员明确点击才创建任务。设备结果持久化后才确认，重复最终回报幂等；晚到的 running 不覆盖最终结果。
