# 真实注册 Agent 的日常诊断故障矩阵

2026-10-01，在专用 Debian 12 虚拟机上，以真实面板、PostgreSQL、普通注册模式 Agent、已验签 r8 制品和 systemd 跑完七个日常检查场景。此项补充 P0 保护与共用任务生命周期的实机证据；完整 NodeQuality、旧客户端业务迁移及 TCP 接入能力仍未签收。全部 full 门禁保持，未正式签名、发布、部署生产或运行暂停的 CI。

机器为 2 CPU、1536 MiB、8 GiB、无 swap 的独立虚拟机，没有生产服务或宿主共享挂载。小内存场景通过真实 Agent cgroup 的有效可用内存触发，不能写成已在 447 MiB 物理配置上完成全部场景。磁盘场景只在本虚拟磁盘分配有界测试文件，始终保留超过 1536 MiB，结束立即删除。

## 固定输入与真实运行路径

| 输入 | 受验身份 |
| --- | --- |
| 源码 | `b0869eff88254c7dc4b770be13c66cd569bee045`，本项只追加验收文档 |
| 源码归档 SHA-256 | `2c309b8893b7a319ee42e5c377e3dc107e5fde1980887cdde0cf14ddf626dabd` |
| Cargo.lock SHA-256 | `02e3ac8f8a90c522b1267049cce2ed27fdd819bd0ab86e4d25c75581cb3bc948` |
| Agent ELF SHA-256 | `5df77631e3807bd3ccdabf9dd47748c862b1a0bd680be2606a1dbc5920bfc25b` |
| 面板 ELF SHA-256 | `99b7414a4eca7714d2b667c99f828fd284f92ebccb7d094ec0aa6b15a9b850b1` |
| NodeQuality | `a92fca6c0067df29ddd03fdc2fee6f3000f64545-r8`，arm64 包 SHA-256 `c2e7a766be030d3c71704a7a69b25839dbe34f1dba88e7d44a54b8bc54b346f3` |
| 代理运行时 | 原生 sing-box 1.14.2 arm64，SHA-256 `fee83ca8457c94449dd04aa17a51830cbc9b449a4dda290e995d3366188e0302` |
| 编译与数据库 | Rust/Cargo 1.97.1；PostgreSQL 15.19-0+deb12u1；锁定依赖补齐后 Cargo jobs=1、无网络构建 |

使用 `scripts/ci-test-trust.py` 输出的公开 TEST_ONLY 根编译真实二进制；不引入可配置生产签名密钥。原 `tools/build-nodequality.sh` 通过十份 canonical 文件的本地传输镜像打包，原 SHA/大小校验保留。`tools/ci-release-fixture.py` 为真实包签测试证明，放入隔离面板数据目录的发布库存；面板与 Agent 均走原签名验证逻辑。**本轮是文件库存准备，没有验收 GitHub Release 导入 API。** 没有替换服务管理后端或用假版本脚本冒充 NodeQuality。

独立数据库经原面板迁移初始化；HTTP 管理接口创建服务器、签发注册令牌、设置采样和拨测目标，真实 `sinan-agent enroll` 后运行普通 `run`，没有使用 `--monitor-only`。采样 1 秒、上传 3 秒，关闭公网 IP 自动发现和自动升级。面板、Agent 与签名制品地址使用同一回环 origin，不绕过 TLS 验证。

日常参数为 `mode=daily, ip_version=ipv4, network_mode=low, upload_report=false`。四个管理员配置的自有回环 TCP 目标填满监听队列，使每次连接按原代码超时；每目标 4 次、每连接最多 1 秒。报告明确显示连接失败/超时和未知延迟。没有执行上游硬件、IPQuality、rootfs 或外部测速。

独立 sing-box HTTP 代理持续转发回环 HTTP 内容，每次核对完整 32768 字节及 SHA-256。它与被测 Agent 同机常驻，但没有通过面板发布代理业务，因此本轮不证明订阅、授权、套餐计量或活跃流量警告的端到端行为。

## 七个独立场景

| 场景 | 实際故障与断言 | 任务状态 / 完整度 | 场景内代理请求 |
| --- | --- | --- | --- |
| 正常完成与重复提交 | 两个 full API 均 409；共用 API 创建 daily，运行中旧 API 重复创建 409；环境和网络章节分别持久化 | succeeded / complete | 174 成功、0 失败 |
| 等待设备确认取消 | 运行中已有环境章节后停止 Agent；取消 API 返回 202，期间两次读取均 cancel_requested 且未确认；恢复同身份 Agent 后清理确认 | cancelled / partial，仅环境章节保留 | 112 成功、0 失败 |
| Agent 重启 | 原 SQLite、配置、身份重开；诊断 PID `14460` 与单调启动时间 `11323816657` 重启前后相同；环境章节原文、版本和采集时间保持 | succeeded / complete | 118 成功、0 失败 |
| 面板断连 | 面板进程停止约 20.66 秒；Agent 离线完成，将 1 个章节和 1 个结果放入真实 SQLite outbox；同数据库面板恢复后补传，已有章节保持 | succeeded / complete | 257 成功、0 失败 |
| 启动时低内存 | Agent MemoryMax 临时 256 MiB；真实预检报告有效可用 247 MiB，小于 daily 64 MiB + 256 MiB 预留；无测试单元启动 | failed / empty | 78 成功、0 失败 |
| 工作目录磁盘不足 | 有界真实分配后预检报告可用 1791 MiB、至少需要 2048 MiB；无测试单元启动；随后删除专属分配文件 | failed / empty | 75 成功、0 失败 |
| 运行中内存保护 | 先通过预检并保存环境章节，再收紧 Agent cgroup；有效可用 78 MiB 低于 128 MiB 阈值，真实监控停止测试；恢复 512 MiB 上限 | failed / partial，仅环境章节保留 | 78 成功、0 失败 |

运行中的诊断单元读回 `MemoryMax=67108864`、`MemorySwapMax=0`、`TasksMax=32`、CPUWeight/IOWeight 10、OOMScoreAdjust 500。Agent 常态 MemoryMax 512 MiB；代理 96 MiB；两者 CPUWeight/IOWeight 200、OOMScoreAdjust -500。资源故障只调整测试 Agent 的限制，不修改诊断配方。

创建成功的运行场景均额外检查跨新旧入口重复提交 409。每个实际启动的任务 journal 只有一次启动记录；取消/保护停止后，主 PID 消失、cgroup 无进程，遍历存活进程 mountinfo 未发现任务相关挂载。日常检查本身不创建 rootfs，因此这一结果不能替代完整工具链的 rootfs 清理验收。

## 心跳、连续业务与收尾

观测使用服务器的 `last_heartbeat_at` 和 `metrics_sampled_at`，没有用 `last_seen` 代替心跳。正常、低内存拒绝、磁盘拒绝、运行保护场景观测到的真实心跳最大间隔均为 20 秒。主动重启 Agent 的场景另列故障窗口。

面板主动断连场景的观测心跳为 `1790836264 → 1790836310 → 1790836330`，最大间隔 **46 秒**；面板恢复后约 14.2 秒才再次记录心跳。它证明断连后的恢复与补传，不能表述为断连期间心跳未中断，也没有放宽离线阈值。各场景原始时间、故障窗口、结果摘要见 [机器可读证据摘要](evidence/registered-nodequality-daily-r8.json)。

2026-10-01 14:25:34.858–14:33:42.588（UTC+8）代理共完成 **1917 次、62,816,256 字节，0 失败**，最长单次约 59.3 毫秒。该总数包含准备、失败的验收脚本尝试及场景间等待，不能与表中场景数重复相加。代理 PID `11933` 全程未变、NRestarts 为 0。SSH PID `406`、NRestarts 0、boot 身份及无 swap 状态保持。

保留部分运行时段的 `systemd-cgtop`、所有场景逐次 cgroup/API 读回、实际单元 journal、磁盘快照和内核日志；不声称 cgtop 覆盖准备至收尾的每一秒。从首次 Agent 心跳所在秒（1790835933）至收尾的内核日志没有 OOM / killed process 记录。全部七项完成后，停止专属流量生成器、代理、目标、Agent、面板和 PostgreSQL；无测试进程或任务挂载残留，数据库与证据保留但不运行。收尾可用磁盘 2,614,874,112 字节、内存 1,313,296,384 字节，默认 PostgreSQL 集群仍停止。

## 原始失败也保留

1. 首次离线构建缺 `allocator-api2 0.2.21`，退出 101；按 Cargo.lock 补齐依赖后重试。
2. 768 MiB 构建 cgroup OOM，systemd journal 明确记录 `oom-kill`，监督进程未写出成功收据。其后的 1 GiB 离线构建成功，memory.events oom/oom_kill 为 0；构建预算没有用作日常诊断预算。该构建失败发生在上述运行验收时间窗之前，不隐去也不归为诊断通过。
3. 第一次正常场景脚本错误地要求 oneshot 服务必须为 active，实际执行期间为 activating；产品任务完成，但该脚本判定失败。修正状态判断后正常场景重新完整通过。
4. 第一次取消脚本停止临时 Agent 单元后，错误地直接 `systemctl start` 已被回收的单元，恢复失败。恢复同一真实二进制/配置后旧任务确认取消，但此时网络章节已完成，不算通过“部分取消”；修正临时单元恢复方式后以新任务通过该场景。没有改动产品代码或放松结果断言。

七项计数只包含最终明确通过的场景。原始失败、脚本各版本、stdout/stderr、库存和来源元数据保存在受限本地证据包，共 110 个索引文件，不包含管理员口令、设备身份密钥、数据库数据文件或环境凭据。证据摘要不公开原始私有日志。

| 收据 | SHA-256 |
| --- | --- |
| 总验收 receipt.json | `c85399d9efffe3753d2d6e61ac8702a2d9537310237eefb078f55308a55534cb` |
| 最终在线状态 | `92a6c885f6ea34d307db032acec32fc357dbba4e0a2781bdd42f3fd0d9e63184` |
| 停止后状态 | `c1dcd1fb80c19741b0d1877570d6b296a791c6fdf836119a7769b07b658703b7` |
| 110 文件索引 | `59a634339c2414bc139fa365f8bce14232519eb4ab075b1edb1b5ce4d853b83c` |
| 私有证据包 | `c129fb40eb3411f5c8a2d75445548c1fa73ada4c1bd85184f9952143313a891b` |

## 尚未验收

本轮没有调用公网查询供应商，403/429/超时与缓存的既有测试仍按 [逐源错误](ip-provider-errors.md)、[缓存](ip-provider-cache.md) 单独归属，不能改记为本次实机通过。未执行硬件完整负载、Geekbench、完整 rootfs/二级工具、流媒体解锁、无外部上传的抓包证明、旧订阅客户端迁移或新 TCP 插件整链。

完整链的授权、固定来源和副作用缺口继续由 #28、#65、#66、#82 跟踪；本项不关闭这些 Issue，也不解除 [完整门禁](nodequality-full-start-gate.md)。按 [整改顺序](ordered-remediation.md) 继续逐项独立签收。四个工作流继续保持暂停，未把本地通过记成 CI 通过。
