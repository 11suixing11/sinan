# NodeQuality 公共认证材料访问边界

2026-10-01，对应 #121、#122。本项新增 r18，基于 r17 的固定源码和原生 curl 身份保护；与 #131 的授权草稿变更一起组成最后一批提交。本文件只记录源码边界，按用户要求直到全部批次完成才统一测试，本项当前没有构建或实机执行。

固定 IPQuality 中 `read_ref` 的公共 Cookie 下载和重试、ipregistry 的网页临时/公共备用 key、DB-IP 的网页临时 key，以及 Disney+、YouTube Premium、ChatGPT 的公共或固定认证材料均没有正式节点操作者配置。本项给这些实际函数加入不可由环境选项跳过的缺配置返回：先清本次结果与继承的 Cookie，再按来源写入明确原因，返回之前不调用 curl、DNS、进度子进程或认证流程。原始许可证、固定源字节、函数入口和来源/媒体结果列保留；受影响来源没有消失，也没有被记为成功、低风险或解锁。

| 节点来源 | 本次行为 | 能力恢复条件 |
| --- | --- | --- |
| ipregistry | 不访问首页抓 key，不用源码公共备用 key；Usage/Company 为 JSON null，因素未知 | 经授权的操作者正式节点接口配置与独立适配，目前没有该适配 |
| DB-IP | 不抓页面 data-api-key，不查询临时授权接口；评分/因素保持未知 | 经授权的操作者正式节点接口配置与独立适配，目前没有该适配 |
| Disney+ | 不读取在线公共 Cookie、不发固定授权材料；状态未知、Region/Type 为 JSON null | 与服务条款匹配的认证适配及受控验收，目前没有该适配 |
| YouTube Premium | 不发源码固定 Cookie；状态未知、Region/Type 为 JSON null | 对应正式认证适配与受控验收，目前没有该适配 |
| ChatGPT | 不发源码固定 Cookie；状态未知、Region/Type 为 JSON null | 对应正式认证适配与受控验收，目前没有该适配 |

静态范围按固定源码的 Cookie/Authorization/Bearer/API key 标记及 curl `-b` 选项逐函数核对：实际读取/发送材料的入口为 `read_ref`、`db_ipregistry`、`db_dbip`、`MediaUnlockTest_DisneyPlus`、`MediaUnlockTest_YouTube_Premium`、`OpenAITest`，均已覆盖。其余函数未发现这些认证材料形状；这只覆盖脚本中的明确材料，不能认证所有第三方服务或二进制的访问条件。`check_mail` 等非 curl 命令的 `-c` 不构成 Cookie 用法。`read_ref` 只包含公共 Cookie 与未使用的 IATA 地址声明；IP 的 iso3166/dnsbl 和 Net 的五份非凭证参考文件由原 `data-policy.py` 在各读取表达式内直接嵌入，新增返回不会阻断这七份数据。

新增 `.ProviderAccess` 按来源提供 `Status=not_attempted`、`ErrorKind=credential_not_configured`、`Execution=node_self`、`Attempted=false` 与具体中文 `Reason`；不会增加未隐藏的目标 IP。受影响媒体的 `.Media.<source>.Reason` 和文本报告保留原因，其余来源和原来的 Netflix 变换保持。原版访问材料仍位于保留的固定源内，但这些运行入口不会读取、发送或重试；字节保留用于来源及许可完整性，不代表 Sinan 将它们当作操作者授权。

面板现有 `SINAN_ABUSEIPDB_API_KEY` 正式接口适配和最近成功缓存不变。它执行于面板，不能被当作节点出口的认证或流媒体能力，不会自动把面板密钥传播到节点。节点对应 provider 的正式适配并不存在，本项不能把“没有使用公共材料”签收为“已恢复正式认证能力”；#121/#122 的安全访问边界可以统一测试，但相关能力恢复继续待明确授权/配置与实机验收。

r18 使用独立身份并保留 r17、r16、r15、r14 和更旧支持版本的收集/取消与日常任务支持。它不覆盖历史 r14/r15 制品，不修改 full 门禁，不接受第三方条款、不部署、不触发或恢复 CI。#28/#65/#66/#82 的完整工具权利和执行链缺口继续见 [第四批记录](nodequality-batch4-readiness.md)。

统一回归入口是 `tools/test-nodequality-public-access-policy.py`：用固定源中的真实函数和外部请求记录桩证明两个 IP family 均不访问未授权路径；验证旧结果清空、各来源原因、JSON null、无 Cookie 下载/重试、未涉及来源保留及 helper/input/output 身份/文件边界。所有请求桩只记录私有哨兵，禁止访问原服务；静态输入不运行顶层脚本、硬件测试或专有工具。当前仅编写回归，未执行。
