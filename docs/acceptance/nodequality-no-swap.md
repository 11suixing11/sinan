# NodeQuality r8 禁止脚本修改 swap

本项关联 #66，基于 #106 的固定来源和公开报告策略，保持独立 PR。r8 不加载未使用的旧 swap helper，不在入口清理时调用 swapoff；HardwareQuality 不分配、格式化、启用或删除临时 swap。源码仍保留 canonical 入口、五个首层脚本和四份完整许可证，已有不可变制品不改写。

## 已确认的问题

固定入口只 source 旧 swap helper，没有调用 check_swap。此前 Issue 将“独立调用 helper 的模拟结果”表述成“入口实际调用”，现已纠正，旧模拟本身保留。真正会执行的路径是入口 clear_mount 的 swapoff，以及固定 HardwareQuality 的 test_cpu_gb5 自动创建 .gb5_tmp.swap。后者先执行 fallocate/dd，再 mkswap/swapon；若 swapon 失败，need_swap 仍为 0，清理不会删除已分配文件。系统调用拒绝本身不能限制这段磁盘写入。

## 修改与保留能力

签名 runner 新增固定 swap-policy helper。构建先校验 canonical 入口再做三个精确替换：取消宿主 swapoff、取消旧 helper source、让硬件内存拒绝码 70 穿过 tee 中止后续章节。三处均保持原行数，post_cleanup 的原文及正常终止 exit 1 的第 455 行不变；现有正常/异常退出观察器继续区分两条路径。

HardwareQuality 仍先经过 r7 公开报告策略，再去掉自动 swap 创建和清理块。原 MemAvailable <950 MiB 分支改为输出明确错误并退出 70；未知内存也拒绝，不再静默跳过硬件或依赖既有 swap。达到条件时保留原 Geekbench 调用、参数和解析代码；不添加 -p，不关闭 CPU/GPU。

950 MiB 来自原脚本的宿主内存判断，只是保留的拒绝阈值，**不是已测得的 Geekbench 峰值，也不证明 cgroup 预算足够**。现有 full 默认预算 512 MiB 与真实工具需求尚未完成配套验证。所有版本 full 启动门禁保持关闭，不能据此开放完整验机。

helper 只能从 source-helper 同目录按常规文件/64 KiB/固定 SHA 读取，执行读到的已核验本地字节。两个已知角色的输入、唯一锚点、输出长度与最终 SHA 必须全部匹配；没有任意补丁入口。入口在打包时变换，硬件在真实 curl shim 的 serve 路径变换，完整 helper 位于原签名二进制中。未知或被改动的源拒绝，不回退在线来源。

## 独立验收

专项脚本 `tools/test-nodequality-swap-policy.py` 使用命令桩、私有目录和最多 9 字节的模拟分配。旧 swapon 失败负对照确实留下文件；修复后的低内存和未知内存分支不分配文件，不接触已有同名文件，不调用 swap 命令或基准程序。tee 对照验证拒绝后不启动下一章节，仍触发退出清理。固定原文验证通过真实 source-helper 打包/入口/serve 路径，逆向移除补丁后逐字节恢复 r7 内容，并核对第 455 行。

- host 首轮来源 16 项通过；swap 专项 8 项通过（包含已保存固定源的静态验证）；原 wrapper 34、daily 7 项通过。测试前原型曾错误选取较早的同名边界，来源夹具在 setup 阶段拒绝；修正精确函数边界后才记录上述通过结果，没有运行任何上游基准。
- 最终来源16、swap8和公开报告6通过，包含新增helper的测试签名断言；release32运行，28通过/4既有条件跳过。Rust版本/API与专用Debian guest结果待本项冻结后回填。未执行的项目不计为通过。

复核使用既有固定来源缓存，不下载或执行上游程序：

```sh
python3 tools/test-nodequality-swap-policy.py --readonly-upstream-dir <已校验固定来源目录>
python3 tools/test-nodequality-sources.py
python3 tools/test-nodequality-report-policy.py
```

本项只修复已定位的脚本 swap 路径，不能证明所有上游工具的宿主副作用已消除。rootfs、二级工具、上传与许可、完整负载下的取消/重启/断连/部分报告及持续代理业务仍按总故障矩阵验收。没有正式签名、发布、部署或恢复 CI，#66 保持打开。
