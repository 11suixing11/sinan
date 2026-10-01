# 开放问题统一修复与验收边界

2026-10-01 用户要求核对全部开放 issues、检查 PR、完成修复后一次提交 PR，中途不测试。本轮从 `74b403c1365cfee157c2d191bd6255f7bc146aff` 隔离分支开始；初始开放 PR 为零。实现阶段仅阅读、修改代码及编写回归，所有本地验证在整合冻结后统一执行。远端 CI 继续暂停，不签署、发布或部署正式制品。

## 本轮源码修复

| 问题 | 本轮处理 | 仍需的证据 |
| --- | --- | --- |
| [#63](https://github.com/theLucius7/sinan/issues/63) | core 周期拨测复用原调度与历史，新增线路、地区、地址家族和绑定目标的授权记录；未配置、已同步撤销、已知过期均不执行。管理员表单需明确授权，不预置公共测速地址。 | 正式兼容 Agent 发布后，操作者自有或同意目标的各平台持续监测验收。 |
| [#24](https://github.com/theLucius7/sinan/issues/24)、[#122](https://github.com/theLucius7/sinan/issues/122) | r19 日常诊断从节点使用操作者本地私有凭据调用 Ipregistry 与 DB-IP 固定官方 self API；正式节点来源、面板聚合入口、面板 AbuseIPDB 分开。缺配置、拒绝、限流、超时与字段错误均未知，旧成功报告保留。 | 实际运营者正式凭据的节点访问与费用/授权；流媒体正式认证适配与实机证明。未恢复网页临时 key、公共 Cookie 或固定授权材料。 |
| [#28](https://github.com/theLucius7/sinan/issues/28)、[#65](https://github.com/theLucius7/sinan/issues/65)、[#66](https://github.com/theLucius7/sinan/issues/66)、[#82](https://github.com/theLucius7/sinan/issues/82) | r19 候选制品自身在写工作目录或调用上游工具前拒绝新 full；禁在线 rootfs/nexttrace 回退；构建检查并嵌入 daily/历史回收准入说明。独立静态库存工具只读有界归档，可核对固定摘要与逐执行文件声明，不展开、挂载或执行；声明相符仍不授权完整执行。 | 完整依赖图、rootfs 可复建来源、Ookla/Geekbench 精确版本权利、所有第三方上传与宿主副作用和完整故障回收仍缺。两架构已有历史静态盘点，本轮不冒称重新扫描。 |
| [#3](https://github.com/theLucius7/sinan/issues/3) | 已有标准版本选择保持；验收驱动新增双端实际版本、设备进程替换与配置版本/独立运行时不变检查，拒绝假升级。 | 标准流程真实旧版→新版、同设备身份/文件、独立运行时 PID/配置摘要、账本与流量连续性的独立实机验收。 |
| [#130](https://github.com/theLucius7/sinan/issues/130) | 既有显式启用/初次空配置/签名安装状态保留。验收驱动读取真实安装状态，明确失败即保留私有证据并结束，待发布的新配置不能被认作健康安装完成。 | 专用 Debian 12 首次签名安装及故障/丢 ACK 的真实全链证明。 |
| [#6](https://github.com/theLucius7/sinan/issues/6) | 失败证据补本地 DNS、预传输及观察阶段；不增加超时、不重试，保留原失败码、载荷与账本检查和两次公开白名单。 | 原公网间歇卡住根因仍未知；需同环境客户端、Reality 目标与服务端对照。本地替身不能证明公网已修复。 |
| [#58](https://github.com/theLucius7/sinan/issues/58) | 保持独立 Rust TCP、SDK-only、固定六文件完整签名来源及有限生命周期；新目标来源受当前有效拨测授权过滤。未引入无许可的上游 TcpQuality。 | 固定 b562 分发不自动变成本轮引擎；新可信包安装、启动、重连、取消清理及报告上传整链与前置实机签收待完成。 |

源码补修、隔离本地验证、真实机器签收与正式发布分别记录；此 PR 不用自动关闭语句关闭仍缺外部证据的 issue。r19 的构建/pack 严格检查准入记录、17 份固定原始来源及各政策/正式 IP helper 的身份；不自动调用静态库存工具，也不把匹配的归档声明或许可证文本转换为权利证明。无库存声明只产生观察结果并返回未登记状态，声明精确相符也保留 `full_start_allowed=false`、`rights_verified=false`。准入记录内嵌于 runner，正式签名时才随制品进入签名覆盖；本轮源码与 TEST_ONLY 本地证据不代表 r19 已正式签署或发布。旧 Started 任务仍按保存的精确版本回收，不要求旧 binary 新增准入清单或重新运行 r19。

## 周期探测的迁移

原保存 JSON 不改写，既有历史保留。旧配置缺明确目标授权时停止执行，补登记来源、范围和真实到期后再启用；变更目标、方式、端口、地区、线路或地址家族应创建新目标，避免旧历史被改向。旧 Agent 无法校验离线缓存授权/期限与家族，因此面板给其禁用的旧格式配置；支持授权检查的新 Agent 还在源码候选阶段，本 PR 不发布它。匿名看板仅显示授权是否生效和非敏感地区/家族，不公开授权来源与范围。断连不能立即收到新的撤销：沿用最多 24 小时配置缓存并严格执行已知到期；不声称远端即时停止。新建 NodeQuality daily/TCP 快照只接受无到期且自动家族的目标，避免旧协议丢失精确截止或家族。

## 同期 PR 整合

实现期间出现 [PR #136](https://github.com/theLucius7/sinan/pull/136)，作者正常合入主线 `4b4ee6e3342443c2a027e89fda3627ff2c16849b`。本分支普通合入该主线，保留作者的运维、命令取消、订阅和混合链路实现。混合路径已包含来源节点的不可变版本、有序内部/最终跳、独立公开入口、候选验证/切换及持久恢复屏障，已超过旧的仅设计阶段；缺能力仍等待兼容 Agent，不降格直连。原生 HTTPS HEAD 与本地回环证明仅对应受测时刻和目标，不能认证真实机场中段、UDP、持续可用或生产入口单次计量；作者已有隔离证明也不替代本聊天最终整合输入的验证。

静态审查发现新运行时迁移与已有安装迁移重复使用 `0023`；保留原 `0001`–`0023` 原字节，新五份迁移顺延为 `0024`–`0028`，补真实 PostgreSQL 旧安装 schema/历史命令升级与重复迁移回归源码。命令清理失败后的退役屏障、无法等价的订阅 HTTP/HTTP2 传输转换也纳入本次统一修复。首次统一前端验证还揭示作者主线的资源路由/节点表整合缺口与服务器概况未挂载运维入口；本分支补齐统一直连/链路页、有序编辑器、资源详情路由和已启用插件的运维入口，保留有限服务器路由与授权人工确认。首失败日志保留，补正后的精确源码、产物映射及最终验证结果在末节记录。

## Reality 与原地升级操作

`scripts/e2e-driver.py install --refresh --agent-version <已校验兼容版本>` 仍只保存标准私有安装描述，不能在当前机器执行它。操作人完成独立节点安装后，先 `ready --agent-version <版本>`；流量暂停、outbox 排空建立 `verify --label before --status-command '<只读状态命令>'` 基线。升级后使用：

```text
verify --label after-agent --unchanged-from before --agent-version <新版本> --agent-restarted --status-command '<同一设备只读状态命令>'
```

PID 替换只证明所观察 Agent 进程变化；仍需独立运行时 PID、设备身份文件及配置实际摘要对照，不把版本字符串当作制品验签。本地 CI runner 的 TEST_ONLY 制品/流量目标不替代生产签名或公网真实验收。

curl 的 `time_connect` 对 SOCKS 请求描述到代理的 TCP 连接；`time_pretransfer` 包括传输前协商。新增阶段只记录观察进度：尚未连接本地代理、已连接但传输前未完成、准备传输但未收到响应、已开始接收响应、完成或未知。不能据此把超时归因到远端某一跳。[curl 官方字段说明](https://curl.se/docs/manpage.html#-w)。公共证据不包含地址、URL、配置、令牌、日志或报告正文。

## 统一最终验证

全部实现结束后才启动统一最终验证；随后修正最终验证发现的问题并复验，没有按 issue 逐项运行中途检查。

- 完整 Rust/PostgreSQL 冻结输入 `d50ca445917623ea5461a090222bf3b992847ebd`：`cargo test --locked --workspace --all-targets -- --test-threads=1` 的 80 个结果组共 **599 通过、0 失败、20 条件忽略**。忽略项要求专用 Linux/systemd/root、平台 ICMP、固定官方 sing-box 或 Pebble，不算通过。旧 schema 安装状态/历史命令升级、重复迁移、真实进程组退役清理、授权期限/撤销、订阅传输及缓存版本回归均包含在这轮实际结果中。macOS umask077 原子写/链接专项 1 项通过，属于重复检查，不叠加到 599；workspace 全 targets Clippy、fmt/core 分层检查均通过。只使用本任务 `127.0.0.1:55432` PostgreSQL，已停止。
- 最终前端输入 `2cdb7ce26499b547d17c5f375c1c1232efa0d61e`：Bun **45 通过、970 断言**，TypeScript 两次、Vite 两次成功，115 模块生成的 **19 个 dist 文件逐字复现**；全部 **22 套实际 Chromium** 通过。完整 Rust 轮次验证同一份真实嵌入产物。浏览器全部使用私有回环 API 替身，不认证真实设备或机场链路。
- 最终脚本按各套件有效结果去重：**308 个唯一方法，299 通过、9 方法条件跳过，另 1 个 setUpClass 跳过，0 失败/错误**。实际 macOS PowerShell 7 的 15/15、节点官方 IP helper 的 12/12 均包含其中；前者不认证 Windows PowerShell5.1/UAC/ACL，后者使用 TLS/API 替身及真实回环/进程边界，不调用正式账户。固定 17 份官方缓存共 687,969 字节逐项匹配摘要；9 套真实 Bash3/Bash5 与 Netflix Bash5 另有 166 方法执行（150 完整方法通过、15 方法跳过、1 方法含10子用例跳过），只作为可移植性重复证据，不加到唯一方法总数。Bash5 的 90 项全部通过。actionlint 通过，未触发 GitHub Actions。
- 首次 TypeScript 和浏览器失败揭示的主线产品整合缺口已补正并重新构建及全部浏览器复验；库存元组、driver 共享快照循环引用、runtime_operations 缺显式启用均为夹具错误，修正后保留原拒绝门禁和负例。派生 bootstrap 先前仍内嵌 r18，已按当前 r19 源重生并验证；PowerShell 派生产物逐字未变。首失败日志及各独立补验保留，不把首轮失败算成通过。

耐久私有证据分别为 `issues-final-rust-repair1-20261002/proof.json`、`issues-final-web-20261002/summary.json`、`issues-final-python-summary-20261002/proof.json`；原失败位于 `issues-final-rust-20261002`、前端的 `build-32a8c32-first` / `browser-all-first` 及 `issues-final-python-20261002`。最终合入树与这些输入按文件摘要对应，后续只新增本文与进度记录；旧504/473等结果不覆盖本轮输入。外部权利、正式凭据、真实节点、多平台及公网验收仍按上表保留；公开 Release 仍为 agent-v0.3.0，本轮不正式签署/发布/部署候选 Agent0.3.1 或 NodeQuality r19，CI 继续暂停。
