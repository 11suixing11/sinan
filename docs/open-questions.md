# 执行中的问题与选择

## 2026-09-30：验证环境

当前工作机是 macOS；使用已有 Rust stable 1.97.1 和独立的本地 PostgreSQL 16 运行测试。systemd 和 Debian 12 运行链路另由 Linux CI 和实机脚本验收，不能把本机测试视为实机完成。

## 内部依赖边界

“agent-core 只依赖 adapter-sdk 和 protocol”指 workspace 内部依赖；基础库仍使用规定技术栈。适配器内部只依赖 adapter-sdk。

## 本地文件读取

系统曾把刚创建的部分文件标记为 dataless，读取超时。仅按本次已知内容原子重建新文件；后续编辑优先原子替换，保护已有数据。

## 开源许可

文档未指定许可证；采用 AGPL-3.0-only，适用于自托管网络服务。发布时保留依赖原有许可证。

## 无可用授权时的订阅

links 格式返回空文本的 Base64（空字符串）；singbox 格式返回“无已应用节点”的错误，避免生成空 selector 或悄然走直连。跨服务器节点允许相同端口；同一服务器内禁止端口重复。

## G4：接入、制品和历史记录

- 安装尚未产生设备 session，增加 `/api/bootstrap/{version}/{arch}?token=...`，只允许有效未消费注册 token 下载 Agent，下载不消费 token。运行时制品与配置包仍要求设备 Bearer 会话。
- 制品保存为 `artifacts/{agent|sing-box}/{version}/{amd64|arm64}`，每个版本目录使用 SHA256SUMS 列出架构文件哈希。面板校验文件与清单匹配才提供下载。
- 同设备升级可用新 token 再次注册同一公钥；已有服务器不允许替换成另一公钥。安装脚本先用暂存二进制完成注册，再激活版本。
- 服务器、节点和用户采用软删除，保留流量与部署历史。增加 usage_batches 表保证同一批次编号不能以不同记录集合重复入账；usage_records 仍保留规定唯一约束。
- 管理员会话 24 小时、设备会话 1 小时。管理员密码只在首次创建时生效，密码校验并发限制为 4，避免匿名请求同时分配无上限 Argon2 资源。

## G4：本地 Rust 链接器兼容

macOS 27 的动态库加载器暴露了 Rust/LLVM 删除调试信息后的 LINKEDIT 对齐问题。dev/test 显式设置 strip="none" 后编译通过，保留 debug=0 控制磁盘使用。参考上游问题：https://github.com/rust-lang/rust/issues/157750 。Linux 部署不依赖此工作机环境。

## G5：通用契约与运行恢复

- `Privileged`、`ServiceManager` 的 trait 定义放在 adapter-sdk，core 的 system 模块重导出并实现，避免具体适配器反向依赖 core。
- 注册身份额外持久化面板 origin，重试沿用同一设备密钥；密钥和目录在网络请求前落盘，禁止把既有身份意外注册到另一面板。
- 本地 status 返回 JSON，包含连接、模块版本、健康和待确认批次数；先占用私有 socket，再恢复状态，防止重复启动同时恢复同一意图。
- 计量序号与累计计数在 SQLite 中保存为十进制文本，支持协议的完整 u64 范围；每次采样的基准与 outbox 在同一事务提交。未知 ack 不改变账本。
- 断线只替换认证会话，对账与计量 worker 持续运行。相同已应用版本仍重新发送健康结果，修复回报丢失造成的面板状态落后。
- 制品同版本不可覆盖；缓存同时核对归档与已安装二进制的哈希。若首次安装在写入缓存元数据前中断，安全拒绝复用，需删除尚未激活的不完整版本后重试。
- 失败恢复优先重启旧版本以兼容跨内核回滚。未能读到重载前终值时取消应用；外部重载或异常终止仍可能产生无法观测的短缺失窗口，日志明确记录，已落盘批次不会丢失。

## G6：原生统计与重载边界

- 原样保留上游 proto 的 package；上游运行时实际注册服务名与 proto 不同，因此显式调用 `/v2ray.core.app.stats.command.StatsService/QueryStats`，使用 `patterns` 和 `reset=false`。
- `systemctl reload` 只保证发出 HUP。适配器在重载前建立已完成 HTTP/2 握手的统计连接，并等待该旧连接关闭，确认旧运行代已终止后才让 core 开启新计量周期；健康检查再等待新的端口和统计 API。
- 适配器支持稳定版 1.14.x，要求实际二进制版本与清单完全一致且包含统计标签。当前面板固定发布 1.14.2；没有扩展运行时版本选择界面。
- 首次成功应用或版本变化后补发静态信息，使面板及时看到运行时版本，无需等待下一次连接。

## G7：发布、已应用订阅和历史计量

- `dirty_at` 使用 PostgreSQL 时钟的 Unix 毫秒，精确等待至少五秒；模型修改与标脏在同一事务中，发布时锁定服务器，避免漏掉并发变更。启动后的发布器继续处理数据库中遗留的脏标记。
- 节点不支持迁移服务器或手动改端口；端口在锁定服务器后从默认范围取最小空位。节点私钥只保留于数据库、部署快照和设备配置包，不出现在管理列表或发布历史响应中。
- 配置包哈希相同时不创建 revision，仅刷新该版本的订阅元数据。订阅使用已应用且健康的快照，并与当前未删除授权的 UUID 取交集；撤销立即从订阅移除，重新授权要等新凭证成功应用。
- 认证心跳可修复丢失的成功回报，但只接受本服务器已发布且更高的 revision；不倒退已应用版本，不抹去最新失败目标的错误。
- 流量允许已经撤销或删除用户的历史统计名重发，前提是该身份曾为此服务器发布；整批事务提交后才确认。同一批次编号更改内容会被拒绝，记录顺序变化不影响去重。
- PostgreSQL 计量字段改用 NUMERIC(20,0) 保存协议完整 u64；汇总可超过 u64，API 使用十进制字符串传递，避免浏览器数字精度损失。

## G8：前端构建与路径权限

- 使用 hash 路由，不增加路由或组件库；未知 API 路径和缺失资源保留 404，避免被页面回退掩盖。
- 提交体积较小的 `web/dist`，使新检出的仓库可以直接 `cargo build/run`。CI 使用 lockfile 重新构建并检查产物一致，修改前端必须同步提交构建结果。debug 与 release 都使用内嵌资源。
- HTML 禁止缓存，带内容哈希的 JS/CSS 长期缓存；静态资源带正确 MIME 和 nosniff。
- 审查发现自定义身份目录或 status socket 父目录可能指向已有共享目录。改为仅为新建目录设置私有权限，保留已有目录权限；密钥和 socket 仍保持 0600，并验证不安全目录条件。

## G9：部署与构建边界

- Compose 使用 PostgreSQL 16 和非 root 面板容器，分别使用命名卷；数据库不映射宿主端口，面板默认只映射到宿主回环地址。公开地址和密码由部署者提供，数据库密码示例使用十六进制，避免 URL 转义歧义。
- 安装脚本将既有 TOML 的解析、路径覆盖保留和设备 origin 校验交给 Agent 注册命令，不使用精确文本 grep 判断面板地址。成功注册后才原子写配置并切换 Agent；升级需要为同一服务器重新生成一次性安装命令，已消费的旧命令不可复用。
- 上游 `release/DEFAULT_BUILD_TAGS` 包含 `with_naive_outbound`，保留全部标签的 Linux 构建需要 CGO/glibc 和上游 Chromium 工具链。固定 Linux/amd64 构建容器，交叉输出 amd64/arm64；只追加 `with_v2ray_api`，不追加 purego/musl 标签，不修改上游源码。只有 Agent 要求 musl 静态链接。
- sing-box 固定 v1.14.2 及对应提交，使用上游指定 cronet 工具链版本；GPG keyring 使用临时私有目录，避免影响构建者的默认密钥环。归档仅包含一个可执行文件，固定归档元数据；同版本同架构产物不可覆盖，添加另一架构时保留并核验既有校验和。
- 本机为 macOS 且无 Docker；本地已启动真实 PostgreSQL、面板和浏览器，并验证真实上游运行时 check/统计/HUP。Docker 部署和 Linux musl 构建通过 GitHub CI 验证，结果在最终报告单列；真实 Debian systemd、公网 Reality 客户端与网络条件仍需执行手工验收。

## 后续调整：使用 Bun

- 按用户要求，将前端依赖管理与脚本运行统一为 Bun 1.4.2；本地开发说明、Docker 前端构建阶段和 CI 固定同一版本。
- 从已有依赖锁迁移到文本 `web/bun.lock`，验证后移除旧锁文件；自动构建使用 `bun install --frozen-lockfile`，不顺带更新前端依赖。
- TypeScript、Vite 的开发、构建和预览脚本显式使用 Bun runtime，避免工具的 Node shebang 导致隐式回退；React、Vite 和 Rust 内嵌前端的结构保持原样。

## 用户后续需求：NodeQuality

- NodeQuality 上游没有指定 IP 的质量查询接口，提供的是测试编排与报告上传。按其使用的 IPQuality 源码，面板查询 `ipinfo.check.place` 已核实的七个数据库，显示独立来源和原值。网站的访问者 IP 接口不能用来冒充节点 IP 质量。
- Agent 通过已有 sysinfo 获取网卡 IPv4/IPv6；NAT 公网地址可通过 `public_ips` 配置补充。未引入任意 IP 回显网站或猜测反向代理请求头，避免误将面板访问者、代理或出口当成节点地址。
- 查询由管理员手工刷新，缓存一天；非公网地址不发送给外部服务。各源错误独立保存；本机实际查询 403，因此公开记录在线服务可用性未验证，测试夹具只证明接口解析及错误处理。
- 新需求明确要求节点实际运行完整报告，采用独立 systemd 一次性服务和诊断适配器，不混入代理配置对账。主程序固定上游提交并由面板分发；测试外插运行时访问上游依赖和测试站点的必要例外记录于 ADR 0016。
- NodeQuality 没有 `-y`，正常清理也可能 exit 1。包装器使用固定交互输入，保存完整本地结果，检查实际报告后才以成功结束；只有在线链接或日志不能视为完整报告。上传失败仍保留本地报告，链接为空。
- 默认低流量、双栈，硬件/IP/网络/回程项目只在点击后执行；30 分钟超时，独立挂载命名空间和进程组终止。每台节点串行任务，启动前持久 checkpoint，启动边界不确定时检查原服务而不重跑。
- 结果在本地持久后重传，确认后再清 outbox。面板主动过期与 Agent 真正终态分别记录，允许连接恢复后的最终报告保存；晚到 running 和重复终态不能倒退结果。已过期但尚未启动的任务不会在重连后突然运行。
- workspace 版本升到 `0.2.0`：新增 Agent 能力需要新的不可变制品版本，不能用不同内容覆盖已经发布的 `agent/0.1.0`。协议主版本仍为 1，旧 Agent 的遥测、代理配置及计量继续兼容。
- 上游 NextTrace 下载写死 amd64；在 ARM64 上通过包装器只映射精确的官方资产 URL，保留 NodeQuality 源码原样。上游存在未引用的工作路径，因此拒绝包含空白和通配符的自定义外插目录，避免错误展开。
- 面板保留最多 256 KiB 报告文本并明确截断，节点保留原始 ZIP；上传响应流最多读取 64 KiB，包含 chunked 响应，避免上游服务无界写盘。
- 延迟下载、准备或恢复不能延长任务绝对截止时间；启动服务时取配置时限与剩余时间的较小值，截止后恢复仍在运行的服务会被停止。已经完成但晚到的报告仍可回传。

## 后续调整：Agent 多平台编译产物

- 用户明确选择仅增加可下载的 Agent 编译产物。增加 Linux glibc、macOS arm64、FreeBSD 与 Windows 双架构构建；非 Linux 的部署命令继续明确要求 Linux/systemd，避免把编译支持描述成设备部署支持。详见 ADR 0015。
- glibc 固定 Ubuntu 24.04 双架构的动态库；macOS 使用最新 arm64 runner，Windows 使用 Visual Studio 2026 双架构 runner，工具链均为最新 Rust stable。Actions 工具固定到本次核实的最新稳定版本。
- “FreeBSD 13 以上”采用 13.5 基线构建，再验证同一产物在最新 14/15 系列 VM 的启动；早期 13 小版本及未来主版本不做未经验证的兼容承诺。FreeBSD 构建使用系统 `protoc`，其他平台保留 vendored 方案。
- 原 Linux musl 制品可继续导入面板。新增制品按完整 target 分目录，附 `SHA256SUMS`；要替换为 Linux glibc 设备制品，需将对应二进制复制为面板期望的架构文件，并重新生成目录内校验清单。

## 后续调整：Rust 2024

- 按用户要求，workspace edition 更新为 2024，全部 crate 继续继承统一设置。虚拟 workspace 显式使用 `resolver = "3"`，遵循 [Rust 2024 的依赖解析规则](https://doc.rust-lang.org/edition-guide/rust-2024/cargo-resolver.html)；现有最低 Rust 1.88 已支持该 edition。
- Rust 2024 将进程环境变量修改标记为 unsafe，构建脚本改用现有 `tonic_build::Config::protoc_executable` 选择编译器。保留显式 `PROTOC` 与 vendored 回退，不增加依赖、不放宽 unsafe 禁令。

## 后续调整：OpenRC

- 用户明确要求新增 OpenRC，覆盖原 MVP 对 OpenRC 的排除项；仍限 Linux。服务生命周期继续经过 `ServiceManager` 与 `Privileged`，不修改适配器、协议、账本或独立运行时架构，详见 ADR 0017。
- 根据运行标记自动选择 init，systemd 优先；OpenRC 服务名由现有标识移除 `.service` 得到，保留 `@main`。不增加设备配置字段，旧 TOML 可继续使用。
- 安装支持 shadow 工具或 BusyBox 系统账号工具；Agent 与运行时分别由 supervise-daemon 监督，日志写入 `/var/log/sinan/`。安装与升级只重启 Agent，HUP 发送给被监督的运行时进程。
- 不因 OpenRC 扩大运行时 libc 构建范围：Alpine 需要另有匹配的 musl 运行时制品。真实 OpenRC 进程夹具、代理专项验证与公网实机验收分开记录。

## PR 准备：同步上游后的 OpenRC 边界

- 同步 `upstream/main` 的 `7a8fb70`，继承 NodeQuality、Agent `0.2.0`、公开报告上传缺省关闭和 Linux musl 双架构 CI；OpenRC 双架构检查作为本次 PR 的新增任务保留。
- 上游诊断 ADR 已占用 0016，OpenRC ADR 顺延至 0017，并同步所有引用。临时项目设计参考及其入口、问题和进度记录按用户要求移除。
- 一次性诊断依赖 systemd 的任务监督与挂载命名空间；OpenRC 入口不注册诊断适配器，服务管理器在执行任何命令前拒绝诊断启动和状态查询。普通代理服务的生命周期仍路由至所选 init；systemd 的诊断启动与恢复继续使用上游实现。

## 主分支修复：OpenRC 缓存与多平台构建

- OpenRC CI 的 amd64 默认运行级别启动超时已在干净 Alpine 环境复现：初始依赖树生成与两次安装都发生在同一秒，OpenRC 只比较整秒文件时间戳，缓存没有新服务；`rc-service` 能单独启动，但 `openrc default` 不会列出它们。安装后运行 `rc-update --update` 强制更新依赖树；回归夹具让初始缓存时间晚于新服务，避免依赖机器速度，并检查缓存包含两个服务。
- 用户要求将近期修改与修复放到 `main`，后续不新建分支。参考本地 NodeFlare 的标签构建与 Linux cross / FreeBSD VM 验证方式，为收敛后的 CI 补充手动或标签触发的多平台制品工作流，保持 Ubuntu 24 glibc、macOS ARM64、Windows 与 FreeBSD 双架构；FreeBSD 在 Linux 安装目标标准库，不依赖 VM 内的 ARM64 rustup 安装器。详见 ADR 0015 后续决策。

## 多平台构建：统一工作流与 Linux 链接方式

- 用户澄清多平台应在日常 CI 中构建，OpenRC 无须单独拆出；将九个目标统一回 `ci.yml` 的 push/PR/手动入口，删除临时多平台工作流。OpenRC 真实服务检查移入两个 Linux musl 任务，GNU/Linux 则在 Ubuntu 24.04 双架构任务验证动态 glibc 链接。
- init 与 libc 分开处理：Alpine/OpenRC 使用 musl 静态 Agent，Ubuntu 24.04/systemd 可使用 glibc 动态 Agent；服务管理继续自动识别，已有安装接口和制品导入方式保留，不将 GNU/libc 强行用于 Alpine。FreeBSD cross 与 VM 验证方案、macOS 仅 ARM64、Windows 双架构均保留。

## Agent 能力对齐：用户确认全部补齐

- 用户确认将基础监控细节、离线遥测补报、持续拨测、通用命令、升级恢复和非 Linux 常驻部署全部纳入，覆盖原来的仅编译产物限制与自更新排除项；按 ADR 0018 分阶段实现。
- sing-box 上游支持 musl。原脚本只调用 glibc 工具链，属于本仓库缺口；新增官方 musl 工具链与标签，不删除默认功能或修改上游源码。
