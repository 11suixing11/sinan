# 开放 issue 第一批：#3、#4、#6、#14、#15

日期：2026-10-01。审查基线为主线 `bb9638b`。按用户本次要求每五项提交一次，批次期间不运行测试、Clippy 或构建；全部批次整合后统一测试。源码处理、最终测试、实机能力和关闭 issue 分别记录，本批未关闭 issue、发布制品或部署生产。

| Issue | 基线现状及本批处理 | 本批结论 |
| --- | --- | --- |
| [#3](https://github.com/theLucius7/sinan/issues/3) 安装版本耦合 | `artifacts.rs`、`installation.rs` 已支持显式版本与独立可信 bootstrap，默认由设备平台选择最新兼容稳定版。本批修复验收驱动拒绝 `latest/null` 自动描述、文档错误传入 `--tag null` 的剩余回归；显式选择分别报告未导入已签版本、协议不兼容和平台无可安装制品，新增未执行回归 | 已修复可确认的源码/驱动缺口；最终测试待执行。历史未签 0.1.0/0.2.0 仍按迁移边界处理，不凭本批重新签收原地升级 |
| [#4](https://github.com/theLucius7/sinan/issues/4) CDN/origin 诊断 | 部署文档新增实际设备/独立客户端两种来源及公开 CDN/同域 TLS 直达 origin 的独立检查，列出安装、注册、WebSocket、制品、状态和 ACK 路径。Agent 注册保留具体 HTTP 状态，并针对 401/403/429/跳转给出方向，不回显响应体，新增未执行回归 | 诊断与错误提示已补齐；真实部署 CDN 策略和设备全部路径未在本批修改或重验 |
| [#6](https://github.com/theLucius7/sinan/issues/6) Reality 间歇超时 | 现有 `e2e-traffic-evidence.py` 已保留固定 90 秒、部分字节、原退出码及有限 HTTP/TCP/TLS/负载证据。本批补齐文档的固定预算、逐层定位及失败判定；不修改时间预算或吞掉失败 | **根因未解决**。没有可重复的新现场失败，本批不将历史两次成功或同源码重跑成功当作根因已修复 |
| [#14](https://github.com/theLucius7/sinan/issues/14) 诊断资源预算 | `adapter-sdk/src/resources.rs` 已约束数字预算，`agent-core/src/system/jobs.rs` 已显式传递五项属性及固定 `MemorySwapMax=0`。本批纠正文档仍声称 OpenRC 可以警告后启动的过期描述：新诊断要求可验证的 systemd 保护 | 原源码缺陷已由既有实现覆盖；预算/内核行为最终测试待执行，完整上游联合负载不因本批签收 |
| [#15](https://github.com/theLucius7/sinan/issues/15) 常驻服务争抢保护 | 两份源 systemd 单元与生成的独立 bootstrap 都已含 `OOMScoreAdjust=-500`、`CPUWeight=1000`。本批补齐安装器来源与当前生成同步路径，不人为修改既有权重 | 原单元缺陷已由既有实现覆盖；最终生成一致性待检查，正式单元安装与完整上游争抢场景未在本批执行 |

## 最终统一测试入口

最终整合提交执行格式、Clippy 和完整 Rust/PostgreSQL 测试，保持 CI 暂停。新增回归为 `explicit_installation_distinguishes_missing_protocol_and_platform_failures`、`enrollment_reports_status_and_direction_without_echoing_untrusted_bodies` 及 `scripts/test-e2e-driver.py` 的自动描述/显式升级/非法标签用例。既有版本、身份保留、重定向拒绝、资源构造/持久化和服务属性用例一并覆盖；这些名称是待执行入口，不是通过结果。

Python 部分统一运行 `scripts/test-e2e-driver.py` 和 `scripts/test-e2e-traffic-evidence.py`，生成一致性使用 `tools/render-bootstrap.py --check` 及既有发布工具测试。源码预算真实属性与常驻优先级按[诊断资源预算](diagnostic-resource-budget.md)和[常驻服务优先级](resident-service-priority.md)在专用 Linux/systemd 节点验证；当前 macOS 上条件忽略不能作真实 systemd 通过证据。

完整 NodeQuality 仍受[整改次序及完整执行门禁](ordered-remediation.md)约束。既有有限联合负载、真实日常链路和原 CI 只认证各自记录的提交/场景；本批没有运行完整上游负载、公网压测、真实旧客户端迁移或修改生产配置。#6 根因及这些明确实机边界继续保留待完成。

最终整合后的本地测试与剩余条件见[统一验证记录](issues-batches-validation.md)；本文件未测试的描述保留为批次提交时点的状态。
