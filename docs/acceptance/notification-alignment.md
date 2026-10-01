# 通知渠道与告警对齐验收

本轮对照本机 NodeFlare 的通知、Webhook、告警查询及设置界面，实现一个 Telegram 渠道与一个 Webhook 渠道并行使用。保留既有离线、上线恢复、资源超限与恢复、到期、网卡流量递增提醒，不增加重复的告警类别，也不执行续费、停用或远程命令。

## 行为与边界

- Webhook 提供通用 JSON、Bark、Discord、Slack、企业微信、钉钉、飞书、ntfy、Gotify 预设。钉钉和飞书使用关键词等接入方式，当前不生成动态签名。
- URL、请求头和 JSON 模板均只写不回显；模板可能包含设备密钥。相同预设下空白输入保留原值，请求头可显式清除；切换预设不能继承旧渠道凭据。Telegram 机器人令牌继续只写不回显。
- 管理员明确选择 HTTP/HTTPS 接收地址，可使用自建服务。请求不使用环境代理、不跟随重定向；限制连接和整体超时、请求模板与响应体大小，禁止覆盖连接、代理和消息长度相关请求头。错误不包含 URL、远端描述、认证头或响应原文。
- 自建服务可为管理员授权的内网目标，DNS 解析遵循该目标，未宣称公网 SSRF 地址过滤。设备观测和模板占位符不能改变接收地址；连接及 DNS、发送、响应读取共用 8 秒请求截止。
- Discord 请求固定 `wait=true`，空的 204 不再冒充保存确认；需本次返回有效消息 ID。Slack 需纯文本 `ok`，Gotify 需正整数消息 ID，ntfy 需消息 ID、`message` 事件及与请求一致的 `topic`；飞书双成功码若冲突会拒绝。通用 JSON 仍将 HTTP 2xx 作为自建端接收确认。
- JSON 解析后只在字符串值内替换允许的占位符。字符串会正常 JSON 转义，插入值中的占位符不会二次展开。`event` 为事件类型，`event_id` 为站内事件编号；Telegram 原有 `event` 编号语义保持兼容。
- 一条事件为每个已启用渠道分别写入投递记录。每渠道的告警与恢复按序发送，独立重试，最多尝试 8 次；指数退避并遵守限流等待时间。Telegram 已成功时，Webhook 重试不会再发送 Telegram。一个渠道的模板超限不会阻止另一个渠道入队。
- Webhook `Retry-After` 支持非负秒数和 HTTP 日期，拒绝负数与非法值；下次尝试从实际请求完成时计算，单次等待范围为 1–86400 秒。Telegram 保留响应中的 `retry_after`，同样从完成时算等待。发送尝试时间和成功确认时间分别记录。
- 接收端接受消息而面板尚未提交成功记录时发生进程故障，重试仍可能重复；提供至少一次投递，不承诺外部服务的精确一次消费。接收端可使用 `event_id` 与 `event` 去重。
- 修改 Telegram 接收目标，或修改 Webhook 地址、请求头、模板，仅取消对应渠道的待发送消息；关闭通知总开关取消全部待发送消息。配置变化不会补发历史事件。历史投递状态保留，界面区分渠道、重试次数、下次尝试时间与发送结果。
- 测试使用已保存配置，每渠道至少间隔 30 秒，记录最近一次结果；可以在自动通知未启用时测试。测试记录和错误均不包含秘密。HTTP 2xx 或预设成功码仅表示接收服务接受请求，不证明管理员已阅读通知。
- 资源恢复通知附带恢复时的实际规则观测值；原触发内容和恢复内容分别保留。窗口平均按真实样本数加权，持续超限检查真实最小值；选中网卡在同一采样时刻先合计再聚合，范围标识与当前配置绑定。缺失指标、混合范围、旧末值、部分分钟及未跨窗口末端的持久水位均保持未知，不误报恢复。保存间隔越长，资源告警等待完整持久窗口的延迟也会增加；设备心跳不依赖历史写入。

这里“持续”限定为窗口内有效观测均满足条件；每分钟有观测不证明逐秒覆盖，也不证明两次采样之间的数值。

## 自动验证

对应测试：

- `notifications::webhook::tests`：模板转义与单次替换、非法目标及请求头、回环 HTTP 429、重定向拒绝、预设业务错误、响应体上限及错误脱敏。
- `notifications::telegram::tests`：保留 Telegram 429 `retry_after`、话题 ID、关闭链接预览及远端秘密不回显。
- `notifications::outbox::tests`：顺序与并发领取、渠道独立重试、成功渠道不重复、最大重试次数及投递时间记录。
- `notification_channels`：迁移保留旧 Telegram 投递、管理员鉴权、凭据只写与空白保留、切换预设拒绝继承旧秘密、已保存配置回环测试与冷却、单渠道变更取消，以及离线和恢复的实际回环发送。
- `notification_rules`、`notification_migration`：既有告警设置与迁移回归。
- `web/tests/notification-channels.mjs`：1440、390、320 像素视口下的九种预设、空白秘密字段、保存后测试、失败状态、分渠道记录和配置删除；现有 `monitoring.mjs` 与 `server-operations.mjs` 同步新设置只读接口。

2026-10-02 专项结果：

- `cargo test -p sinan-panel --lib notifications::`：10 项通过，含真实聚合窗口权重、最小值、缺测、混合网卡范围与持久水位门禁。
- `cargo test -p sinan-panel --test notification_channels --test notification_rules --test notification_migration`：分别 4、3、1 项通过。
- `cargo clippy -p sinan-panel --all-targets -- -D warnings`、通知相关 Rust 格式化及 `git diff --check`：通过。
- `bun run build`：通过。最终资源判断说明已纳入主代理的整合构建，整体验证结果见 [PROGRESS](../../PROGRESS.md)。
- 本机 Chromium 执行 `notification-channels`（1440/390/320）、`monitoring`（1440/390/320）、`server-operations`（1440/390）：通过。旧夹具补齐新增只读接口，并明确区分 Telegram 与 Webhook 模板预览定位；通知页面按既有局部样式模式支持 320 像素，未修改全局最小宽度。
- 截图证据位于本机 `/tmp/sinan-operations-screenshots/notifications-{1440,390,320}.png`、`monitoring-settings-{1440,390,320}.png`、`operations-settings-{1440,390}.png`。

数据库测试使用专用回环 PostgreSQL 和公开测试信任根，HTTP 测试只使用回环模拟接收端，未向真实 Telegram 或 Webhook 发送消息。CI 持续暂停；没有真实通知平台、生产设备或正式部署验收。

## 2026-10-02 整合补修待验

上述作者专项结果对应原作者输入，不覆盖本次 ACK、限流时钟及迁移编号补修。本次只完成源码审查和回归源码，全部冻结后统一执行验证，未在中途运行测试或构建。新增回归涵盖九种预设确认/错误、Discord `wait` 固定、HTTP 日期与非法限流值、实际发送耗时后安排重试。最终结果按组合冻结提交记录。

本次接口依据为 [Discord](https://docs.discord.com/developers/resources/webhook#execute-webhook)、[Slack](https://docs.slack.dev/messaging/sending-messages-using-incoming-webhooks/)、[Gotify](https://gotify.net/api-docs)、[ntfy](https://docs.ntfy.sh/publish/#publish-as-json)、[Telegram](https://core.telegram.org/bots/api#making-requests) 及 [RFC 9110](https://www.rfc-editor.org/rfc/rfc9110.html#name-retry-after)，只读取公开文档，未发送真实通知。
