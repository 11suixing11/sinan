# 执行进度

## 交付加固第 0 阶段：已完成，main CI 全绿

- 自动 CI 精简为 Rust/前端检查、Compose 持久化 smoke、Linux musl amd64/arm64；runner 固定 Ubuntu 24.04。原 FreeBSD 工具链候选修复及原未提交差异保存在仓库外，未纳入本次提交。为避开两个活跃聊天共享目录的写入，本任务改用独立 worktree；既有部署与凭据保持私有。
- release profile 开启 `strip=true`、`lto=true`、`codegen-units=1`；musl jobs 对同源码、同工具链的旧 profile 和新 profile 分别构建，精确字节数、缩减比例随 artifact 与 Actions summary 保存。同源 CI 实测：amd64 从 21,326,304 降到 9,483,600 字节（缩减 55.53%）；arm64 从 20,547,008 降到 8,156,432 字节（缩减 60.30%）。
- NodeQuality 新任务默认 `upload_report=false`，创建任务时显式选择公开上传。Agent 与包装器严格传递此选项；本地完整报告仍保留。固定上游源码未改，包装制品升为 `-r2`，旧 Agent 拒绝新任务。已经排队或运行的旧任务沿用旧策略，应先结束再升级。
- 本地验证：`cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked` 全部通过，116 项通过、4 项按既有原因默认忽略；使用真实 PostgreSQL 16.15，包含 HTTP/WebSocket、诊断和账本恢复测试。Bun 冻结锁文件安装及生产构建通过，依赖未增加。构建脚本 4 项通过；NodeQuality Python 20 项中 15 项通过，5 项 Linux/root 夹具留给本阶段 CI。r2 双架构包装实际构建通过，嵌入上游字节保持一致。actionlint、YAML/矩阵与 diff 检查通过。
- 已核对前一个 main 的全部 jobs：仅 FreeBSD arm64 工具链安装失败；这些历史通过状态不能认证本次变更。本阶段 [7a8fb70 的 main CI](https://github.com/theLucius7/sinan/actions/runs/36684582275) 已全部成功：check、Compose smoke、musl 双架构；Linux/root 的 NodeQuality 全部夹具及真实 systemd 专项也在该提交通过。两架构 artifact 已实际下载核对体积记录。
- 下一步：完成干净 Linux/systemd Agent 的 Reality、重启计量和 0.1.0→0.2.0 升级验收；现有部署聊天已完成，不清空现有面板数据库。

## 交付加固第 1 阶段：已完成真实链路与升级验收

2026-09-30 在既有云主机上创建独立 Debian 12 / systemd 252 容器、独立 Docker 网络与私有证据目录。面板/PostgreSQL 沿用既有部署；仅创建专用验收服务器、节点、用户。Mac 独立客户端通过公网 Reality 完成 HTTPS 访问与受控双向流量。未修改宿主 Agent、原服务器记录或宿主反代。

- 初次 Agent 0.1.0 应用修订 1，运行时 1.14.2，包含 with_v2ray_api；统计仅在回环地址监听。
- 首轮上传 1,049,440 字节、下载 2,102,444 字节。停止客户端后，Agent 重启与运行时重载均保持累计值不变、待确认批次 0，运行时 PID 203 不变。
- 使用正式新令牌与完整安装脚本执行 0.1.0 → 0.2.0。升级 UTC 08:24:19.681271 开始、08:24:20.404582 结束；身份各文件摘要、服务器 ID、配置摘要、修订目录、运行时 PID 均保持不变。SQLite 文件保留，schema_version 6 保留，序列从 6 增至 7。独立客户端一次 2 MiB 下载于升级开始后 0.123 秒发起、结束后约 2.830 秒完整成功，提供跨升级窗口的实际请求证据。
- 恢复流量后的实际累计上传 21,573,061 字节、下载 44,832,934 字节，总计 66,405,995 字节。停止客户端后，连续三样本、间隔 35 秒确认累计值稳定且待确认批次 0。
- 专用节点修改 SNI 后修订 2 健康应用，恢复后修订 3 健康应用，目标修订均等于已应用修订；累计流量保持上述数值，身份与账本连续，原服务器部署状态不变。

私有证据包括 before、after-agent、after-runtime、resumed、after-upgrade、upgrade-before/after/execution、managed-republish、final-identity-ledger。运行时 PID 最终仍为 203。

边界与 workaround：这是干净 Debian 12 容器的真实公网链路验收，不是重装整台云主机；面板沿用原 PostgreSQL。原产物版本不可变，因此升级使用面板已有的历史 0.2.0 制品，优化构建的体积验证另由第 0 阶段 CI 提供。设备访问公开面板经过外部网关返回拒绝响应，验收容器仅覆盖自身域名解析指向 Docker 网关，TLS/SNI 校验仍启用；私有 API 驱动只对面板主机名使用回环解析。节点默认端口与容器公网映射不同，客户端仅在私有验收配置中覆盖公网端口。历史 0.1.0 安装脚本由原正式脚本私有副本修改版本及校验值，原文保留；相关问题见 issue #3。证据、身份、令牌与环境地址不提交仓库。

- 公网连续载荷测试有两次 15 秒请求超时，首次发生在升级前；90 秒预算的升级后双向载荷校验通过，不宣称连续所有请求无错误。[版本选择 #3](https://github.com/theLucius7/sinan/issues/3)、[CDN 与 origin 诊断 #4](https://github.com/theLucius7/sinan/issues/4)、[手动端口 #5](https://github.com/theLucius7/sinan/issues/5)、[公网超时 #6](https://github.com/theLucius7/sinan/issues/6) 已建立。
- 新增可恢复的专用资源 API 驱动、私有证据保存与精确流量/健康/身份/outbox 断言，以及受控双向 HTTP fixture。7 项驱动契约测试通过；全 workspace fmt、所有 targets Clippy（warnings 为错误）、cargo test 通过，116 项通过、4 项按既有原因默认忽略，数据库测试使用真实 PostgreSQL 16.15。
- 下一步：将实际安装、systemd、Reality 流量和账本复核固化为每个 PR 的 CI；签名阶段按用户指定 minisign、多编译时公钥及本地离线签名执行。

## 交付加固第 2 阶段：已完成，PR 真实 CI 全绿

- 新增 Ubuntu 24.04 `real-e2e`，在每个 PR 与 main push 执行；复用同次 CI 的优化 musl amd64 Agent。固定上游运行时 1.14.2，以版本和构建脚本摘要缓存，恢复后仍核对 SHA-256、归档边界、ELF 架构、Go 1.26.8、固定 revision 与完整默认标签加统计标签。第 0 阶段已保留同源体积对照，常规 CI 不再重复构建旧 profile。
- 专用 Compose 面板与 PostgreSQL 提供制品；真实安装脚本在干净 runner 宿主安装，由 systemd 启动 Agent 和独立运行时。本地 Reality 客户端向受控回环 HTTP fixture 下载 2 MiB、上传 1 MiB，核对内容与精确计量。暂停后连续三样本、间隔 35 秒确认计量稳定与 outbox 空，再验证 Agent 重启、运行时 HUP、恢复流量和同版本重新安装的身份、运行时 PID 与计量连续性。
- 首次 [PR CI 36693149491](https://github.com/theLucius7/sinan/actions/runs/36693149491) 实际完成运行时冷构建与校验，并发现 [HUP 后同量新流量漏计 #9](https://github.com/theLucius7/sinan/issues/9)：上游新代按用户惰性创建统计，成功空响应未归零旧基准；恢复相同载荷后累计计数等于旧基准，因而被算成零增量。第 1 阶段恢复了更大的流量，未覆盖这个边界。
- 修复只在无状态适配器中，根据已应用配置的统计用户白名单补齐零计数；未知用户不计入，RPC 与格式错误继续失败。未修改 core、协议或上游源码。CI 加入只读 SQLite 基准对照，要求首轮面板总值等于基准、HUP 后基准为零、恢复流量的面板增量等于新周期基准。
- [43ab0e5e 的 PR CI 36697058255](https://github.com/theLucius7/sinan/actions/runs/36697058255) 全部成功：Rust/前端检查、Compose 持久化 smoke、musl amd64/arm64、真实安装与 Reality 计量 job。该次运行实际命中运行时缓存，重新校验通过后执行验收。
- 公开摘要已实际下载并核对。首轮上传 **1,048,821** 字节、下载 **2,097,454** 字节；暂停、Agent 重启与 HUP 后面板累计值均不变，HUP 后 SQLite 基准准确归零。恢复相同载荷后累计上传 **2,097,642** 字节、下载 **4,194,908** 字节，总计 **6,292,550** 字节；新增量与只读账本新周期基准精确相等。同版本重新安装后累计值、身份文件摘要与独立运行时 PID 保持不变。
- 脚本先拒绝已有 Sinan 安装，仅清理本次创建的服务、Compose 项目、私有临时目录与测试映射。公开 artifact 与 Actions summary 仅含白名单中的版本、阶段和十进制用量，不上传身份、令牌、订阅、配置或原始日志。
- README 精简并拆出 `docs/deploy.md`、`docs/dev.md`；部署文档保留真机发现的版本耦合、CDN 拒绝、端口映射和公网超时问题。Docker 面板默认两个 Rust 编译任务，降低 LTO 构建的内存压力。
- 本地验证：fmt、全 targets Clippy（warnings 为错误）、默认 cargo test 通过，119 项通过、4 项按既有原因默认忽略；显式真实运行时 HUP 与同量新流量专项通过。7 项驱动与 3 项缓存契约、Shell/Python 语法（含嵌入块）、文档相对链接、actionlint 与 diff 检查通过。账本对照临时契约拒绝错误总值、丢失第二批、非零重载基准和待确认批次，并确认数据库只读检查不修改原数据。检查日志保存在仓库外。
- 验收边界：回环 CI 不覆盖外部 CDN、云 DNS、防火墙或公网超时，也不替代第 1 阶段 0.1.0→0.2.0 的跨版本升级证据。[版本选择 #3](https://github.com/theLucius7/sinan/issues/3)、[CDN 设备路径 #4](https://github.com/theLucius7/sinan/issues/4)、[手动端口 #5](https://github.com/theLucius7/sinan/issues/5)、[公网超时 #6](https://github.com/theLucius7/sinan/issues/6) 仍是后续处理范围。此阶段制品尚无签名，不能把 SHA-256 校验视为已建立发布信任链。
- PR #7 已合入 main，合入提交为 `8a9ad2f`；[该提交 main CI 36724902150](https://github.com/theLucius7/sinan/actions/runs/36724902150) 实际全部 5 个 job 成功，包含真实 Reality 安装和精确计量。下一步：继续发布、离线签名与版本解耦阶段。

## 交付加固第 3 阶段：实现已合入，正式发布待用户签署

- 先采纳 ADR 0017，按用户决定使用 minisign-verify、构建时多个公钥和独立产品版本。Agent 为 0.3.0，面板仍为 0.2.0，协议范围为 1..=1；公钥不由面板或安装脚本向 Agent 下发，生产命令没有运行时换根开关。
- protocol 的完整四行签名、canonical 清单和 metadata 绑定通过 14 项测试；包括正文与 trusted comment 篡改、多根与真实测试根轮换。core 下载、缓存、准备、应用、同 revision、回滚、未完成事务恢复及诊断启动验证实际二进制与签名证明；70 项单元测试通过，1 项真实 Linux/systemd 专项仍由 Linux CI 验证。CLI 和 systemd 预检绑定期望的制品角色及格式，不能用另一种已签制品替换执行目标。
- 新增只读 verify-cache 升级预检，检查已应用、未完成 target/previous、诊断检查点和无数据库 current。主库与活动 WAL 字节保持测试通过；旧未签缓存不能自动认可。GNU mv、systemd ExecStartPre 与实际签名安装路径已由本阶段 Ubuntu 24.04 CI 验收。
- 面板固定官方 GitHub 仓库，从 tag 导入整个签名 Release；校验完整内容后单次发布目录。6 项存储测试通过：失败原子性、篡改拒绝、幂等、组件不可覆盖、兼容 Agent 选择、软链和缺信任根拒绝。HTTP 网络阶段也受并发限额约束；Agent 仍从配置的面板同源下载并独立验签。
- 前端新增 Release 导入和显式 Agent 版本选择，生产 Bun 构建通过；空制品列表不显示已验证徽章。全 workspace fmt、全 targets Clippy（warnings 为错误）与测试通过：145 项成功，4 项依赖真实上游运行时或 Linux/systemd 的专项默认忽略。签名夹具覆盖面板诊断、真实传输、丢 ACK 恢复和 bootstrap/鉴权。
- 最终共享树在隔离 Debian 12 容器使用真实 minisign 0.11 执行 47 项 Python 测试，全部通过且无跳过。审查发现并修复安装器依赖 Python assert 的缺口：现在使用隔离 Python 与显式长度、SHA-256 拒绝逻辑，任何新二进制执行前完成独立校验；优化模式下同长度篡改、超出已签长度的流及下载重定向均拒绝，合法签名安装仍通过。正式面板来源使用 HTTPS，仅明确回环地址允许 HTTP。
- 发布候选流程构建双架构 Agent、运行时及诊断制品，输出 metadata 和 SHA256SUMS，先创建 draft。用户在仓库外本机生成带口令私钥、只提供公钥、本地签署并上传 minisig；CI 不取得生产私钥。正式发布要求全资产验签和对应 main 必需 CI，已知测试根在正式流程中拒绝。当前没有正式公钥、正式签名或正式 Release，不能把测试根验收当成生产信任链已经建立。
- 发布流程在缓存恢复或新构建后，以归档、ELF 和 Go metadata 检查两种架构、固定源码 revision、工具链及构建标签；检查不执行缓存二进制。此信息用于发现错误产物，不作为独立构建证明。12 项验收驱动、8 项签名 CI 契约、3 项既有缓存契约及 actionlint、Shell/Python 语法检查通过。
- [PR #11](https://github.com/theLucius7/sinan/pull/11) 已合入 main（`20d09ca`）。[PR CI](https://github.com/theLucius7/sinan/actions/runs/36738530095) 的 5 项全部通过，包括真实签名安装、篡改二进制/证明/旧未签缓存拒绝、恢复 systemd 验签器、Reality 双向流量、重启、HUP、精确计量和同版重装。首轮上传/下载为 1,048,821/2,097,454 字节，重载后相同流量累积精确为两倍。使用 TEST_ONLY 根，不能替代正式发布签名。
- [main CI](https://github.com/theLucius7/sinan/actions/runs/36740903057) 的全部 5 项也已通过。正式公钥与本地签署仍待用户完成，安全功能和链式 ADR 继续推进。

## 交付加固第 4 阶段：实现与本地验收已完成

- 在线删除先持久下发退役请求，Agent 停运行时和诊断、提交持久用量、清身份与运行配置，再通过设备签名回执确认删除。断线和清理失败可恢复，退役后退出 78 且不自动重启；离线软删除明确未确认清理。短期生命周期锁统一连接注册、删除及回执边界，防止重连插入已删除设备会话。
- 管理员登录与安全设置共用持久 IP/全局限速；TOTP 的设置、确认、登录和关闭使用事务消费验证码，启用/关闭撤销其他会话。被限流拒绝的 IP 不继续消耗全局额度。无密码单独关闭二步验证的接口，部署所有者恢复步骤在部署文档。
- 订阅链接可原子重置，旧链接立即失效，既有用户授权、配置和账本保持。节点创建与编辑允许手动指定 443 等端口，同服务器冲突拒绝，创建留空仍从 20000–29999 分配。
- 本地最终 fmt、全 targets Clippy 与 workspace 测试通过：168 项成功、4 项运行时/Linux 专项默认忽略。真实 PostgreSQL/HTTP/WebSocket 覆盖退役回执、失败恢复、重连竞争、登录限速、TOTP 并发消费、端口竞争及订阅旧链接失效。既有删除测试显式断开设备，以分别验证离线撤销和在线退役语义。
- Chrome 浏览器实测通过 TOTP 完整设置/登录/关闭、真实剪贴板、443 创建及中文冲突提示、旧订阅 404/新订阅可读，无 JavaScript 异常；使用独立数据库并清理全部临时进程和秘密。订阅夹具未连接 Agent，其结果不代表流量验证。
- 人工验收脚本支持隐藏输入 TOTP 或一次性环境变量，不把验证码写入 state。17 项驱动、8 项签名 CI、3 项缓存契约与 Shell/Python 语法、actionlint、diff 检查通过。真实 CI 让非 root 运行时实际监听 443，将伪装 TLS 放到独立容器；在签名/Reality/重装流程末尾增加单次在线退役，检查服务停止、凭据清理、账本保持和再次启动拒绝；Linux 结果以本阶段 PR 运行记录为准。

## 交付加固第 5 阶段：设计文档已完成，未实现链路代码

- [ADR 0020](docs/adr/0020-chained-transit.md) 定义逐链路内部 UUID 生成和持久轮换：出口双接受、入口真实候选探测、切换、固定可恢复代数、再撤旧。Reality 私钥变更使用双端点过渡，离线及紧急撤销不宣称无中断。
- 用户流量仅在公开入口计量，内部监听、出站和探测排除在统计白名单及用户批次之外；用户账本结构和去重键保持，历史授权来源兼容旧快照，不能随显示投影覆盖历史事实。
- 出口投影变化通过反向依赖、事务发布待办和精确 generation/revision/hash 确认触发入口重编。编译输入、依赖模型、恢复屏障与订阅边界均列明影响；回环探测还需短期独立鉴权及受控目标限制，不能成为免计量本机代理。
- 已对照现有 compiler、adapter、core、publisher、计量与订阅实现审阅。所有链式行为验收条目均明确列为未来工作，本阶段没有新增链路代码、接口或数据库迁移。

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

## 后续调整：VPS 部署、HTTPS 与本机 Agent（2026-09-30）

- 在 Debian 13 amd64 VPS 实际部署 PostgreSQL 16 和非 root 面板容器，面板仅监听宿主回环地址；复用现有 Caddy 配置追加独立站点，公网 CDN 与源站 HTTPS 均验证成功，设备通过公网域名完成注册和 WebSocket 连接。
- 本机安装 CI 提供并验证过 SHA256、版本和静态 ELF 的 `0.2.0` Agent，交由 systemd 管理并启用开机启动。面板显示在线且持续上报 CPU、内存、磁盘、网络及系统信息；重启 Agent 和重建面板后仍能恢复连接，原设备身份和服务器记录保留。
- 使用工具链容器实际构建 Linux/amd64 sing-box 1.14.2：固定上游提交、Go 1.26.8、完整默认标签与 `with_v2ray_api`，版本启动和校验清单检查通过。Agent、运行时和 NodeQuality 三类 amd64 制品均已导入面板并可被认证接口识别。
- 新增 `scripts/init-env.py`，用独立随机密码和创建时 `0600` 权限生成 Compose 配置，拒绝覆盖文件或符号链接，支持外部文件及自选宿主端口。新增实际 CLI 回归测试及 CI 检查，并用生成结果验证 Compose 配置解析。
- 新增可追加的 Caddy 示例和 HTTPS 部署说明；Docker Rust 编译默认并发降为 2，支持 build arg 调整。修复未发布配置的设备在开机时反复启动代理失败的问题：systemd 仅在运行时及配置就绪时启动，实机无配置时确认跳过且重启计数为 0。
- 实测 `scripts/ci-compose-smoke.sh` 通过，覆盖前端资源/API、登录、数据库和数据卷在容器重建后的持久化。公网浏览器通过桌面和手机布局、登录、设备详情、实时指标及制品列表验证，无页面脚本错误；未认证的管理、设备清单和无效接入令牌请求均返回 401。
- 验证边界：此次为现有 Debian 13 VPS 部署，不是全新 Debian 12 双架构验收。未创建真实业务节点或用户，未执行公网 Reality 客户端全链路与真实 NodeQuality CPU、磁盘、带宽测试。已有 FreeBSD ARM64 工具链问题不在本次部署调整范围内。
- 完整检查：Rust 1.98.1 下 `cargo fmt --all --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked` 通过，默认 114 项成功、4 项按原有条件忽略；随后使用此次构建的真实 Linux 运行时显式运行 3 项运行时专项，全部成功。容器内没有 systemd，既有独立诊断服务专项未在容器执行；本次修改的运行时启动条件已在 VPS systemd 实测。配置初始化 3 项、制品构建 4 组及 NodeQuality 包装器 16 项 Python 检查全部通过，后者没有跳过。

## 2026-10-01：整合 VPS 部署 PR

- 解决 PR #2 与签名发布、账户安全及设备退役改动的冲突，保留精简 README 和分章部署文档。初始化命令同时生成空的构建时发布信任根配置，部署前仍须独立核对公钥。
- 保留 systemd 的签名缓存启动前复验，并添加未配置运行时的路径条件；Docker 编译并发参数仅保留一处定义。
- 本机初始化 CLI 3 项、制品构建脚本 4 项测试和差异空白检查通过；Linux Compose、systemd 和完整集成检查交由本次 PR 的远端 CI 验证。

## 2026-10-01：P0 常驻服务优先级

- Agent 和 sing-box systemd 单元分别增加 `OOMScoreAdjust=-500`、`CPUWeight=1000`，降低常驻进程的 OOM 候选优先级并提高资源争抢时的 CPU 权重；没有添加 CPU 配额。安装器直接嵌入这两份源单元，渲染后逐字核对一致。
- 专用 Debian 12/systemd 252.39、1 GiB/2 CPU 验收容器中，从源单元生成独立短命夹具，分别保留 root 和运行时用户/能力设置。systemd 属性、内核 oom_score_adj 和 cgroup cpu.weight 均验证为 -500/1000，CPUQuotaPerSecUSec 为 infinity。已有 Agent 和运行时 PID、运行状态、重启数保持不变，夹具已停止并删除。
- 单元语法校验使用临时可执行文件路径，未调用真实业务程序；安装器 shell 语法、构建与签名发布 Python 检查、差异空白检查通过。独立步骤和验收边界见 [常驻服务优先级验收](docs/acceptance/resident-service-priority.md)。完整 Rust/Compose 检查由该 PR 的 CI 执行，完整诊断与持续代理流量压力验收仍待其他 P0 改动完成。
