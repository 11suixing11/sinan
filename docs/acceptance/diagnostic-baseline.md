# 诊断整改基线与独立验收

基线代码为 `be8b792`。所有整改归入 [诊断安全与插件边界整改 milestone](https://github.com/theLucius7/sinan/milestone/1)。每个需求项单独 PR，依赖项采用可审阅的顺序分支；不将不同需求压入一个提交。每个 PR 给出适用的自动测试、故障矩阵、实机证据和未完成边界。

## 已实测

2026-10-01（Asia/Taipei）在实际面板容器网络命名空间中，用已有 Debian 12 镜像的标准 curl 发起一次默认查询及一次 `db=ipqualityscore` 查询。辅助容器只读、无 capabilities，64 MiB 内存、16 tasks、0.25 CPU，不改 UA、不重试、不关闭 TLS。默认查询结果：HTTP **403**，DNS 0.136800 秒，TCP 0.141125 秒，TLS 0.179850 秒，总计 1.032817 秒。DNS、连接和 TLS 均已完成；失败发生在 HTTP 层。第二种响应形状同样 HTTP **403**，总计 0.111477 秒；不把同一入口的不同 db 参数声称独立 provider。

现有专用测试容器确认 Debian 12，cgroup 限制 1 GiB、2 CPU，Agent 和运行时均 active；磁盘仍共享生产宿主。因此该容器只用于受控小型夹具，不执行完整硬件压测。完整 NodeQuality 的“资源耗尽/心跳延迟/仅测试失败”分类尚未复现，不能仅从代码缺预算推断已经 OOM。

旧 `Metrics` 没有采集时间，服务器记录仅有 `last_seen`。基线脚本明确记录指标时间未知，不能用心跳时间替代。后续心跳/遥测 PR 必须补采集时间和过期显示。

## 采集工具

在专用节点执行测试前、测试中及结束后采集：

```sh
python3 scripts/diagnostic-baseline.py --host sinan-test --output /private/evidence/before \
  --panel-origin https://panel.example.com --server-id 123 --cookie-file /private/session-cookie
```

`sinan-test` 是 SSH 配置别名。cookie 文件为完整 Cookie header，只允许当前用户读取，不进入参数值或输出。输出目录必须是新目录，权限 0700，文件 0600。可加 `--samples 12 --interval 10` 记录过程；若只做容器夹具，加 `--container disposable-test`。脚本只读取 cgroup、内核 OOM、磁盘、内存/负载、服务 PID/重启数、诊断单元和挂载，并读取面板白名单时间字段；它不启动测试、不重启服务、不修改宿主。

完整验机前必须核对独立机器的系统版本、磁盘空间、内存、用途和现有服务。仅向自己控制的流量目标发送受控载荷。测试期间同步采集，比较 Agent 心跳间隔与采集时间、运行时 PID、NRestarts、OOM 日志、磁盘和挂载。取消确认后比对进程组与挂载消失，保留已经完成章节。内核日志不可读或虚拟化不暴露 OOM 时，记录观测缺口。

## 通用故障矩阵

| 场景 | 必须保留的证据 |
| --- | --- |
| 小内存/磁盘不足 | 预算、预检拒绝原因、未启动服务；运行中阈值停止 |
| 查询 403/429/超时 | 每个 provider 分类、时间、耗时、上次成功未覆盖 |
| Agent 重启/面板断连 | 原任务 ID、无重复执行、恢复回报、心跳/指标分开 |
| 取消/重复提交/部分报告 | 等待确认状态、设备确认、无残留、章节仍可见 |
| 持续代理流量 | 受控载荷、运行时 PID/重启数、计量连续、诊断预算 |

对单独 PR 不适用的场景解释边界；所有场景在综合专用节点验收再执行一次。文档中的操作步骤不算通过证据。
