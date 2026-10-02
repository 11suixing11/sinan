# ADR 0063：收集实际 Debian 诊断输入

> 主线整合编号 0061；作者原独立分支编号 0046。旧主线同号决策及链接保留，历史验收仅认证原冻结输入。

状态：实现及契约验收完成；实际 Debian ARM64 包/源码材料取得完成，builder、完整构建与验机待验。日期：2026-10-02。

## 问题与决定

[ADR 0043](0043-nodequality-offline-rootfs.md) 的准备、构建和导出接口只消费已经取得的输入锁与缓存。自有夹具证明接口行为，不能提供真正的 Debian 包、对应源码或签名索引。新增独立的收集器，取得这些实际材料；提供真实候选 builder 时产出原 schema 1 的候选 `inputs-lock.json`，否则保存未绑定材料，不再人工填写依赖或摘要。

收集与构建分别执行。在线部分只访问固定的官方 Debian Snapshot 导入列表、InRelease、索引及已认证的包/源码地址。依赖选择交给 Debian APT，在独立网络命名空间中使用本次已认证的本地镜像求解；不执行包安装、维护脚本、诊断、在线 shell 或 rootfs 构建。收集不会调用 `prepare`、`build` 或开放完整验机。

## 固定来源与闭包

每个请求明确给出架构、构建时间与 main/security 的固定时间、仓库和 suite；两个架构分别收集。保存 Snapshot 实际导入列表的原始响应，要求请求时间恰在列表中，拒绝网站自动退到更早快照的替代结果。导入列表身份与包认证分别记录，不能把 HTTPS 返回或 Snapshot 文件索引当成 Debian 签名。

认证顺序沿用 `InRelease → Release 中 SHA256 → Packages/Sources → 实际缓存字节`。使用原 Debian 12 指纹范围、签名时间和摘要条件，明确提供独立取得的 keyring 及来源说明。来源说明是调用者的材料，不是收集器自己颁发的信任证明。

有界索引读取按单个 Debian control 段落展开，不把整个 Packages/Sources 同时复制成字节、文本和字典。收集器仅保存已选包及其对应源码，最终来源复核也逐段匹配所需身份；压缩输入、展开总量、单段大小和 deadline 均保留限制。这使收集流程可以在小内存专用节点上受控执行，实际峰值仍以冻结后的收据为准。

APT 的初始配置由私有 `APT_CONFIG` 指定，清空宿主配置目录、preferences、dpkg 状态和架构继承；只使用本次 `file://` 仓库。显式选择固定 main 索引中的 Essential 包、APT 与原工具库存，关闭推荐包和重试。APT 处理虚拟包、版本约束、Pre-Depends 和替代依赖，收集器只将最终 URI/版本/架构映射回已经认证的索引，不能用手写递归依赖解析取代求解。

native 和 all 包一起入锁。每个二进制包按其 `Source` 名称、版本寻找对应的签名 Sources 记录；不得假定安全仓库的二进制源码必然也在同一仓库。源码文件库存须完整包含该记录中的全部 SHA256 项，保留 `.dsc`、原始源码与 Debian 修改，不能只收一个 tarball。

main 与安全仓库的 pool 路径分别是 `pool/main/` 和 `pool/updates/main/`，按该记录所属 archive 精确限定；不能把安全更新误拒，也不能扩大为任意 pool/组件。路径仍须与其签名 Packages/Sources 原字段完全一致。

## 有界工厂操作与证据

只创建本次新的私有输出目录。下载使用流式单文件限额、显式总字节上限、磁盘保留量和整体期限；阻塞的网络工作进程也属于有界子进程回收范围。失败、取消及容量不足不得覆盖旧缓存或输出成功收据。工厂缓存及求解文件所需空间与设备的 256 MiB 外层制品预算分别核算，原设备预算保持。

在认证索引、APT 选择与完整对应源码闭包已确定后，正文下载前持久化有界容量计划；失败记录关联它，使容量拒绝能够说明实际已选材料需求。计划的引用总量、内容去重量、已用元数据与求解开销分别记录；原保守总文件预算保持，不将估算或规划升级为缓存正文认证。

失败下载记录实际观察的响应、错误阶段及预期/实际正文身份，退出临时目录前尽力保留有界 worker receipt 和选定响应头，仍受原预算与期限限制。缺失记录明确未知；失败正文不持久化，不以重试或放宽来源规则掩盖原错误。详细字段及已证实的旧证据丢失见[收集说明](../nodequality-rootfs-collection.md)与[后续闭包记录](../acceptance/complete-debian-materials.md)。

保存实际导入响应、签名原文、压缩索引、包与完整源码、求解配置及输出，以及实际收集工具的字节身份和系统包来源。最终收据绑定候选锁及缓存，区分认证的 Debian 输入与候选 builder 的外部声明。

候选 builder 字段与原 schema 兼容，但收集收据明确 `builder_approved=false`、`full_ready=false`、`reproducibility_verified=false`。从当前运行环境读到三个工具的字节，或者把候选 image 摘要再传一遍，都不能证明实际虚拟机来自已审批的完整镜像。后续准备、原生构建、双架构复建和正式签名仍使用 ADR 0043 的条件。

没有候选 builder 时只发布 `materials.json` 与 `unbound-inputs.json`，后者记录 `builder=null` 和 `lock_ready=false`，不是原构建器可消费的输入锁。`bind` 在新的私有目录重新认证已有材料、完整实际导入时间对及候选工具，再生成严格 schema 1 锁。`validate_materials` 和 `verify_authenticated_sources` 仅认证没有 builder 字段的材料；原 `validate_lock` 和 `verify_inputs` 仍分别要求完整 builder 与先行独立审批，不允许未绑定材料进入 prepare。

## 验收范围

整步修改结束后才执行收集器契约验收与实际官方输入取得。记录原始失败、修复和受影响范围的补验，不穿插测试。实际取得和签名认证材料只证明该次输入收集；没有真正执行的原生构建、完整 NodeQuality、持续 Agent/sing-box 负载及故障矩阵继续待验。

本步冻结后的[验收记录](../acceptance/debian-inputs-and-singbox-snapshots.md)保留 92 个不同 Linux 契约方法通过及实际来源失败。真实 ARM64 收集通过签名元数据与隔离 APT 求解，但完整闭包超过本次 900 MiB 总材料预算，在包/源码正文下载前拒绝；未生成完整输入锁，不将此部分结果写成来源取得完成。

后续[完整材料大步骤](../acceptance/complete-debian-materials.md)在正文前发布有界容量计划，并修复 worker 失败细节在临时目录清理时丢失的问题。最终冻结的 33 个不同 collector 方法通过；新的独立 ARM64 实际收集取得 237 个 deb 与 539 个完整对应源码文件，776 个正文独立 Size/SHA256 复核全匹配。实际限额触发、无 OOM、清理及身份结果分别记录，原失败原因继续未知。此次只完成 Debian 材料收集，仍为未绑定输入，不能替代 builder 认证、原生构建或完整诊断。

Geekbench、Ookla 和其他原工具的许可及能力缺口保持明确，不以 Debian 开源基础库存替换原完整验机目标。操作接口、所需容量和输出见[收集说明](../nodequality-rootfs-collection.md)。
