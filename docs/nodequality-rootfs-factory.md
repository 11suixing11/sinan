# NodeQuality 离线工厂操作边界

本文件说明容量接口与后续实际操作顺序。容量保护及回收的[独立验收](acceptance/nodequality-factory-capacity.md)已完成，真实完整工厂构建仍待。设计决定见 [ADR 0048](adr/0048-nodequality-factory-capacity.md)。不执行上游 benchmark，不安装运行时，不签署正式 Release，也不解除完整验机门禁。

## 已有材料与尚缺条件

[完整 Debian 材料步骤](acceptance/complete-debian-materials.md)取得实际 ARM64 的 237 个 deb 和 168 个源码版本的 539 个源文件，776 个正文的 Size/SHA256 已独立复核。该次材料仍未绑定 builder：`builder=null`、`lock_ready=false`。大缓存保留在专用 guest，不能凭本文件重新收集、删除或移走旧材料。

现有 factory 通过 `tools/nodequality-rootfs-collect.py bind` 把重新认证后的材料与候选 builder 结合，之后由 `tools/nodequality-rootfs-build.py` 的 prepare/build/export 消费。候选只声明固定 image 身份和 gpgv/mmdebstrap/unshare 工具身份；本机摘要、包归属声明和候选锁不等于独立镜像审批。

实际启动工厂之前还须具备合法、固定来源的原生 Debian 12 builder，以及它的镜像/provisioning/工具与 hook 来源和启动身份材料。本步骤没有取得或假定该审批；历史私有盘点缺少 mmdebstrap 及 file-mirror-automount hook。新的安装或环境调整需要明确执行范围，不能从旧盘点推断当前工具已经可用。

## 先规划，再分别准入

新增 `plan` 允许使用通过 schema 校验的未绑定材料，不需要伪造 image 摘要。它只输出容量推导，不能产出来源认证、builder 审批、成功 build/export 或 `full_ready` 证明。prepare/build/export 的实际身份校验仍按原有契约执行。

显式工厂预算默认如下：

| 参数 | 默认值 | 含义 |
| --- | --- | --- |
| `--max-output-bytes` | 4,294,967,296 | 当前阶段新输出的最大字节预算，4 GiB |
| `--reserve-free-bytes` | 536,870,912 | 目标文件系统至少保留 512 MiB |
| `--reserve-free-inodes` | 1,024 | 目标文件系统至少保留的可用 inode |

每份计划对应一个 phase，列出目标文件系统、该阶段新增量、scratch、字节和 inode 需求、实际剩余量以及 admitted/reasons。已存在的缓存及前阶段输出已经消耗磁盘，每个实际阶段都重新准入；不能把所有阶段的默认 4 GiB 当成整个工厂已独占的容量，也不能反复使用同一份初始空闲量。目录分开不代表容量分开。对于当前实际 ARM64 材料，仅 prepare 的缓存与 mirror 复制声明就为 1,069,414,018 字节；还须为元数据和后续树/归档预留。旧 guest 最后记录的 782,073,856 字节不足以容纳这些新副本；实际执行必须重新读回容量，不能把历史值作为当前状态。

以下是参数形状，所有大写项为需要提供的实际路径或已独立批准的身份，不是可直接运行的样例材料：

```sh
python3 tools/nodequality-rootfs-build.py plan --materials MATERIALS_OR_LOCK_JSON --phase prepare --output-parent OWNED_PARENT
python3 tools/nodequality-rootfs-build.py plan --materials MATERIALS_OR_LOCK_JSON --phase build --output-parent OWNED_PARENT
python3 tools/nodequality-rootfs-build.py plan --materials MATERIALS_OR_LOCK_JSON --phase export --output-parent OWNED_PARENT

python3 tools/nodequality-rootfs-build.py prepare --lock INPUTS_LOCK_JSON --cache INPUT_CACHE --output NEW_PREPARED --approved-builder-image-sha256 APPROVED_IMAGE_SHA256
python3 tools/nodequality-rootfs-build.py build --prepared PREPARED_DIRECTORY --output NEW_BUILD --approved-builder-image-sha256 APPROVED_IMAGE_SHA256
python3 tools/nodequality-rootfs-build.py export --tree BUILD_TREE --prepared PREPARED_DIRECTORY --output NEW_EXPORT --approved-builder-image-sha256 APPROVED_IMAGE_SHA256 --outer-reserve-bytes OUTER_RESERVE_BYTES
```

四种命令均可显式附加 `--max-output-bytes 4294967296 --reserve-free-bytes 536870912 --reserve-free-inodes 1024`，与默认值一致。plan 的输出为独立 schema 的容量 JSON，未认证或审批；实际阶段在新输出目录保存 `capacity-plan.json` 和 `factory-capacity.json`。前者绑定阶段、输入描述与预算，后者保存末次/峰值观察并标明 `hard_quota=false`。这两份文件不改变原 prepared/build/export 收据字段和签名制品契约。

准入失败后停止。不能删除旧材料、扩磁盘、换生产机器或重试直到有空间。动态 guard 触发时只停止本次拥有的动作和子进程，保存失败及清理证据。它是主动磁盘保护；没有内核配额或受限文件系统证据时，不宣称零超调。

## 实际操作顺序

1. 对固定材料做 plan。保存输入身份、容量推导、预算与目标文件系统，保留未认证/未批准标记。
2. 在实际原生 builder 中取得固定工具及独立镜像来源证据。调用现有 bind，对原材料的 imports、签名索引和正文重新验证后产生严格输入锁；binding 收据仍不代替审批。
3. 用独立批准的 image 身份执行 prepare。完整 source cache 和本地镜像均须预算；只创建本次私有输出，不覆盖旧目录。
4. 对 prepared 材料执行原生 build。现有命名空间隔离和 file mirror 路径保持；构建不使用在线 main、宿主 APT 凭据或任意来源。记录实际 mmdebstrap 阶段、退出码/信号、原有界输出和清理结果，失败不重跑来补造原证据。
5. 对成功树执行 export。复核安装包集合、对应源码和许可库存，输出固定元数据的归档/清单及收据。原 256 MiB 外层总界限保持，不能拆旧大包规避。
6. 成功 export 之后才可安排原有离线制品打包/公开 TEST_ONLY 签名验证。它不等于正式签名、发布、部署或完整工具链验收；默认制品选择不自动改变。

不得用 plan 结果代替 `verify_inputs`/`verify_prepared`/`verify_export`，也不得把候选工具摘要与传入的 image SHA 相等描述成完整镜像来源验证。

## 留存与终态

至少保留容量计划、阶段输入身份、实际单位预算和文件系统余量、子进程退出状态、有界原输出、源材料与旧输出是否保持、成功收据或失败详情，以及进程组/挂载/本次目录的清理读回。错误状态、容量拒绝与清理失败分别记录；证据不可得时明确未知。

失败证据写入输出父目录中新建的 `<output-name>-failure-<random>` 私有目录。`failure.json` 保存实际阶段/错误和已经观察到的 command/capacity 信息；有命令记录时 `command.log` 保存最多 8 MiB 的原输出，其长度和摘要与 failure 收据绑定；`cleanup.json` 独立记录原输出目录 removed/retained。它不代替实际子进程或挂载证据，不能仅凭目录已删除宣称所有资源清理成功。

失败目录也需要空间与 inode，并遵守同样的管理预留。容量不足时在原异常附加 recording note；确认本次输出成功清理后，仅可以再尝试保存异常对象中原来捕获的字节，不重新执行命令。仍不足、身份变化、残挂或写入异常时保留部分记录和未知状态；不能保证任何失败都具有完整日志和 cleanup 文件，不另起构建重试来倒填。

失败时可以清理确认归属的未发布树和归档，先在守住管理预留的条件下尽力保存有界错误证据。剩余挂载未确认清理时不得递归删除目录。旧材料、旧成功输出、其他用户文件和其他任务进程不属于本次清理范围。

本步骤只为真实离线工厂建立可执行的安全前提。没有真实 build 时不写 rootfs 构建完成；没有两次独立原生导出的相等证据不写复建通过；没有合法全部原工具及专用节点的完整 Agent/sing-box 故障矩阵，不签收原 NodeQuality 完整能力。日常、历史报告及原 full 门禁保持。
