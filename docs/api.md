# 面板 HTTP API

本页记录面板管理接口，供中文前端和集成测试使用。所有路径都相对于 `SINAN_PUBLIC_URL`。管理接口使用同源 Cookie；请求 JSON 时发送 `Content-Type: application/json`。应用错误返回 `{"error":"中文说明"}`，常见状态码为 400（输入无效）、401（未登录）、404（资源不存在）、409（冲突）、429（请求过多）、500（内部错误）。框架对无法解析的 JSON 或路径参数也可能返回文本错误。

策略组、套餐组、两跳链路与代理用户分配接口见 [sing-box 策略与套餐 API](singbox-groups.md#api)。这些业务仅属于 `/api/plugins/sing-box`；管理会话和错误约定沿用本页。

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
| `POST /api/servers` | `{"name":"服务器名称"}`；可选 `agent_settings`、`probes` 与 `asset_settings` 一起初始化 |
| `GET /api/servers/{id}` | 服务器详情 |
| `PATCH /api/servers/{id}` | `{"name":"新名称"}`；可选 `asset_settings` 完整替换资产配置，省略则保留 |
| `DELETE /api/servers/{id}` | 在线时先退役并等待回执，离线时软删除；成功返回 204 |
| `POST /api/servers/{id}/enrollment` | 签发一次性接入令牌，无请求体；可选查询 `agent_version=0.3.0`、`platform=unix/windows`、`agent_target=auto/签名ABI`，指定版本、入口与兼容目标 |

创建时 `agent_settings` 使用下文 Agent 设置的完整结构，省略时为 1 秒采样、3 秒批量上传、关闭自动更新、开启公网地址识别；上传间隔不能小于采样间隔，两者均须为 1–60 秒整数。`probes` 默认为空数组，结构与单条拨测创建一致，最多 32 条，传入的 `id` 由服务端重建。服务器、设置和初始拨测在同一事务中保存，任何配置无效或写入失败均不创建服务器。原仅包含 `name` 的请求保持兼容；重命名不会修改监控与拨测设置。接入命令单独签发，命令获取失败后可针对已创建的服务器重试。

`asset_settings` 默认为空资产配置，字段如下。创建/编辑的名称与资产一起校验，任何字段无效均不保存；允许用完整默认对象清空资产信息。隐藏仅控制展示总览，不改变管理员权限或设备运行。

```json
{
  "region": "JP", "group_name": "主力", "tags": ["线路:BGP"], "hidden": false,
  "price": "12.50", "currency": "USD", "billing_cycle": 30,
  "expires_at": null, "auto_renewal": false,
  "traffic_limit": "107374182400", "traffic_limit_type": "sum",
  "reset_day": 1, "network_interface": "eth*,!eth1"
}
```

金额为最多两位小数的十进制字符串或 null（未填写），0 表示免费；上限为 1000000000，币种为三位字母。费用周期单位天，0 为一次性，最多 3650 天；自动顺延需要到期 Unix 秒和非零周期，只维护日期记录。地区/分组最长 16/40 字；标签最多 16 个，每个 1–32 字。额度为精确非负整数字节字符串，最大 2^64−1，0 表示不设额度；口径为 `sum|max|min|up|down`，月重置日为 UTC 1–31，短月取月末。网卡支持逗号分隔的 `*` 和 `!`，最多 16 项、每项 64 字，排除优先。

列表、详情及编辑响应新增 `asset_settings` 与 `traffic`。`traffic={cycle_start,cycle_end,uploaded,downloaded,used,limit,remaining,percent,exceeded,observed_from,last_sample_at,incomplete,interfaces}`。周期边界为 Unix 秒，右端不含；观测时间为 Unix 毫秒。字节均为十进制字符串；无额度或尚无观测时 `remaining`、`percent` 为 null，不能将尚无 `observed_from` 的零值显示成真实计量。`incomplete` 标记断档、接口变化或计数重置，`exceeded` 仅提示状态。创建响应的 `traffic` 暂为 null。历史按原始网卡与 UTC 日保存，改变口径与账单日不会清空累计。测量语义与边界详见[服务器资产](server-assets.md)。

服务器对象的既有字段：

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

接入令牌响应为 `{token,expires_at,install_command,installation,warning}`。有兼容签名 Agent 时，`installation={version,tag,target,platform,bootstrap_url,install_command}`；自动模式 `version="latest"`、`tag=null`，在目标服务器执行时识别 ABI 后选择最新兼容稳定版，显式选版返回精确 version/tag。`target` 默认 `auto`，`platform` 默认 `unix`（Shell，Linux/macOS/FreeBSD），`windows` 返回 PowerShell 单行命令。缺少所选平台/版本的签名 proof 时命令与 installation 为 null，并返回中文 warning。令牌 24 小时有效、成功注册后只能消费一次。重新签发可用于同一设备升级，已经注册的服务器只接受原设备公钥。`GET /install.sh?token=…&agent_version=…&agent_target=…&platform=…` 返回同一安装描述 JSON；`GET /install.ps1` 固定 Windows 入口。两者验证活跃令牌，不返回面板可执行脚本。完整命令下载固定官方 GitHub 入口并核对摘要，入口自动准备依赖与独立验证发布签名。

删除服务器使用面板实际持有的 WebSocket 连接判定在线，与列表按最近 60 秒消息显示的 `online` 不同：

- 在线且声明 `server:retire-v1`：先持久保存退役请求并发送指令。Agent 阻止新对账/诊断，等待正在执行的操作，采集可观测终值并停止运行时和诊断服务；已持久用量全部收到确认后清理设备凭据及运行配置，再返回签名完成回执。面板核对后软删除、撤销设备会话和接入令牌，返回 204。
- 在线旧 Agent 缺少退役能力：返回 409，提示先升级；不将未知指令当成成功。
- 当前无连接：直接软删除并撤销面板会话、接入令牌和未完成诊断，返回 204；此分支没有设备清理确认，本机服务和凭据需由操作者处理。

在线分支发送指令后最多等待 20 秒回执；发送失败、设备报告失败或等待超时返回 409，保留面板记录与原请求 ID，操作者可重试。不会在同一次请求内因掉线自动当作离线删除。后续主动重试时若确实离线，可以执行离线分支，但内部状态标记为未确认，不伪造成功回执；尚有用量未确认的设备此时不能再自动完成清理。删除后节点和订阅不再包含该服务器，已确认历史用量保留。

`POST /api/agent/v1/retirement/receipt` 是无需 Cookie/Bearer 的设备恢复端点，请求 `{server_id,request_id,signature}`。它仅接受与持久退役请求、原注册公钥匹配的 Ed25519 签名，不能用任意请求触发删除；有效重复回执返回 204，错请求或签名返回 401，不存在的服务器返回 404。清凭据后的 Agent 可据本地保存的回执恢复提交，避免响应丢失后必须重新注册。签名格式与恢复边界见 [ADR 0019](adr/0019-server-retirement.md)。

## sing-box 插件启用

代理管理接口与前端在同一版本切换为 `/api/plugins/sing-box/...`，旧根管理路径不保留别名，返回 404。已有 `/sub/{token}` 订阅地址永久保留，旧表、业务编号、令牌、密钥、授权和用量账本不重写，详见 [ADR 0030](adr/0030-singbox-plugin-business.md)。

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/plugins/sing-box/servers` | 未删除服务器的插件启用元数据列表 |
| `GET /api/plugins/sing-box/servers/{id}` | 单台服务器的插件启用元数据 |
| `POST /api/plugins/sing-box/servers/{id}/enable` | 空 JSON `{}`；管理员明确启用，重复请求幂等 |

元数据为 `{id,name,enabled,online,agent_supported,read_only,source}`。source 为 `administrator`、当前 `agent_capability`、兼容 `legacy_nodes` / `legacy_deployments` 或 null。设备声明与既有配置来源显示为只读；没有当前能力、管理员选择或历史代理配置的服务器保持关闭，保存过但已消失的设备声明不会单独启用。创建节点和读取部署需先启用，否则返回 409；启用不表示设备已经声明支持。关闭服务器的详情不请求代理节点或部署，后台不生成代理配置；已有网卡遥测继续显示。

## 节点

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/plugins/sing-box/nodes` | 有效服务器下的节点列表 |
| `POST /api/plugins/sing-box/nodes` | `{"name":"节点名称","server_id":1,"public_host":"node.example.com","sni":"www.example.com","port":443}`；`port` 可省略 |
| `GET /api/plugins/sing-box/nodes/{id}` | 节点详情 |
| `PATCH /api/plugins/sing-box/nodes/{id}` | 可选 `name`、`public_host`、`sni`、`port`，至少一个字段；省略 `port` 保留现值 |
| `DELETE /api/plugins/sing-box/nodes/{id}` | 删除节点及现有授权，并安排重新发布 |

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

修改端口会安排重新发布；字段实际未变的节点 PATCH 不重新标记 dirty，省略端口不会自动重新分配。设备应用新配置后，客户端需更新订阅。Reality 创建时自动生成 X25519 密钥对和 8 位十六进制 short ID；私钥仅保存在数据库及对应设备配置中，管理 API 不返回。不能迁移所属服务器或手动提供协议密钥。

名称限制为去除首尾空格后的 1–128 个字符，不接受控制字符。`public_host` 为合法 DNS 名或 IP 地址，不包含协议、端口或路径；IPv6 输入原始地址。Reality 和 TLS 协议的 `sni` 必须为有效 DNS 名；Shadowsocks 2022、Snell v6 的 `sni` 省略或为空。创建和修改会先经过与原生编译相同的字段验证。

可选 `protocol_config` 决定协议，省略时仍为 `{"type":"vless-reality"}`。支持以下结构：

```json
{"type":"hysteria2","tls":{"mode":"acme","email":"admin@example.com","challenge":"http-01"}}
```

`type` 还支持 `tuic`、`anytls`、`naive`（同样必须带 `tls`）、`shadowsocks2022`（可选 `method` 为 `2022-blake3-aes-128-gcm`，或 `2022-blake3-aes-256-gcm`）以及 `snell-v6`。协议和 SS2022 加密方法创建后不可更改。PATCH 可提交相同协议的完整 `protocol_config` 修改 TLS；省略则不变。

`tls.mode=acme` 必须提供邮箱及 `challenge=http-01|tls-alpn-01`；自动管理 Let's Encrypt 签发和续期。同一服务器共享邮箱与验证方式；编辑已有自动证书时这两项原子同步到该服务器所有自动证书节点。验证 TCP 端口与托管 TCP 节点冲突会返回 400，整个更新回滚。`tls.mode=manual` 创建时必须同时提供 `certificate`、`key` 两个 PEM 字符串；更新时两项均省略会保留原证书，替换时仍需同时提供。格式、长度或密钥不匹配返回 400。

节点响应额外返回脱敏的 `protocol_config`：自动证书含邮箱与验证方式，手动证书仅含 `{"mode":"manual","configured":true}`，SS2022 仅含加密方法。节点 PSK、授权密码、TLS 私钥均不回显。新增协议统一使用 `format=singbox` 订阅；`format=links` 遇到新增协议返回 409 并提示使用 JSON。详见 [协议与证书](proxy-protocols.md)。

没有用户授权的节点保留在业务模型中，原生配置不生成它的入站监听。删除节点保留用量历史；运行中的节点访问权限要等设备应用新版本才从运行时移除。

## 用户与节点授权

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/plugins/sing-box/users` | 用户列表 |
| `POST /api/plugins/sing-box/users` | `{"name":"用户名称"}` |
| `GET /api/plugins/sing-box/users/{id}` | 用户详情 |
| `PATCH /api/plugins/sing-box/users/{id}` | `{"name":"新名称"}` |
| `DELETE /api/plugins/sing-box/users/{id}` | 删除用户和全部现有授权；历史用量保留 |
| `POST /api/plugins/sing-box/users/{id}/subscription/reset` | 无请求体，原子替换订阅令牌，返回 200 和更新后的完整用户对象 |
| `GET /api/plugins/sing-box/users/{id}/accesses` | 用户的有效节点授权列表 |
| `POST /api/plugins/sing-box/users/{id}/accesses` | `{"node_id":1}`，授权成功返回 200 |
| `DELETE /api/plugins/sing-box/users/{user_id}/accesses/{node_id}` | 撤销授权，重复撤销仍返回 204 |

用户对象为 `{"id":1,"name":"示例用户","subscription_token":"…","subscription_url":"https://panel.example.com/sub/…"}`。订阅令牌可直接获得该用户的代理凭据，界面应把订阅地址作为敏感内容按需展示或复制。

重置生成新的随机 32 字节订阅令牌并立即使旧链接返回 404；新链接指向相同用户，重复重置会再次替换令牌。用户不存在或已删除时返回 404，未登录返回 401。重置不修改节点 UUID、授权、配置版本或流量账本，也不安排重新发布；已经下载的客户端配置仍可连接。需要撤销代理访问时，应撤销相应节点授权。

授权对象为 `{"user_id":1,"node_id":1,"uuid":"UUID","stat_name":"u1_n1"}`。每个“用户 + 节点”具有独立 UUID；新增协议还会生成独立密码或密钥，仅随当前用户的有效订阅下发。重复授权保持原凭据；撤销后重新授权生成新凭据，但统计名称仍由用户与节点编号决定。用户和节点列表均不包含已删除对象。

## 自动发布与部署状态

影响配置或订阅投影的节点、用户或授权变更在同一个数据库事务中更新对应服务器的 `dirty_at`；仅重置订阅令牌和字段未变的节点 PATCH 不触发发布。发布任务每秒检查一次，在最后一次变更后等待完整 5 秒，然后编译该服务器的完整快照。`dirty_at` 内部使用 Unix 毫秒，重启面板不会丢失待发布状态。新变更会重新开始合并窗口。

原生配置及包序列化结果确定；包 SHA-256 与最新发布版本相同则不增加版本号。节点名称、公开地址等仅影响客户端的元数据，在这一情况下更新原版本的订阅快照。配置内容变化时增加服务器版本、保存完整包和模型快照，再通知 Agent 拉取。只有事务提交后才发送通知；通知丢失由设备心跳和重新对账恢复。

`GET /api/plugins/sing-box/servers/{id}/deployments` 返回：

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

`GET /api/plugins/sing-box/usage` 返回所有已确认流量，也可传入正整数 `user_id` 和/或 `node_id` 筛选：

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

`GET /api/artifacts` 返回签名与实际内容均验证通过的制品数组，每项包含 `name`、`version`、`arch`、`sha256`、`bytes`，arch 可为旧 amd64/arm64 或精确签名 ABI，例如 `{"name":"sing-box","version":"版本","arch":"amd64","sha256":"摘要","bytes":123}`。插件目录按组件身份归并不同版本和架构；`agent` 独立展示为基础组件，已登记插件为 `sing-box`、`nodequality`、`tcpquality`，未登记组件仅展示分发信息。此响应不是任何服务器的已安装列表，也不能代替服务器能力、版本或安全门禁检查。

`GET /api/artifacts/targets` 返回 `{default_targets,supported_targets}`，仅管理员可读取。默认目标由现有服务器上报的 Agent/运行时平台架构推断；没有可用上报时使用面板宿主平台。

`GET /api/artifacts/agent-versions` 需要管理员；`GET /api/bootstrap/versions?token=…` 需要有效接入令牌。查询可选 `target=auto/签名ABI`、`platform=unix/windows/linux`、`agent_version=latest/精确版本`。返回 `{versions:[{version,tag,targets,cached_targets,protocol_min,protocol_max}]}`，默认最新目录仅含稳定版并按数字版本降序，Linux 的显式合法预发布版本可单独查询；原生服务入口按现有服务管理器约束只提供稳定版。`targets` 来自完整签名 proof，`cached_targets` 只列已缓存且字节验证通过的 Agent；缺少缓存不隐藏合法签名目标。目录只提供候选，客户端须独立验签并检查本机 ABI/协议。

`GET /api/bootstrap/{version}/{arch}?token=…` 验证接入令牌后仍返回 409：Agent 二进制必须从 GitHub Release 或独立 HTTPS 镜像下载，面板不提供 Agent。独立安装入口使用已签 metadata 的 asset_name 构造固定官方 tag 地址，只取本机系统/CPU/ABI，不向 GitHub/镜像发送 token 或设备凭据。

`POST /api/artifacts/import-release` 是部署维护接口，插件目录不提供此操作。请求 `{"tag":"agent-v0.3.0","targets":["linux-gnu-arm64"]}`，`targets` 可省略以自动匹配，不接受空数组、重复或未知目标。仅接受固定官方仓库的规范 tag，不接受 URL。成功返回 `{tag,targets,artifacts,signature_verified:true}`。完整 proof 验签后，仅下载所选平台的兼容制品；ARM 不下载 AMD。所选内容在同文件系统私有 staging 完成核对，随后公布本地清单。相同标签可追加目标或重导以修复缺失/损坏的普通文件；旧完整目录兼容。同一身份不同内容返回 409，并发导入返回 429；下载/验签失败保留原集合。草稿、缺签名、非法根、软链路径或内容篡改均拒绝；面板镜像缺少编译时公钥时也返回 409。此接口只准备已验证的分发文件，不安装或运行插件。目录布局、独立 bootstrap 和轮换步骤见部署文档与 ADR 0017、0037、0038、0041。

## 服务器 IP 信息

| 方法与路径 | 请求与用途 |
|---|---|
| `GET /api/servers/{id}/ip-quality` | 返回 `{ip_addresses,public_ip_addresses,private_ip_addresses,quality,providers}`；公网/非公网分类复用质量查询的地址规则，保留原 `ip_addresses` 和数量上限；只读取当前 IP 缓存，不依赖 NodeQuality 能力、在线或制品准备 |
| `POST /api/servers/{id}/ip-quality/refresh` | 无请求体；查询并保存质量结果，返回质量数组 |

两个接口均要求管理员会话和未删除的服务器。读取不会发起外部查询或创建诊断任务。

每个质量对象是 `{ip,checked_at,expires_at,status,databases,provider,last_attempt_at,last_success_at,fresh_until,last_error}`，status 为最近查询批次的 `succeeded`、`partial`、`failed`。每个数据库是 `{database,label,status,fields:[{label,value,kind}],error,provider,target_ip,attempted_at,elapsed_ms,error_kind,http_status,last_attempt_at,last_success_at,fresh_until,last_error,historical,available,unavailable_reason}`；数字零和布尔 false 保持原值，缺失字段省略。`provider` 标识真实入口：`check-place` 是旧聚合入口，七个 `database` 是该入口的响应形状（MaxMind 地理/ASN、IPAPI、Scamalytics、AbuseIPDB、IP2Location、IPData、IPQualityScore）。接口参数依据上游 IPQuality 源码，不假造 NodeQuality 的按 IP 查询接口。每种响应分别展示，不推导统一评分。正式来源 `abuseipdb-api` 仅有 `abuseipdb-v2` 响应视图，固定只读 CHECK、30 天窗口，展示文档定义的用途、国家代码、ISP、Tor 与 0–100 原始滥用置信度；必须确认目标 IP/版本/公网状态一致。正式响应中的该 score 仅接受 JSON 整数 0–100，缺失/空值/错误类型不推导为零分或 false。

`providers` 是入口描述列表，字段为 `{provider,label,kind,execution,enabled,reason,databases:[{database,label}]}`。kind 为 aggregator/credential_api/node_self，execution 为 panel/node；enabled 仅表示可执行或已配置，不证明凭据授权/额度或最新查询成功。缺失/无效 `SINAN_ABUSEIPDB_API_KEY` 不发正式接口请求，原因不含凭据，`ipquality-node` 目前明确未启用。没有请求就没有新缓存行或尝试时间。所有入口共用四并发/40 秒批次/单请求 3 秒连接与 6 秒总超时/64 KiB 上限，403/429 不重试或更改 UA。流媒体解锁保持未知，面板查询不冒充节点出口自查。

`available` 表示缓存所属入口目前是否已启用，`unavailable_reason` 提供中文原因。已关闭或未注册入口的成功快照保留并标记 historical，不影响原尝试/成功时间、字段或当前错误；没有发生失败请求时不编造错误类别。旧接口仍可读取省略新增字段的旧 payload。

`target_ip` 是查询目标；`attempted_at` 为本条尝试开始的 Unix 秒，`elapsed_ms` 为包含解析、连接和响应读取的耗时毫秒。成功时 `error_kind` 和 `http_status` 为空；失败类别为 `dns`、`connect`、`tls`、`timeout`、`http_403`、`http_429`、`http_other`、`non_json`、`schema_mismatch`、`body_error`、`response_limit`、`request_error`。本地拒绝非公网 IP 为 `not_public`，入口地址无效为 `invalid_origin`，批次总超时前未开始的请求为 `not_attempted`，后者不编造尝试时间或耗时。HTTP 失败另保存 `http_status`；旧记录没有逐条时间、耗时或分类时这些字段为空，保留原错误和数据。

字段 `kind` 是可选的 `text`、`country_code`、`boolean`、`score`、`asn`、`latitude`、`longitude`；旧 payload 缺失时仍可读取。已知字段按语义验证：布尔标记只接受 JSON bool，评分只接受非负有限数字或可确认的原始数字字符串（IPAPI 的数字加括号评级保留原文），ASN 为正整数、坐标在合法范围，文本和国家代码必须有效。缺失/null/对象/数组/空白/占位字符串、错误类型与非法数值不作为事实；不转为 0 或 false。响应明确失败、success/status 类型不可信或含错误时，不采纳其中默认字段。没有有效字段记为 schema_mismatch，信息未知；部分字段有效时只保留该部分，不推导未返回的标记。旧缓存读取对可识别字段重新校验，原磁盘快照保留；原缓存没有保存响应包，不能追溯确认当时遗漏的来源失败标志。页面对无效字段显示未知，未知状态不计为当前成功。

缓存按 `(server_id,ip,provider)` 保存，每个数据库独立保留最后成功快照。数据库的 status/error 和逐条尝试元数据始终描述最近尝试，fields 可以同时包含之前成功的字段；此时 `historical=true`。成功快照超过原有效期也标为历史；从未保存成功数据时 fields 为空，信息未知。`last_success_at` 只在该数据库成功时更新，`fresh_until` 为该成功时间加一天；失败不会覆盖成功数据或延长其有效期。`last_error` 为最近尝试的 `{kind,message,http_status,attempted_at,elapsed_ms}`，成功后为空，旧错误的 kind 可以为空。

入口 `last_attempt_at` 为查询批次时间，`last_success_at` 为最近一个数据库成功时间；仅所有已知数据库都有成功快照时入口 fresh_until 有值，取各数据库有效期的最早值，不能据此推断本轮全成功。入口 `last_error` 按 database 索引本轮错误。兼容字段 expires_at 取入口 fresh_until，缺少时为 0；它不再随着失败刷新向后延长。旧 payload 原样保留并迁移明确成功的字段，旧成功时间精度只到原查询批次，未知逐条时间与分类不补造。

页面读取不自动刷新；管理员手工刷新至少间隔一分钟，同机并发刷新在服务器行锁事务内去重。异常退出的运行租约过期后允许恢复。最多处理八个地址，每个源有限时及 64 KiB 响应上限，整体限时并限制并发。非公网地址不向第三方发送，并明确说明原因；外部服务 403、429、超时、非 JSON 或未知响应形状都作为相应源的失败保存，不是零风险。换 IP 不删除旧记录；页面仍只显示当前 IP，旧 IP 再出现时可读取原成功数据。

## NodeQuality 报告

| 方法与路径 | 请求与用途 |
|---|---|
| `GET /api/servers/{id}/node-quality/reports` | 返回 `{plugin_ready,plugin_reason,reports,cancel_supported,proxy_activity}`，NodeQualityView 不包含 IP 查询字段 |
| `POST /api/servers/{id}/node-quality/reports` | 日常：`{mode:"daily",ip_version:"both"}`；完整：`{mode:"full",confirm_full:true,acknowledge_traffic_warning:true,ip_version:"both",network_mode:"low"}`；创建一次性报告，返回 201 和任务记录 |

`plugin_ready` 需要设备在线、明确 Linux、支持的架构、声明 `diagnostic:nodequality`、`diagnostic:nodequality-modes`、`diagnostic:report-sections` 与 `artifact:minisign-v1` 能力，以及有效对应 r4 签名制品；未就绪时 `plugin_reason` 提供原因。仅声明旧运行时能力的 Agent 不能领取诊断任务。报告读取不访问 IP 缓存，IP 缓存损坏或查询失败不会阻止读取已保存报告。

过渡兼容保留 `GET /api/servers/{id}/node-quality` 的原 `{ip_addresses,quality,plugin_ready,plugin_reason,reports}` 组合响应，由 LegacyNodeQualityView 汇合两个视图；`POST /api/servers/{id}/node-quality/refresh` 继续作为相同 IP 刷新的别名。兼容读取路由、刷新间隔和历史保留；新建完整报告仍必须明确管理员确认。新前端只使用独立接口，服务器概况不加载这两个视图，子导航分别访问 `#/servers/{id}/ip-info` 和 `#/servers/{id}/node-quality`。

任务记录包含 `{id,status,job,report,error,created_at,updated_at,expires_at,agent_completed,cancel_requested_at,cancel_error,expected_sections,report_completeness,sections}`。status 为 `queued`、`running`、`cancel_requested`、`cancelled`、`succeeded`、`failed`；job 的协议结构见 [设备协议](protocol.md)。入口 `mode=daily|full` 默认为 full，旧空请求因缺少明确完整确认而返回 400；`confirm_full` 和 `acknowledge_traffic_warning` 必须是真正 JSON bool。完整需要 confirm_full=true，流量 active/unknown 时还需要 acknowledge_traffic_warning=true，否则返回 409。`proxy_activity={state,reason,checked_at,last_positive_at}` 的 state 为 active、unknown 或 not_enabled，近一分钟正向代理计量为 active；配置存在但无新正向计量时为 unknown，不以网卡流量推断无连接。确认和该次流量证据作为 job 的额外审计字段保存。IP 版本允许 `both|ipv4|ipv6`；full 网络模式允许 `low|normal`，默认 both/low。daily 必须 low、关闭 upload_report，目标来自该服务器最多4个已启用TCP拨测，不能通过该接口传任意目标。每台设备同时最多一个活跃任务；等待确认取消也保持同机互斥；并发点击由事务锁与数据库唯一约束去重，返回 409。full 执行时限为30分钟，daily为90秒，面板均另留五分钟传输窗口。日常入口同时调用独立IP刷新接口，查询失败仍按逐源历史缓存显示；Agent只执行有界TCP检查，DNS2秒/每连接1秒/每地址族4次。资源profile日常64MiB/32tasks、完整512MiB/128，保留现有预检和运行保护。启动资源、负载与实际ServiceJob预算随检查点保存为environment独立章；日常预期2章，完整6章。

report 为 `{text,report_url?}`，文本以纯文本呈现，协议接受上限 512 KiB；NodeQuality 适配器输出最多 256 KiB，超过时标注截断，原始 ZIP 保留在节点本地。可选链接限定 NodeQuality 官方 HTTPS origin。在线上传失败仍可保存本地报告。报告会执行节点上的资源和带宽测试，上游可能生成公开链接；只有管理员明确点击才创建任务。设备结果持久化后才确认，重复最终回报幂等；晚到的 running 不覆盖最终结果。
