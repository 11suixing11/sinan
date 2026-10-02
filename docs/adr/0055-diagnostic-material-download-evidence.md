# ADR 0055：诊断材料收集的下载失败证据与工厂容量

状态：源码整合与回归准备；本轮最终测试 pending，未签收真实完整工具链。
日期：2026-10-02。
关联：#141、[ADR 0049](0049-nodequality-offline-rootfs.md)。

## 问题

原生 Debian 材料收集曾在下载一个已认证包时失败。父层只留下 `response_invalid`、未知 HTTP 状态与 `trusted build/verification command failed`；工作目录退出后，worker 的具体异常、阶段、receipt 和响应头也随之删除。这些材料不足以判断底层原因，不能据此推测 DNS、TLS、403、429 或超时。

面板外的材料收集和工厂构建使用不同输入权限：收集器可以访问明确的官方 Snapshot，构建器仅使用已取得、已认证的本地材料。二者都不能替管理员批准镜像、第三方许可或完整验机。

## 文件与整合决定

本步骤整合六个文件：

| 文件 | 责任 |
| --- | --- |
| `tools/nodequality-rootfs-collect.py` | 收集官方元数据与签名链对应材料、固定完整依赖闭包、保存收集及失败证据 |
| `tools/test-nodequality-rootfs-collect.py` | 小型自有材料与真实回环 worker 的错误、期限、预算、取消和回收回归 |
| `tools/nodequality-rootfs-request-amd64.json` | amd64 的明确仓库/导入时间请求，不含虚构包摘要或 builder 审批 |
| `tools/nodequality-rootfs-request-arm64.json` | arm64 的独立请求，不把另一架构验收当作通过 |
| `tools/nodequality-rootfs-build.py` | 来源认证与工厂 prepare/build/export 的容量计划、过程保护和失败记录 |
| `tools/test-nodequality-rootfs-build.py` | 来源、容量、短写、挂载、信号、进程回收及导出绑定回归 |

收集器及工厂接口来自未合入的诊断准备分支，按当前主线窄整合。保留主线 `copy_locked` 的普通文件与 `O_NOFOLLOW/O_NONBLOCK` 检查、FIFO 替换拒绝、复制前后大小及 SHA256 核验；结合 prospective capacity 和短写拒绝。原 FIFO 回归保持原方法。未覆盖主线的 `nodequality_rootfs_artifact.py`，未改 canonical r19、离线准备 r20、正式节点查询 r21、原来源锁与许可库存。

## 来源与容量边界

收集仅允许 HTTPS `snapshot.debian.org` 的明确 archive、timestamp discovery 和 file 路径；不使用环境代理、设备凭据、浏览器身份或公共借用密钥。每次下载有独立 worker，沿用总操作期限、单 worker 最多 180 秒、底层网络读取 30 秒以及每个文件的字节上限。官方重定向继续逐跳核对同一白名单，没有自动重试或因失败补发公网请求。

只接受实际导入列表中的精确时间。使用独立取得且带审阅记录的 keyring，按 InRelease 签名、索引摘要、包及完整对应源码摘要认证。APT 在私有配置和独立无网络 namespace 中，只读取本地已认证索引来选完整依赖，不安装或自行下载包。`main` 和 `debian-security` 各自固定 `pool/main/` 与 `pool/updates/main/` 范围，路径合法不能替代签名索引对具体身份的覆盖。

收集必须显式声明总字节预算，最大 64 GiB、整体期限最大 7200 秒，磁盘预留不得低于 512 MiB。容量计划区分引用字节、唯一缓存正文、已观察到的元数据及 APT 派生目录；在下载二进制和源码正文前保存计划并执行准入。不会用当前可用空间自行扩大预算。

工厂 prepare/build/export 另有容量计划，默认输出上限 4 GiB、可显式设定但最大 16 GiB，最低空闲磁盘 512 MiB、最低预留 inode 1024。过程轮询目录身份、普通文件、成员数、分配字节及磁盘/inode 预留，复制和导出写入前也检查预期增长。轮询不是文件系统配额或独占磁盘预订；外层隔离、配额和并发使用仍须另行验收。设备制品仍保持 256 MiB 总预算。

## 失败证据契约

worker 保存有界的异常类型、具体原因和失败阶段。HTTP 状态只来自已收到的实际响应；未收到响应或可用 receipt 缺失时明确为 `null`，不能从分类、stderr、响应头是否存在或旧成功记录推断状态。HTTP 403/429 保持实际状态与独立分类；DNS、TLS、超时、取消、非预期状态、空/截短正文、非法 Content-Length 与字节超限分别保留原因及已观察到的上下文。

父层失败记录与 worker 观察分开，保留父层类型、原因、阶段、原始及清理后退出码。父层超时和取消明确标注；worker 不可用时其类型、原因及 HTTP 状态仍保持未知，不让父层通用错误冒充底层错误。未完成、损坏或缺失 receipt 都不能生成成功缓存描述。

`failure.json` 的 `failed_download` 关联本次失败记录；`failure_evidence` 以相对路径、大小和 SHA256 关联可用的 worker receipt 与脱敏响应头。响应头只保存固定诊断字段，排除 Cookie、Authorization 和其他非白名单字段；URL 去掉 userinfo、query 与 fragment，错误文本移除控制字符并限长。collector 的下载失败证据不包含下载正文、环境、完整命令参数或原始 stdout/stderr；factory 的既有有界可信构建日志另按来源和容量合同保存。

下载超时或取消后，仅执行最多 2 秒的元数据清理保存，不延长下载期限或增加请求。先删除本 worker 的未认证正文，再检查原总字节预算和磁盘预留后保存最多各 64 KiB 的 receipt/响应头。预算不足、文件损坏或保存失败时，记录证据保存未完成与有界原因，不能称为证据完整。进程组仍由原有 finally 停止并回收；不能只凭临时目录移除宣称所有进程清理完成。

所有失败保留已经取得的认证缓存与失败证据，并撤回本次未完成发布的成功材料、输入锁和成功收据。成功文件以 exclusive 创建，在写入前登记实际设备与 inode；异常仅删除仍匹配本轮身份的文件。已有文件不能覆盖或认领，身份变化时拒绝删除并报告回滚未完成。失败收据自身不能写入时，也保留不完整目录并附加原因，不能因此删除认证缓存。binding 拒绝带 `failure.json` 的不完整收集。不会因缺少 builder 信息而伪造 builder digest：可完成未绑定材料收集，`lock_ready=false`；后续绑定需要明确匹配架构的候选 builder。候选身份、官方 HTTPS、Debian 输入链认证都不能代表独立镜像审批、可复建证明或第三方分发许可。

## 验收与签收

本轮最终测试尚未运行，新增回归仍是 pending；已有另一源码的真实材料与历史回归不能认证当前冻结输入。先在小型自有回环场景验证错误和保留契约、期限内终止、SIGTERM/SIGHUP、预算拒绝与清理；最终发布阶段再注入故障，确认失败不会遗留成功材料或输入锁。随后对工厂保留主线的 FIFO/复制时摘要回归与容量/清理回归，按实际源码摘要记录结果。

这项验收不要求重新下载大型公网材料，不运行 Geekbench/Ookla，不自动接受许可、不恢复 CI、不打开 full。双架构完整合法工具链、真实 builder、完整运行故障矩阵及服务器保护联合验收仍须独立完成。执行记录见[下载证据验收](../acceptance/diagnostic-material-download-evidence.md)。
