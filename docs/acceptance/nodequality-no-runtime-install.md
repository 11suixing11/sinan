# NodeQuality r9 禁止运行时安装依赖

本项是 #28 的独立修复：已有固定首层脚本仍会调用包管理器、下载并安装 Geekbench、curl-impersonate、NextTrace、speedtest 和 stun。入口 pin 不能约束这些二级安装。r9 将已定位的安装路径改为明确拒绝，保留原探测能力和所有版本完整验机门禁。

## 修改边界

新增固定 `dependency-policy.py`，接入 builder、单文件 runner 和真实 source-helper serve 路径。入口经过原 swap 策略后，再把 NextTrace 下载/修改权限替换为 rootfs 内 `test -x`；工具缺失即拒绝。三个首层脚本在公开报告策略、硬件 swap 策略之后，将精确安装函数区段换成依赖检查和拒绝安装的函数。包管理器和下载器不会在这些函数中执行，`-n` 也不能跳过检查。

检查用 `command -v` 列出缺少的工具，不执行工具；硬件根据原 fast/privacy/verbose 参数要求 sysbench、Geekbench 和额外依赖，不擅自启用轻量或隐私模式。Net 同时检查其包列表内的 bc。缺项返回 70 并说明禁止运行时安装。IP、网络、回程章节使用局部 pipefail，避免 tee 把失败当成功；在原 EXIT 清理函数和真实 DEBUG 观察器下，最终退出码可能变成 1，但不会写正常完成标记或继续下一章节。

每个 helper、输入、唯一锚点、输出长度和输出 SHA 均验证；未知文件、重复锚点、超限、FIFO、符号链接或篡改均拒绝。完整 canonical 源码及四份 AGPL 原文仍在嵌入包中。原清理第 455 行、硬件字符集函数、探测、参数和序列化在指定区段外保持字节不变。r2–r8 历史报告和 r4–r9 日常入口保持兼容。

存在性检查**不证明**工具版本、来源、许可证或完整性。旧 chroot shim 的架构映射保留兼容，但 r9 入口已不会请求该下载。rootfs 下载、二级数据、浏览器伪装、工具内上传及完整许可证链仍未解决，#28 不关闭，本项不签收完整 NodeQuality。

## 独立验收

受验源码为 `fc4e49f`，基于 `55f1c2a`。随后正常合入 `8ffcc44` 得到 `9d99b28`，只解决 PROGRESS 双方追加记录；crates、plugins、tools、Cargo.lock 与 CI 测试登记字节保持受验值。随后 `4b35801` 只给面板 queued full 回归补回显式 r8 历史版本，并重跑 Rust/API 19 项及 Clippy/fmt/core。Python 和 guest 产品输入未变，保持原收据。主线新增看板不借用本项专项作为完整业务验收。最终仅回填本项文档，机器可读摘要见 [r9 验收索引](evidence/nodequality-no-runtime-install-r9.json)。

| 检查 | 结果 | 证明范围 |
| --- | --- | --- |
| macOS 依赖专项 | 11 运行，10 通过、1 因系统 Bash 3 跳过完整源码语法 | 逐个缺工具、存在检查不执行、显式安装函数拒绝、不能用 -n 绕过、NextTrace 旧下载负对照、真实退出观察器、原文逆还原、helper 和构建器失败关闭 |
| macOS 既有回归 | swap10、来源16、公开报告6、wrapper34、daily7 全部通过；release32 运行，28 通过/4 既有条件跳过 | 嵌入接线、公开 TEST_ONLY 签名覆盖、不可变制品、原参数与报告处理；没有正式签名 |
| Rust 与真实 HTTP/PostgreSQL | adapter15、panel gate3、日常接口1 全部通过、0 忽略 | 明确覆盖历史 r8，当前版本 r9；旧报告保留和所有 full 门禁；专属数据库按 PID/启动时刻停止，进程不存在/端口关闭 |
| Debian 12 Bash 5.2.15 | 依赖11、swap10、原文函数体 FD 组合2、wrapper34、daily7 全部通过；来源16 运行，15 通过/1 缺 minisign 跳过 | 四份真实固定源的变换及完整语法、Linux 原 FD 接线；原探测和 serializer 是惰性替身，网络只到自身回环 recorder |

adapter 全 targets Clippy（warnings 为错误）、workspace fmt、core 边界和差异检查通过。未重跑全部 workspace/全部面板测试或其它平台。初版 macOS 完整源码语法失败原日志保留；Debian 上原始脚本和变换后脚本均通过，确认其要求 Bash >=4。审查发现的字符集函数边界已在冻结前收紧；未把早期失败计入通过。

Debian 在专用虚拟机的独立 systemd 单元内运行 45.658 秒，共 80 运行、79 通过/1 条件跳过。内部持久化读回 MemoryMax=256MiB、MemorySwapMax=0、TasksMax=64、CPUWeight/IOWeight=10、OOMScoreAdjust=500、PrivateNetwork/PrivateMounts/NoNewPrivileges=yes、KillMode=control-group。内存峰值 76,435,456B、pids 峰值 11，memory.events max/oom/oom_kill 均为 0。

单元结束后以真实 journal 启动时刻检查内核，没有新 OOM、夹具进程、挂载或 cgroup 残留；SSH PID406、重启0，同一 boot、swap0。33 份输入前后不变，其中 22 份仓库文件与最终整合输入匹配。保存的 result/postcheck/receipt SHA 分别为 `1ed70cb9737489a21d332f28afe962307166fc997993640628ee32c831d0b692`、`0dd4fcc65a5f6e74259bfb313adea9d03bb590442dfbf832a00559e9d090f520`、`5f3200122302a7082eb6437d26f9cb6e5b13ffc0ef207343f115ce33573581ee`。Rust 收据 SHA 为 `9c7014087734781eae79a5064fb153afad16eaafa19b067f6d67b7c80297aa4f`。

## 复核入口与总故障矩阵

以下命令只运行惰性夹具和已校验原文的有限函数体，不执行整份上游脚本；原文参数必须指向此前逐文件核验过的只读缓存。完整语法检查在 Linux Bash >=4 执行：

```sh
python3 tools/test-nodequality-dependency-policy.py --readonly-upstream-dir <固定来源目录>
python3 tools/test-nodequality-swap-policy.py --readonly-upstream-dir <固定来源目录>
python3 tools/test-nodequality-sources.py
python3 tools/test-nodequality-report-policy.py --readonly-upstream-dir <固定来源目录> WiringTests
python3 tools/test-nodequality.py
python3 tools/test-diagnostic-modes.py
```

本项低内存证据是 256MiB 预算内的有限检查及既有拒绝回归，未执行真实硬件压测；磁盘不足、403/429/超时、Agent 重启、面板断连、取消、重复、部分报告、持续 sing-box 负载的完整工具验收仍待整链就绪。这些不会被缺工具测试代替。已完成的真实注册 Agent 日常矩阵仍严格归属 [PR #109 的 b0869ef/r8](registered-nodequality-daily.md)，本项没有重跑或追认其为 r9 完整验机证据。

四个仓库工作流继续暂停，只登记未来专项命令；没有触发 CI、正式签署、发布或部署，没有解除 full 门禁。
