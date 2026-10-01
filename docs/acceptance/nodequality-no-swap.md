# NodeQuality r8 禁止脚本修改 swap

本项关联 #66，基于 #106 的固定来源和公开报告策略，保持独立 PR。r8 不加载未使用的旧 swap helper，不在入口清理时调用 swapoff；HardwareQuality 不分配、格式化、启用或删除临时 swap。源码仍保留 canonical 入口、五个首层脚本和四份完整许可证，已有不可变制品不改写。

## 已确认的问题

固定入口只 source 旧 swap helper，没有调用 check_swap。此前 Issue 将“独立调用 helper 的模拟结果”表述成“入口实际调用”，现已纠正，旧模拟本身保留。真正会执行的路径是入口 clear_mount 的 swapoff，以及固定 HardwareQuality 的 test_cpu_gb5 自动创建 .gb5_tmp.swap。后者先执行 fallocate/dd，再 mkswap/swapon；若 swapon 失败，need_swap 仍为 0，清理不会删除已分配文件。系统调用拒绝本身不能限制这段磁盘写入。

## 修改与保留能力

签名 runner 新增固定 swap-policy helper。构建先校验 canonical 入口再做三个精确替换：取消宿主 swapoff、取消旧 helper source、让硬件失败穿过 tee 中止后续章节。三处均保持原行数，post_cleanup 的原文及正常终止 exit 1 的第 455 行不变；现有正常/异常退出观察器继续区分两条路径。

HardwareQuality 仍先经过 r7 公开报告策略，再去掉自动 swap 创建和清理块。原 MemAvailable <950 MiB 分支改为输出明确错误并退出 70；未知内存也拒绝，不再静默跳过硬件或依赖既有 swap。达到条件时保留原 Geekbench 调用、参数和解析代码；不添加 -p，不关闭 CPU/GPU。

950 MiB 来自原脚本的宿主内存判断，只是保留的拒绝阈值，**不是已测得的 Geekbench 峰值，也不证明 cgroup 预算足够**。现有 full 默认预算 512 MiB 与真实工具需求尚未完成配套验证。所有版本 full 启动门禁保持关闭，不能据此开放完整验机。

helper 只能从 source-helper 同目录按常规文件/64 KiB/固定 SHA 读取，执行读到的已核验本地字节。两个已知角色的输入、唯一锚点、输出长度与最终 SHA 必须全部匹配；没有任意补丁入口。入口在打包时变换，硬件在真实 curl shim 的 serve 路径变换，完整 helper 位于原签名二进制中。未知或被改动的源拒绝，不回退在线来源。

## 独立验收

专项脚本 `tools/test-nodequality-swap-policy.py` 使用命令桩、私有目录和最多 9 字节的模拟分配。旧 swapon 失败负对照确实留下文件；修复后的低内存和未知内存分支不分配文件，不接触已有同名文件，不调用 swap 命令或基准程序。tee 对照验证拒绝后不启动下一章节，仍触发退出清理。固定原文验证通过真实 source-helper 打包/入口/serve 路径，逆向移除补丁后逐字节恢复 r7 内容，并核对第 455 行。

- host 首轮来源 16 项通过；swap 专项 8 项通过（包含已保存固定源的静态验证）；原 wrapper 34、daily 7 项通过。测试前原型曾错误选取较早的同名边界，来源夹具在 setup 阶段拒绝；修正精确函数边界后才记录上述通过结果，没有运行任何上游基准。
- 最终来源16、swap8和公开报告6通过，包含新增helper的测试签名断言；release32运行，28通过/4既有条件跳过。冻结 ebf7302 的 Rust/API 19 项全部通过、0 忽略；专用 Debian guest 结果见下表。未执行的项目不计为通过。

复核使用既有固定来源缓存，不下载或执行上游程序：

```sh
python3 tools/test-nodequality-swap-policy.py --readonly-upstream-dir <已校验固定来源目录>
python3 tools/test-nodequality-sources.py
python3 tools/test-nodequality-report-policy.py
```

## 冻结源码与专用节点结果

受验实现为 `ebf7302eae54e857bc6a11b73b84fbe366921cee`，基于 r7 `fc3bb5a`，最终仅回填文档。canonical source-lock SHA 保持 `3d20398eeda72654c59b3271fd03b35ca8c0b4e92ee92a054a4a8c432a62723a`。swap helper SHA 为 `d43b3e6fa6bfc0a31fcff5e3bc7a3228cdd1501c9f15f7e27175d09dada9a96b`；执行入口 SHA 为 `70863b1038cd650977ea9f378741a6fcb530f87d45f07fbb28c418f07c30ad88`，最终硬件脚本 SHA 为 `e1f90cb9eaf80098a48beb008187b422cb664fc32cf2a4ee306b398ee473567c`。

| 验证 | 实际结果 | 范围 |
| --- | --- | --- |
| Rust/API | adapter15、panel gate3、真实HTTP/PostgreSQL日常1，全部通过/0忽略 | r2–r7历史、r4–r8日常及全部full门禁；公开TEST_ONLY编译信任根、离线编译 |
| Debian12来源 | 16运行，15通过、1缺minisign跳过，3.072秒 | curl7.88实际chunked/声明超限、来源/helper/构建边界 |
| Debian12 swap专项 | 8通过、0跳过，0.391秒 | 低内存/未知值拒绝、9字节遗留负对照、同名文件保护、tee退出与清理、固定原文逆还原 |
| Debian12原函数体组合 | 2通过，38.730秒 | Bash5原FD路径，旧/true四次回环POST、false/default零；全部探测与serializer均stub |

adapter全targets Clippy、workspace fmt、core分层、shell语法与diff检查通过。Rust五输入/Cargo/trust前后不变；专属PostgreSQL55439按PID和启动时刻归属停止，PID不存在、端口关闭。Rust收据 SHA：`c0ba55b1e305b2b26ed78fc77d34a610d0521af84496524f3991d7a09f459dea`。

guest在独立单元运行并内部持久化限额读回：MemoryMax256MiB、Swap0、Tasks64、CPU/IOWeight10、OOMScoreAdjust500、PrivateNetwork/PrivateMounts/NoNewPrivileges=yes、KillMode=control-group。memory.peak为64,737,280字节，pids.peak为11，memory.events max/oom/oom_kill均0。实际journal起点后的内核无新OOM；自然完成后无进程/挂载/cgroup残留，SSH406重启0、同boot、swap0。29份输入前后不变，当前对应产品文件逐个匹配；没有执行实际swapon、上游bootstrap、Geekbench或外部网络探测。

私有 `nodequality-r8-fd-20261001` 的 result/postcheck/receipt SHA 分别为 `f5d3c6cb978cb25c447fe0641185c4e78cca87be5d6581c54f6e1bf35510e544`、`76b95895319c6d610b83ed6eef838fbd906d2f57f5b29b1dcd9d56eb0b9f8565`、`841d30bb57d4f47deb06fb6c10e31cc9e33fbf5d5065e3991088ee80494413dc`。每条证据仅对应列明源码和有限场景，不把重复回归累计为新功能数。

本项只修复已定位的脚本 swap 路径，不能证明所有上游工具的宿主副作用已消除。rootfs、二级工具、上传与许可、完整负载下的取消/重启/断连/部分报告及持续代理业务仍按总故障矩阵验收。没有正式签名、发布、部署或恢复 CI，#66 保持打开。

## 合并审查补修：与真实退出观察器相容

上述作者验收对应 `ebf7302`。合并审查发现，初版在 tee 后读取 `PIPESTATUS[0]`，但真实 `exit-observer.sh` 的 DEBUG trap 会先改写该数组。独立夹具保留固定入口的 post_cleanup/sig_cleanup 原文和第 455 行，启用产品实际观察器，把硬件、挂载、chroot 和删除全部替换为惰性桩；负对照确实在硬件拒绝 70 后继续下一章节，并写出正常完成标记。

补修将硬件执行和 tee 放在启用 pipefail 的子 shell 中，通过 `|| exit $?` 拒绝硬件或来源失败，避免依赖会被 DEBUG trap 改写的数组。成功分支仍保留正常完成标记；失败分支不启动后续章节、不写完成标记，退出时仍做原清理。原 sig_cleanup 的 post_cleanup 最终会把拒绝码 70 改为 1；这个结果保持失败，不能当作正常成功。

补修 swap helper SHA 为 `1d6acda7821d013773b273d77db12973d7075631b0309614dadb9c5cfc09ff24`，执行入口 SHA 为 `a68a42e8f508fdc1ed5a7170fda4b114ab81afc184a9c035b48613407a1fdc93`，其余 canonical 与硬件输出不变。这份补修在 macOS Bash 3.2 的有限夹具中验证；正式 runner 仍要求 Bash ≥4，Bash 5、实机完整链及负载未在合并审查中复演，不把原作者的较早 guest 结果移作新补修证明。
