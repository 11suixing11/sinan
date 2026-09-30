# 完整 NodeQuality：小内存节点 OOM 基线

对应第 0 步“复现状态异常并分类”，独立于节点准备、资源预算和预检实现。本次提交只记录实测，不修改诊断行为。

## 环境和复现步骤

使用已明确转为专用测试用途的 Debian 12 节点：物理内存 447 MiB、约 12 GiB 剩余磁盘、已有 2 GiB swap，原代理业务已停止和禁用。Agent/面板为主线 `541f52d` 的 0.3.0 GNU 调试构建，运行时为 1.14.2，NodeQuality 入口为 `a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2`；使用独立 TEST_ONLY 签名安装和私有测试面板，没有运行启动预检 PR 的新代码。

1. 确认 Agent 和 sing-box 在线且配置健康，记录 Agent 子进程、监督进程和运行时 PID，以及 systemd NRestarts、OOMScoreAdjust 和 CPUWeight。
2. 开始记录 `systemd-cgtop --batch --iterations=1 --depth=4`、`journalctl -k` 的 OOM 条目、`df -h`、MemAvailable/负载、单元状态和挂载；同步记录面板 API 的最后设备消息与任务状态。
3. 管理员调用 `/api/servers/{id}/node-quality/reports`，参数为 `ip_version=ipv4`、`network_mode=low`、`upload_report=false`。此入口依然启用硬件测试，低流量选项只限制网络部分。
4. 任务启动并进入 Geekbench 后观察到资源异常。SSH 恢复后取回内核日志，停止该诊断单元并核查进程和挂载。重新连接私有面板隧道，等待 Agent 正常回报终态，没有重启 Agent 或运行时。

## 资源与结果

| 项目 | 实测结果 |
| --- | --- |
| 提交预算 | MemoryMax=512 MiB、MemorySwapMax=0、TasksMax=128，诊断 OOMScoreAdjust=500 |
| 常驻保护 | Agent 和 sing-box 的 OOMScoreAdjust=-500、CPUWeight=1000 |
| 资源异常 | 内核记录 `global_oom`，受害进程属于诊断单元的 Geekbench |
| 被杀进程内存 | anon-rss=275192 KiB，oom_score_adj=500 |
| 单元终态 | Result=oom-kill，MainPID=0；Agent 回报 failed，error 含 result=oom-kill / status=143 |
| 常驻服务 | Agent 子进程、监督进程和运行时 PID 均与开始前相同，NRestarts=0，恢复后连接和配置健康 |
| 报告 | report=null；本次没有生成可用完整报告，不能记为完整验机成功 |
| 清理 | 诊断 cgroup 内 PID 列表为空；Geekbench/fio/iperf3/nexttrace 无残留，host findmnt 无 BenchOs/.nodequality 挂载 |

因此，主分类为“资源耗尽 / 全局 OOM”，并伴随面板离线和指标停更，不能归为仅测试失败。默认 MemoryMax 高于整机物理内存：单个 cgroup 的有限上限仍可能先触发全局 OOM，必须结合启动前可用内存和保留预算判断。

## 心跳与指标证据边界

旧面板的 `last_seen` 是最后设备消息，界面称“最后消息”，并非专用心跳时间；没有独立 `last_heartbeat_at`。数据库已有毫秒级 `metrics_sampled_at`，但该基线 API/页面未暴露指标时间。因此分别记录最后设备消息、未知的专用心跳时间，以及私有测试数据库中只读取得的真实指标时间，不用消息时间替代采集时间。

面板采样中有 66 个离线样本；其保存的当前指标时间在采样记录中最大跳过 319.014 秒。测试连接通过 SSH 隧道，资源压力期间节点 SSH 和该隧道均失去响应；这个空窗包含测试传输链路断开，不能当成纯心跳停顿时长，也不能证明 Agent 停止采集了相同时间。原始日志和截图只保存在私有目录，没有节点地址、令牌或完整报告内容进入 Git。

## 独立验收与下一步

已完成：一次完整硬件入口提交、OOM 实机复现和分类、最后设备/指标时间的证据与未知边界、常驻服务生存及诊断停止后的清理核查。此处停止通过 systemd 完成，不能作为未来“管理员接口—协议—设备确认”取消功能验收。

启动预检 PR 应在这台节点拒绝默认完整诊断并解释内存不足；低内存运行保护、独立心跳缓存、确认式取消、部分报告及日常/完整入口分别验收。磁盘不足、403/429/超时、重启/断连、重复提交和持续代理流量尚不因这一次基线复现而自动通过。后续任何完整验机运行须先满足资源预算，不能反复重演无预检压力。
