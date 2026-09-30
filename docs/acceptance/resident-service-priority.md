# 常驻服务优先级独立验收

日期：2026-10-01。对应改动仅包含 Agent 和 sing-box 的 systemd 单元资源优先级。

## 行为与部署路径

`deploy/sinan-agent.service` 和 `plugins/sing-box/sinan-singbox@.service` 均设置：

```ini
OOMScoreAdjust=-500
CPUWeight=1000
```

负的 OOM 调整值降低这两个常驻进程被内核选择为 OOM 牺牲者的优先级；使用 -500 保留内核在极端内存压力下选择它们的空间。CPU 权重采用 1000，为默认 100 的十倍；CPU 争抢时优先分配给常驻服务，空闲机器仍可使用可用 CPU。没有添加 CPUQuota。

安装器的 `@@AGENT_UNIT@@` 和 `@@RUNTIME_UNIT@@` 由 `tools/release.py render-installer` 直接替换为这两个文件的内容，仓库中没有另一份安装器内嵌单元需要修改。渲染出的安装器已逐字核对两个 heredoc 与源单元一致，并通过 shell 语法检查。

升级安装器写入单元后会执行 daemon-reload；已有进程的 OOM 调整应在下次服务启动时核对。此次验收没有重新安装、替换或重启已有 Agent 和运行时。

## 已执行的独立验收

专用验收容器使用 Debian 12 bookworm、systemd 252.39、cgroup v2，内存上限 1073741824 字节，CPU 限额 200000/100000（2 CPU）。所有临时服务操作均在该容器内执行。

1. 记录现有 `sinan-agent.service` 与 `sinan-singbox@main.service` 的 MainPID、ActiveState、NRestarts，两者均 active。
2. 对两个源单元执行 `systemd-analyze verify`。验收环境没有本次签名发布的 Agent/运行时执行路径，因此仅将 `/opt/sinan/...` 的可执行文件路径替换成临时目录内可执行的空脚本，其他单元设置保留。该步骤验证单元语法和引用路径，不执行签名门禁或业务程序。
3. 从两个源单元生成 `/run/systemd/system` 内独立命名的临时单元。仅删除 ConditionPathExists、ExecStartPre、ExecReload，把 ExecStart 替换成 `/bin/sleep 120`、Restart 改成 no；保留 OOMScoreAdjust、CPUWeight，以及运行时的 User、Group、能力和 NoNewPrivileges。分别以 root 和既有 `sinan-singbox` 用户启动短命夹具。
4. 用 `systemctl show` 验证属性，并读取夹具进程 `/proc/<MainPID>/oom_score_adj` 与服务 cgroup 的 `cpu.weight`。
5. 停止并删除仅属于夹具的单元、执行 daemon-reload；核对现有两个业务单元 MainPID、ActiveState、NRestarts 与验收前完全一致。

| 检查 | Agent 夹具 | sing-box 夹具 |
| --- | --- | --- |
| OOMScoreAdjust | -500 | -500 |
| 内核 oom_score_adj | -500 | -500 |
| CPUWeight | 1000 | 1000 |
| cgroup cpu.weight | 1000 | 1000 |
| CPUQuotaPerSecUSec | infinity | infinity |

源单元校验、两种用户的实际属性、已有服务保持运行、夹具清理均通过。构建脚本和签名发布 Python 检查、安装器模板与渲染结果的 shell 语法检查通过；完整 Rust/Compose 检查由该 PR 的 CI 执行。

## 再次验收与边界

只能在专用 Linux/systemd 测试节点重复上述夹具步骤。服务名称需独立，启动前确认临时文件不存在，结束时只清理自己创建的文件和单元。不要把夹具内容写入现有业务单元。

完成正式升级后，分别检查真实服务的以下属性及内核 oom_score_adj：

```sh
systemctl show sinan-agent.service sinan-singbox@main.service \
  -p MainPID -p ActiveState -p NRestarts -p OOMScoreAdjust \
  -p CPUWeight -p CPUQuotaPerSecUSec -p ControlGroup
```

本 PR 证明单元设置可由 systemd 和内核实际应用；完整 NodeQuality、持续代理流量、实际 OOM 压力以及心跳连续性仍需与诊断资源预算、预检和心跳解耦改动一起在专用节点验收。CPUWeight 是相对权重，负 OOM 调整也不保证极端内存压力下服务绝不会被杀。
