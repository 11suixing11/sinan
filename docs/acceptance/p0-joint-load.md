# P0 有限联合负载验收

本项补验真实 Agent 心跳、遥测、持续回环代理流量与有限 systemd 诊断夹具的同时运行。它是独立验收项，不解除完整 NodeQuality 门禁，不签收全部 P0，也不发布或部署后续 TCP 能力。执行器为 [tools/p0-joint-load.py](../../tools/p0-joint-load.py)，所有身份都是公开 TEST_ONLY 编译信任根下的私有夹具身份。

## 受测输入和边界

- Agent 固定源码 `b5289a93a85445d0ca2fcb5d5e817f650abef9a3`，版本 **0.3.0**，实际以 `--monitor-only` 运行。SHA-256 `42fbd18aa4589fe8ceaff3e8a99a609fc7c8cbb6767a08a37a7e5a27171783ba`，54,815,352 字节。未以旧 Release Agent 代替本次源码；未认证后续 main 或 Agent 0.3.1。
- ARM64 core 测试二进制来自 `ab13f5e`，SHA-256 `0d3a7647345277b20fe9774a2ccade9bae3643fd221d80cf2a7aa92687b79026`。构建前后逐文件核对本次 Agent 源码未变，core、SDK、protocol 与 ab13 受测源码逐字节一致。
- sing-box 1.14.2 ARM64 ELF 是公开 `agent-v0.3.0` 固定 Release `75cd846f152f61d7b5daa913b31c74579eed3d22` 的原字节，SHA-256 `fee83ca8457c94449dd04aa17a51830cbc9b449a4dda290e995d3366188e0302`，86,063,568 字节。只取得所选 29,531,806 字节 ARM64 归档与四份证明文件；归档 SHA-256 `60d8827a226aa2bb466cb2e2c9e625c984c9a87d67d2a3df9db5b781ba7d6135`。正式公开信任根的完整 minisign 签名、规范 manifest 与所选归档/ELF 身份经独立核验；没有正式私钥、重新签名或运行旧 Agent/NQ。
- Guest 为独立 Debian 12 ARM64、2 CPU、1.5GiB RAM、8GiB 磁盘、systemd/cgroup v2、无 swap、无 host 共享目录。端口规则修正与实际 TCP 隔离另见 [PR #96](https://github.com/theLucius7/sinan/pull/96)。本次运行 boot_id 的原始字节 SHA-256 为 `5a2730c8d35f994be6e7931791fa3e7637cf12043d87875e4bfcc571a3850217`，实际实例配置 SHA-256 为 `5e7bc6afbcaaabffa142085ba41e434a8db0132b66755628441f340d410fd709`。编译在之前的 boot 完成；这里不混用两个 boot 的服务 PID。

替身面板是真实 WS/HTTP 接收器，但没有实际 Panel/PG/UI。monitor-only 没有代理/诊断适配器，运行时由本夹具独立启动；六个 core 夹具通过真实 ServiceManager 创建、停止、读取 systemd 单元。它们不等于 Agent 领取诊断任务，也不证明管理员取消接口、设备取消确认、诊断 checkpoint/outbox 的完整端到端流程。plain VLESS 仅为回环传输夹具，不能认证生产 Reality。完整 NodeQuality、硬件/公网负载、许可/供应链完整链仍待验。

## 限制与执行

Guest 内一次 Agent 构建使用 Rust 1.97.1、固定 git archive、`Cargo.lock --locked`、公开 TEST_ONLY 信任根、jobs=1、incremental=false。编译单元 `MemoryMax=1100M`、`MemorySwapMax=0`、`TasksMax=128`、CPU/IO weight=10、OOMScoreAdjust=500；不安装额外编译依赖，protoc 已 vendored。冻结源目录保持原字节，验收脚本放在独立 harness 目录。

编译前的准备目录权限检查失败后，一次原名 build unit 因不存在 WorkingDirectory 返回 `200/CHDIR`，没有执行 Cargo。保留该失败；修正本任务私有目录权限后的唯一实际 Cargo 构建在 `-v1` 单元成功，约 121.5 秒。重启前完整成功日志已保存；准备失败的 transient 单元原 journal 未来得及复制，不声称已保存。

联合控制器限 192MiB/128 tasks/300 秒，Agent 限 256MiB、运行时限 192MiB，两者 swap=0、CPUWeight=1000、OOMScoreAdjust=-500、RuntimeMaxSec=240s。这两份驻服务是验收 transient 单元，不替代正式安装器单元验收。运行时以 nobody、NoNewPrivileges、空 CapabilityBoundingSet 运行；仅监听 guest loopback，路由只允许 loopback，其他目的地址拒绝。请求目标没有可配置域名或公网入口，每次真实 echo 每方向 1,024 字节、约每秒一次；唯一请求 worker 硬 3 秒 kill/reap。

执行前拒绝优化模式 `python -O`，核对固定源码/二进制、正式运行时核验 receipt、当前 boot 的端口隔离 receipt、无其他诊断、根分区至少 3GiB 可用。先列举六个夹具，再打开监听。

```sh
sudo systemd-run --unit=sinan-p0-joint-controller-<本次唯一名> --wait --pipe \
  --property=MemoryMax=192M --property=MemorySwapMax=0 --property=TasksMax=128 \
  --property=CPUWeight=10 --property=IOWeight=10 --property=OOMScoreAdjust=500 \
  --property=KillMode=control-group --property=RuntimeMaxSec=300s -- \
  python3 <私有 harness>/tools/p0-joint-load.py \
  --agent <冻结 Agent> --core-tests <ab13 core-tests> --inputs <built-inputs.json> \
  --runtime <正式核验的 ARM64 ELF> --runtime-receipt <runtime-verification.json> \
  --isolation-receipt <port-rule-runtime.json> --output <新的私有证据目录>
```

1. 至少三个实际 WS Heartbeat，采样跨度至少 39 秒；预定 20 秒心跳的连接内间隔上限 30 秒。
2. 持续 echo 中串行执行唯一一次 `real_systemd_diagnostic_ --ignored --test-threads=1 --nocapture`，要求 6 passed/0 failed/0 ignored；至少一个实际心跳在夹具阶段收到。baseline 和此阶段必须保持同一个 WS session，不以意外重连掩盖长间断。
3. 暂停 telemetry ACK，实际 Agent 重启，核对原采样 ID 重放与收到的 `sampled_at`；最终接收器在每次 POST/ACK 前记录原值，重复 ID 的时间戳不得改变，运行时 PID 和流量不中断。原联合 v1 接收器未记录重启前未 ACK 三条的原始时间戳；后文单独小补验证明时间值，原 v1 记录不改写。
4. 显式关闭 WS、HTTP 返回 503 共 10 秒，随后要求实际心跳恢复，再等待下一个真实心跳；完整原始全局间隔保留，故障期间的间断不混入连接内正常间隔。
5. 每 100ms 观察本次诊断单元、cgroup PID/内存事件；对每个观察到的 UUID 单元及两个驻服务分别停止、读取 PID/cgroup，核对挂载、临时目录和内核 OOM 差异。某一步清理失败继续其他清理并保留原失败，清理完成后才能判为成功。

## 本次结果

2026-10-01 03:45:39–03:47:42 UTC，修正端口规则后的同一 boot 上执行一次，控制器退出码 0，systemd runtime 123.117 秒。

| 项目 | 实际结果与限制 |
| --- | --- |
| 有限 systemd 六夹具 | 6 passed / 0 failed / 0 ignored，2.28 秒；取消及私有挂载清理、锁权限/inode、64MiB OOM 与 TasksMax、queued-start、manager 重建/超时、资源读回/同机互斥 |
| 真实 Heartbeat | 8 次；baseline 3 次跨度 40.0324 秒；baseline/诊断阶段同一 WS session，夹具阶段实际收到 1 次；连接内最大间隔 20.0334 秒，低于预定 30 秒 |
| 全局心跳间隔 | 原值 19.9590 / 19.9990 / 20.0334 / 19.9661 / 3.8504 / 18.1245 / 19.9585 秒；显式重启/断连均保留，没有剔除原始全局间隔 |
| 遥测 | 120 次 ACK 成功、120 个独立采样 ID，原 `sampled_at` 范围 1790826339517–1790826459181ms；正常 baseline 最大收样年龄 1.5599 秒、诊断阶段 1.4111 秒、故障恢复积压最大 16.4600 秒；不把接收时间当采样时间 |
| Agent 重启 | 主 PID 767→1951；停 ACK 后 3 个旧采样 ID 重放成功；本次 v1 未保存旧原始时间值，时间逐值对照由独立小补验提供；手动重启不能从 NRestarts=0 推断没重启 |
| 明确面板断连 | WS 关闭与 HTTP 503 共 10 秒，新 WS session 与实际心跳恢复；未模拟真实面板数据库/服务器重启或非回环链路 |
| 持续真实 VLESS echo | 113 次、0 失败，每方向 1KiB，约每秒一次；baseline 73、诊断 3、Agent 重启 3、面板断连 16、恢复 18 次；运行时 PID 765 全程保留、NRestarts=0 |
| 驻服务与保护 | 初始/夹具后 Agent PID 767、运行时 PID 765 均不变；读回 CPUWeight=1000、OOMScoreAdjust=-500、swap=0；SSH PID 406、NRestarts=0 保持。它们是测试单元而非正式安装器验收 |
| OOM 与清理 | 内核差异只有诊断 64MiB cgroup 的 CONSTRAINT_MEMCG OOM，杀死夹具 python3 PID 1706、oom_score_adj=500；无 global/驻服务 OOM。30 次观察覆盖 9 个诊断单元；结束 MainPID/ControlPID=0，无观察到的存活 PID、诊断单元、夹具挂载或公开临时目录，两个 worker 线程已退出 |
| 现场复核 | 无 sinan loaded units、无 Cargo，根分区可用 3,989,807,104 字节；swap 表仍空；证据与旧基线目录保留 |

六项完整名字按二进制 `--list` 读回，执行记录见私有 `core-systemd.log`。退出后诊断单元已由各夹具清理，控制器补 stop 返回 5 的条目同时读回 `LoadState=not-found`、两个 PID=0；记录为已不存在，没有伪造 stop 成功。退出后 systemd GC 的默认预算值不能代替运行中属性；本项保存运行中 controller 与驻服务属性。

实际执行的 controller SHA-256 `fa914138347269cbf9eac86489f64cd6332735a530a4af308c023fb5a51c7c2e`。私有完整结果 `result.json` SHA-256 `f87883114128382e0f0badb8ee6c2ca69dcf433ef4277b3323ccc406aee26ef1`；脱敏数字汇总 `joint-summary.json` SHA-256 `6b00e98d8a1bddffbc4237c84e10afcbdfebcd1d4e0f95d105659b72e3781019`。受测辅助 `agent-smoke.py`、`native-service-smoke.py` 与固定源码同字节；本次只借用它们的标准库 loopback 接收器/传输函数，没有运行其安装/升级主流程。

## 审查后的窄补验

保留 v1 文件及 SHA 不变，不重跑六夹具或代理联合负载。最终判据另要求：恢复后固定 WS session 至少两个真实心跳，跨度至少 19 秒；任何未计划重连拒绝。原 v1 完整事件/心跳事后核证仅三个预期 session，恢复 session 3 已收到两次心跳、跨度 19.9585 秒，满足判据。原五条 OOM 提示（含 `oom_kill_process` 栈）完整归到同一诊断 MEMCG 事件与 PID 1706；新检查对只有孤立 OOM 提示、缺 constraint、全局 OOM 或 PID 不一致均返回 unknown/disallowed，不以 `all(empty)` 通过。

接收器新增每次 gzip POST 在 ACK 判定之前的 ID/原采样时间记录，503 语义不变。2026-10-01 03:59:25–03:59:32 UTC，单独 `--replay-only` 启动受限 monitor-only Agent，不启动运行时、不执行任何 core 夹具，实际 systemd runtime 7.349 秒、退出码 0。Agent PID 2472→2496，三个真实收到 503 的旧 ID 在重启后恢复，`sampled_at` 原值与重放值分别逐值相同：`1790827168019`、`1790827169024`、`1790827170030` ms。结束 Agent 单元 not-found、两个 PID=0、无 cgroup，无 sinan 单元或代理运行时；SSH 仍 PID 406、NRestarts=0，窄补验内核差异没有 OOM 提示。这只是补足遥测时间重放取证，不增加心跳/代理/诊断场景通过次数。

窄补验执行脚本 SHA-256 `d27d3acf777c37d7b7e24644aa0b5d96d00711a22dc44f13ae0b5375fef9ca40`，结果 SHA-256 `e9a797a014c4702bfd4bdc8ba0ac7e99af3057834b8a78d5b8e62ad8e4d36f42`；之后增加恢复心跳跨度下限，并修复清理确认读取异常必须返回未确认：即使两个 PID=0，只要 cgroup 进程读取失败也拒绝 clean。两项修改以行为负例和原证据复核验证，未重跑。两次原结果与原脚本分别保留，最终判据对 v1 原证据复核另存 receipt。

最终提交脚本 SHA-256 `28383c62556c203279a11ee947b788e2b127da462691010c6cd657e6e971c235`；其对 v1 证据的只读复核 receipt SHA-256 `a2d015dd09117712862e9779cdc1bd4d0ac828e35dcdf1183b1e93ba8ee7c8b7`。原 v1 联合执行、窄 v2 重放执行与最终静态判据验证分别记录，不把后续脚本修改称作已重新执行完整联合场景。

行为回归 `python3 tools/test-p0-joint-load.py` 共 14 项通过：实际心跳跨度、遥测不能替代心跳、显式故障全局间断保留、代理失败保留、意外重连拒绝、优化 worker 拒绝、采样重放时间不变、端口隔离 receipt 对应当前 boot、恢复 session 两次心跳与实际周期、真实 POST/503 后改值重放负例、未知/全局 OOM、完整事件/PID 归属、清理失败继续检查下一单元、PID=0 不能掩盖 cgroup 读取失败。Python 语法、core 分层与差异检查通过。本项没有修改 Rust 产品代码，不重复 Rust/PG 编译、正式签名或 GitHub CI；四个 workflow 保持 disabled_manually。

私有证据保留在 guest `/home/l7.guest/sinan-joint-load/evidence/` 与 host 本任务私有 evidence 目录。状态数据库、身份、配置与原始日志不提交仓库；公开文档只保存摘要、数字和明确的证据边界。
