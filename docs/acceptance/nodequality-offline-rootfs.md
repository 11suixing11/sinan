# 离线 rootfs 准备与加载契约

本记录主体保留de54084步骤的原始r17/r18身份与验收；最新整合采用[独立制品命名](../adr/0045-nodequality-artifact-lineages.md)，当前默认为 `sinan-native-r1`、离线准备为 `offline-rootfs-r1`，新组合须独立验收。

本项属于统一分支中的一个完整修改步骤，按用户最新要求，所有实现与测试代码先集中完成，之后冻结输入、统一验收并提交。编辑期间没有运行测试、构建或完整验机。本记录确认代码契约与小型夹具验收；真实环境构建、许可和完整联合负载仍待验收。

当前日常检查默认保持 r17，原 runner、首次来源锁、完整许可证和各层策略保持原身份。r18 是显式选择的离线准备版本，制品必须精确包含 `nodequality`、`rootfs.tar.gz`、`rootfs-manifest.json` 三个普通文件。SDK 按任务版本选择签名辅助文件清单；r2–r17 继续使用原空清单。旧报告和已开始任务的恢复不重新执行诊断。

准备流程分别负责来源材料和环境导出、受控打包、设备本地加载：

1. `tools/nodequality-rootfs-build.py` 准备固定 Debian 12 snapshot 的官方签名索引、两个架构各自的包与对应源码、已审核 keyring 身份及固定构建工具身份。缺少实际值的锁不产生准备证明；不使用旧来源不完整的 BenchOS 包。构建与导出必须保留来源和许可库存，真实构建、许可审核和复建分别记账。
2. `tools/build-nodequality-offline.py` 从本 checkout 的精确 r17 包派生 r18。入口仍对应原 canonical 源码和已有策略，只将 rootfs 初始化改为本地验证与展开，并移除旧 rootfs 网络直通。双辅助文件参与原 Release 的摘要、签名、下载和每次安装确认；外层 TAR 的总展开流保持 256 MiB，不能拆分旧大包规避边界。
3. `plugins/nodequality/rootfs.py` 不下载、不执行归档内容。普通文件、gzip 与 USTAR 流、完整目录清单、逐文件摘要、压缩/展开字节数、成员数量、路径、模式及链接目标都有独立界限。硬链接、设备、FIFO、稀疏和扩展头不进入环境；合法相对符号链接须落在清单内。先检查工作区磁盘，再在私有暂存目录验证、展开，全部成功后原子发布；失败或中断清理本次目录。

签名只绑定制品及维护者记录的准备材料。来源索引签名、二进制身份、适用许可、复建和完整负载验收分别需要证据；不能从 `source_authenticated` 自述或单次打包推出它们都完成。准备版本的 provenance 明确 `full_ready=false`、许可库存未完成审核、未证实复建，并保留剩余工具能力。r18 包装器也在任何完整执行 I/O 前拒绝 full；现有面板和 Agent 的完整门禁继续保留。

本轮没有 Pro 许可，因此不执行 Geekbench，不借用公共授权，不默认为用户同意删除原 CPU/GPU 或网络能力。Ookla、NextTrace、设备 GPU 运行库及其依赖仍需实际版本、来源和适用授权；Debian 基础包准备不能替代完整工具链。具体条件见[完整工具条件](nodequality-full-tool-prerequisites.md)与[ADR 0043](../adr/0043-nodequality-offline-rootfs.md)。

设备侧下载、辅助文件复验和内层扫描使用流式长度与摘要校验。构建工厂中的打包与 Release 检查仍有有界整包缓冲，须在独立且有足够内存的工厂执行；它们不在 Agent 任务路径中，也不能描述为同样的低内存执行。来源缓存与派生镜像另占工厂磁盘，本记录原受验步骤没有跨文件的总磁盘预算，256 MiB 是最终制品边界。构建中断须回收子进程与本次输出；发现残留挂载时保留失败目录，禁止递归删除挂载内容。完整制品的工厂峰值、真实双架构合法环境与专用节点联合负载尚无证据，不能据此签收“小内存机器上的完整验机安装/运行安全”。最终验收只认证实际执行的边界与场景，CI继续暂停，未签署正式 Release、发布或生产部署。

后续[工厂容量步骤](../adr/0048-nodequality-factory-capacity.md)集中补充 plan 和 prepare/build/export 的阶段准入、动态字节/inode 保留量，以及 mmdebstrap 失败原输出留存。其 `capacity-plan.json`/`factory-capacity.json` 与旧收据独立，默认当前阶段输出 4 GiB、剩余保留 512 MiB/1,024 inode，明确 `hard_quota=false`；失败/log/cleanup 尽力写入也保留管理空间，清理成功后只保存异常内的原捕获字节，不重跑构建。仍不足或有残挂时保持未知。独立 builder 审批与 full 门禁不降低。该后续步骤的[独立容量验收](nodequality-factory-capacity.md)已完成，原构建与完整工具条件仍待；不借下文旧通过数认证新逻辑。命令和失败记录边界见[操作说明](../nodequality-rootfs-factory.md)。

## 本步骤统一验收结果

输入为 `e809395676ef1f4436d4448c3a72600eb6de990f` 加本步骤冻结修改，完整摘要和原始收据摘要见[机器记录](evidence/nodequality-offline-rootfs-r18.json)。最终 Rust 的 374 个输入前后相同；原 r17 的 23 份来源/策略/模板与默认构建脚本逐字保持基线。

| 范围 | 实际结果 | 认证边界 |
| --- | --- | --- |
| macOS Python、canonical 派生与 Release | 90 个不同方法，77 通过、13 条件跳过 | 9 个 Linux 专项在下行 Debian 12 执行；4 个既有 Release 条件仍未执行 |
| Debian 12 小型归档、来源/导出契约、签名 | rootfs 20、builder 26、artifact 12，58 通过、零跳过 | 原生 Linux 路径、TERM/HUP 子进程回收与展开取消实际执行；官方索引签名状态是自有 mock，未运行 mmdebstrap |
| Rust/PostgreSQL workspace/all-targets | 483 通过、0 失败、16 条件忽略 | 新流式缓存 4 个函数、版本清单 1 个函数的 4 个真实签名组合及 r18 日常/历史/full 门禁通过；忽略不计通过 |
| 静态及格式 | fmt、全 targets Clippy、core boundary、diff check 通过 | 本地冻结输入，不是 GitHub CI |

Debian 12 验收单元实际限制内存 256 MiB、swap 0、进程数 64，使用独立网络与挂载命名空间、无外部路由。峰值内存 51,023,872 B、峰值进程数 5，OOM 计数为零；真实 minisign 0.11 对公开 TEST_ONLY 夹具签名和验签。单元、子进程、挂载、cgroup 与临时目录均无残留，SSH 的 PID/重启数和系统启动标识不变。这些是小型夹具结果，未运行正式 Agent/代理的完整联合压测。独立 PostgreSQL 已停止，PID、监听端口和 socket 均无残留。

初次 macOS 夹具因 `/var` 别名被禁止跟随链接而失败，修正为真实临时路径；私有驱动的 Release 测试路径笔误、临时 PG socket 路径过长，以及一次 Clippy 嵌套条件失败均保留原记录。只补验失败或受修复影响的范围；产品 Python、节点场景没有因 Rust 条件格式修正重跑。工作区完整 Rust 测试只实际执行一轮。

四源码工作流仍为 `disabled_manually`，不恢复 CI。实际 snapshot/package/source 闭包、独立 builder 审批、合法完整工具链、双架构真实构建/复建、专用节点 Agent 心跳和持续代理流量联合负载仍未完成，完整验机门禁保持关闭。
