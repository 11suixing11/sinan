# NodeQuality r11 脚本供给失败独立验收

修复 [#112](https://github.com/theLucius7/sinan/issues/112)，基于 #113 的 r10，范围是报告头、硬件、IP、网络、回程五个脚本加载入口。受验源码 `6decf87`；完整 SHA、输入身份及原始证据摘要见 [机器可读索引](evidence/nodequality-source-failure-r11.json)。

## 问题与改动

原 `bash <(curl …)` 不等待供给器退出。真实 source-helper 拒绝错误数据、stdout 为空时，内层 Bash 执行空脚本并退出0；外层 pipefail 无法感知进程替换失败。硬件管道还可能在供给器最终失败之前执行已经输出的脚本片段。

五个入口现在先将有界供给结果存入局部 Bash 变量，等待供给器成功，再交给原消费者；空的成功结果也以70拒绝。命令替换附加并移除一个哨兵，保留全部尾部换行。失败码直接返回，不启动消费者。仍保留原 FD/stdin 形状、参数、硬件 NQENV 和章节内容，不增加临时脚本。报告头主调用补上失败退出，其余四个章节使用既有失败保护。

六处精确替换不改变行数，原清理函数及观察器依赖的第455行保持。`loader-policy.py` 连同原始源码包嵌入同一 runner，helper/input/output 都校验 SHA256。helper 摘要 `189fda7f90cd91df37ddfecf206c15137d823128a75e7b22c850abb2e2a2fe92`；最终入口19,674B，摘要 `728d15d923ae030b9caf83cfe38b9bdb690800c9e11feb57c75ce99a7623a182`。17文件原始包与许可证不变。

版本升为 r11，显式保留 r10 兼容测试：r2–r10 历史报告仍可读、r4–r11 日常任务保持；所有版本的 full 门禁保持关闭。

## 独立结果

| 检查 | 结果 | 证据范围 |
| --- | --- | --- |
| 加载专项 | macOS10/10、Debian10/10，无跳过 | 五入口旧/新负对照；空失败、部分失败、空成功；成功等待供给完成且原字节/参数只执行一次；供给中取消；真实校验器拒绝；原始主调用、清理和退出观察器；先前章节保留；helper/构建边界 |
| macOS 组合 | 132运行、127通过、5条件跳过 | 加载10、数据6、依赖11、swap10、来源16、报告6、wrapper34、daily7、release32；Bash3完整原文语法跳过1、Linux root条件跳过4 |
| Rust/API | 19通过、0失败/忽略 | adapter15、chain_gate3、真实HTTP/PostgreSQL日常1；adapter Clippy、workspace fmt、core边界、diff通过；没有重跑整个workspace |
| Debian12 Bash5 | 96运行、95通过、1缺minisign跳过，63.561秒 | 来源16、加载10、数据6、依赖11、swap10、原文函数体FD组合2、wrapper34、daily7；原探测和序列化用惰性替身 |
| 真实最小 chroot | 30/30场景 | 原始 chroot_run、生产 chroot shim、宿主原生 Bash/env 和独立 proc 挂载；五入口×空失败/部分失败/成功×旧/新；只执行无害 printf 载荷 |

真实 source-helper 的五路径测试分别篡改 header、hardware、IP 数据、Net 数据及 Net 脚本，均得到校验错误、非零退出、零stdout，消费者未启动。该测试即使保护回归，消费者替身也不会执行真实上游脚本。

真实 chroot 的旧入口负对照会将空输出失败误记为0，且会执行部分失败载荷；新入口返回70并完全不执行载荷。成功路径原 argv 与硬件 NQENV 均保持。最小根目录、挂载和原生二进制摘要在独立收据中记录；这不是完整 BenchOS 或完整 NodeQuality 执行。

首轮专项因测试调用多传一个参数而报错，已修正并保留首轮日志；修正后才冻结受验提交。Mac 的完整语法条件跳过由 Debian 同项实测补充，未记为 Mac 通过。供给中取消测试使用自己的进程组，真实 systemd 清理证据另行读取；没有以此替代管理员取消接口的总验收。

## 资源与清理

两个独立 Debian 单元均在运行期读回：MemoryMax256MiB、MemorySwapMax0、TasksMax64、CPUWeight/IOWeight10、OOMScoreAdjust500、PrivateNetwork/PrivateMounts/NoNewPrivileges=yes、KillMode=control-group。

组合回归峰值71,622,656B/11pids；chroot场景峰值21,327,872B/5pids。两者 memory.events 的 max/oom/oom_kill 为0，按各自 journal 实际启动时刻检查无新内核OOM。结束主/控制PID为0、单元回收，无任务进程、挂载和cgroup；SSH PID406、重启0、同boot、swap0。每套44份输入前后不变，其中26份仓库输入逐份匹配冻结提交；chroot最小根目录已删除。

Rust 专属 PostgreSQL55439 按启动PID和时刻归属停止，确认PID不存在、端口关闭。收据摘要 `ebeeef5efad35f51c1a09f7e98121d8a8e866979cf410ad4f7ea824c5b10aeb8`。Debian组合与chroot收据摘要分别为 `3c3b1e084d46d43c9a826fb0188691eb63afa2b193c5219b01d866e7333a4693`、`8e40a2a7dbfdee0439ba7a0bbcf6de650dcae4b91587d90a7297eaa616eb7bb8`；完整 result/postcheck/driver 摘要见JSON索引。

## 复核与未完成范围

```sh
python3 tools/test-nodequality-loader-policy.py --readonly-upstream-dir <已核验17文件的固定来源目录>
python3 tools/test-nodequality-sources.py
python3 tools/test-nodequality-report-policy.py --readonly-upstream-dir <同一目录> WiringTests
```

只验收本项供给失败传播。rootfs、二级工具来源/授权、cookies/UA/广告、内层上传及第三方探测目标许可仍待解决；完整诊断与持续代理流量并行、小内存/磁盘不足、403/429/超时、Agent重启、面板断连、取消、重复提交、部分报告的全矩阵仍未签收。旧r8日常实机证据不追认为r11完整验机证据。未正式签署、发布或部署；四个工作流继续暂停，CI只登记未来测试命令。
