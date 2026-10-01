# 完整 Debian 输入闭包取得

2026-10-02。本大步骤以 `0548e0c3bc2f13c99b8940d4332372fc1d3ae65f` 为基线。初始容量计划版本已冻结验收，实际下载暴露了错误详情丢失；完成该缺口修复、重新冻结后仅补验受影响范围，不改写首次结果。

前一步的[实际材料收集记录](debian-inputs-and-singbox-snapshots.md)只完成签名元数据与隔离 APT 选择，900 MiB 总材料预算在正文下载前拒绝。本步先按保留的签名 Sources 和原选择清单盘点完整闭包，再为收集器补齐正文下载前的容量计划和失败关联；原缓存与失败收据保持。

只读盘点得到 237 个二进制包、98,800,644 字节，以及 168 个对应源码版本的 539 个文件、833,717,286 字节。正文合计 932,517,930 字节；结合原拒绝前元数据 19,173,315 字节，原预算至少缺 7,972,845 字节。这是既有材料的盘点，不是重新取得或认证正文。

本次在同一专用 Debian 12 ARM64 环境执行新的独立收集目录，显式材料预算为 1 GiB、guest 磁盘预留 512 MiB、宿主容量守卫 2 GiB；保留 256 MiB MemoryMax、零 swap、64 TasksMax、低 CPU/IO 权重及有界期限。不扩大虚拟机、不清理历史材料、不在生产节点执行。先完成实现、回归代码与说明，冻结后只验收受修改影响的 collector 和本次实际闭包。

## 初始冻结与原始失败

550 个功能输入的初始冻结摘要为 `af4d82ebecf2ff45ccd6026f717539776d44432ec92e520729a18f96f178474c`。仅 collector 及对应测试相对基线改变，其余 548 个输入和 19 个 dist 保持。专用 Linux 契约 28 项全部通过，0 失败、0 跳过，耗时 2.922 秒；内存峰值 64,655,360 字节，所属进程、挂载、cgroup 和临时目录收尾通过，无 OOM。

真实收集仅执行一次，90.424 秒后以 1 退出：8 个元数据对象及 30 个 deb 取得，30 个正文合计 14,632,904 字节，独立流式重算摘要全部匹配。完整正文预期 776 个，仍缺 746 个，源码正文尚未取得；没有完成的 `materials.json`、`collection.json` 或 `unbound-inputs.json`。

本次容量计划为 225,838 字节，`required_owned_bytes_before_payloads=951917150`，小于 1 GiB，admission 明确允许。真实 APT 的 lists 为 51,869,835 字节、mirrors 为 9,328,425 字节，plan 阶段 solver 合计 61,264,871 字节；这些是阶段采样，不是连续峰值，派生目录在正文前已删除。

失败发生在固定快照 `gzip_1.12-1_arm64.deb` 的 worker，记录仅为 `response_invalid`、未知 HTTP 状态及父层通用错误。完整 stderr、unit journal 和保留目录的只读取证确认：具体 worker ValueError、失败 receipt 和响应头已经随临时目录丢失，无法从原始材料恢复，不能猜测为 DNS、TLS、403 或 429。已记录为 [Issue #141](https://github.com/theLucius7/sinan/issues/141)，归入原整改 milestone；后续修复不能倒填首次 HTTP 状态或原因。

真实单元内存峰值 160,206,848 字节、最多 12 tasks，无 OOM。宿主守卫 19 个采样最低 6,288,674,816 字节，没有触发。进程、挂载、cgroup、临时目录、源码/trust 身份、boot 与 SSH 单次基线收尾全部通过。原 30 个包、失败及容量计划保留在独立 guest 目录，没有重试或删除。

## 最终冻结与受影响范围

错误证据修复后的 550 个功能输入冻结摘要为 `84bf2bdedc099afafdae81be703490bfcf16e53943d9cf624b2f3e8252dada2a`，仍仅 collector 及对应回归相对基线改变，19 个 dist 不变。最终专用 Linux collector 33 项全部通过，0 失败、0 错误、0 跳过，耗时 4.666 秒；26 个契约方法与 7 个实际 owned-worker 方法包含 403/429、截止和 TERM/HUP，新增 206、无效/短 Content-Length、空正文、摘要不符、缺失/截断 footer 及凭据脱敏。初始 28 与最终 33 重叠，不相加为 61。

最终契约单元内存峰值 63,971,328 字节、最多 6 tasks；无 OOM，15 个终态清理及身份检查全部通过。没有重跑未修改的 Rust、前端、其他 builder 契约或 CI。

修复后只对原 gzip 官方地址执行一次独立 `_fetch`：HTTP 200、Content-Length 与已读/已写字节均为 137,556，SHA256 `cf0f383667fb65f0b0fd730198efc67c8c23582542bcb3c2b61f240c62e43ba9` 匹配原签名索引，最终地址仍为官方 Snapshot file 路径。worker 耗时 1.237 秒，内存峰值 26,136,576 字节、最多 2 tasks；11 个终态清理/身份检查全部通过，原 30 个 deb 和失败证据未改变。这仅证明本次单对象取得，不能倒填首次失败原因，也不能认证完整闭包。

完整后续验收使用另一个全新目录。启动前实际 guest 剩余 1,740,902,400 字节，超过 1 GiB 材料、512 MiB 预留和 16 MiB 准入余量之和 113,512,448 字节；宿主剩余 6,245,584,896 字节。仍受运行中的动态预算、期限和磁盘守卫约束；原失效 attempt 不续跑或改写，新的执行结果单独保存。

大文件正文会计入 cgroup 文件缓存，`memory.events.max` 记录接近上限并触发回收的事件，不能当作 OOM 次数。内核也允许短暂超出 `memory.max`；实际峰值、max、oom 和 oom_kill 分别保留，不将 256 MiB 配置值冒称为实测峰值的绝对上界。[Linux 6.1 cgroup 文档](https://docs.kernel.org/6.1/admin-guide/cgroup-v2.html#memory-interface-files)

## 完整 ARM64 材料终态

最终冻结版本在全新目录执行一次完整收集，以 0 退出，耗时 2,444.594 秒，约 40.7 分钟。784 个 HTTP 对象均为本次观察的 200/完整响应，其中 237 个 deb、168 个源码名称/版本对对应的 539 个源码文件全部取得。独立流式复核 776 个正文的签名索引 Size/SHA256 全部匹配，0 缺失、0 不匹配；二进制共 98,800,644 字节，源码共 833,717,286 字节，正文合计 932,517,930 字节。

main 的精确快照为 `debian/20261001T142720Z/bookworm`，安全源为 `debian-security/20261001T142623Z/bookworm-security`。本次导入响应、InRelease、压缩索引及正文分别有字节身份；原始 gpgv 状态中，main 的两个必需 Bookworm 主指纹 `4D64FEC119C2029067D6E791F8D2585B8783D481`、`B8B80B5B623EAB6AD8775C45B7C5D7D6350947F8` 和安全源的 `05AB90340C0C5E797F44A8C8254CF3B5AEC0A8F0` 均实际验证通过。额外已知 archive 签名不代替必需指纹；keyring 的来源审查仍是调用者提供的独立材料，不升级为收集器自身的信任认证。

本次容量计划的保守预计总量为 951,917,166 字节，最终所属目录共 954,536,198 字节，均在 1 GiB 材料预算内。APT update/plan 的独立网络命名空间只有 lo，未安装软件；求解派生目录在正文前删除。阶段空间观察与实际最终字节分别保留，不宣称采样捕获了全过程峰值。

实际单元 `MemoryMax=268435456`、零 swap、`TasksMax=64`、CPU/IO 权重 10、`OOMScoreAdjust=500`、两小时上限与所属进程组清理均读回。内存峰值 269,475,840 字节，短暂高于配置值；`memory.events.max=100704`，表明限额触发过回收压力，`oom/oom_kill/oom_group_kill` 均为 0，tasks 峰值 13。不能由无 OOM 推断没有资源争抢，也不能将收集单元结果替代完整诊断负载验收。

宿主守卫 482 个采样最低剩余 5,239,861,248 字节，未触发 2 GiB 阈值；guest 最终剩余 782,073,856 字节，满足 512 MiB 预留。11 个清理与身份检查全部通过：所属进程、挂载、cgroup、临时目录无残留，单元消失，boot 与 SSH 基线保持，源码/trust 摘要不变，内核没有本次 OOM 记录。大缓存只保留在专用 guest；原失败目录及单源观察未改写。

完成的 `materials.json`、`collection.json`、`unbound-inputs.json` 分别为：

- `e5faa499e780e99b1c0dcd2c5c631f61a5379f320eff90f2cb2cc010bf696771`
- `bbbbbae811b0513a9d4b34d83c87b2b05d6aa22bcc8cbbe8ddc49ee75a3914d7`
- `e013812bf94e7e59af800eaeb4febc491b314910f95a7dfba265e414331425dc`

正文独立库存摘要为 `0d485b41aea288ed461617566eba193b74abff8f9f38efc6b202ee6b73d63d0e`。终态摘要及收据摘要分别为 `76ad7e7839e3a01260f1bb4b409a71545f6f58bfa34dc22cb5676e08c2659ba1`、`dafa210671c70e75ce1f1ee2d354e7b8022f7fde21f1fd3556672012b3cbe907`。完整字段、冻结输入映射、首次失败与本次终态的独立摘要见[机器证据](evidence/complete-debian-materials.json)。

本次完成的是实际 Debian ARM64 包/源码材料：`complete=true,source_authenticated=true`；未绑定 builder，`builder=null,lock_ready=false,builder_approved=false,runtime_image_identity_verified=false,reproducibility_verified=false,full_ready=false`。没有生成可供原构建器消费的完整输入锁，没有执行 rootfs 构建、双架构复建或完整 NodeQuality，也没有取得/认证 Geekbench、Ookla 等非 Debian 原工具或其许可。候选 builder 绑定、完整镜像来源与启动身份、正式制品及 Agent/sing-box 持续业务联合负载均继续待验，aws-jp0 不可达和混合机场链路的设计范围不因本次材料完成而改变。四源码 CI 保持暂停，未签署、发布或部署。
