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

- 用户明确要求新增 OpenRC，覆盖原 MVP 对 OpenRC 的排除项；仍限 Linux。服务生命周期继续经过 `ServiceManager` 与 `Privileged`，不修改适配器、协议、账本或独立运行时架构，详见 ADR 0021。
- 根据运行标记自动选择 init，systemd 优先；OpenRC 服务名由现有标识移除 `.service` 得到，保留 `@main`。不增加设备配置字段，旧 TOML 可继续使用。
- 安装支持 shadow 工具或 BusyBox 系统账号工具；Agent 与运行时分别由 supervise-daemon 监督，日志写入 `/var/log/sinan/`。安装与升级只重启 Agent，HUP 发送给被监督的运行时进程。
- 不因 OpenRC 扩大运行时 libc 构建范围：Alpine 需要另有匹配的 musl 运行时制品。真实 OpenRC 进程夹具、代理专项验证与公网实机验收分开记录。

## PR 准备：同步上游后的 OpenRC 边界

- 同步 `upstream/main` 的 `7a8fb70`，继承 NodeQuality、Agent `0.2.0`、公开报告上传缺省关闭和 Linux musl 双架构 CI；OpenRC 双架构检查作为本次 PR 的新增任务保留。
- 上游诊断 ADR 已占用 0016，OpenRC ADR 顺延至 0021，并同步所有引用。临时项目设计参考及其入口、问题和进度记录按用户要求移除。
- 一次性诊断依赖 systemd 的任务监督与挂载命名空间；OpenRC 入口不注册诊断适配器，服务管理器在执行任何命令前拒绝诊断启动和状态查询。普通代理服务的生命周期仍路由至所选 init；systemd 的诊断启动与恢复继续使用上游实现。

## 主分支修复：OpenRC 缓存与多平台构建

- OpenRC CI 的 amd64 默认运行级别启动超时已在干净 Alpine 环境复现：初始依赖树生成与两次安装都发生在同一秒，OpenRC 只比较整秒文件时间戳，缓存没有新服务；`rc-service` 能单独启动，但 `openrc default` 不会列出它们。安装后运行 `rc-update --update` 强制更新依赖树；回归夹具让初始缓存时间晚于新服务，避免依赖机器速度，并检查缓存包含两个服务。
- 用户要求将近期修改与修复放到 `main`，后续不新建分支。参考本地 NodeFlare 的标签构建与 Linux cross / FreeBSD VM 验证方式，为收敛后的 CI 补充手动或标签触发的多平台制品工作流，保持 Ubuntu 24 glibc、macOS ARM64、Windows 与 FreeBSD 双架构；FreeBSD 在 Linux 安装目标标准库，不依赖 VM 内的 ARM64 rustup 安装器。详见 ADR 0015 后续决策。

## 多平台构建：统一工作流与 Linux 链接方式

- 用户澄清多平台应在日常 CI 中构建，OpenRC 无须单独拆出；将九个目标统一回 `ci.yml` 的 push/PR/手动入口，删除临时多平台工作流。OpenRC 真实服务检查移入两个 Linux musl 任务，GNU/Linux 则在 Ubuntu 24.04 双架构任务验证动态 glibc 链接。
- init 与 libc 分开处理：Alpine/OpenRC 使用 musl 静态 Agent，Ubuntu 24.04/systemd 可使用 glibc 动态 Agent；服务管理继续自动识别，已有安装接口和制品导入方式保留，不将 GNU/libc 强行用于 Alpine。FreeBSD cross 与 VM 验证方案、macOS 仅 ARM64、Windows 双架构均保留。

## Agent 能力对齐：用户确认全部补齐

- 用户确认将基础监控细节、离线遥测补报、持续拨测、通用命令、升级恢复和非 Linux 常驻部署全部纳入，覆盖原来的仅编译产物限制与自更新排除项；按 ADR 0022 分阶段实现。
- sing-box 上游支持 musl。原脚本只调用 glibc 工具链，属于本仓库缺口；新增官方 musl 工具链与标签，不删除默认功能或修改上游源码。
- 辅助 outbox 和命令表采用幂等建表，不提高账本 `user_version`，避免旧版本回退时拒绝打开数据库。压缩上传复用已有 `flate2` 依赖。
- 管理员可以拨测内网、回环和公网的单播目标；命令使用设备服务账号执行。命令执行状态在启动进程之前落盘，重启后的未完成命令统一标记中断，不重放无法确认副作用的操作。失联超过一天暂停使用缓存拨测配置。
- OpenRC 一次性诊断由同一 Agent 二进制的内部任务入口运行，在独立服务中监督；它不是对外开放的特权 helper。挂载命名空间使用系统 `unshare`，保留 Linux 专属 NodeQuality 范围。

- 原生服务采用系统内置 launchd、rc.d/daemon 和 Windows 计划任务，Agent 与运行时分开注册；Windows 不增加需要 unsafe 的服务控制器库。运行时 DLL 通过通用文件白名单描述，core 保持运行时无关。
- 自动更新只使用面板导入的稳定版，初始关闭；本地 PID/版本检查允许离线启动。Supervisor 与 Agent 同属系统服务，代理进程归另一服务；更新前的持久试运行状态用于恢复中断升级。Windows 文件内容刷盘后原子替换引用，目录元数据断电语义仍是已知验证边界。
- FreeBSD 保留代理普通账户，因此默认使用非特权节点端口；需要 443 等低端口时由管理员配置宿主端口授权。安装器不自动修改全局 sysctl 或加载端口策略模块，避免覆盖现有主机策略；详见 README 和 FreeBSD `mac_portacl(4)`。
- FreeBSD 的 sysinfo 0.33 磁盘枚举调用 `getmntinfo`，其[官方实现](https://github.com/freebsd/freebsd-src/blob/releng/15.0/lib/libc/gen/getmntinfo.c)会修改并重新分配进程全局缓冲区。连接上报和采样可能并发构造 Collector，因此以进程级互斥锁保护完整磁盘枚举及 libgeom 快照；不新增 unsafe 或修改上游依赖。CI 增加重复启动采样和可用时的原生崩溃回溯。
- Windows 新建运行时账户明确加入普通 Users 组，使用固定 SID 和本地账户对象，避免本地化名称与域同名账户歧义，依据 [Add-LocalGroupMember 文档](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.localaccounts/add-localgroupmember?view=powershell-5.1)。运行时保持非管理员；原生 CI 校验账户分组及真实任务执行。Windows ARM64 实测启动权限检查可能超过 60 秒，更新试运行给出 120 秒有界健康检查，其余平台仍为 60 秒。
- Windows runner 的任务继承 DACL 给 SYSTEM/Administrators 的掩码为 `0x1f019f`，不含文件执行位，因此 Agent 可以读写任务但无法启动运行时。注册后按 [SetSecurityDescriptor](https://learn.microsoft.com/en-us/windows/win32/taskschd/registeredtask-setsecuritydescriptor) 明确将本项目任务的控制权限限定为 SYSTEM 和 Administrators，禁止继承及自动添加运行账户控制 ACE；任务仍以原普通账户执行，不改变运行时文件权限。
- Windows ARM64 的测试日志显示升级已激活但状态保存超过夹具等待，安装也在大量权限操作期间超时。权限读写改用内置 .NET Framework 的 [File.SetAccessControl](https://learn.microsoft.com/en-us/dotnet/api/system.io.file.setaccesscontrol?view=netframework-4.8.1) 及目录对应接口，合并同次目录创建和制品文件权限，避免反复自动加载 PowerShell 文件系统/安全模块；新文件在写入内容前设置 ACL。服务启停使用同一任务计划程序的 COM 接口，避免每次导入 CIM 模块；重启先等待旧任务停止。仍保留 ACL 拒绝检查与普通账户隔离，不添加依赖或常驻 helper。
- Windows 任务 DACL 修复后的事件确认错误 `0x80070569`：普通运行账户缺少批处理登录权。安装通过系统 [secedit](https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/secedit-configure) 读取并保留现有 `SeBatchLogonRight`，只追加该专用账户 SID；不更改拒绝策略及其他用户权利。新账户先禁用，设置随机密码后再启用。CI 比较安装前后及重复安装后的全部用户权利，验证只发生预期追加。
- Windows 用户权利模板允许以账户名或带星号的 SID 表示同一主体（[MS-GPSB 2.2.6](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gpsb/3413b381-a445-4d17-b77e-5bbfadda253b)）。重复安装识别这两种表示；原生测试统一转换为 SID 后比较，保留全量权限差异断言与失败详情，避免仅按原始文本误报。

## 磁盘容量：容器挂载与文件系统去重

- 磁盘总量沿用已挂载文件系统的容量口径，不改为裸块设备标称容量；已用空间仍为 `total_space - available_space`，包含普通进程不可用的保留空间，与 `df` 的 Used 列可能不同。Unix 的静态总量、动态已用和逐盘列表共用一次筛选后的快照，读取失败、容量无效或溢出继续表示未知。
- [sysinfo 0.33.1 的 Linux 枚举](https://github.com/GuillaumeGomez/sysinfo/blob/v0.33.1/src/unix/linux/disk.rs)会返回 overlay 与单文件绑定挂载；原来仅按名称累加时，真实分区与名为 overlay 的视图各计一次。按[内核 overlayfs 语义](https://docs.kernel.org/filesystems/overlayfs.html)，Linux 排除非根 overlay/overlayfs/fuse.overlayfs 与单文件挂载；容器中的 `/` overlay 保留。
- Unix 使用挂载点的设备标识去重绑定挂载与设备别名，优先保留根及较浅挂载；元数据不可读时回退设备名，无名时回退挂载路径。Btrfs [子卷共享文件系统存储](https://btrfs.readthedocs.io/en/latest/Subvolumes.html)，继续按可解析的源设备名称去重，不因子卷设备标识不同重复累加；I/O 基线同时使用名称与挂载点。
- Windows 保留原 sysinfo 逐盘列表与按名称首条去重的总量规则。[固定 sysinfo 的 Windows 枚举](https://github.com/GuillaumeGomez/sysinfo/blob/v0.33.1/src/windows/disk.rs)会为同一卷的每个挂载路径生成一条记录，因此不能仅凭路径将它们计为独立容量。不同卷标签相同或为空仍有既有歧义，须在取得真实卷 ID 后另行实现和原生验收；本项不声称修复该歧义、不新增 unsafe、依赖或外部命令。容量无效、溢出和首条记录的未知语义保留。
- 不根据相等容量猜测两个文件系统是否同盘；容器内无法访问 overlay 后端时，也不推断它与额外目录卷的物理归属。此项修复不提供跨命名空间或所有存储池的物理容量映射。


## 交付加固：签名缓存的跨平台验证边界

- Linux 发布版本目录使用真实 GNU `/bin/mv --no-clobber --no-target-directory --`，源和目标必须是同父目录下的受控路径；完成后重新核验实际最终目录。
- 本机 macOS 的文件系统测试只模拟上述精确请求的 rename/sync，不模拟其他特权命令，也不扩展 Agent 的非 Linux 生命周期。只有真实 Linux CI 可以认证 GNU mv、systemd ExecStartPre 和安装器行为；本地测试通过不能替代它。
- 生产信任根仅来自编译时 `SINAN_RELEASE_PUBLIC_KEYS`；Rust 库显式 `with_trusted_keys` 接口用于受信 embedding 与测试，Agent 命令行没有运行时换根开关。测试公钥及确定性公开测试私钥均明确标记为 TEST_ONLY，正式发布工具按真实公钥字节拒绝这些根。

## 后续调整：VPS 部署与本机接入

- 本次“继续开发”先处理实机部署、HTTPS 反代与设备接入发现的问题，保持现有 MVP 功能范围。部署凭据、真实地址、数据库备份与截图保存在仓库外，不提交环境信息。
- 新增标准库配置初始化命令，独立生成密码、创建时限制权限，并拒绝覆盖已有文件或符号链接；宿主端口可选以兼容已有服务。提供可追加的 Caddy 示例，保持 WebSocket 路由与面板 origin 一致。
- 共享 VPS 上的 Docker Rust 构建默认并发为 2，可用 build arg 调整；不改运行期的业务并发。
- 新接入设备还没有运行时或配置时，已启用的代理服务应跳过启动。systemd 单元增加路径条件，后续 Agent 安装运行时并发布配置后仍可正常启动，避免首次重启进入无意义的失败循环。

## 签名与多平台整合后的 CI 修复

- Windows 的文本模式写入会将 LF 转成 CRLF，使安装到磁盘的签名证明不再符合 canonical 格式。夹具、清单及固定安装器都按 UTF-8 原始字节写入；不让验证器替换换行或放宽签名检查。回归在 Linux 模拟 Windows 文本模式，并使用独立 minisign 验证正确证明及拒绝篡改。
- 当前 cryptography 50.0.2 的发布文件没有 Windows ARM64 wheel，直接 pip 安装会进入本机构建。原生 CI 改用 `sinan-protocol` 已有的 ed25519-dalek/blake2 测试签名实现，编译独立 example；仅内嵌公开 TEST_ONLY 密钥，只接受有界 stdin，不接受密钥参数。合并上游 `541f52d` 后统一使用其 `ci-fixture-sign` 和 `SINAN_CI_FIXTURE_SIGNER`，同时覆盖 macOS 与 Windows；未增加生产依赖，也不向 CI 提供正式私钥。
- BusyBox `install -d` 会让隐式父目录受 `umask 027` 影响。安装器显式创建共享的 `/opt/sinan`、`/opt/sinan/plugins` 和 `/var/lib/sinan` 为 0755，保证独立普通运行账户可遍历；身份和 core 账本仍为 0700，运行时私有配置仍按专用组限制。隔离 Alpine 实际启动验证保留，不通过把运行时改为 root 绕过权限问题。
- 静态 musl Agent 可以运行在 glibc 主机上，编译 ABI 不能兼作外部运行时的 libc。保留 `libc` 的 Agent 更新语义，新增可选 `runtime_libc`；在 core 读取系统程序有界 ELF64 头中的 `PT_INTERP` 判断 GNU/musl，不执行额外工具、不依赖 init 类型、不新增依赖。面板与 core 的签名缓存选择使用宿主信息，旧协议字段缺失仍兼容。解析参考 [ELF Header](https://refspecs.linuxfoundation.org/elf/gabi4+/ch4.eheader.html) 与 [Program Header](https://refspecs.linuxfoundation.org/elf/gabi4+/ch5.pheader.html)；无法识别时保留编译 ABI 回退，不将未知 libc 任意映射到 GNU。
- 合入上游 `13f2975` 后统一使用其 `runtime_platform` 实现，移除重复解析与选择逻辑。GNU 宿主上的静态 Agent 保留已有 musl、旧目录、GNU 运行时的优先级，以避免同一已签证明包含多个平台时改变现有不可变缓存的选择；Agent 自更新仍按编译 ABI。
- 上游把扩展平台检查改为手动；按本次此前的多平台自动构建要求，`CI` 继续在 push/PR 构建全部平台，OpenRC 保留在 Linux musl 任务内。上游的独立手动平台工作流保留，便于按需复验。
- macOS 的软链接继承进程 umask，`readlink` 又检查链接自身的读取权限；launchd 的 `umask 027` 因此会阻止普通运行账户解析 root 创建的 `current`。依据 [Apple XNU 的 symlink/readlink 实现](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/vfs/vfs_syscalls.c)，在 `SystemOps::atomic_symlink` 发布前使用 `chmod -h 755` 设置临时链接自身权限；不改变目标文件、私有目录权限或整个进程的 umask。仍保留签名、路径边界、摘要与离线缓存复验，并在 macOS CI 用 `umask 077` 检查链接可读、目标权限不变及悬空链接替换。

## 2026-10-01：策略组与套餐组的分配语义

本轮按用户新增要求扩展 sing-box 插件，不恢复已删除的平台全局代理用户模型。策略组定义可使用的普通节点及两跳链路；多个组与旧单独授权取并集，避免取消一个来源误撤其他来源的权限。套餐组是独立模板，每个用户保留一个当前快照，模板编辑不追溯改变已分配用户。详见 [ADR 0035](adr/0035-singbox-policy-package-groups.md) 和 [操作/API 说明](singbox-groups.md)。

用户未指定首月折算、结转和续期累加，本轮选择完整月度额度、无结转、从分配时刻起计算有效天数。更换套餐不删除流水，按新模板的周期边界统计已有流量；界面明确提示期限是重新起算而非在旧到期日累加。没有套餐的旧用户和新用户保留不限制流量与有效期的兼容模式，必须明确分配套餐才能启用限制。

用户要求策略组包含链路，因此补齐可执行的原生两跳配置，而不是只保存没有运行时效果的链路编号。暂不支持任意多跳、同机链路或入口直接授权；链路内部凭据不进入订阅，用户流量只在入口计量。跨月采样批次完整归入排他结束时刻前一秒的月份，不按时间比例猜测字节分布。配额/到期通过订阅资格和运行时整包发布共同执行，离线与尚未上报的流量仍存在滞后，不能承诺立即断流。生产双机、长连接及离线恢复验收另行执行，本 PR 不部署。


## 2026-10-01：现代协议与证书

- 用户授权新增六种协议并选择自动申请、续期，按 ADR 0034 在现有 sing-box 插件内扩展；原 MVP 排除项据此覆盖。旧 Reality 快照缺省为旧协议，新增字段不改变旧编译字节、凭据、订阅令牌与历史账本。
- 初期 ACME 选择 HTTP-01/TLS-ALPN-01，无需引入 DNS 服务商令牌；同机共享一个提供器避免挑战监听冲突。共享邮箱与验证方式由现有 ACME 节点编辑原子同步，任何端口冲突均回滚。证书持久目录复用 data，不创建额外服务。
- 新协议先统一使用完整 sing-box JSON；既有 links 格式保留 Reality，遇到其他协议显式提示切换，避免不完整订阅或假设非标准 Snell 分享 URI。客户端必须支持相应出站，Naive 的 Cronet 平台限制不等于服务端入站限制。
- 签发为异步过程，服务启动与证书就绪分开判断；适配器增加实际 TLS/QUIC 握手，健康预算最多 240 秒，core 只处理通用有界预算。AnyTLS 的健康握手不擅自添加 HTTP ALPN，避免启用 TLS-ALPN 验证后与其协议协商冲突；TUIC/HY2 显式使用 h3。

## 2026-10-01：NodeFlare 风格的服务器展示页

- 用户明确只复刻服务器信息展示，不参考或改造后台管理。新增独立 `/#/overview` 与服务器展示详情路由，作为登录后的默认入口；后台保留既有路由、样式、业务操作和登录权限，侧栏只增加返回展示首页的入口。没有新增匿名服务器接口。
- 参考 NodeFlare 展示布局、主题和系统图标，保留 MIT 来源说明；用现有 React、原生 SVG 和现有图标实现，无需为展示添加图表或图标运行依赖。样式限定在独立容器内。
- 使用现有服务器、两小时指标与拨测结果接口。成本、配额、国家地区和服务器分组没有足够数据，不伪造；使用状态筛选、实际交换内存和网卡计数。历史缺失不补零，在线与采样新鲜度分开；网页读取失败时保留历史但不继续展示为当前速率。详见 [展示页说明](server-display.md)。

## 2026-10-01：展示页延迟、丢包及 Agent 持续拨测

- 按用户补充在卡片和详情增加延迟/丢包，后台服务器刷新旁添加“服务器展示”。沿用已有管理员配置的 TCP/ICMP 目标，不嵌入默认公网 IP；使用线路备注，不自动识别或伪造运营商。
- 卡片最多展示三个启用目标、每目标 20 个时间格；详情选择单个目标，分别绘制延迟和丢包/连接失败率，以避免不同方向、协议或采样周期混合成误导的平均值。新增认证批量汇总与逐目标索引查询，避免逐卡请求和原全局 4096 条截断慢目标历史。
- 不变更已有结果消息和不可变摘要。`error` 标识检测未完成，旧数值占位不作为实测丢包；只有完整测量的无响应才显示 100%。Unix 解析固定 C 语言环境的收发/平均值汇总，Windows 输出 .NET Ping 的 JSON，不再依赖英文或中文 ping 文本。
- 保留四路并发与原有持久化/确认方式，补齐慢任务独立完成、配置修改取消和时钟偏移下的缓存清理。工具安装与权限交给节点操作者；未在生产机器探测或改变系统策略。平台依据、资源限制和人工步骤见 [服务器展示页](server-display.md)。


## 2026-10-01：完善添加服务器与接入引导

- 用户要求参考 NodeFlare 的添加服务器配置，并明确选择采样设置、自动更新及初始拨测。采用分区配置表单与安装接入两步流程；地区、标签、成本、续费及网卡额度继续作为后续独立工作。
- 复用现有 AgentSettings 与 TCP/ICMP ProbeSpec，在创建接口增加可选参数；服务端统一校验并在同一事务保存服务器、设置和初始拨测，保持旧的仅名称请求兼容，无新增依赖或数据库迁移。
- 不复制 NodeFlare 下载后直接执行的安装方式。保留独立可信 bootstrap 与签名制品，现有 Linux 命令覆盖 systemd/OpenRC；原生平台仍按现有平台文档安装。接入命令失败单独重试，过期隐藏，修改版本先清除旧命令。
- 以认证服务器接口轮询确认注册、在线状态；接口失败显示无法确认，不沿用旧在线状态。已有设备在线不等于升级完成，不自动关闭接入页或宣称升级成功。新增与服务器详情入口共用接入组件。

## 2026-10-01：补齐服务器资产与账单周期流量

- 用户进一步授权将此前分开的地区、标签、成本到期和流量额度一并做好。本次为该独立工作增加持久化资产配置，并贯通新增、编辑、后台详情及展示页；参考 NodeFlare 的字段组织，沿用 Sinan 的管理员权限与现有遥测。服务器分组仅用于筛选，不是代理用户分组。
- 金额以两位小数字符串保存，额度和汇总字节以十进制整数字符串传输，避免浏览器数值精度损失。费用周期按天记录；到期和网卡周期统一使用明确标注的 UTC。自动顺延只维护记录，不对接供应商或付款；到期、超额都不触发服务启停。
- 复用现有 PostgreSQL 日历运算，按每月指定日计算网卡统计周期，短月取月末，无新增时间或金额依赖。按接口保存每日增量与最后计数，修改网卡筛选、统计方向和重置日后重新汇总已有数据；首个完整样本只建立基线，旧累计不会被归入本月。
- 新遥测、计数检查点和日汇总处于同一事务；旧消息摘要、重放确认和代理用户账本不变。计数回退、可识别的重启、长间隔和网卡变化标记观测不完整；跨日增量归于后一采样日，历史缺口不能恢复。额度展示用于服务器观测，不承诺与供应商精确结算一致，详见 [ADR 0036](adr/0036-server-assets-and-traffic.md) 和 [操作说明](server-assets.md)。
- 展示隐藏仅移除总览卡片和汇总，管理员仍能打开详情，不充当新的权限边界。地区手动填写，不执行额外 IP 查询；资产配置默认空值，旧接口可继续只传名称，PATCH 省略资产时保留现有值。

## 2026-10-01：独立服务器看板入口

用户希望有可从管理后台跳转的 Komari / Nezha 类服务器看板。主线已有展示组件与数据读取，所以复用原页面，统一为 `/#/dashboard` 并兼容旧 overview 书签，不安装或复制第二套监测后端。新增表格、排序、明确的成功读取时间与暂停控制，沿用原卡片、资源历史与网络质量；所有数据仍来自现有认证 API。

用户未要求公开服务器数据，因此保留原登录边界；资产的展示隐藏仅影响看板筛选，不作为权限隔离。看板不能触发诊断或创建拨测。单次读取超时、暂停、旧快照和真正空列表明确区分；浏览器在线标志不决定回环/内网面板是否可达。该页面源码与构建产物通过独立 PR 交付，不在本任务中合并或部署。
