# 设备平台与能力

设备继续使用独立 Agent 和代理运行时服务。签名发布、bootstrap 信任根与首次接入步骤见 [部署文档](deploy.md) 和 [发布文档](release.md)；平台扩展不能跳过这些校验。

## 平台与服务

| 平台 | Agent | 服务管理 | 运行时 |
| --- | --- | --- | --- |
| Linux glibc amd64/arm64 | musl 静态或 GNU 动态 | systemd / OpenRC | 对应架构 glibc |
| Linux musl amd64/arm64 | musl 静态 | OpenRC | 对应架构 musl |
| macOS arm64 | 原生 | launchd | 对应架构原生 |
| FreeBSD amd64/arm64 | FreeBSD 13 sysroot 构建 | rc.d / daemon | 对应架构原生 |
| Windows amd64/arm64 | MSVC 静态 CRT | 计划任务 | 对应架构原生及已签 DLL |

init 与 libc 分别选择。Alpine 不能执行 glibc 运行时；每个制品必须对应 Agent 上报的平台和架构。Linux musl 制品保留原目录兼容，其余制品使用完整平台标识。日常 CI 的 Agent 使用公开 TEST_ONLY 信任根，制品名称带 TEST_ONLY，禁止用于正式节点或发布；正式发布工作流当前生成 Linux 双架构六个组件制品；原生平台的生产签名 bundle、独立来源核验与发布验证须另行完成。Linux bootstrap 的静态安装器不代表原生平台已经具有相同的自动首装入口。

原生生产安装先独立验证签名、metadata 和待执行 Agent 的实际内容，将 proof 三文件保存在版本目录后再注册服务；`install-service` 在任何账户或服务修改前以编译根复验自身，并验证安装后的副本。缺少匹配 proof 的生产接入保持拒绝。

Unix 默认配置 `/etc/sinan/agent.toml`、Agent `/opt/sinan/core`、状态 `/var/lib/sinan/core`；macOS/FreeBSD 状态套接字 `/var/run/sinan/agent.sock`。Windows 默认根目录 `%ProgramData%\Sinan`，本机状态使用受保护命名管道。`agent_root` 与代理 `install_root` 分开配置，`run --monitor-only` 用于监控设备。

## OpenRC

OpenRC 安装要求 `supervise-daemon` 支持 capabilities/no_new_privs，并安装 CA、curl、基础账号/文件工具与 `unshare`。安装后将两个服务加入 default runlevel，刷新依赖树。Agent 与代理分别监督，日志在 `/var/log/sinan/`；更新只重启 Agent，reload 将 HUP 发送到实际代理子进程。

NodeQuality 使用独立一次性服务、独立挂载命名空间、超时与持久终态；Agent 重启不重启已开始的诊断。OpenRC CI 同时执行进程夹具及真实 Agent 诊断服务检查；公网 NodeQuality 性能结果仍须独立验收。详见 [ADR 0021](adr/0021-openrc-services.md)、[ADR 0022](adr/0022-agent-capability-alignment.md)。

## 监控、任务与更新

系统指标默认一秒采样、三秒上传，可调整；离线样本持久保留最多一天且不超过 100000 条。SWAP、进程、磁盘 I/O、GPU 和公网地址按设备能力提供，缺失数据保持缺失。公网地址识别可本地关闭。

持续 TCP/ICMP 拨测最多 32 项，间隔 10–3600 秒，离线继续使用的配置最多一天；结果最多保留两小时、4096 条。管理员远程命令使用设备服务账号和系统 shell，最多运行 600 秒；输出分别限制 256 KiB。执行前持久记录，重启后不重放未确认终态的命令。

自动更新默认关闭，仅使用绑定面板内协议兼容且通过发布签名的稳定制品。监督进程与运行时分开，更新保留身份和账本；启动检查失败或中断未确认升级后回退到上一版，不重复尝试已失败版本。Windows 目录元数据断电刷盘仍属验证边界。

FreeBSD 的代理使用普通账户，默认应选择 1024 以上的节点端口。低端口权限由管理员配置系统策略，安装器不修改全局端口授权。

## 验证边界

CI 验证 Linux 双 libc / 双架构、macOS ARM64、Windows 与 FreeBSD 双架构。FreeBSD 使用同一二进制在 13.5、14、15 验证；原生任务检查注册、遥测补报、命令去重、拨测、升级回退、服务与回环代理流量。成功结果须以合并提交的 Actions 为准；已有分支结果不能证明合并后状态。

GPU 实际负载、公网 Reality 客户端、真实 NodeQuality 性能和整机断电/重启由专用设备验收，见 [验收文档](e2e.md)。
