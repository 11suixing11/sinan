# ADR 0065：NodeQuality 离线工厂的容量与失败证据

> 主线整合编号 0065；作者原独立分支编号 0048。旧主线同号决策及链接保留，历史验收仅认证原冻结输入。

状态：实现及容量/回收验收完成；真实完整构建及镜像条件待验。
日期：2026-10-02。

## 问题

现有离线工厂已经提供 `prepare`、原生 `build` 和 `export`。实际 ARM64 Debian 材料已取得，见[完整材料记录](../acceptance/complete-debian-materials.md)；来源完成不代表 builder 已审批，也不代表 rootfs 已构建。

当前准备流程会复制完整 input-cache，再复制构建所需的本地 mirror。按该次已成功 `materials.json` 中的声明计算，去重 input-cache 为 951,537,693 字节，mirror 副本为 117,876,325 字节，合计 1,069,414,018 字节，尚未包括输入锁、收据和目录开销。这是既有认证材料的容量推导，没有重新下载或认证正文。该次专用 guest 最后记录的剩余空间为 782,073,856 字节，仅准备副本就不能在保留原缓存的前提下容纳；该历史读数不是本步骤的实时准入证据。

此外，`mmdebstrap` 退出失败时，原 `run_bounded` 丢失已经捕获的输出，`build.log` 只在成功后写入，失败输出目录随后被清理。实际工厂第一次构建失败必须能够定位阶段、退出状态和原始有界输出，不能只留下通用异常后重新构建来猜原因。

## 决定

在现有 `tools/nodequality-rootfs-build.py` 接口中补充整步容量规划和各阶段执行保护，不新增替代 benchmark，不改变 Debian 来源链、原生 builder 条件或 full 门禁。

plan/prepare/build/export 共用三个显式参数：`--max-output-bytes` 默认 4 GiB，即 4,294,967,296 字节；`--reserve-free-bytes` 默认 512 MiB，即 536,870,912 字节；`--reserve-free-inodes` 默认 1,024。max-output 约束当前阶段新输出，不表示所有旧缓存和各阶段输出合计最多 4 GiB，也不表示已经预留独占空间。工厂预算与设备外层 256 MiB 制品预算分别核算，后者保持不变。

### 规划与认证分开

新增 `plan --materials <materials-or-lock.json> --phase prepare|build|export --output-parent <owned-dir>` 可以读取通过 schema 校验的未绑定 `materials.json` 或完整输入锁，计算所选阶段的容量。规划不执行来源下载，不安装包，不运行 `mmdebstrap`，不产生 builder 审批，不把 `builder=null` 填成虚构身份。

每份容量计划对应一个 phase，列出该阶段 payload、按文件系统块大小核算的分配量、scratch、所需字节/inode、实际剩余量、预算以及 admitted/reasons。prepare 核算 input-cache 和 mirror 两组副本；build 对展开树、成员尾块和临时开销做保守规划；export 核算归档、清单和 sidecar。已经存在的材料及前一阶段输出会消耗该文件系统实际剩余容量，下一阶段重新准入，不能在每个 phase 都复用同一份初始空闲量。允许按不同文件系统分别准入，但不能用不同目录名称证明它们具有不同容量。

计划记录输入摘要、原始声明和推导范围。没有实际文件认证的规划必须保持未认证状态；不能由文件名、包数或一个自述的 `source_authenticated` 推出正文验证通过。

实际操作把规划保存在独立 schema 的 `capacity-plan.json`，把末次观察和峰值保存在 `factory-capacity.json`；观察标明 `hard_quota=false`，不作来源或 builder 认证。原 `prepared.json`、`build-receipt.json`、`export-receipt.json` 字段不变，新增容量信息不进入原制品身份契约。

### 执行前准入和执行中保护

prepare/build/export 在创建本次输出前检查预算、目标文件系统可用字节和 inode。拒绝时说明所需容量、已有可用量和保留量，不覆盖旧输出，不删除历史缓存，不自动扩容、换机或继续尝试。

执行期间继续检查本次输出总量和文件系统余量。复制、写入和子进程运行均应有受控检查点；触发上限时停止本次动作，并终止、回收本次拥有的子进程组。原材料、旧输出和其他进程不得作为释放空间的对象。

这类动态检查是主动停止保护。没有独立受限文件系统或内核配额证据时，不能宣称它是零超调的硬磁盘配额；`tree_entries` 结束后的 2 GiB 展开检查也不能充当 `mmdebstrap` 执行中的配额。容量计划与执行收据须明确预算覆盖的普通文件、额外临时空间和未覆盖的外部消耗。

### 有界失败证据与清理

失败记录至少保存阶段、实际退出码或信号、超时/输出上限/容量拒绝分类以及已捕获的有界原输出。没有观察到的状态记为未知，不补写一个成功退出。失败输出和清理结果分别保存，不能让后续清理异常遮盖最初错误。

原输出目录的父目录中使用新的 `<output-name>-failure-<random>` 私有目录，保存 `failure.json`；有实际命令记录时另存最多 8 MiB 的 `command.log`，随后独立写 `cleanup.json`。失败收据绑定日志长度/摘要，不复用或覆盖旧失败目录。证据写入也必须保留配置的字节与 inode 管理预留，不能为取证继续侵占它。容量不足时先附 recording note；确认本次输出已成功清理后，可以再次尝试保存异常对象中原来捕获的字节，不能重新运行构建。仍不足、目录身份改变或残挂时，保留已经写出的部分证据和未知状态，不能声称日志或清理证明必然齐全。

输出清理只针对本次创建并确认归属的对象。清理前核实挂载；存在未回收挂载时保留目录并报告，不能递归进入挂载删除。可清理未发布的树和归档，先在不牺牲管理预留的条件下尽力留存有界诊断证据；不得删除旧 prepared/export、原材料或已发布成功制品。信号和截止时间遵守同一归属与回收约束。

## 来源与镜像条件保持

`plan` 不进入 `verify_inputs` 的审批路径。未绑定材料仍通过现有 `collect ...` 与 `bind` 接口处理：bind 重新核实实际 imports、签名链及正文，再绑定同架构候选工具身份；候选或 binding 收据仍为 `builder_approved=false`、`runtime_image_identity_verified=false`。

实际 prepare/build 仍要求独立批准的固定 builder 身份，按 [ADR 0043](0043-nodequality-offline-rootfs.md) 校验。三个工具的本机摘要、dpkg 声明或再次传入 image SHA 不能证明实际镜像及其 provisioning 来源。已有私有 builder 盘点显示当时缺少 `mmdebstrap` 和 `file-mirror-automount` hook；不能用这份历史盘点宣称当前已具备工厂依赖。

当前默认与离线准备制品谱系按 [ADR 0045](0062-nodequality-artifact-lineages.md) 保持。日常入口和旧报告不依赖新增真实 rootfs；完整能力没有删减，Geekbench、Ookla、NextTrace 等原工具仍按原要求取得来源、许可及能力证据。工厂容量修复不解除完整入口门禁。

## 验收要求

集中实现完成后冻结输入，再统一验收。本步骤须覆盖实际小型受限文件系统中的字节和 inode 不足、聚合阶段准入、写入期间余量下降、失败/超时/TERM/HUP 的原输出保留与进程回收、已有输出及旧材料不变、来源和 builder 审批拒绝条件。小型契约证明这些行为，不证明真实 `mmdebstrap` 已成功。

真实工厂验收另记录完整 bind→prepare→build→export 的源材料身份、独立 builder 身份、实际预算和峰值、安装包集合、许可库存、归档/清单摘要、失败或成功终态和清理结果。复建相等、双架构、合法完整工具链及 Agent/sing-box 连续负载矩阵各自需要实际证据，不能由一次容量计划或本步骤测试推导。集中冻结后的[容量验收](../acceptance/nodequality-factory-capacity.md)包含真实受限 ext4 的磁盘/inode 阈值、进程回收及 loop/mount 清理，不表示 mmdebstrap 已运行。
