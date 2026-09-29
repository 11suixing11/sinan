# 面板 HTTP API

本页记录 G7 的管理接口，供中文前端和集成测试使用。所有路径都相对于 `SINAN_PUBLIC_URL`。管理接口使用同源 Cookie；请求 JSON 时发送 `Content-Type: application/json`。应用错误返回 `{"error":"中文说明"}`，常见状态码为 400（输入无效）、401（未登录）、404（资源不存在）、409（冲突）、429（请求过多）、500（内部错误）。框架对无法解析的 JSON 或路径参数也可能返回文本错误。

## 登录

| 方法与路径 | 请求 | 成功响应 |
|---|---|---|
| `POST /api/login` | `{"password":"初始管理员密码"}` | `200 {"id":1}`，设置会话 Cookie |
| `GET /api/me` | 无 | `200 {"id":1}` |
| `POST /api/logout` | 无 | `200 {"ok":true}`，清除会话 |
| `GET /healthz` | 无 | `200 ok`，无需登录 |

会话 Cookie 名为 `sinan_session`，带 `HttpOnly; SameSite=Strict; Path=/`；公开地址使用 HTTPS 时同时带 `Secure`。管理员密码只在首次初始化时设定，后续启动不会用环境变量覆盖已有密码。

除订阅、健康检查及另行说明的设备接入端点外，下述接口全部要求管理员登录。列表都是按编号排序的 JSON 数组，无分页。创建返回 201，普通读取和修改返回 200，删除返回 204。

## 服务器与接入

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/servers` | 服务器列表 |
| `POST /api/servers` | `{"name":"服务器名称"}` |
| `GET /api/servers/{id}` | 服务器详情 |
| `PATCH /api/servers/{id}` | `{"name":"新名称"}` |
| `DELETE /api/servers/{id}` | 从面板删除并吊销该设备的面板会话 |
| `POST /api/servers/{id}/enrollment` | 签发一次性接入令牌，无请求体 |

服务器对象：

```json
{
  "id": 1,
  "name": "示例服务器",
  "device_public_key": null,
  "static_info": {},
  "last_seen": null,
  "latest_metrics": {},
  "manifest_rev": 0,
  "online": false
}
```

`last_seen` 为 Unix 秒，距最后消息不超过 60 秒视为在线。静态信息和指标字段见 [协议文档](protocol.md)。未采集到的指标缺省，前端显示“暂无数据”；不得把缺失值显示为测得的零。

接入令牌响应为 `{"token":"…","expires_at":1790000000,"install_command":"curl … | sh"}`。令牌 24 小时有效、成功注册后只能消费一次。安装命令直接展示并允许复制。重新签发令牌可用于原设备升级，已经注册的服务器只接受同一设备公钥。设备注册、WebSocket、制品下载的鉴权方式见协议文档。

删除服务器保留历史用量，节点和订阅不再包含该服务器。Agent 离线时会继续运行本机最后一份可用配置，因此面板删除不是远程停止服务；需由管理员在服务器本地停止相关服务。

## 节点

| 方法与路径 | 请求或用途 |
|---|---|
| `GET /api/nodes` | 有效服务器下的节点列表 |
| `POST /api/nodes` | `{"name":"节点名称","server_id":1,"public_host":"node.example.com","sni":"www.example.com"}` |
| `GET /api/nodes/{id}` | 节点详情 |
| `PATCH /api/nodes/{id}` | 可选 `name`、`public_host`、`sni`，至少一个字段 |
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

创建时按服务器自动分配 20000–29999 中最小空闲端口，并生成 X25519 密钥对和 8 位十六进制 short ID；并发创建通过服务器行锁保证端口唯一。私钥只保存在面板数据库及下发给对应设备的完整配置包中，管理 API 不返回私钥。MVP 不支持迁移所属服务器、修改端口或手动提供密钥。

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
| `GET /api/users/{id}/accesses` | 用户的有效节点授权列表 |
| `POST /api/users/{id}/accesses` | `{"node_id":1}`，授权成功返回 200 |
| `DELETE /api/users/{user_id}/accesses/{node_id}` | 撤销授权，重复撤销仍返回 204 |

用户对象为 `{"id":1,"name":"示例用户","subscription_token":"…","subscription_url":"https://panel.example.com/sub/…"}`。订阅令牌可直接获得该用户的代理凭据，界面应把订阅地址作为敏感内容按需展示或复制。

授权对象为 `{"user_id":1,"node_id":1,"uuid":"UUID","stat_name":"u1_n1"}`。每个“用户 + 节点”具有独立 UUID。重复授权保持原 UUID；撤销后重新授权生成新 UUID，但统计名称仍由用户与节点编号决定。用户和节点列表均不包含已删除对象。

## 自动发布与部署状态

节点、用户或授权变更在同一个数据库事务中更新对应服务器的 `dirty_at`。发布任务每秒检查一次，在最后一次变更后等待完整 5 秒，然后编译该服务器的完整快照。`dirty_at` 内部使用 Unix 毫秒，重启面板不会丢失待发布状态。新变更会重新开始合并窗口。

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

无可用节点时 links 返回空文本，singbox 返回 409 和中文说明；删除用户后其订阅令牌返回 404。不支持的格式返回 400。

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

`GET /api/artifacts` 返回已通过 SHA-256 校验的可用制品数组，每项为 `{"name":"agent 或 sing-box","version":"版本","arch":"amd64 或 arm64","sha256":"摘要","bytes":123}`。制品上传由管理员放入配置的数据目录完成，MVP 没有网页上传接口。目录布局和安装步骤见部署文档及协议文档。
