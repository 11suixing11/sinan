# ADR 0018：补齐 Agent 监控、任务、升级和多系统部署

## 背景与范围

用户在核对本地 NodeFlare 后明确要求「全部差距一起补齐」，包含此前仅提供编译产物的 macOS、FreeBSD、Windows 常驻部署，以及 Agent 自动更新、通用远程命令。此要求覆盖原 MVP 对非 Linux 运行、自更新和高频监控的排除项；代理协议、单实例、三层依赖、特权 trait、独立运行时、不可变制品和中文界面约束保留。

## 决策

- sing-box 继续固定上游 1.14.2、原样源码、平台对应的官方默认标签和 `with_v2ray_api`。Linux 增加官方 `with_musl` 与 `build-naive --libc=musl`，同时保留 glibc 构建；按平台、libc 和架构选择制品，不将 init 等同于 libc。
- 系统遥测采用一秒采样、默认三秒批量上传；补充 SWAP、进程数、磁盘 I/O、GPU、公网地址。缺失指标保持缺失。持久 outbox 限制大小与保留期，面板按样本身份去重；迟到样本不能倒退当前指标。
- 持续 TCP/ICMP 拨测与通用命令通过认证面板配置和下发，任务具有期限、超时、输出上限与持久终态。重启后不重复执行状态不明的命令，结果确认后才清理。所有外部命令继续经过 `Privileged`。
- 自动更新只从绑定面板下载已校验的对应平台制品，配置可关闭；保留身份和账本，服务监督独立于代理运行时。更新前保存恢复信息，启动验证失败自动恢复旧 Agent，避免反复更新同一失败版本。
- Linux 使用 systemd/OpenRC；macOS 使用 launchd；FreeBSD 使用 rc.d；Windows 使用计划任务和受保护状态目录。各系统均保持 Agent 与代理服务独立，状态查询仅开放本机且有访问保护。NodeQuality 维持 Linux 适配器，增加 OpenRC 任务监督；不将该 Linux 外插宣称为跨系统插件。
- 优先使用已有依赖、标准库和系统工具，新增依赖须另行说明。协议主版本保持 1，通过新增消息和能力声明兼容旧 Agent。

## 实现阶段与验证

1. Linux musl/glibc 运行时制品与自动选择、安装失败恢复。
2. 扩展遥测、可配置批量上传与持久补报。
3. 拨测、命令、OpenRC 诊断和面板管理入口。
4. 多系统注册、常驻运行、独立服务、安装和状态查询。
5. Agent 自动升级、故障回退和完整 CI 验证。

每阶段执行格式、Clippy、完整 Rust 测试及与变更对应的专项检查。CI 验证平台原生运行和服务安装；真实代理、公网 Reality、硬件 GPU 和整机重启结果单独记录，不将进程夹具等同于实机验收。

参考：[sing-box 官方源码构建](https://sing-box.sagernet.org/installation/build-from-source/)、[cronet-go 工具链](https://github.com/SagerNet/cronet-go)。
