# 专用 Debian 12 虚拟机与有限 P0 验收

本项只准备独立测试节点和记录实际夹具；诊断资源、取消、遥测、IP 查询及工具链各保留独立 PR。它不签收完整 NodeQuality，也不允许在生产宿主重复硬件压测。此前指定的 aws-jp0 仍需恢复 SSH；这台隔离 guest 用来继续可受控的 Linux/systemd 验证。

## 可复建配置

Mac ARM64 使用 Lima 2.2.0 与 Apple Virtualization。[固定配置](../../tools/p0-debian12-vm.yaml) 只有一个 Debian 12 ARM64 镜像，固定发布目录和 SHA512，未继承默认模板或 `latest` 回退。配置为 2 CPU、1536 MiB 内存、8 GiB 磁盘；宿主目录、额外磁盘、containerd、Rosetta、SSH agent/X11 转发关闭。业务端口自动转发全部拒绝，管理 SSH 只监听宿主 loopback。Guest 使用独立 Linux 内核与磁盘，不借用生产容器的宿主内核。

示例命令（私有目录须新建并设置 0700）：

```sh
LIMA_HOME=/private/sinan-test/lima limactl validate tools/p0-debian12-vm.yaml
LIMA_HOME=/private/sinan-test/lima limactl start --tty=false --name=sinan-p0-debian12 tools/p0-debian12-vm.yaml
LIMA_HOME=/private/sinan-test/lima limactl shell --workdir=/tmp sinan-p0-debian12
```

独立 `LIMA_HOME` 保留本任务的配置、磁盘、日志和自动生成的管理身份，不修改默认实例。禁止把管理身份、原始日志或其他业务凭据提交到仓库。软件依据 [Lima VZ 文档](https://lima-vm.io/docs/config/vmtype/vz/)；镜像依据 [Debian 官方 SHA512SUMS](https://cloud.debian.org/images/cloud/bookworm/20260712-2537/SHA512SUMS) 与 [Lima 固定版本模板](https://github.com/lima-vm/lima/blob/v2.2.0/templates/_images/debian-12.yaml)。

合并审查在 [PR #91](https://github.com/theLucius7/sinan/pull/91) 补齐 `guestIPMustBeZero: false`：固定 Lima 2.2.0 在未设置此字段时，显式 `guestIP: 0.0.0.0` 的忽略规则只匹配该监听地址，guest 回环监听会落入自动转发。修改后的配置通过同版本 `limactl validate`；下文分别记录原启动/六夹具和实际应用修正后的端口补验，不用配置校验代替运行期验证。

## 已取得的证据

2026-10-01（Asia/Taipei）Lima 配置校验通过，实际启动完成。重新读取缓存原始镜像 341,114,880 字节，其 SHA512 与配置及官方清单一致：

```text
52eb678130e85a9bf9f3cf4f7181e060fa60a5db45373717474c5ff9305645c5e6cab8b05ca042bcfa7921d2ca29cf9fd2e815f75abe2d168557ad7207af127b
```

实际读回 Debian 12 bookworm、aarch64、systemd `running`、cgroup v2 的 cpu/io/memory/pids 等控制器。初始 `MemTotal=1517844 KiB`、`MemAvailable=1349344 KiB`，根文件系统约 6.4 GiB 可用；无宿主 virtiofs/9p/sshfs 挂载。管理 SSH active、重启数 0，`/proc/swaps` 无已启用项；管理端口 loopback 绑定单独检查。

上述资源是启动时快照。运行前重新读取，不把启动快照当成持续代理业务或负载期间的可用内存证明。Guest 只安装 build-essential 与 pkg-config，以及摘要逐项校验的 Rust 1.97.1 rustc/cargo/std 三组件；宿主工具链不修改。构建只在 guest 中的独立单元执行，限制 1100 MiB、无 swap、128 tasks、CPUWeight=10、OOMScoreAdjust=500、一个 Cargo 编译任务；测试串行执行，先结束和核实清理，再开启下一项。

## 冻结源码的实际有限验收

源码固定为 `356350e17d9106dffdc0c17a4c0e6d068081d170`，全部归档普通文件在构建和测试前后逐字节一致。构建退出 0、耗时 7 分 2.422 秒；ARM64 ELF 为 50,439,408 字节，SHA256 为 `61dde3d0d64bd87c2367f9d567aff8b031f56d5b31da19209a0ba8f1c6d35a1b`。构建峰值低于 1100 MiB，cgroup oom/max 事件为 0。此身份属于冻结基线，不认证之后 `3f65b42` 或其他提交。

先通过二进制 `--list` 确认精确六项，再用下列过滤串行实际执行，6 通过、0 失败、0 忽略、120 未选中，Rust 报告耗时 2.30 秒；外部观察耗时 2.412 秒、没有超时。测试主程序另受 256 MiB/无 swap/64 tasks/5 分钟单元限制，夹具的诊断单元分别施加预算。

```sh
sudo <frozen-arm64-core-tests> real_systemd_diagnostic_ --ignored --test-threads=1 --nocapture
```

| 真实夹具 | 已执行范围 |
| --- | --- |
| resource_budget_restricts_memory_and_children | 内存与子进程预算实际约束 |
| cancellation_confirms_process_and_private_mount_cleanup | 停止后子进程和私有 tmpfs 清理确认 |
| lock_blocks_unprivileged_open_and_preserves_held_inode | 非特权进程不能打开诊断锁、持有 inode 不替换 |
| queued_start_stays_running_until_executed | 排队启动未执行前不能伪造完成 |
| jobs_survive_manager_recreation_and_enforce_timeout | 服务管理器对象重建、期限停止；不等于实际 Agent 或 PID 1 重启 |
| preflight_reads_resources_and_enforces_exclusive_execution | 真实资源读取与同机执行互斥；不等于全 Agent 预检整链 |

观察到 9 个新 UUID 诊断单元、30 条实际属性快照。内核新增 OOM 仅发生在 64 MiB 诊断 cgroup，约束为 `CONSTRAINT_MEMCG`；没有全机 OOM。测试结束剩余诊断单元、夹具进程及挂载均为空，没有手动补清理；管理 SSH PID 与重启数前后不变。这不能证明未参与此夹具的 Agent 或代理业务仍连续服务。

另在 guest 唯一 0700 临时目录的私有 16 MiB tmpfs 上预分配严格最多 16 MiB，实际有限写入返回 `ENOSPC`（errno 28），statfs 可用字节为 0。单元实际为 64 MiB/无 swap/16 tasks/PrivateMounts，峰值 24,055,808 字节、oom 0；随后卸载并删除目录，根盘仍余 4,951,310,336 字节，swap 前后不变。这是文件系统原语复核，不计为 Agent 持久化/预检失败端到端已通过。

私有证据包含源码与 ELF 身份、六项名称/日志、测试前后内核差异、单元活动期属性、最终进程/挂载状态与 ENOSPC 结果。transient 单元回收后属性可能回退默认，实际预算以活动期间保存的快照为准；不公开原始日志或管理身份。

## 端口转发规则的运行期补验

2026-10-01，确认本任务构建结束、未启动 Agent 或业务监听后，仅停止本任务实例，使用 `limactl edit --set '.portForwards[0].guestIPMustBeZero = false'` 修改原有规则，并重新启动一次。保留原磁盘、身份和证据，没有创建或重建其他实例。模板对应 PR #91 的 `363ebd856b98183f5621322117254acc4e5ade6d`，SHA256 为 `8f1b29a013fdfcefd43092e0ff75840fcd2a0b132589ff0552651429a1da3f04`；实际实例配置 SHA256 为 `5e7bc6afbcaaabffa142085ba41e434a8db0132b66755628441f340d410fd709`。

新 boot ID 的 SHA256 为 `5a2730c8d35f994be6e7931791fa3e7637cf12043d87875e4bfcc571a3850217`。启动日志明确关闭除 SSH 外的 TCP 和 UDP 转发；新管理 SSH 读回 active、MainPID=406、NRestarts=0，宿主管理监听仍仅为 loopback。此处是有意重启后的新身份，不能宣称 PID 仍为原启动的 446。

两个短期 HTTP 夹具分别绑定 guest 的 `127.0.0.1` 和 `0.0.0.0`，只返回固定验收文字，无凭据或业务数据。guest 内两次正向请求均成功；启动后 0、5、10 秒，宿主对两端口的六次 TCP 连接均返回拒绝（errno 61），对应六次 `lsof` 检查均无宿主监听。夹具 15 秒正常退出，最终读回无该进程、监听或准备文件，无宿主共享文件系统；根盘仍余 3,990,147,072 字节。UDP 关闭仅由启动日志确认，本项未执行 UDP 收发，不把 TCP 结果扩展为 UDP 线路实测。

私有结果保存在本任务证据目录 `port-rule-runtime/result.json`，含正/负对照、三轮检查、配置与 boot 摘要和最终清理。该补验只证明上述两类 IPv4 TCP 监听的实际端口隔离，IPv6 回环监听及线路未单独实测；原六夹具及 swap 专项仍各对应原 boot 和原源码，新 Agent 与持续代理联合负载另行记录。CI 保持暂停，本项不重跑 Cargo，也不补签完整 NodeQuality 或其他阶段。

## 验收范围与缺口

| 场景 | 证据要求 | 状态 |
| --- | --- | --- |
| VM 隔离 | 固定镜像摘要、资源读回、无宿主目录共享、无业务转发 | 已检查；不代替任务保护验证 |
| 真实 systemd 有限夹具 | 对应源码的 ARM64 test binary、六项实际执行、退出/进程/挂载读回 | 冻结基线六项通过，不能认证之后主线或全工具链 |
| 小文件系统 ENOSPC | guest 内唯一受限文件系统，实际写入失败，根磁盘不填满，最终全部卸载清理 | 16 MiB 私有 tmpfs 原语已验；Agent 整链仍待验 |
| 最新 Agent 预检与保护 | 实际低内存/磁盘拒绝原因及运行阈值停止 | 待执行；不能用一般系统命令代替 Agent 整链 |
| 心跳、持续 sing-box 业务、断连/重启/取消/部分报告 | 真实 Heartbeat 与采样时间、受控代理成功记录、设备清理确认和检查点恢复 | 待执行；有限夹具不签收完整负载 |
| 完整 NodeQuality | 固定且获准的完整工具链、禁上传、宿主零改动，持续业务下总验 | 门禁保持，授权及受控执行链尚未解决 |

CI 按当前仓库安排保持暂停，未运行或取消的检查不计通过。按照 [整改顺序](ordered-remediation.md) 逐阶段签收；这台专用 VM 可以补有限真实故障，不以其存在或一次成功报告宣称目标完成。
