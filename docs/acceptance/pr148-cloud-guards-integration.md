# PR #148 多云功能与保护补修整合

## 源码与来源

PR #148 作者提交 `366f2f5 → 478043a → 403da459` 新增多云 DDNS、阿里云 CDT/ECS 管理和入口目录整理。作者同期将 `403da459` 合入上游 `1f4ea0b`；本聊天正常整合该 main，通过后续 [PR #150](https://github.com/theLucius7/sinan/pull/150) 交付保护补修，保留作者与原 main `3e99ce9` 的全部祖先，不 force push。

Rust、插件、锁文件、部署工具与前端运行输入冻结于 `1a00e5a77a0d0797032562ce3a8828b950404317`。之后 `830b3dd` 只修正两份浏览器夹具，普通合入最新 main 的 `6243ae6` 与 `830b3dd` 整树相同。全部实现完成后才统一验证；后续收尾只追加本记录与进度导航。作者历史的 772/61 项等结果保留独立来源，不作为本聊天最终结果。

旧 0001–0035 SQL 原字节保留；0036–0038 是作者原迁移，编号唯一连续。Cargo manifests/lock、四个 workflow、既有 PowerShell 接入、NodeQuality 与 17 个固定官方来源（687969 B）、许可/helper 身份保持。根插件布局和薄 Rust 桥、85 条面板路由、鉴权、正文限制、旧看板及节点路径均保留。

## 保护补修

- 实际云操作与 DNS 提交同步读取当前快照，复核账号/资源修订和预览身份、意图、期限；刷新中、失败、实体消失、陈旧账号修订或待核对操作不发写请求，草稿保留。恢复自动开机及移除提交携带观察到的修订。
- 未确认带宽/启停回执阻止更换账号站点或密钥；仍可停用查询。人工结束未知变配同时关闭降速与启停策略、保留手动暂停、撤销旧预览。抢占式保活的已知失败至少从实际响应起等待 900 秒，不按旧任务创建时间缩短冷却。
- 自动控制用 CDT 合计要求完整分页、明确 `PayAsYouGoBill`、GB 单位及实例身份；退款、调整、缺类型或身份保留原项目供展示，但用量授权未知。相同计费维度跨页重复，即使数值或备注变化也拒绝合计；嵌套兼容读数的分页元数据同样核对。
- Huawei 签名保留并排序重复查询参数；Huawei 异常分页、Tencent 非数值权重和 Aliyun 非布尔锁标记拒绝，不继续 DNS 写入。保留旧 Cloudflare/default-provider、显式接管和 Huawei 原描述/标签。

## 本聊天最终本地证据

| 检查 | 实际结果与范围 |
| --- | --- |
| Rust/PostgreSQL | 同一 workspace 默认特征与锁文件，73 批覆盖 89 个 metadata 目标；788 通过、0 失败、20 条件忽略。认证结果为首 19 批加独立续跑 54 批，不称一次不中断整轮。 |
| 首轮记录工具错误 | `diagnostics` 实际 23 项通过，但原脚本把导入的 `probe_support` 误认成第二入口，未认证该批。原日志/失败 footer 保留；按 Running 主入口、可执行文件身份和精确 `.d` 修正后重跑，原 23 项不叠加。 |
| 其他 Rust 检查 | fmt/core、workspace 全 targets Clippy 拒绝警告通过；macOS umask077 原子文件/链接 1 项与真实双栈回环 ICMP 1 项另列，不改完整 20 忽略数。 |
| 前端构建 | Bun 69 项、1429 断言；两次强制 TypeScript/Vite 通过，23 dist 逐字复现。137 份前端运行/构建输入和 18 份单元夹具保持受验字节。 |
| Chromium | 当前 35 套中八套受影响夹具通过，来源为首六套加两套修正后的仓库内通过。原首轮 6/2 保留；外部修复演练不增加覆盖数。云快照 10 场景、34 次拒写；DDNS 1440/390/320 下两路轮询继续更新且保留草稿、零意外写入。全部私有回环 API。 |
| 实际内嵌前端 | 相同冻结输入编译的 debug-embed 面板：23 资产 GET/HEAD 字节、SHA、长度、MIME、cache、nosniff 与根 index 一致；不是正式 release build。 |
| 静态审查 | 四 YAML actionlint、core 边界与冻结输入 605 个本地链接（含 17 个 Markdown 锚点）通过；收尾文档另核对。 |

耐久证据位于本任务私有 `sinan-merge-test/evidence/`：`pr148-final-rust-resume1-20261002/proof.json`（SHA-256 `88038b5cf2d254716c489ca76f016b7af57ac687dc68893daa7e51b2513504be`）、`pr148-final-web-20261002/browser-affected-current-coverage.json`（`060eb3ba5c1b1193e2261e43cb7bd4f943f75e7f24ad55444b2544ed61360595`），及 `pr148-final-embedded-rust-20261002/proof.json` 与 `pr148-final-source-review-20261002/proof.json`。本记录封存的是源码验证阶段；最终主线和 PR #150 合并状态在普通合入后实时读取，交付收据届时生成于 `pr148-final-delivery-20261002/final-main-proof.json`，不把待生成文件当作现有证据。原首轮工具错误、浏览器旧预期失败及外部演练分别保存，不覆盖、不算最终通过。

## 未验证与交付边界

20 项忽略仍要求 Linux/root/systemd、平台 ICMP、固定官方 sing-box/协议运行时或 Pebble。macOS 回环不认证 Linux 设备自然停止、重启/断连、授权撤销或持续代理联合负载。

云接口测试使用签名固定向量、私有 HTTP 替身与专属 PostgreSQL；没有真实云账号、RAM 权限、资源开通/变配/启停、费用或 DNS 传播验证，也未发送真实 Telegram/Webhook。账单有延迟，阈值策略不保证实时硬额度。CDT 兼容读数只展示；原始费用/币种不混加。

四源码 CI 按用户安排继续暂停，取消/未执行不算通过。自己的 PostgreSQL 与临时 HTTP 已停止；无正式签署、发布、部署，公开仍是 `agent-v0.3.0`。既有 NodeQuality full 新启动/旧排队门禁、精确 Started 恢复与确认取消、历史 JSON、私有 FD/UID、旧正式 0.3 接入守卫和原生验收前置顺序保持，相关 issue 不因源码合入关闭。

参见[阿里云插件说明](../alicloud.md)、[DDNS 说明](../ddns.md)和[整改顺序](ordered-remediation.md)。
