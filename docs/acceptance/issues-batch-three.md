# 第三批：独立 IP、共用诊断、响应确认与原生 TCP

本批核对 #25、#27、#42、#47、#58，基线为 `bb9638b`。这些原始源码缺陷已有实现，本次不重复重写；本批未运行测试。最终整合后的测试结果另行记录，历史收据不认证当前交付。

| Issue | 当前实现与最终检查入口 | 尚未完成的验收 |
| --- | --- | --- |
| #25 | `/api/servers/{id}/ip-quality` 独立调用 `ip_quality::get/view`；诊断任务由独立路由返回；`ip_addresses` 与 IP 缓存/来源回归检查未知及私网分组 | 当前 dist 桌面/手机与真实设备的独立 IP 视图复核 |
| #27 | `diagnostic_plugins.rs` 登记 NodeQuality/TCP；`diagnostics/service` 统一预算、互斥、生命周期；`diagnostic_service`、`diagnostic_sections`、`tcpquality_plugin` 覆盖旧入口和历史 | 新完整 NodeQuality 的安全工具链、许可与联合负载仍待完成；不由历史可读签收完整执行 |
| #42 | `ip_quality/fields.rs` 按字段类型、范围和响应状态确认值；根 errors 及 AbuseIPDB data 失败不采纳默认零；保留真正 0/false；fields、缓存 HTTP/PG 回归为最终入口 | 正式外部来源账户和当前 UI 实机证据；无配置来源保持不可用 |
| #47 | `agent_api::connection` 仅捕获一次 server_time，同时计算 expires_at 并持久化/回应；foundation 实际阻塞 INSERT 跨秒后严格断言 3600 与数据库一致 | 最终提交重新运行该 PostgreSQL 回归，不放宽一小时断言 |
| #58 | 原生 `crates/tcp-probe` 与 `adapter-tcpquality`，构建/签名采用 `tools/build-tcp-probe.py`、`tcp_probe_artifact.py`；没有引入未获许可的上游脚本/rootfs | 已登记原生 TCP 的签名安装、真实启动、断连恢复及取消整链；不把单元测试或来源审计当作此项完成 |

源码已覆盖和开放 issue 的完整验收是不同状态。不得仅因本表或常规测试通过而关闭尚有实机条件的 issue，也不解除完整 NodeQuality 门禁。

最终整合后的本地测试与剩余条件见[统一验证记录](issues-batches-validation.md)；本文件未测试的描述保留为批次提交时点的状态。
