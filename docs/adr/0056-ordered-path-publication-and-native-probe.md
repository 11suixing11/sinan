# ADR 0056：有序路径发布与具体出站验证

状态：采用，整步实现与本地集中验收完成；真实 Agent 托管整链与正式部署仍待验。日期：2026-10-02。

本步骤把 [ADR 0040](0040-mixed-chains-and-subscriptions.md) 的完整路径模型接入编译、部署、授权、订阅和管理界面，并确定通用设备验证协议。按照用户要求，源码、迁移、界面、回归代码及文档全部完成后，才冻结输入并统一验收、提交；修改期间不运行测试、构建或格式检查。CI 继续暂停。

## 业务模型与发布

公开入口属于受管服务器，随后是有序的 1 至 8 个受管或外部代理跳。外部节点始终引用来源身份 epoch、节点 ID、不可变节点版本和成功批次；不能把来源当作服务器或从原始配置复制 tag、路由、DNS、控制接口。候选创建时冻结全部输入，发布阶段不重读 latest。`follow_node` 只能跟随同一来源身份内同一明确节点；`pinned` 只接受管理员显式选择同节点版本。获取失败、归档、缺失及歧义保留旧快照和相应错误，不自动换节点或直连。

旧链路首次捕获保留 ID、入口、relay UUID、订阅和原生字节，并标明 legacy，不能补造已经验证的事实。公开入口仅计量终端用户；内部接受身份及 Agent 自查均不进入终端用户统计。部署来源采用认识版本化快照的新读取路径，旧 `Vec<Node>` 历史保持可读，账本不改写。

发布顺序为准备全部受管内部依赖、精确确认其配置与运行实例、准备入口候选、验证候选、切换用户路由、验证切换结果、固定恢复屏障、清理旧接受身份。每次确认同时绑定链路代数、完整依赖向量、部署 UUID、revision、bundle hash、activation、运行实例和请求 ID。超时、断连、迟到结果或旧 revision 心跳不能推进阶段。固定恢复边界后，不得自动回滚到已经撤旧的配置。

探测和屏障请求排队与完整向量关联必须在同一数据库事务中持久化，提交之后才通知设备。插件传入不含业务语义的独立请求 UUID，不能把无法证明关联同一向量的孤立 pending 请求重标为新向量；通用接口保留原请求与原期限重放。发出屏障前再次确认当前完整向量已有切换后探测成功。未知屏障可能已在设备提交，缺少回执或超过期限都不能证明可恢复低代；此时保留新代路由和两代接受身份，等待持久回执，而最低代数仅由真实已承诺结果提高。

重连时有界重发尚无终态回执的过期屏障，UUID、摘要及原期限均不变。设备先重放持久结果，未执行过的过期请求只能返回持久失败，不能重新提交副作用；面板得到终态后停止重播。这样既不把未知当失败，也不会让从未送达的请求永久阻断恢复。

共享服务器上的另一条链路推进可能使已发探测的运行确认失效。通用收据仍严格记录为 `superseded`；插件保留旧请求与回执，仅清除自己的向量关联，等待当前完整向量后发送新的请求 UUID。等待其他请求不接管其 UUID，不推进阶段、不增加共享服务器脏标记，也不把失效证明当作原生网络失败或当前成功。真正的失败、超时和不匹配仍按失败处理。

当前、候选和恢复代共同参与删除与退役保护。删除保留软墓碑、不可变版本和幂等收据；旧接口不能把完整路径降格处理。若受管端点的新旧参数不能共存，拒绝在线修改并说明应使用替代节点或新链路，不先破坏可恢复旧快照。

## 具体出站验证与旧设计的调整

本实现用同一受管 sing-box 的 authenticated Clash API 对指定**具体出站**发起 HTTPS HEAD。覆盖 ADR 0020 原来规划的临时 SOCKS 探测入口及其临时代理认证方案：固定 1.14.2 没有可依据的临时 SOCKS 凭据原生过期能力，不能把 Agent 请求超时当作代理凭据失效。这里不新增可转发用户数据的探测监听或另一个代理实例。

入口的原生配置启用 `127.0.0.1:18086` 私有控制端口，与既有统计端口分开。控制端口冲突拒绝候选，不改已有节点监听。每台入口服务器使用独立随机 256 bit 控制秘密，保存在既有受保护的插件数据库及签名配置中；普通资源、订阅、日志、错误及探测消息不返回该秘密。该秘密是受管运行时管理凭据，不是短期代理用户。它不声明已有应用层加密。

额外签名文件 `runtime-probes.json` 只允许固定 schema、runtime 1.14.2、完整构建 feature tag 以及最多 256 个验证绑定。每个绑定是不可变 probe UUID、具体末端出站 tag 和本面板自有 HTTPS `/health`。不接受任意 URL、用户认证、查询、fragment、目标站点、自由 shell 参数或 selector/urltest 分组。HTTP 本地面板只能用于旧业务及测试夹具，不能签发新路径的真实验证计划。计划须包含 `with_clash_api` 和 `with_v2ray_api` 以及每种承载的必要 feature；Agent 依据实际二进制版本输出及原生 check 验证，代码能力声明不代表实际制品已经安装。

通用 SDK 只接收已应用计划中的 probe UUID；core 不理解链路、机场、用户或套餐。协议增加 `runtime.path_probe.request/result/ack` 和 `runtime:path-probe-v1`。请求绑定精确 checkpoint，正常 60 秒、最长剩余 120 秒；SDK 请求不超过 5 秒，原生 URL test 参数 4500 毫秒。执行与 apply/recovery 共用 gate，前后核对同一 activation、受控进程、配置和无未完成 intent。结果先持久化，再传输与 ACK；重启或断连重放不能续期、换目标或换代，成功探测不推进恢复 floor。原生失败只返回固定脱敏错误。

原生 HTTP 请求只到受保护回环控制端口，关闭环境代理、重定向和重试；控制响应最多 4096 字节。Agent 崩溃后没有临时代理入口可长期存活；已发原生 HEAD 受其 4500 毫秒时限约束。取消或超时不能把结果当作当前候选成功，清理仍依完整版本状态与恢复屏障执行。

固定版本的一手依据：

- [Clash API server](https://github.com/SagerNet/sing-box/blob/v1.14.2/experimental/clashapi/server.go)：TCP 控制监听和 bearer authentication。
- [具体 outbound delay](https://github.com/SagerNet/sing-box/blob/v1.14.2/experimental/clashapi/proxies.go)：按指定 outbound 查找并执行 URL test；失败返回错误。
- [URL test](https://github.com/SagerNet/sing-box/blob/v1.14.2/common/urltest/urltest.go)：经指定 detour 发起 TCP/TLS/HEAD，正常证书验证且不跟随重定向。

成功只证明本次具体出站对指定 HTTPS 目标完成 TLS 与 HTTP 响应；上游没有检查响应 HTTP status，也没有返回实际出口 IP，不能把成功记录写成 HTTP 200、出口证明、UDP 通过或完整链路联合验收。指定最终出口、逐跳经过、中间跳故障无旁路、UDP 和入口单次计量必须用专用原生夹具另行证明。

固定 1.14.2 的 [`with_grpc` 包装](https://github.com/SagerNet/sing-box/blob/af6e64c3b69e6132ebaee0e1a3d24e93903f6709/transport/v2ray/grpc.go) 选择 Google gRPC 实现；[无此标签的包装](https://github.com/SagerNet/sing-box/blob/af6e64c3b69e6132ebaee0e1a3d24e93903f6709/transport/v2ray/grpc_lite.go) 仍提供 lite 实现。该标签与计量 API 的 `with_v2ray_api` 分开：本次 HTTP→Reality 三/四跳流量夹具不含 gRPC 承载，因此不要求 `with_grpc`。产品对规范化 gRPC 承载仍保守要求 Google 实现标签，以保持可配置的 `permit_without_stream` 等语义；lite 原生配置检查通过不能替代这些参数的等价实流证明。

## 依赖与验收边界

sing-box adapter 复用 workspace 已锁定的 `reqwest` 完成有界 authenticated loopback HTTP，无新增库版本或平台命令。复用现有 tokio TCP、自有手写 HTTP、外部 curl 或自由 shell 均不能在更少代码和权限下提供相同 TLS/超时/redirect/body 处理，因此不另引入这些实现。规范化外部类型移到纯编译边界，YAML/URL/来源解析仍留在插件，compiler 和 core 不增加解析器依赖。

本步[集中验收](../acceptance/ordered-paths.md)记录工作区回归、真实 PostgreSQL、桌面与窄屏流程，以及当前 compiler 三/四跳实际流量。完整路径顺序和承载、旧字节兼容、入口单次统计、资源 CAS 与幂等、候选失败、迟到结果、持久重放、恢复屏障和引用删除分别有证据。显式搬移端口的编译图与设备回执夹具不能替代真实新 Agent 托管发布、实际账本事务和多机故障矩阵；这些以及其它条件忽略继续待验，不据此宣布正式制品、生产迁移或整体整改完成。NodeQuality 许可、builder、联合负载及原有门禁继续保留。
