# ADR 0017：增加 OpenRC 独立服务管理

## 背景

用户明确要求新增 OpenRC，覆盖原 MVP 排除 OpenRC 的限制。Linux 设备继续使用既有对账、身份和账本；代理运行时必须独立于 Agent，Agent 重启不能终止运行时。

## 决策

- 安装脚本与 Agent 在启动时检测运行中的 init 系统：优先识别 `/run/systemd/system`，其次识别 `/run/openrc/softlevel`。不修改系统 init，不增加配置字段或依赖。
- `SystemServiceManager` 在既有 `ServiceManager` 边界内选择 systemctl 或 rc-service，所有命令仍经过 `Privileged`。OpenRC 将通用服务标识末尾的 `.service` 移除，保留实例名；不在 core 中加入具体运行时名称。
- OpenRC 安装两个独立的 init.d 脚本：`sinan-agent`、`sinan-singbox@main`，加入 default runlevel，并强制刷新依赖树，避免秒级时间戳使新服务未进入缓存。升级仅重启 Agent；运行时由对账首次启动，并可在已有配置时随系统启动。
- 两个服务使用 OpenRC 自带 supervise-daemon，五秒延迟自动重启，日志写入 `/var/log/sinan/`。运行时保留专用非特权用户、绑定低端口的 ambient capability 和 no_new_privs。安装前检查 supervisor 是否提供所需能力选项。
- 运行时 reload 使用 supervise-daemon 向被监督进程发送 HUP，保留已有统计连接的关闭屏障，不误向 supervisor PID 发送信号。PID 文件记录 supervisor，验收通过 Linux procfs 分别记录 supervisor 与实际运行时子进程。

## 验证与边界

覆盖 init 检测、服务命令失败与状态、非法服务名拒绝、安装分支与重复升级。CI 增加真实 OpenRC 服务脚本的启动、重载、异常退出恢复和 Agent 重启独立性检查；该检查使用进程夹具，不能代替真实代理与公网验收。

OpenRC 支持仍限 Linux。Agent 的 musl 制品可用于 musl 系统，但运行时必须另有与宿主 libc、架构及构建标签兼容的制品；现有 glibc 运行时构建不会因增加 OpenRC 自动兼容 Alpine。原 systemd 部署继续支持。

与上游 NodeQuality 功能合并后，完整诊断仍依赖 systemd 的一次性服务与挂载命名空间，见 [ADR 0016](0016-nodequality-diagnostics.md)。OpenRC 设备保留遥测、IP 上报、代理对账及流量计量，Agent 入口不注册诊断适配器；服务管理器明确拒绝诊断任务的启动和状态查询，避免调用 systemd 命令。systemd 设备继续注册并运行诊断功能。

参考：[OpenRC 服务脚本说明](https://github.com/OpenRC/openrc/blob/master/service-script-guide.md)、[supervise-daemon 手册](https://github.com/OpenRC/openrc/blob/master/man/supervise-daemon.8)。
