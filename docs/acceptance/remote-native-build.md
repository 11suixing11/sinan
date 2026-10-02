# Debian 12 ARM64 构建及集成交付收尾

2026-10-03，冻结源码的 Debian 12 ARM64 离线构建、原生二进制核对和 TEST_ONLY 验签完成，构建单元收尾确认。Agent 与面板二进制已有实际摘要；完整机器记录见[收据](evidence/remote-native-build.json)。服务器安装和真实故障矩阵仍待验。

本步骤只在已有 Debian 12 ARM64 主机的受限构建单元中准备原生制品，不执行硬件诊断、注册设备、安装插件或操作已有业务服务。执行上下文为 `shared_build_host`，独立生产者为 `private-resource-limited-native-builder`；没有调用要求专用测试节点的原准备入口，不能作为专用节点或真实托管矩阵验收。

源码来自 `864767c4161379b919abc6d579b4147c1067127e` 的封存归档，最后功能提交仍为 `5adda2d`。784 份功能输入（含 19 份实际 `web/dist`）逐项认证；功能摘要为 `504880b192ff8ee705774fbe80e8d3c5e52a651c896c124771a214ffb582a623`，与当前集成分支及采集器受验输入一致。未修改产品源码或重跑未变的 Rust／前端测试。

## 原失败及补验范围

首次运行 UUID 为 `ed790dd9-9a23-4242-a0da-e3f52f6357c4`。九份固定材料传输、逐项 SHA256 与长度绑定、全新源码及锁定原始 registry 准备完成；离线编译实际进入 `sinan-panel` 后失败。该 UUID 单元日志明确记录 OOM killer 和 `result 'oom-kill'`，对应实际读回的 1 GiB MemoryMax／零 swap；不是由任意 SIGKILL 推断原因。

宿主采样最小剩余磁盘 72,955,105,280 B、最小可用内存 22,104,977,408 B，工作区采样最高实际占用 2,528,350,208 B，管理预留没有触发。构建收尾确认仅清理该单元，随后读回 MainPID=0、Job 为空、单元卸载且 inactive。单元卸载前未保存内存峰值和 OOM 事件计数，两项保持未知；该结果不证明宿主整体或已有业务的负载验收通过。原 target、日志和全部失败材料保留。

第一次 fresh-r2 传输在新目录创建后失败：普通账户无法完整计量首次目录内 root 所有的取证目录。没有传输输入或启动编译；空目录实际分配 4096 B，所属单元不存在，失败收据与目录均保留。集中修正受影响编排：固定目录创建及完整只读计量统一经 sudo，独占创建的新目录显式归属 `1000:1000`，SCP 与 Cargo 仍使用普通账户；不改旧目录权限或忽略计量错误。

最终补验使用全新 r3 目录、UUID、Cargo home、target 和输出，只复用重新认证的原始封存归档。**仅构建单元 MemoryMax 明确增加到 4 GiB**，产品的诊断预算保持。新工作区实际占用上限仍为 6 GiB；保留首次数据目录与新数据目录共用明确的 9 GiB 上限，另保留上述 4096 B 空目录。宿主磁盘 4 GiB／可用内存 2 GiB／16384 inode 管理预留不降。远端自身每秒守卫，即使 SSH 断开仍约束构建；轮询属于软保护，不冒称硬配额。

## 隔离及结果

Cargo 使用 Ubuntu 普通账户、一份新工具链、`--locked --offline` 和单 build job；实际单元须读回 CPUQuota 100%、CPUWeight／IOWeight 50、TasksMax 128、MemorySwapMax 0、OOMScoreAdjust 500 和 900 秒运行期限。关闭网络并使用严格文件系统、私有 tmp／devices、空 capability；root 只负责认证、单元编排与固定只读查询。没有修改宿主软件包。

控制脚本先集中完成，冻结后统一做语法、固定输入传输及实际构建；仅补原失败或未执行范围。r3 UUID 为 `f7c98d0a-5e5a-4a08-a222-cb817853920c`；语法检查、九份固定输入传输、远端身份绑定及原生准备均完成，总准备时间 304 秒，Cargo 退出 0。

| 集中执行结果 | 证据及边界 |
| --- | --- |
| 原生 Agent／面板 | `aarch64-unknown-linux-gnu`，ELF／glibc 及 Agent 版本核对完成；Agent 64,672,800 B，SHA256 `6ad099b2cfff6c3fcbd5f1e80650b25367110686ddadb06a6b5efe6a9d88bcac`；面板 71,074,216 B，SHA256 `9a8344432a4081ba652eb1f701d677cffefecdced29ccf7517fb75688707fb74`。未启动面板 daemon |
| 签名准备 | sing-box 1.14.2 固定正文及所需 tags 核对；TEST_ONLY 校验和／minisign／新 Agent `verify-release` 通过。正式发布路径拒绝该测试信任根，配套 installer 为 inert 测试资产，不能当生产安装包 |
| 缓存充分性 | 实际离线编译证明本 ARM64 Linux 目标足够；完整锁缓存仍缺七份 Windows payload，`whole_lock_cache_complete=false`，不认证 AMD64 或其他目标 |
| 实际限制 | 读回 `memory.max=4294967296`、`memory.swap.max=0`、`pids.max=128`、`cpu.max=100000 100000`；网络命名空间／OOMScoreAdjust／普通账户和 systemd 隔离核对完成 |
| 宿主管理余量 | 新目录采样最高 3,167,395,840 B，两数据目录合计采样最高 5,695,778,816 B；最小剩余磁盘 69,787,459,584 B、可用内存 22,027,853,824 B，保护线未触发 |
| 收尾 | producer 的自有单元清理收据为 confirmed；独立只读取证再次确认 MainPID=0、Job／ControlGroup 为空、inactive／not-found。监督器自身没有冒称远端清理；当前单元日志没有 OOM 消息，未保存的峰值和事件计数保持未知 |

源树、runtime 与输入摘要在构建及签名后再次核对，所有原失败材料保留。原生准备收据 SHA256 为 `723128d36524c8e928a3120b9420fe48b09f3c83fa2462da802921e5fbb60a49`，16,447 B；原始收据及有界日志保存在私有构建记录，公开机器摘要只投影本次结果及身份，不含真实环境凭据。

## 未签收范围

原本地 VM 两次磁盘拒绝及正常停机仍见[原准备记录](registered-native-preparation.md)，不被本步骤覆盖。完整 NodeQuality 的许可／工厂、当前注册日常矩阵、三设备真实托管、实际账本和持续 Agent／sing-box 联合故障负载仍待验。服务器 sing-box 插件安装与生产切换未完成，四份源码 CI 继续暂停；本步骤经同一[集成 PR #151](https://github.com/theLucius7/sinan/pull/151)交付，不声明整体实机目标完成。
