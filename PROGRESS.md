# 执行进度

## 交付加固第 0 阶段：实现与本地检查完成，等待 main CI

- 自动 CI 精简为 Rust/前端检查、Compose 持久化 smoke、Linux musl amd64/arm64；runner 固定 Ubuntu 24.04。原 FreeBSD 工具链候选修复及原未提交差异保存在仓库外，未纳入本次提交。为避开两个活跃聊天共享目录的写入，本任务改用独立 worktree；既有部署与凭据保持私有。
- release profile 开启 `strip=true`、`lto=true`、`codegen-units=1`；musl jobs 对同源码、同工具链的旧 profile 和新 profile 分别构建，精确字节数、缩减比例随 artifact 与 Actions summary 保存。体积结果待对应 CI 实际构建，不以历史二进制冒充同源对照。
- NodeQuality 新任务默认 `upload_report=false`，创建任务时显式选择公开上传。Agent 与包装器严格传递此选项；本地完整报告仍保留。固定上游源码未改，包装制品升为 `-r2`，旧 Agent 拒绝新任务。已经排队或运行的旧任务沿用旧策略，应先结束再升级。
- 本地验证：`cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked` 全部通过，116 项通过、4 项按既有原因默认忽略；使用真实 PostgreSQL 16.15，包含 HTTP/WebSocket、诊断和账本恢复测试。Bun 冻结锁文件安装及生产构建通过，依赖未增加。构建脚本 4 项通过；NodeQuality Python 20 项中 15 项通过，5 项 Linux/root 夹具留给本阶段 CI。r2 双架构包装实际构建通过，嵌入上游字节保持一致。actionlint、YAML/矩阵与 diff 检查通过。
- 已核对前一个 main 的全部 jobs：仅 FreeBSD arm64 工具链安装失败；这些历史通过状态不能认证本次变更。本阶段推送后逐项检查 main，再补充实际 CI 与体积证据。
- 下一步：等待 main 全绿，完成干净 Linux/systemd Agent 的 Reality、重启计量和 0.1.0→0.2.0 升级验收；现有部署聊天已完成，不清空现有面板数据库。

## G1：已完成

- 建立七个 crate 的 Rust 2021 workspace，每个 crate 根禁止 unsafe。
- 完成 AGENTS 分层与禁止事项、术语表、11 条既定架构 ADR、CI、执行计划。
- 验证：Rust 1.97.1 下 `cargo build`、`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test` 全部通过。
- 环境问题：系统网络直连不稳定，使用系统现有代理的进程级配置重试；项目文件曾被 macOS 标记为 dataless，重写本阶段新建文件后恢复。PostgreSQL 16 正在准备。
- 下一步：G2 协议与兼容测试。

## G2：已完成

- 完成全部 WebSocket 消息与 HTTP 注册、清单、配置包的类型；信封可保留并忽略未知消息，未知字段保持兼容。
- 完成协议规范，明确 nonce 签名编码、会话范围、哈希原始字节、重发及确认语义。保存原始任务说明便于验收追踪。
- 验证：全 workspace fmt、clippy、test 通过；协议 9 项测试覆盖全部消息往返、未知字段/类型、必需字段错误及缺失指标省略。
- 环境恢复：项目已在 Finder 标记保留下载，编译产物与测试数据库移出同步目录；文件读取已恢复。托管 worktree 创建失败，继续使用原仓库。
- 下一步：G3 确定性编译器与黄金测试。

## G3：已完成

- 完成确定性 VLESS + Reality 服务端编译、分享订阅和最小 sing-box 客户端配置。
- 无授权用户的节点不生成入站；节点、授权和统计列表稳定排序；校验重复端口/身份、主机名、密钥格式和 short_id；订阅支持 UTF-8 转义及 IPv6。
- 验证：全 workspace fmt、clippy、test 通过。编译器 7 项测试通过，覆盖三份黄金文件、乱序输入、空模型、非法模型和用户凭证隔离。
- 真实运行时 check 测试暂时 ignore，原因是指定上游二进制仍在下载依赖并构建；G6 阶段补跑。未把黄金测试视为真实代理连通性验收。
- 下一步：G4 面板数据库、认证、服务器接入与下载接口。

## G4：已完成

- 完成完整 PostgreSQL 迁移、首次管理员与 Cookie 会话、服务器 CRUD、24 小时原子一次性注册、Ed25519 挑战认证和设备一小时会话。
- 完成在线与遥测更新、清单/原始配置包接口、部署回报基础、带 SHA256 验证的制品下载、受注册 token 约束的安装引导及脚本渲染。
- 安装脚本和 systemd 模板已接入渲染；实际 Debian 执行仍留到最终实机验收。前端目前只有 API 启动页，按计划 G8 实现完整界面。
- 验证：全 workspace fmt、clippy、test 通过；真实 PostgreSQL 上 6 项 HTTP/WebSocket 集成测试均通过（认证/过期、并发注册、挑战重放、配置包身份隔离、制品校验及路径边界），另有公共 URL 校验测试。
- 补充 G3 实测：上游未修改 v1.14.2 二进制已完成构建，服务端和客户端黄金配置的真实 check 测试均通过。已验证统计 gRPC 路径必须使用上游运行时改写后的服务名。
- 下一步：G5 Agent 状态、计量账本、传输与通用对账。

## G5：已完成

- 完成设备身份、CLI、持久连接与重连、遥测、私有 status socket、独立持续运行的对账和计量 worker。
- 完成 SQLite 迁移、基准与 outbox 事务、确认重发、完整 u64 计数、重启恢复；完成无具体运行时依赖的适配器契约及特权操作实现。
- 完成同源制品与配置包校验、安全解包、版本缓存、串行版本合并、超时、意图恢复和旧版本回滚。审查修复重复进程在获得独占 socket 前恢复意图的问题。
- 验证：全 workspace fmt、clippy、test 通过；core 测试覆盖 12 项账本、6 项对账、10 项制品与文件操作、3 项身份及传输/遥测/status 单元测试。CLI help 与参数解析验证通过。
- 真实 systemd 操作尚待 Debian 手工验收；没有将 FakeServiceManager 测试视为真实服务验收。
- 下一步：G6 真实运行时适配器、版本标签校验与统计 gRPC。

## G6：已完成

- 实现无状态具体适配器并在 Agent 入口注册：1.14.x 精确版本与统计标签校验、原生配置 check、计划判断、服务操作、端口与统计 API 健康检查。
- 原样复制上游 1.14.2 proto，使用随包 protoc 构建；统计请求使用真实运行时服务路径、patterns 和 reset=false。
- 重载用已完成握手的旧 HTTP/2 连接作为运行代边界，避免 HUP 尚未结束时重复计量；首次应用后同步更新面板可见运行时版本。
- 验证：全 workspace fmt、clippy、test 通过；适配器 5 项单元、4 项契约测试通过。显式运行默认忽略的真实运行时测试通过，实测原生 check、监听端口、gRPC、正上传/下载计数、重复累计读取、HUP 后归零和新流量从零累计。
- 真实运行时测试在本机临时进程和本地 VLESS 流量夹具完成，仍未代替 Debian systemd 与公网 Reality 客户端验收。
- 下一步：G7 面板业务、自动发布、流量入库和完整进程内端到端测试。

## G7：已完成

- 完成节点、用户和授权 CRUD，自动端口与 Reality 凭证生成；并发修改通过行锁保持一致，管理响应隐藏节点私钥。
- 完成持久化五秒合并发布、完整快照、哈希去重、部署历史与状态修复；订阅仅使用已应用且健康、仍获授权的凭证。
- 完成整批事务化流量入账、重复确认、历史身份重传、完整 u64 字段和精确汇总。修复丢失成功回报后较旧失败结果覆盖新健康状态的问题。
- 验证：全 workspace fmt、clippy、test 通过；新增 4 项业务、4 项流量 PostgreSQL 测试和完整进程内端到端测试。端到端使用真实数据库、HTTP/WebSocket、生产五秒发布和三十秒采样，确认重启重发仍只有一批记录、累计 100/200 字节且运行时没有被重启。
- 显式 Reality 实测通过：上游命令格式、X25519 密钥对应关系、真实面板发布配置的原生 check。
- 下一步：G8 中文管理界面及内嵌静态页面。

## G8：已完成

- 完成 React、Vite、TypeScript 中文响应式界面：登录、服务器及详情、节点管理、用户与授权、双格式订阅复制、按用户及节点流量、制品列表。
- 使用普通 CSS 和 hash 路由；加载、空状态、失效会话与失败提示均可见，缺失遥测不填零，流量使用十进制字符串和 BigInt 保持精度。
- 前端通过 rust-embed 内嵌；提交构建产物，并由 CI 重建校验。新增真实 HTTP 测试覆盖 HTML、JS/CSS 内容与 MIME、缓存、HEAD、未知路径 404 和鉴权。
- 浏览器实测通过：登录、添加服务器和安装命令、详情缺失指标、创建节点自动分配 20000、创建用户、授权与刷新持久化、切换并复制订阅、制品空状态、退出登录。
- 审查修复慢请求轮询、授权中切换用户、按用户节点流量展示，以及已有本地目录被意外 chmod 的问题；目录权限保护增加回归测试。
- 验证：npm ci、TypeScript 与 Vite 生产构建通过；全 workspace fmt、clippy、test 通过；core 禁用词检查通过。
- 下一步：G9 部署、构建脚本、快速上手、最终报告与推送。

## G9：已完成实现与本地验证

- 提供多阶段 Dockerfile、PostgreSQL 16 Compose、配置示例、持久命名卷和非 root 面板运行。CI 增加真实 Compose 启动、页面/API、容器重建后数据库与数据卷持久化检查。
- 提供双架构原生 Linux musl Agent 构建脚本，以及按固定上游 tag、完整默认标签加统计标签构建运行时的脚本和 Linux/amd64 工具链容器。归档、SHA256SUMS、已有制品保护均有验证；CI 在 amd64/arm64 runner 分别检查静态 ELF 和原生 CLI。
- 完成 README 快速上手、制品导入、升级说明及 Debian 12 人工验收脚本。验收辅助命令只读取设备状态和账本，可保存按用户/节点筛选的精确用量；不会自动修改业务或重启服务。
- 安装脚本复用 Agent 的 TOML 解析及身份绑定检查，保留自定义路径，并在成功注册之后更新配置和当前二进制。
- 本地最终验证：全 workspace fmt、clippy（所有 targets，warnings 为错误）、cargo test 通过；79 项通过，3 项真实运行时专项默认忽略，均已在 G3/G6/G7 显式运行通过。前端依赖锁定安装、TypeScript/Vite 构建、内嵌 HTTP 和浏览器操作通过。
- 构建脚本语法、help 和 18 个校验清单场景通过；Compose/CI YAML、安装/验收脚本、内嵌 Python、Docker 编译输入及 git diff 检查通过。本地真实 PostgreSQL 和 cargo run 面板启动已验证。

## 最终交付与验收边界

G1–G9 的 MVP 代码、中文界面、文档和部署入口均已实现，核心路径没有仅占位的 TODO。所有阶段分别完成检查并使用 Conventional Commits 提交；目标仓库为 https://github.com/theLucius7/sinan 。远端检查结果以对应提交的 [GitHub Actions](https://github.com/theLucius7/sinan/actions) 为准；Docker 与 musl 的流水线包含实际执行检查，不能仅凭配置文件宣称通过。

已实测的证据分为三层：

1. 真实 PostgreSQL、HTTP/WebSocket、五秒发布、三十秒采样和持久 outbox 的进程内端到端测试；重启后重传被确认且不重复入账，运行时服务不因 Agent 重启而被重启。
2. 未修改的上游 1.14.2 本机二进制：真实配置 check、Reality 密钥格式与对应关系、gRPC reset=false、正上传/下载、HUP 后计数重置及重新计量。
3. 本地浏览器通过中文界面操作真实面板；部署状态和缺失遥测按实际状态呈现。

当前工作机是 macOS，没有运行 Docker 或 Linux systemd。Linux 工具链容器及 sing-box Linux 双架构产物尚未在本机编译；最终公网使用前需按 README 构建并在目标架构检查版本标签。未提供全新 Debian 12 测试机和公网客户端，因此还没有执行完整的真实 systemd、网络可达性、Reality 客户端上网和实机重启计量验收。

需要人工完成的下一步：准备可达的 HTTPS 面板及设备制品，在专用 Debian 12 节点按 `scripts/e2e-real.sh guide` 逐项运行，保留前后状态与流量快照。外部强制重载可能造成未采样窗口，已持久化/已确认批次的恢复与去重有自动测试，但不宣称恢复从未采集到的字节。未实施任务范围之外的配额、计费、其他协议、链式代理、多管理员或自动更新。

## 后续调整：前端切换 Bun

- 按用户要求改用 Bun 1.4.2；前端 packageManager、开发/构建/预览脚本、Docker 和 CI、README 统一更新。使用 `web/bun.lock` 替代旧锁文件，逐项确认 119 个依赖版本保持一致。
- 自动构建使用冻结锁文件安装，TypeScript 与 Vite 显式使用 Bun runtime。本地在没有 Node 可执行文件的 PATH 下，从干净目录安装和构建通过；锁文件未被改写，HTML、JS、CSS 与已提交产物逐字节一致。
- 开发和生产预览服务启动通过，HTTP 验证首页、开发模块及生产 JS/CSS 均成功且 MIME 正确。fmt、全 targets Clippy 与 diff 检查通过。
- 本地同步目录曾阻塞旧依赖文件读取，因此使用隔离临时目录验证；本机剩余磁盘不足以保留完整 Rust 测试编译产物，完整 Rust 测试、Linux 构建和 Compose 验证由同一提交的 CI 执行，实际结果见上方 Actions 链接。

## 后续功能：IP 质量与 NodeQuality 外插

- Agent 上报网卡 IPv4/IPv6，支持 NAT 公网地址配置；面板按上游 IPQuality 实际接口查询七个数据库，独立显示原值、缺失和错误，缓存一天并限制刷新、并发和响应大小。旧协议主版本与旧 Agent 保持兼容，workspace 升至 `0.2.0`，已有制品不可覆盖。
- 增加独立诊断适配器及固定版本 NodeQuality 制品。管理员可选择 IP 版本和网络流量模式，一键创建节点测试；独立 systemd 服务拥有挂载命名空间、进程组终止和时限。core 持久任务及结果，重启不重复启动，离线完成后可重传；延迟领取不延长绝对截止时间。
- 包装器内嵌原样上游脚本和完整 AGPL 许可证，确认四项完整报告后才成功；保存原始 ZIP 和有限文本，在线上传失败仍保留本地报告。ARM64 仅映射上游写死的 NextTrace 下载 URL。构建脚本真实生成两架构包，核验上游字节、单文件归档、确定性和重复构建拒绝。
- 验证：全 workspace fmt、所有 targets Clippy 和 cargo test 通过，114 项通过、4 项默认忽略，包含真实 PostgreSQL/HTTP/WebSocket/Agent 诊断端到端测试；Agent 在服务启动后重启仍只启动一次，并回传报告。Python 16 项中本机通过 12 项、4 项完整 runner 夹具因需要 Linux/root 跳过；CI 使用 sudo 执行全部。三个原有真实运行时专项及新增真实 systemd 专项默认忽略，systemd 专项由 Linux CI 显式执行。
- Bun 1.4.2 冻结锁文件安装和 TypeScript/Vite 构建通过；中文浏览器夹具验收通过 IP 来源、零评分、false 标记、第三方错误、任务选项保存、重复执行禁用和本地报告显示。夹具明确标为模拟数据，不视为实际性能结果。
- 验证边界：本机实际 IP 数据库请求返回 403，尚未证明第三方服务在目标面板可用；没有在真实节点执行完整 NodeQuality CPU、磁盘和带宽测试。上线需按 README 导入外插和 `0.2.0` Agent 制品，在专用 Linux 节点点击报告并验证外网下载、测试及上传。Linux systemd、Docker 与双架构 musl 的最终结果以本功能提交的实际 CI 为准。
- 收尾时远端合入多平台编译与 Rust 2024 更新，已保留并整合；NodeQuality 仍只在 Linux 设备注册。修复公共路径校验模块迁移后的引用，合并后重新执行完整检查，不以合并前结果替代最终结果。外插 ADR 顺延至 0016，平台 ADR 保留 0015。
- 远端实际验收：[db432f5 的 Actions](https://github.com/theLucius7/sinan/actions/runs/36681161613) 中 Linux check 全部成功，16 项包装器夹具无跳过，真实 systemd 的独立服务、失败与超时专项通过；Bun 产物一致性、完整 Rust 检查、Docker 启动和 Linux GNU/musl 双架构制品均通过。macOS 与 Windows 双架构构建通过。这些结果不等同于真实 NodeQuality 性能测试或第三方服务可用性验证。
- 继承的已知构建问题：FreeBSD ARM64 在安装 Rust 工具链时失败，rustup 返回 `aarch64-unknown-freebsd` 安装器不存在；项目编译尚未开始。远端合并前的 a08ad04 已有同一失败，未在本功能中改换该平台的工具链或移除矩阵，因此整个多平台 workflow 不能宣称全绿。

## 后续调整：Agent 多平台编译产物

- 按用户确认的构建产物范围，CI 保留 Linux musl amd64/arm64，新增 Ubuntu 24.04 glibc 动态 amd64/arm64、macOS arm64、FreeBSD amd64/arm64、Windows MSVC amd64/arm64，共九个目标。Windows 使用 Visual Studio 2026 runner 与静态 CRT；各平台使用最新 Rust stable。
- GitHub 工具更新为本次核实的最新稳定版：checkout/upload-artifact 7.0.1、setup-python 7.0.0、rust-cache 2.9.2、setup-bun 2.2.0、freebsd-vm 1.5.8。FreeBSD 在 13.5 构建，再由 CI 在最新 14/15 系列 VM 执行同一二进制；13.5 以前小版本不宣称已验证。
- 新增跨平台构建脚本，验证 ELF/Mach-O/PE 架构、真实 CLI 版本及帮助，glibc 还验证动态解释器、libc 与共享库解析。新制品按完整 target 分目录并附 SHA256SUMS，拒绝覆盖已有产物；Actions 保留七天。原 Linux musl 部署目录保留。
- Unix 系统和 IPC 模块条件编译，通用路径校验归入制品模块。FreeBSD 可显式选择系统 protoc；未新增 Rust 依赖或 unsafe。非 Linux 的注册、运行、状态命令以中文明确返回 Linux/systemd 限制。
- 本次验证：fmt、全 targets Clippy（warnings 为错误）、完整 cargo test 通过；79 项成功，3 项真实运行时专项按已有原因忽略。本机临时编译 PostgreSQL 16.15，在回环地址运行数据库/HTTP/WebSocket/账本恢复集成测试，完成后停止临时数据库。测试初次受到本机代理干扰，回环请求直连后通过。
- 四组构建脚本测试通过，包含原有 18 个清单场景、新增七个目标的错误架构拒绝和校验清单/制品不可覆盖检查；错误宿主在构建前拒绝。actionlint 工作流检查、core 边界与 diff 检查通过。已核实官方新增 Windows arm64 VS2026 标签，并补充 actionlint 的标签允许列表。
- 本机 Linux/amd64 glibc 2.44 的 release 构建、动态库解析、版本/帮助及 SHA256SUMS 检查通过；这不等于 Ubuntu 24.04 glibc 基线已实测。其他架构、macOS、Windows、FreeBSD 13/14/15、Linux musl 和 Compose 的实际结果仍需以远端 CI 为准。下一步检查九个目标的远端构建和下载内容；非 Linux 安装与服务管理不属于本次范围。

## 后续调整：Rust 2024

- 按用户要求，将 workspace edition 更新为 2024、resolver 更新为 3；Cargo metadata 确认七个 crate 均继承 Rust 2024。最低 Rust 版本保持 1.88，协作规则、需求和执行计划同步更新。
- 构建脚本通过现有 `tonic_build::Config` 选择 protoc，移除进程环境变量修改，继续支持显式 `PROTOC` 与 vendored 回退。无需新增 Rust 依赖，保留全部根文件的 unsafe 禁令，Cargo.lock 未变化。
- 按 2024 格式规则重新格式化，并将 Clippy 要求的三处嵌套条件改为 let chains；审查确认其余 51 个 Rust 文件只有格式变化。
- 本次验证：`cargo fmt --all --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked` 均通过；79 项成功，3 项真实运行时专项按原有原因忽略。数据库、HTTP/WebSocket、账本恢复和端到端测试使用临时 PostgreSQL 16.15 完成，测试后已停止数据库。
- 默认 vendored protoc 构建与显式 `PROTOC` 的适配器检查均通过；core 禁用词和 diff 检查通过。各平台编译仍使用现有 CI 矩阵，远端结果待对应提交的 GitHub Actions 验证。

## 后续调整：OpenRC 服务支持

- 按用户追加要求增加 Linux OpenRC，更新协作约束、需求、ADR 0017、README 和实机指南。Agent 根据 init 运行标记选择 systemd/OpenRC，继续经过 `ServiceManager` 与 `Privileged`；未增加 Rust 依赖、unsafe 或设备配置字段。
- 安装脚本自动选择服务模板，支持 shadow 与 BusyBox 系统账号工具。修复 BusyBox 不支持 SHA-256 长参数、项目父目录受 umask 影响而阻止运行时访问的问题。成功注册后才切换 Agent；重复升级保留身份、账本与独立运行时。
- 新增两个独立 OpenRC 服务，使用 supervise-daemon、五秒自动恢复、default runlevel 与 0640 日志。运行时保持非 root 用户、仅绑定低端口的有效/ambient capability 与 no_new_privs，HUP 发送给被监督的进程。
- CI 增加 amd64/arm64 OpenRC 检查，使用 Alpine 3.24.2 的临时容器与明确的进程夹具，覆盖安装、重复升级、注册失败、日志、HUP、异常退出恢复、default 启动与只读验收快照。systemd 安装分支另验证命令和模板契约，不宣称真实 systemd 实机验收。
- 本地验证：fmt、全 targets Clippy、完整 cargo test 通过；83 项成功，3 项真实运行时专项按已有原因忽略。真实 PostgreSQL 16.15 集成测试完成后已停止临时数据库；actionlint、四组构建脚本测试、shell/内嵌 Python 语法、core 边界与 diff 检查通过。
- 本机无 Docker，使用校验过 SHA-256 的 Alpine 3.24.2 根文件系统，在独立的用户、网络与 PID 命名空间中运行同一 OpenRC 检查。OpenRC 0.63.2、BusyBox 1.37.0 下全部检查通过，确认实际运行时 UID 非 0、CapEff/CapAmb 仅含 CAP_NET_BIND_SERVICE、NoNewPrivs=1，Agent 重启不改变运行时 PID；宿主服务未被安装或重启。
- 下一步以远端 CI 确认双架构 Docker 检查，在专用 OpenRC 设备按 `scripts/e2e-real.sh guide` 验证真实运行时、Reality 客户端和整机重启恢复。现有运行时构建依赖 glibc，Alpine 仍需匹配的 musl 运行时制品；进程夹具不代替完整实机验收。

## PR 准备：同步上游并整合 OpenRC

- 在 `feat/openrc-support` 分支同步上游 `main` 的 `7a8fb70`；继承 `0.2.0` Agent、NodeQuality、公开报告上传缺省关闭、release profile 与 Linux musl 双架构 CI。临时设计参考及相关入口、问题和进度记录已按用户要求删除。
- 解决特权操作与 Agent 入口冲突，保留两种 init 的运行时服务管理。systemd 继续注册并监督 NodeQuality 诊断；OpenRC 不注册诊断能力，服务管理器执行前明确拒绝诊断启动和状态查询，遥测、IP 上报、代理配置与流量计量保持可用。新增两项回归测试覆盖 OpenRC 无 systemd 命令调用和 systemd 诊断监督/状态。
- 保留上游诊断 ADR 0016，OpenRC ADR 顺延至 0017并同步引用；OpenRC 安装夹具同步为 Agent `0.2.0`。README 说明完整诊断需要 systemd；PR 的功能增量为 OpenRC 服务、安装、兼容边界、文档和双架构检查。
- 最终本地验证：fmt、全 targets Clippy（warnings 为错误）、完整 `cargo test --locked` 通过，122 项成功、4 项真实运行时/systemd 专项按既有原因忽略。真实 PostgreSQL 16.15、HTTP/WebSocket、诊断端到端和账本恢复测试完成后，已停止临时数据库。
- Bun 1.4.2 冻结安装与 TypeScript/Vite 生产构建通过，`web/dist` 与上游逐字节一致。四组构建脚本检查、actionlint、shell/Python 语法、文档链接、core 边界和 diff 检查通过。NodeQuality 包装器 20 项测试在隔离的 Alpine 用户/网络/PID 命名空间全部通过，无 root 夹具跳过。
- 在相同隔离环境再次运行真实 OpenRC 检查，安装、重复升级、注册失败、独立运行时、HUP、非 root 权限、日志、异常恢复、default 启动、只读快照和 systemd 安装命令契约全部通过。主机没有安装或重启服务；Docker 双架构、真实 systemd、代理运行时和公网 Reality 的完整结果仍以 PR CI 与专用设备验收为准。

## 主分支修复：OpenRC 依赖缓存与可选平台构建

- 用户要求将近期修改与修复放到 `main`，不再新建分支；OpenRC、上游同步及参考文档删除均已在本地主分支保留。
- 远端 `aab35d0` 的 OpenRC arm64 检查通过，amd64 在 default runlevel 启动超时。使用干净 Alpine 根文件系统，按整秒边界启动测试，复现初始依赖树与服务文件同秒生成时的漏更新：缓存没有两个新服务，直接启动正常但 `openrc default` 没有启动进程。安装脚本增加 `rc-update --update` 强制刷新依赖树。
- 回归夹具让初始缓存时间晚于新服务，稳定覆盖缓存未自动失效的场景，并检查依赖树包含两个服务；默认运行级别输出和失败时的 rc-status/rc-update 状态会保留在日志。修复后在干净、隔离的用户/网络/PID 命名空间完成安装、重复升级、HUP、权限、异常恢复及 default 启动等全部真实 OpenRC 检查；本机无 Docker，不将本地结果当作双架构容器验收。
- 对照用户指定的本地 NodeFlare Actions，新增手动或 `v*` 标签触发的多平台制品工作流。保留日常 Linux musl CI；GNU/Linux 继续使用 Ubuntu 24.04 动态库，macOS 仅 ARM64，Windows 与 FreeBSD 提供双架构。FreeBSD 使用同一固定 cross 提交和 FreeBSD 13 sysroot 镜像，在 Linux 安装目标标准库，并在 13.5/14/15 VM 验证同一二进制；只在最新 15 VM 安装 Python 并校验打包，避免 ARM64 rustup 安装器和旧版本包仓库依赖。编译目录放在工作区外，避免将中间产物反复复制进 VM。
- 构建脚本增加已有二进制验证模式，仍校验架构、版本、CLI、动态库与部署限制，成功后才创建不可覆盖制品和 SHA256SUMS。五组构建脚本检查通过，包含所有目标在没有 Rust/PATH 的情况下拒绝非法二进制；本机 Agent `0.2.0` 的实际 GNU/Linux 二进制验证、打包与 SHA-256 校验通过。
- fmt、全 targets Clippy、完整 `cargo test --locked` 通过，122 项成功、4 项实机专项按原原因忽略；临时 PostgreSQL 的连接用户确认后完成全部集成测试。actionlint、shell/Python/Cross.toml 语法与 diff 检查通过。临时数据库已停止；后续以推送后的 OpenRC 双架构 CI 和手动多平台工作流确认远端执行结果，非 Linux 服务部署仍不在范围内。

## 主分支调整：统一日常多平台构建

- 按用户进一步澄清，将九个 Agent 目标统一移回 `ci.yml`，每次 push/PR 或手动执行均构建。删除临时的独立多平台工作流，OpenRC 不另设构建目标或 CI 任务。
- Linux 使用同一个 libc × 架构矩阵：musl 静态 amd64/arm64 用于 Alpine 等设备，Ubuntu 24.04 glibc 动态 amd64/arm64 用于兼容的 systemd 设备。OpenRC 安装、缓存、重载、权限与恢复检查并入 musl 任务；原 systemd 专项保留在主检查任务。两种 init 仍在运行时识别，二进制 libc 与服务管理分别处理。
- macOS ARM64、Windows 双架构和 FreeBSD 双架构纳入日常矩阵；FreeBSD 保留 Linux cross 编译、固定 sysroot 及 13.5/14/15 的同二进制验证。制品名称继续区分系统、libc 和架构。
- 本地 actionlint、五组构建脚本检查、fmt、全 targets Clippy、完整 `cargo test --locked` 均通过，122 项成功、4 项实机专项按原原因忽略；临时 PostgreSQL 已停止。未修改 Agent、安装接口、协议或前端代码。
- 上一提交 `b16fffd` 的 [CI 36699121265](https://github.com/imengying/sinan/actions/runs/36699121265) 所有任务均成功，包括真实 systemd、Compose、musl 与 OpenRC 双架构。新的九目标构建仍须以本次提交的完整 CI 结果为准，不将前一提交结果视为已验证新增目标。

## Agent 对齐阶段 1：运行时 musl 与安装恢复

- 用户确认全部补齐 Agent 差距，新增 ADR 0018 并更新协作约束。workspace 升至 `0.3.0`，为后续 Agent 新行为保留不可变版本；协议主版本仍为 1。
- 运行时脚本新增官方 musl 工具链及 `with_musl`，保留完整上游标签和 `with_v2ray_api`；四个 Linux ABI/架构制品按独立名称保存，旧 glibc 制品保留。面板按上报 OS/libc 选择，musl 缺失不会误退回 GNU；兼容旧 Agent。
- 安装校验版本及既有制品内容，切换后检查本地状态；启动失败恢复旧版本和原 TOML，仅操作 Agent。隔离 Alpine/OpenRC 的安装、失败回退、独立运行时、HUP、权限、缓存与 default runlevel 检查全部通过。
- fmt、Clippy、完整 Rust 测试通过，124 项成功、4 项实机专项仍按原原因忽略；新增测试覆盖 GNU/musl/FreeBSD 选择、未知 ABI 拒绝和旧路径兼容。五组构建脚本检查、actionlint、shell 和 diff 检查通过。
- CI 增加四个真实运行时构建、对应 libc/架构运行及本地流量计数和重载检查；本机没有固定 Go/Chromium 工具链，不能将脚本和夹具检查宣称为真实构建通过，结果须以该提交远端 CI 为准。
- 下一步：扩展遥测、批量上传和持久补报，然后完成任务、多系统常驻运行和 Agent 自动升级。
- 远端实际验证：[26590ea 的 CI](https://github.com/imengying/sinan/actions/runs/36711000286) 全部 15 个任务成功；GNU/musl 双架构运行时均完成真实构建、对应 libc 运行和流量/重载专项。

## Agent 对齐阶段 2–3：遥测补报与设备任务

- 新增 SWAP、进程数、逐盘 I/O、GPU 与可关闭的公网地址识别。一秒采样与三秒压缩上传独立运行，普通指标持久补报，大小和时间保留有界；面板按样本身份去重、事务确认，迟到样本不倒退当前指标。
- 增加管理员命令、持续 TCP/ICMP 拨测和服务器详情页入口。命令限制领取期、执行时长和输出，执行前持久记录；重启不重跑状态不明的命令，已完成结果不可修改。拨测结果持久补报，删除目标后确认丢弃迟到结果，保持设备间隔离。
- OpenRC 注册 NodeQuality 能力，使用独立一次性服务、挂载命名空间、持久终态和超时终止；实际 Agent 已在隔离 Alpine/OpenRC 环境通过成功、失败、超时、重复执行拒绝与挂载隔离检查。安装、回退、代理独立存活及 default runlevel 检查继续通过；双架构实际运行纳入原 musl CI。
- SQLite 辅助表保持账本版本兼容，旧 Agent 回退后仍能打开账本。未增加库，面板仅将已有 flate2 测试依赖移至运行依赖。
- 本地完整 Rust 测试 136 项通过、4 项实机专项按原原因忽略；真实 PostgreSQL 覆盖压缩解码上限、重复批次、迟到样本、命令认证与不可变结果、拨测去重和删除目标后的确认。fmt、全 targets Clippy 和 TypeScript/Vite 生产构建通过。
- 下一步：完成 macOS、FreeBSD、Windows 的注册、常驻服务和安装，以及默认关闭的 Agent 自动更新与启动失败回退。设置项已经提供，更新执行器将在下一阶段接入。


## Agent 对齐阶段 4–5：原生服务与更新恢复

- macOS ARM64、FreeBSD 双架构与 Windows 双架构接入完整 Agent 运行。新增 launchd、rc.d/daemon 和 Windows 启动时计划任务；运行时使用独立普通账户/服务，Windows 状态通过 ACL 与带令牌的本地命名管道保护。Agent 安装目录独立配置，代理适配器保持无状态。
- 安装入口自动选择 OS/libc 对应产物，增加 PowerShell 安装脚本和面板中的系统选择；原生安装检查失败恢复旧 Agent。Linux 安装加入独立监督入口并备份旧服务定义，允许回退到没有 supervise 命令的旧 Agent。
- 自动更新默认关闭，只选绑定面板内更高、匹配平台的稳定版；校验摘要、格式和实际版本后保存待升级状态。监督进程验证新 PID/版本，失败恢复旧版本，未确认升级中断时恢复上一版本；失败版本有界记录。Agent 退出时取消并清理正在执行的命令，持久结果不重放。
- 新增固定上游的 macOS/FreeBSD/Windows 运行时构建，Windows 附带对应 DLL；制品缓存校验辅助文件集合及摘要。CI 仍为一个工作流，原生 Agent 加入遥测、命令、拨测和升级回退测试，并增加真实服务、运行时配置及回环流量验证；FreeBSD 13.5/14 验证二进制兼容，15 验证完整服务。
- 本地真实 Agent 已通过注册、压缩上报、离线重启补报、命令去重、TCP 拨测、升级成功、启动失败回退、失败版本抑制及退出清理。隔离 Alpine/OpenRC 的安装、旧服务回退、default 启动和实际 Agent 一次性诊断全部通过。
- 本地 fmt、Clippy、完整 Rust/PostgreSQL 测试通过（140 项成功、4 项原有实机专项忽略）；TypeScript/Vite 构建、五组构建脚本测试、actionlint、Python 语法与 core 分层边界检查通过。
- 下一步：检查本提交远端 CI，修复原生系统执行差异。上一阶段 48728c8 的九平台 Agent、四平台运行时、Compose 和 OpenRC 已成功，主 check 失败；本轮修复了负进程组 kill 的参数歧义，但尚未取得旧任务完整日志，不能宣称已确认其唯一原因。macOS/Windows/FreeBSD 服务尚待本提交原生 CI；公网 Reality、GPU 负载和整机重启不属于已完成验收。
