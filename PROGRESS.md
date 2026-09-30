# 执行进度

## 2026-10-01：会话签发时间跨秒修复

- 对应 [Issue #47](https://github.com/theLucius7/sinan/issues/47)。[PR #45 的 CI](https://github.com/theLucius7/sinan/actions/runs/36767325898) 中服务夹具已通过，认证测试暴露两次取时跨秒：存储的会话过期时间与稍后 ACK 的服务器时间相差 3599 秒。会话签发和 ACK 现在使用同一时间快照，过期时间仍由数据库保存并供 HTTP/WebSocket 强制校验。
- 在独立 PostgreSQL 测试库中先锁住会话表，确认认证 INSERT 实际等待，再跨秒并释放；旧生产代码稳定触发 3599 与 3600 不等，修复后 6 项 foundation 测试全部通过。保留严格的一小时断言，并核对数据库与 ACK 的过期时间一致。协议文档明确此时间是签发快照，客户端校时仍受握手延迟影响。
- workspace fmt、全 targets Clippy 与分层检查通过。完整测试首轮因本机临时磁盘不足失败，空间恢复后的完整 Rust/PostgreSQL 重跑通过：261 项成功、8 项真实 systemd/运行时专项按条件忽略；新提交的 Linux CI 尚待验证。
- main `13f2975` 的真实 Reality 安装/计量 job 已通过：443 流量首次为 3,146,275 字节，Agent 重启与运行时 HUP 后不变；第二批精确为 6,292,550 字节，重装不重复计量。4 项签名拒绝、在线退役和再次启动拒绝均通过；其主检查因旧服务夹具失败，不能把该结果视作 main 全绿。

## 2026-10-01：诊断锁服务测试夹具同步

- 对应 [Issue #43](https://github.com/theLucius7/sinan/issues/43)。main `13f2975` 的 [CI 36766239541](https://github.com/theLucius7/sinan/actions/runs/36766239541) 中两项服务集成测试失败：旧特权替身没有返回锁目录元数据，或完全拒绝新增目录操作。仅更新 `services.rs` 夹具，生产代码和锁权限校验不变。
- 夹具显式允许固定目录、root/0700 和精确 stat 参数，其他文件操作继续拒绝；补齐 systemd 的 flock 完整参数、OpenRC 私有 umask 及调用顺序断言。6 项服务测试、workspace fmt、全 targets Clippy 和完整 Rust/PostgreSQL 回归通过：261 项成功、8 项真实 systemd/运行时专项按条件忽略；分层检查通过。新提交的真实 Linux CI 待完成。
- 此前 ABI 修复 [PR #39](https://github.com/theLucius7/sinan/pull/39) 的 [五项 CI](https://github.com/theLucius7/sinan/actions/runs/36763913065) 全部通过：Reality 443 的首次 3,146,275 字节在 Agent 重启/HUP 后不变，第二批后精确为 6,292,550，重装不重复计量；4 项签名拒绝和 12 项在线退役断言通过。该证据属于 PR 提交 `d660db5`，不替代新增诊断改动后的 main 验证。

- 本轮保留作者 `7317311` 的夹具修复并整合 main `07e8f58`（含旧运行时缓存 ABI 兼容），生产锁目录和服务逻辑未改动。合并源的 6 项 `services` 集成测试全部通过，专用 target Clippy（warnings 为错误）、workspace fmt、core 门禁及差异检查通过；该结果覆盖两份原失败夹具与已有命令/服务状态拒绝回归。没有重复运行完整 workspace 或实际 init 服务专项，前述贡献者完整回归属于其原提交，最终合并源的 Linux/systemd/OpenRC 与 Reality 验证由最终 CI 执行。

## 2026-10-01：修复静态 Agent 的宿主运行时选择

- 对应 [Issue #37](https://github.com/theLucius7/sinan/issues/37)。`41000c8` 的 [CI 36757545068](https://github.com/theLucius7/sinan/actions/runs/36757545068) 中 check、Compose、双架构 musl 均通过，真实 systemd 队列/资源测试及实际 Rust OpenRC 诊断也通过；Reality 安装成功后等待配置应用超时，未进入流量验收。之后 main `e2d898c` 仍在相同步骤失败，不能沿用旧提交的绿色结论。
- 使用真实 PostgreSQL/HTTP 回归复现：musl Agent 在 GNU 宿主只获得旧格式 GNU 运行时发布时，清单错误返回 404。新增 `runtime_libc` 区分宿主与 Agent 编译 ABI，仅读取有界 ELF 解释器信息；未知宿主保守沿用编译 ABI，旧设备缺字段兼容。Agent 自身更新仍按编译 ABI。
- GNU 宿主上的 musl Agent 保留原 musl、legacy 架构键优先级，然后才尝试 GNU 完整标识；面板与 Agent 验签使用一致顺序，避免旧缓存同一 proof 中有多个 ABI 时换选摘要。musl Agent 在 musl 宿主不由面板获得 GNU 运行时；已存在但校验失败的候选不降级。
- 合并审查补充反向兼容：GNU Agent 已经通过兼容层运行在 musl 宿主时，旧 GNU 完整标识及 legacy 缓存仍排在新 musl 标识之前。纯 musl Agent/宿主路径继续拒绝 GNU；新增 signed-selection 和面板双 ABI/缺失首选内容回归，不把结构夹具作为 gcompat 实机支持证据。
- 反向兼容补丁基于 main `13f2975` 验证：ABI 3 项、签名缓存 7 项、协议 18 项、真实 PostgreSQL 平台清单/Agent 更新 4 项全部通过，共 32 项且无忽略。验收驱动 21 项、CI helper 9 项及签名 CI 8 项 Python 检查通过；workspace 全 targets Clippy（warnings 为错误）、fmt、core 分层、shell 语法及差异空白检查通过。未运行本补丁的完整 workspace 测试、gcompat 实机或 Linux Reality 验收，其结果继续由对应提交的 CI/专用节点确认。
- 超时只在私有文件保存最终原始状态，权限为 0600；公开失败诊断只输出有界配置修订号、就绪布尔值及固定 systemd 单元状态。新增包含令牌、主机名、IP 与自由文本的回归，验证这些内容不会进入公开摘要。
- 在 main `e2d898c` 上整合后，完整 workspace fmt、全 targets Clippy（warnings 为错误）及 Rust/PostgreSQL 测试通过：245 项成功、6 项真实 systemd/运行时专项按条件忽略；原始 404 回归修复后通过。Python discovery 72 项通过、5 项既有条件忽略，验收驱动 21 项、缓存 3 项和签名 CI 8 项通过，分层与 shell 语法检查通过。修复后的真实 Linux 安装/Reality/计量仍须本次提交 CI 验证。

## 2026-10-01 P0 有界流量 outbox：自动验收完成，专用节点待验收

- 对应 [Issue #18](https://github.com/theLucius7/sinan/issues/18)，仅处理第 1 步「有界读取」并独立提交。`pending_usage()` 在 SQL 层先 LIMIT 64，再按全局序号及累计字节取前缀；单轮 usage 消息预算 1,048,575 字节，包含 envelope 预留。增加部分排序/字节索引，读取超限旧正文时只检查 SQLite 字节元数据。
- 新样本按 128 KiB 预算切批，所有切批、累计基准和全局序号同一事务落盘。既有 `(epoch, seq)` 与正文保留；超限旧批不假 ACK，明确日志报错，本地 status 显示阻塞数量与对账错误，后续可发送批仍能继续。面板按批次身份幂等入账，不要求连续序号；退役仍等待全部真实 ACK。
- 每 15 秒发送轮最多调度 1 秒，逐批 yield 并优先处理控制消息与心跳；剩余窗口留待下一轮。一次正在进行的 socket 写入仍保留原有 10 秒超时，1 秒不是连接循环硬截止时间。采集逻辑未改；采集解耦单独推进。
- 本地 `cargo fmt --all --check`、workspace 全 targets Clippy（warnings 为错误）、`cargo test --locked` 通过：178 项通过、4 项按原有原因忽略。签名集成测试使用仓库公开 TEST_ONLY 根与隔离 PostgreSQL 16；未设置编译公钥的首轮失败在按开发文档补齐环境后通过。新增 7 项账本/SQL 专项和 1 项真实回环 WebSocket 专项，覆盖 4,096 批积压、字节前缀、旧巨批、切批原子性、序号耗尽、模拟磁盘写入失败、旧账本辅助索引增加与旧 Agent 回滚兼容、20 秒心跳、ACK 控制、断连及数据库重开后准确重传。status 同时验证空队列和巨批阻塞提示。
- 独立步骤及验收边界见 [有界读取验收](docs/acceptance-bounded-usage.md)。旧超限批的历史账本恢复、小内存/小磁盘 Debian 12 实机、持续代理流量下完整验机仍待后续独立验收，不以回环通过替代。下一步：PR CI、专用节点验证，以及独立的心跳采集解耦 PR。
- 与当前主分支 `a62968e` 集成时保留辅助表、退役门禁和跨平台共享 status；有界索引改为增量创建，保持 `user_version=1`，新增旧 Agent 迁移重新打开试运行账本的回滚兼容断言。集成后的 core、对账及账本专项共 88 项通过、2 项真实 systemd 专项在 macOS 跳过；core 全 targets Clippy 与 workspace fmt 通过。

## 正式发布准备：生产公钥已提供，候选尚未签署

- 维护者在本机交互生成带口令的 minisign 密钥，私钥保存在仓库外；项目只取得公开根。`deploy/release-public-keys.json` 的 key ID 为 `44B019C8269669B8`，已通过生产根校验，并与 Actions 的 `SINAN_RELEASE_PUBLIC_KEYS` 变量一致。离线保管与正式签署仍需由维护者完成，不把公钥配置视为已发布。
- main `be8b792` 合入部署条件检查后，[CI 36746601899](https://github.com/theLucius7/sinan/actions/runs/36746601899) 的构建、检查、Compose 和真实 Reality/计量部分通过，但末尾退役验收误把 systemd 条件跳过的零退出码当作失败。发布候选继续受完整 main CI 门禁约束；修复将同时核对条件结果、服务状态、进程与启动时间，避免把未执行运行时和启动成功混淆。

## 新合入平台能力的整合修复

- 本次合并整合时先通过完整 workspace Rust/PostgreSQL 测试：230 项成功、5 项平台条件忽略；随后整合作者最新队列修复，并增加升级预检、旧进程停机及候选启动三处退出 78 的终止回归。最终升级专项 6 项、系统专项 11 项通过（3 项真实 systemd 在 macOS 忽略），workspace 全 targets Clippy、fmt、前端构建与 actionlint 通过。
- Windows CI 的签名夹具文本写入会将 LF 改为 CRLF，导致实际 Agent 拒绝证明；现改为精确 UTF-8 字节写入。新增模拟 Windows 文本 I/O 的回归，旧实现负对照失败，新实现通过真实 minisign 与 Agent 验证；Python discovery 71 项、66 成功、5 项既有条件忽略。最终 Linux/OpenRC/Reality 与原生服务验收继续以新提交 CI 为准。

- PR #13 的 [CI 36749216636](https://github.com/theLucius7/sinan/actions/runs/36749216636) 五项通过，包含 Reality 443、签名拒绝、重启/HUP 后精确两倍用量与在线退役。之后 main 合入 PR #10；其 [CI 36750812350](https://github.com/theLucius7/sinan/actions/runs/36750812350) 的 musl jobs 在 OpenRC 夹具校验公钥目录所有权时失败，旧提交的成功状态不能认证新源码。
- 自动 CI 恢复为 check、Compose、musl amd64/arm64 和真实 Reality 验收；其余平台的完整构建与服务 smoke 保留在手动 `platforms.yml`。OpenRC 仅将公开测试根复制到容器内受保护目录，不改变宿主源码所有权，也不放宽正式安装器检查。
- 新增的远程命令能力改为本地顶层 `allow_remote_commands` 显式开启，默认关闭；面板设置不能开启它。未启用时不领取或恢复命令，拨测继续运行，面板拒绝创建任务并解释所需的本地操作。显式启用意味着面板可按 Agent 服务账户执行任意命令，超出制品签名的约束范围。
- 自动升级在停止旧 Agent 前，让已认证候选独立验证既有缓存；失败保持旧进程与身份、配置、账本。macOS/FreeBSD/Windows 运行时服务每次启动先执行 Agent 验签。带受管历史的安装拒绝切换成仅监控模式，避免退役遗漏受管进程；macOS 固定系统别名规范化保持任意符号链接清理限制。
- 已整合 main `541f52d` 的常驻服务优先级、诊断资源限制、基线记录和有界用量读取。前一整合提交 `64dfee2` 在本机以真实 PostgreSQL 通过 221 项 Rust 测试，5 项平台/真实运行时专项忽略；完整 fmt、全 targets Clippy 通过。新增原生服务 proof 篡改重启回归保留在手动流程，尚不能宣称各原生平台均通过。
- [PR #33 首轮真实 CI](https://github.com/theLucius7/sinan/actions/runs/36755201070) 的 Compose 与普通 Rust/数据库检查通过；新增 systemd 资源专项暴露异步启动排队误判，双 musl 的 OpenRC 暴露 BusyBox 私有 umask 创建父目录导致的运行时访问拒绝，Reality 因依赖失败尚未执行。已在独立 Debian/systemd 与一次性 Alpine 容器分别复现，不将该轮记为通过。
- systemd 单次状态查询现包含 Job：有效未完成作业保持运行中，完成仍要求真实启动和正常退出；新增受控 After 阻塞的真实队列回归。安装器复用 main 的显式父目录权限修复，OpenRC 补非 root 执行运行时、读配置/写数据及拒绝访问身份与账本检查。最终合并后的 Rust 与 Linux 验收继续按当前提交核对。
- 在一次性 Alpine 3.24.2 容器完整通过实际 OpenRC 安装/重装、身份与账本保留、失败恢复、非 root 低端口能力、HUP、SIGKILL 自动恢复、默认 runlevel、快照及安装命令契约；4 项 root 信任根夹具全过。该运行使用公开 TEST_ONLY 进程夹具，未提供真实 Agent 二进制，真实 Rust OpenRC 诊断任务与 Reality 仍以当前提交 CI 为准。

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
- 发布候选流程构建双架构 Agent、运行时及诊断制品，输出 metadata 和 SHA256SUMS，先创建 draft。用户在仓库外本机生成带口令私钥、只提供公钥、本地签署并上传 minisig；CI 不取得生产私钥。正式发布要求全资产验签和对应 main 必需 CI，已知测试根在正式流程中拒绝。本阶段实现时尚无正式公钥、正式签名或正式 Release；最新公钥及发布状态见本文顶部，测试根验收不替代生产签名。
- 发布流程在缓存恢复或新构建后，以归档、ELF 和 Go metadata 检查两种架构、固定源码 revision、工具链及构建标签；检查不执行缓存二进制。此信息用于发现错误产物，不作为独立构建证明。12 项验收驱动、8 项签名 CI 契约、3 项既有缓存契约及 actionlint、Shell/Python 语法检查通过。
- [PR #11](https://github.com/theLucius7/sinan/pull/11) 已合入 main（`20d09ca`）。[PR CI](https://github.com/theLucius7/sinan/actions/runs/36738530095) 的 5 项全部通过，包括真实签名安装、篡改二进制/证明/旧未签缓存拒绝、恢复 systemd 验签器、Reality 双向流量、重启、HUP、精确计量和同版重装。首轮上传/下载为 1,048,821/2,097,454 字节，重载后相同流量累积精确为两倍。使用 TEST_ONLY 根，不能替代正式发布签名。
- [main CI](https://github.com/theLucius7/sinan/actions/runs/36740903057) 的全部 5 项也已通过。正式公钥与本地签署仍待用户完成，安全功能和链式 ADR 继续推进。

## 交付加固第 4 阶段：实现与本地验收已完成

- 在线删除先持久下发退役请求，Agent 停运行时和诊断、提交持久用量、清身份与运行配置，再通过设备签名回执确认删除。断线和清理失败可恢复，退役后退出 78 且不自动重启；离线软删除明确未确认清理。短期生命周期锁统一连接注册、删除及回执边界，防止重连插入已删除设备会话。
- 管理员登录与安全设置共用持久 IP/全局限速；TOTP 的设置、确认、登录和关闭使用事务消费验证码，启用/关闭撤销其他会话。被限流拒绝的 IP 不继续消耗全局额度。无密码单独关闭二步验证的接口，部署所有者恢复步骤在部署文档。
- 订阅链接可原子重置，旧链接立即失效，既有用户授权、配置和账本保持。节点创建与编辑允许手动指定 443 等端口，同服务器冲突拒绝，创建留空仍从 20000–29999 分配。
- 本地最终 fmt、全 targets Clippy 与 workspace 测试通过：170 项成功、4 项运行时/Linux 专项默认忽略。真实 PostgreSQL/HTTP/WebSocket 覆盖退役回执、失败恢复、重连竞争、登录限速、TOTP 并发消费、端口竞争及订阅旧链接失效。既有删除测试显式断开设备，以分别验证离线撤销和在线退役语义。
- Chrome 浏览器实测通过 TOTP 完整设置/登录/关闭、真实剪贴板、443 创建及中文冲突提示、旧订阅 404/新订阅可读，无 JavaScript 异常；使用独立数据库并清理全部临时进程和秘密。订阅夹具未连接 Agent，其结果不代表流量验证。
- 人工验收脚本支持隐藏输入 TOTP 或一次性环境变量，不把验证码写入 state。17 项驱动、8 项签名 CI、3 项缓存契约与 Shell/Python 语法、actionlint、diff 检查通过。真实 CI 让非 root 运行时实际监听 443，将伪装 TLS 放到独立容器；在签名/Reality/重装流程末尾增加单次在线退役，检查服务停止、凭据清理、账本保持和再次启动拒绝；Linux 结果以本阶段 PR 运行记录为准。
- [PR #12](https://github.com/theLucius7/sinan/pull/12) 已合入 `4ccd1e7`，该提交的 [main CI 36744518543](https://github.com/theLucius7/sinan/actions/runs/36744518543) 五项全部通过，包含实际 Reality 443 和完整在线退役。该证据认证此提交；后续提交的状态见上方发布准备记录。

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

## 原生 CI 跟进：补充平台指标与失败摘要

- d454b3b 的 Linux 四目标、Linux 运行时四目标、主检查和 Compose 均通过。macOS 已通过 Agent 实际运行/升级回退和真实运行时构建，在原生服务测试失败；FreeBSD amd64 已通过构建及 13.5/14 二进制验证，15 检查失败。Windows 双架构失败，公开 API 未提供完整日志，尚不能确定错误原因。
- 为原生 CI 命令增加有长度限制的失败注释，保留标准输出，并输出安装子进程的实际错误，后续可从公开检查注释定位。Windows Python 固定 UTF-8，避免中文注册提示依赖 runner 的代码页。
- 补充 macOS/FreeBSD/Windows netstat 连接数，修复 Windows 单 GPU JSON 返回对象的识别，Linux 优先读取 PCI GPU 型号；Windows 不将不存在的 load average 报为 0。macOS 对已卸载服务重复 stop 按成功处理。
- 本地 fmt、Clippy、完整 Rust/PostgreSQL 回归通过，141 项成功、4 项原有专项忽略；actionlint 通过。下一步继续根据原生 CI 的真实错误修复，不将尚未通过的原生服务宣称为验收完成。

## 原生 CI 跟进：模块路径、测试依赖与服务退出

- b6c74e8 的十个 Linux/主检查/Compose 任务通过。失败注释确认 Windows 的路径重定向模块寻找了错误的子模块目录，现显式指定原生部署模块路径；FreeBSD 15 的 Python 缺少独立打包的 SQLite 模块，现安装同版本 Python/SQLite 包。
- macOS 已通过 Agent 行为和实际运行时构建，原生服务在运行时健康检查失败。预先创建可由运行时账户写入的日志，增加服务状态诊断；受监督 Agent 保留 launchd 服务进程组，避免服务重启遗留子进程。此项仍待下一次原生 CI 验证。
- 监督进程正常退出时先通知 Agent 清理任务，再等待退出；补充实际进程测试，覆盖摘要错误拒绝、升级试运行中断后恢复旧版和监督服务退出后的状态清理。本地真实 Agent 冒烟通过。
- 本地 fmt、全 targets Clippy、完整 Rust/PostgreSQL 回归通过，141 项成功、4 项原有实机专项忽略。继续检查修复后的原生 CI，尚不宣称跨系统常驻部署验收完成。

## 原生 CI 跟进：macOS 验证通过与更新下载测试

- [d0556aa 的 CI](https://github.com/imengying/sinan/actions/runs/36731187269) 中 macOS ARM64 全部通过，包含原生服务注册、运行时真实流量、Agent 重启/重装和停止后代理独立运行；十个 Linux/主检查/Compose 任务通过。
- Windows 双架构编译已通过，实际 Agent 检查失败：ARM64 在权限设置命令超时，AMD64 的错误被 GitHub 单条 4096 字符限制截断。权限设置使用独立的 90 秒上限并标注路径，CI 失败输出分段保存，同时省略测试主动断开连接引起的无关堆栈。超时调整仍待原生验证。
- FreeBSD 双架构通过 Agent 行为测试，完整服务夹具因干净系统没有 `/opt` 而提前退出；现创建必要父目录，继续验证服务。
- 增加实际 HTTP 下载/可执行文件版本验证的更新测试，覆盖摘要、格式、来源、版本、不可变制品、待升级状态权限和失败版本不重复下载。修正 README 的旧 glibc 导入说明，避免覆盖 musl 兼容产物。
- 本地 fmt、Clippy、完整 Rust/PostgreSQL 回归通过，142 项成功、4 项原有实机专项忽略；actionlint、Python 语法和 diff 检查通过。Windows/FreeBSD 常驻服务仍待下一轮 CI。

## 原生 CI 跟进：FreeBSD 后台描述符与测试时序

- dd338ad 的 macOS 再次通过；Windows AMD64 已通过常规 Agent 行为和成功升级，但后续回退检查超时。升级夹具改用原子文件替换，并等待上一请求被消费后再提交下一请求；监督进程增加试运行、激活、验证拒绝和回退日志，失败时打印持久状态。
- Windows ARM64 注册成功，远程命令测试失败；该 runner 的 PowerShell 冷启动较慢，测试命令期限由 5 秒改为 60 秒并保留失败结果。产品的每任务期限语义不变，仍按管理员指定时间终止。
- FreeBSD ARM64 服务安装因 `daemon` 后台监督进程继承输出管道而超时，依据上游源码增加 `-f`，保留 syslog。AMD64 在升级测试超时，尚待新增状态日志确认；不能将两个架构的不同失败合并为同一个原因。
- Linux GNU ARM64 的运行时任务遇到 Go 模块代理 HTTP/2 INTERNAL_ERROR，增加最多三次构建重试，复用模块缓存且保持固定上游、只读依赖和产物验证。
- 增加实际 TCP/ICMP 双拨测、原生运行时配置重载与旧监听关闭检查；本地真实 Agent 已通过双拨测、补报、升级及恢复。文档补充 FreeBSD 普通账户低端口授权和 Linux 自定义服务入口要求。
- 本轮 fmt、Clippy、完整 Rust/PostgreSQL 回归通过（142 项成功、4 项原有实机专项忽略）；五组构建脚本测试、actionlint 和 Python 语法检查通过。下一步验证 FreeBSD 服务与 Windows 剩余流程，不将本地检查替代原生验收。

## 原生 CI 跟进：FreeBSD ARM64 完整服务通过

- [55e4345 的 CI](https://github.com/imengying/sinan/actions/runs/36734970741) 中，十个 Linux/主检查/Compose 任务、macOS ARM64 和 FreeBSD ARM64 通过。原生服务检查现包含真实配置重载和旧监听关闭；FreeBSD ARM64 同时完成 13.5/14 启动兼容和 15 完整服务验证。
- Windows AMD64 的 Agent 全流程已通过，但测试结束时 Python SQLite 连接仍持有数据库文件，导致 Windows 删除临时目录失败；现显式关闭连接。
- FreeBSD AMD64 和 Windows ARM64 在重启补报等待超时。测试将重新连接与补报分开确认，增加本地 outbox、面板请求和未确认样本诊断，保留对每个断网样本均须重传的断言，继续核实原因。
- 原生服务检查与无服务 Agent 检查独立执行，前一项失败仍收集后一项结果，工作流最终保持失败状态。避免单一夹具问题掩盖另一条部署路径。
- 本地真实 Agent 全流程通过；本轮仅修改测试和工作流，actionlint、Python 语法及 diff 检查通过，Rust 代码沿用上一轮 142 项通过结果。下一步继续收敛剩余原生测试。

## 原生 CI 跟进：FreeBSD 并发磁盘采集与 Windows 账户

- 1c1874b 的 CI 仍有 12 个任务成功，macOS 与 FreeBSD ARM64 再次完成完整原生验证。FreeBSD AMD64 常驻服务检查通过，但无服务 Agent 重启时退出码为 SIGSEGV；未确认样本仍在 SQLite 中，排除把超时直接归因为补报丢失。
- sysinfo 的 FreeBSD 磁盘枚举使用会重新分配全局缓冲区的 `getmntinfo`，当前静态上报与采样存在并发调用。增加进程级互斥保护，CI 增加五次启动/采样检查，并在可用时记录 LLDB 崩溃回溯；本轮尚待原生结果确认段错误是否消除。
- Windows AMD64 已通过 Agent 全流程与真实运行时构建，服务启动报 AccessDenied。运行时账户补充普通 Users 组成员身份，保证公共可执行文件 ACL 可读；保留非管理员权限，并增加分组断言和任务事件诊断。随机任务密码保证覆盖复杂度字符类型。
- Windows ARM64 已完成重启补报，监督进程首次启动超过夹具原 60 秒等待；增加平台启动等待及 120 秒有界更新健康检查。原生 Go 构建也补充三次有限重试，应对本轮模块代理 HTTP/2 INTERNAL_ERROR。
- 本地 fmt、Clippy、完整 Rust/PostgreSQL 测试通过（142 项成功、4 项原有专项忽略），真实 Agent 全流程通过；五组构建脚本检查、actionlint、Python 语法和 diff 检查通过。下一步继续核实 FreeBSD AMD64 和 Windows 双架构完整 CI。

## 原生 CI 跟进：FreeBSD 双架构通过与 Windows 任务 DACL

- [a67aeae 的 CI](https://github.com/imengying/sinan/actions/runs/36739867701) 中 FreeBSD 双架构均通过新增的五次重复启动/采样和完整常驻服务检查，macOS 及九个 Linux/Compose 构建任务通过。磁盘枚举互斥后本轮未出现段错误。
- Windows AMD64 已通过 Agent 全流程和真实运行时构建；新增诊断显示任务继承 DACL 的 SYSTEM/Administrators 掩码不含执行位，加入普通 Users 组不能解决此问题。注册后明确设置本项目任务的管理权限，保留运行时普通账户，仍待原生执行确认。
- 主 check 在 Rust 测试失败，公开接口未返回具体用例；本地连续五轮完整回归通过。将同一个错误摘要包装器用于 Rust/PostgreSQL 测试，下一次失败保留用例和堆栈，不通过重试隐藏失败。Windows ARM64 的真实运行时构建通过，但 Agent 行为检查失败、服务检查尚在运行，继续等待具体诊断。
- 任务权限修复后 fmt、Clippy、完整 Rust/PostgreSQL 测试再次通过（142 项成功、4 项原有专项忽略）；actionlint、Python 语法和 diff 检查通过。继续跟进原生 Windows 与主检查结果，尚未将全部平台标为完成。

## 原生 CI 跟进：Windows 登录权与权限操作开销

- 5931b34 的主检查已通过，包括 Rust/PostgreSQL 和真实 systemd 专项；Linux、Compose、macOS 和 FreeBSD 双架构共 13 项成功。Windows AMD64 的任务 DACL 已正确含执行权，事件进一步确认运行时缺少批处理登录权（`0x80070569`）。
- 安装读取现有用户权利，只在 `SeBatchLogonRight` 中追加专用运行账户 SID。新账户在设置随机密码前保持禁用；保留普通 Users 身份与任务控制边界。原生夹具比较首次/重复安装前后的全部用户权利，检查其他授权未变。
- a67aeae 的 Windows ARM64 升级已激活，但写入确认状态超过夹具等待；安装也在重复权限操作时超过 180 秒。权限处理改用内置 .NET ACL 接口，合并新建目录及制品文件的权限更新；新文件先保护再写内容。状态文件不存在时直接返回，服务启停改用任务计划程序 COM 接口并等待旧任务停止。新增实际身份 ACL 拒绝测试与基础命令耗时诊断，效果仍须以原生结果为准。
- 本轮 fmt、Clippy、完整 Rust/PostgreSQL 回归通过（142 项成功、4 项原有专项忽略）；actionlint、Python 语法、core 分层边界和 diff 检查通过。下一步验证 Windows 双架构完整服务及升级流程。

## 原生 CI 跟进：Windows ARM64 Agent 全流程通过

- [1bc6f86 的 CI](https://github.com/imengying/sinan/actions/runs/36745302341) 中 Windows 双架构 Agent 全流程通过，包含实际私有身份 ACL 拒绝、遥测补报、命令、双拨测、成功升级及失败恢复。ARM64 该步骤约 91 秒，此前十分钟后仍在升级状态保存时超时；权限路径优化已得到原生验证。
- 两个 Windows 架构均完成服务安装，但原始文本的用户权利比较失败，原错误未记录具体差异，因此尚不能断言存在额外授权变化。按微软格式规范，将账户名和 SID 统一后再比较全部权限，并增加差异详情；重复安装同时识别以名称导出的已有授权。权限保持检查未删除。
- 本地混合名称/SID 样本验证通过，仍能拒绝额外权限变化；fmt、Clippy、完整 Rust/PostgreSQL 回归通过（142 项成功、4 项原有专项忽略），Python 语法和 diff 检查通过。Windows 真实运行时尚待通过权限断言后的完整服务验证。

## Agent 能力对齐完成：全部平台 CI 通过

- [d8d951f 的完整 CI](https://github.com/imengying/sinan/actions/runs/36747223630) 已完成，15/15 任务成功，九份 Agent 与九份运行时制品均已上传。Linux musl/glibc 分别覆盖 amd64/arm64，macOS 覆盖 arm64，Windows 与 FreeBSD 覆盖 amd64/arm64；OpenRC 检查仍在同一 CI 的 musl 任务内。
- Windows 双架构通过首次安装与重复安装的全部用户权利比较：仅新增专用普通运行账户的批处理登录权，其余有效授权保持不变。账户名称和 SID 规范化后比较通过，确认前一轮断言失败源于文本表示差异。真实运行时配置应用、重载及旧监听关闭、代理流量、Agent 服务重启、重装保留身份、停止 Agent 后代理继续工作均通过。
- macOS 与 FreeBSD 双架构再次通过相同常驻服务验证；FreeBSD 同一二进制在 13.5/14 检查启动兼容、15 检查完整服务，重复启动/磁盘采集未再次出现段错误。Windows、macOS、FreeBSD 和 Linux 均通过 Agent 遥测补报、命令去重、TCP/ICMP 拨测、升级成功及失败恢复检查。
- 主检查、真实 PostgreSQL、systemd 诊断专项、Compose 持久化、双架构 OpenRC 诊断及独立服务、四种 Linux 运行时实际构建与代理专项均通过。本地代码最后一轮 fmt、Clippy 和完整 Rust/PostgreSQL 测试为 142 项成功、4 项原有实机专项忽略；此前前端 TypeScript/Vite、构建脚本、工作流语法检查已通过。收尾只更新 README 与本文件，使用 `[skip ci]` 文档提交，运行代码及工作流与上述已验收提交一致。
- 本次确认的差距已补齐：一秒可配置遥测与 SWAP/进程/磁盘 I/O/GPU、压缩上传和持久补报、持续拨测、通用远程命令、OpenRC NodeQuality、跨系统常驻部署、默认关闭的 Agent 自动更新及失败恢复、musl/glibc 和原生系统制品选择。README 已更新实际验证范围并整理部署与监控说明；未恢复已删除的参考分析文档。
- 验证边界：GPU 实际负载、公网 Reality 客户端、整机断电/重启和早于 FreeBSD 13.5 的系统未作实机验收；NodeQuality 仍是 Linux 外插。Windows 标准库方案仍不承诺目录元数据断电刷盘语义。以上边界不以 CI 进程/回环检查代替。
- 下一步：按部署目标在专用设备执行上述实机验收；当前授权的能力补齐与跨平台 CI 工作已完成，修改直接提交到 `main`。

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

## 2026-10-01：诊断资源预算（独立 PR，对应 Issue #14）

- 合入最新平台/Agent 基线后的 CI 暴露夹具读取竞态：systemd-run 的非阻塞启动确认可能早于 ExecStart，初始 inactive 不是退出。真实预算夹具改为先确认启动时间戳，再判断执行结果，不修改生产成功判定。本机 core 单元测试 59 项通过、2 项 systemd 专项按平台忽略；修复提交 3b0096b 的 CI 36754771881 中 check（含 Linux/systemd 专项）通过；该次 Windows/OpenRC 外部安装专项仍失败，不能称全部 CI 通过。夹具修复在预检 PR 中保留，旧提交记录不替代最终提交验收。

- ServiceJob 新增五项数字预算及构造/反序列化范围约束，拒绝无界值、零上限、非法权重和负诊断 OOM 保护。NodeQuality 默认限制为 512 MiB/128 个任务、CPUWeight=10、IOWeight=10、OOMScoreAdjust=500；core 显式渲染所有 systemd 属性，固定 MemorySwapMax=0，防止把诊断内存争抢转为 swap 压力。
- 旧 checkpoint 缺字段时使用默认值，重启后继续观察原单元并保留报告，不重复启动或追溯修改已有单元。已保存的新预算完整往返保留。
- 独立验收见 [诊断资源预算验收](docs/acceptance/diagnostic-resource-budget.md)。测试覆盖默认和自定义真实命令参数、参数边界、非法持久值及旧 SQLite 恢复；Linux CI 扩展真实属性读回并运行 64 MiB 内存 OOM/8 个任务子进程上限与停止后的 PID 清理夹具。
- 本机 macOS：workspace fmt、所有 targets Clippy（warnings 为错误）、完整 cargo test 及 core 分层检查通过；174 项成功、0 项失败、5 项按条件忽略，PostgreSQL/HTTP/WebSocket/诊断端到端使用独立临时数据库验证。两个真实 systemd 专项和三个原有真实运行时专项按条件忽略，远端 CI 结果待本 PR 最终提交确认。
- 验收边界：本项未跑完整 NodeQuality；专用 Debian 12 的真实负载下心跳/业务存活验收尚未执行。常驻服务保护、启动预检、遥测解耦、取消和报告完整度分别由后续独立 PR 完成，不将资源预算通过等同于第 1 步整体验收。
- 合并兼容：保留发布签名、退役保护、原生服务和 OpenRC 管理；新增测试显式选择 systemd 后端，既有 ServiceJob 构造补齐默认字段。资源预算仅在 systemd 强制执行，OpenRC 保留已有诊断行为并在启动时明确记录未执行 systemd cgroup 预算的警告；有限默认字段不能作为 OpenRC 同等资源限制的证据。
- 合并后本机验证：workspace fmt、全 targets Clippy（warnings 为错误）通过，agent-core、NodeQuality 和 SDK 测试共 111 项成功、0 项失败；两个真实 Linux/systemd 专项在 macOS 按条件忽略，资源预算的真实内核执行仍须以合并提交的远端验收结果为准。
## 2026-10-01：整合 OpenRC 与原生平台 PR

- 保留主分支的发布签名、生产根拒绝测试钥、独立 bootstrap、真实 Reality/accounting CI 与精简 README。Linux 双 libc / 双架构及原生矩阵合并到日常 CI，全部测试 Agent 显式编译公开 TEST_ONLY 根，上传制品名称显式标记 TEST_ONLY；FreeBSD cross 显式传递该编译环境变量。发布门禁继续要求两项 Linux musl 与真实验收任务，并同步新 job 名称。
- OpenRC 与能力 ADR 顺延为 0021/0022，平台文档与部署文档分开记录正式 Linux 发布和原生测试范围。原生测试 Agent、运行时及 Windows DLL 具有已签 TEST_ONLY proof；更新、服务安装夹具保留验证和回退，最新 Windows 账户权限及身份 ACL 专项检查一并保留。
- 本机验证：构建脚本 5 项、环境初始化 3 项、验收驱动 17 项、运行时缓存 3 项、签名 CI 8 项、发布门禁 11 项测试通过；两个工作流 actionlint、Python/Bash/静态安装器语法及差异空白检查通过。使用真实 minisign 分别验证 Python cryptography 与 minisign 路径生成的公开测试签名，并核对 Windows ARM64 辅助 DLL 的已签摘要与大小。OpenRC 测试 bundle 的签名、metadata 与安装器摘要也经独立 minisign 校验。
- 本机未运行 Linux/OpenRC、FreeBSD 或 Windows 真实服务。OpenRC 进程夹具的缓存预检是明确 stub，实际签名缓存由 Rust 与 systemd real-e2e 验证；合并后的平台服务、CI 与公网验收结果须分别以实际执行为准，不能由上述本机结构/签名验证推断。
- 随后使用合并源码实际构建的 macOS ARM64 Agent 在回环 HTTP/WebSocket 夹具完成注册、压缩遥测 ACK、离线重启补报、命令去重、TCP/ICMP 拨测，以及签名 Agent 的启动替换、坏摘要拒绝、失败启动回退、失败版本抑制、中断升级恢复与子进程清理，全部通过。此检查使用 `--monitor-only` 和独立监督进程，没有注册 macOS 系统服务；不替代各目标 init 的原生服务任务。
- 将新增命令、拨测、更新、遥测与公网 IP 后台任务纳入退役同步保护，等待已经开始的操作结束后再清理；请求退役后禁止继续领取、执行、暂存更新或重新写入配置。清理命令文本及结果、拨测与遥测 outbox，保留流量账本；监督进程将退役退出码 78 作为终态。新增退役回归验证不再执行命令、发起面板请求或补回已清理数据。
- 合并贡献者截至 `70dee9a` 的全部提交，保留 Windows SID 规范化与权限差异诊断；其 `d8d951f` 历史 CI 通过记录保留，但不用于认证本次合并后的代码。本次完整 Rust/PostgreSQL 回归通过，随后退役后台任务专项 56 项成功、1 项原有 systemd 专项忽略；Clippy 与格式检查通过。Linux/OpenRC、原生服务、Compose 与真实 Reality 检查以本次远端 CI 结果为准。

## 2026-10-01：P0 常驻服务优先级

- Agent 和 sing-box systemd 单元分别增加 `OOMScoreAdjust=-500`、`CPUWeight=1000`，降低常驻进程的 OOM 候选优先级并提高资源争抢时的 CPU 权重；没有添加 CPU 配额。安装器直接嵌入这两份源单元，渲染后逐字核对一致。
- 专用 Debian 12/systemd 252.39、1 GiB/2 CPU 验收容器中，从源单元生成独立短命夹具，分别保留 root 和运行时用户/能力设置。systemd 属性、内核 oom_score_adj 和 cgroup cpu.weight 均验证为 -500/1000，CPUQuotaPerSecUSec 为 infinity。已有 Agent 和运行时 PID、运行状态、重启数保持不变，夹具已停止并删除。
- 单元语法校验使用临时可执行文件路径，未调用真实业务程序；安装器 shell 语法、构建与签名发布 Python 检查、差异空白检查通过。独立步骤和验收边界见 [常驻服务优先级验收](docs/acceptance/resident-service-priority.md)。完整 Rust/Compose 检查由该 PR 的 CI 执行，完整诊断与持续代理流量压力验收仍待其他 P0 改动完成。

## 2026-10-01：诊断整改基线（独立 PR）

- 新建统一 milestone，将已确认代码缺陷与实机尚未复现的症状区分，保留既有未解决 Issue。
- 实测面板容器网络命名空间的 IP 查询为 HTTP 403；DNS、TCP、TLS 均成功。私有原始日志不进入 Git。
- 新增只读基线采集脚本与故障矩阵，输出仅写私有目录，面板时间采用白名单；旧指标采集时间明确未知。
- 专用 Debian 12 容器 1 GiB / 2 CPU 可用于小夹具；完整 NodeQuality 不在共享生产磁盘上运行。独立 Debian 12 节点选取与完整症状复现仍进行中。

## 2026-10-01：诊断启动预检与运行内存保护（独立 PR，对应 Issue #16）

- 基于资源预算 PR #30：启动要求有效可用内存不少于任务上限加 256 MiB、工作目录至少 2 GiB 可用空间、一分钟负载不超过可用 CPU 数的 1.5 倍，且同机没有其他活动诊断。宿主和 cgroup 全祖先限制一起计算有效内存；读取失败明确拒绝，不以未知当充足。
- 通过 Privileged/ServiceManager 检查资源和真实服务状态，固定 Linux flock 封住同时启动竞态，保留主线 systemd/OpenRC/非 Linux 服务能力。运行中每 5 秒检查 128 MiB 保留阈值；保护停止原因持久化，停止失败/未确认保留处理中，确认后才收已有报告并回传 Failed。面板离线、Agent 重启和 SQLite 写失败均不取消保护动作或重复执行任务。
- 独立验收见 [诊断预检验收](docs/acceptance/diagnostic-preflight.md)。行为测试覆盖小内存/cgroup 限制、磁盘不足、负载/资源读取错误、同机冲突、停止失败、存储写失败及断连/重启后的报告恢复；新增真实 systemd 双单元独占夹具，和已有资源专项串行执行。
- 最终源已整合主线 21e6a01，受限 Debian 12 构建容器（1.5 GiB 内存、2 CPU、禁止 swap）中的 fmt、Clippy --all-targets -D warnings、完整 cargo test --locked 通过：243 项通过、0 项失败、7 项忽略（4 项真实 systemd，3 项既有外部运行时专项）。独立 PostgreSQL/HTTP/WebSocket/面板完整 e2e 已运行，构建容器无 OOM。面板 HTTP 挂起仍在下一保护节拍停止的行为测试已通过。专用 Debian 12 小内存节点使用最终 Linux core 二进制（SHA256 283e657b26de1717bbe09dedf7032035d6a3e648dc0256b8ed4f3293c2ca50c8）串行通过 4 项真实 systemd 专项，0 失败，耗时 2.79 秒；覆盖预算 OOM/TasksMax、queued-start、重建超时和预检独占。NodeQuality 基线出现全局 OOM 杀 Geekbench，Agent/常驻运行时未重启；SSH 采集隧道断连造成 319.014 秒指标空窗，不能据此宣称完整心跳验收通过，详见独立验收文档。

- 本次合并整合 main `e2d898c`，保留核心/代理业务门禁、IP 查询逐条错误分类、诊断制品签名与启动前复验、退役同步和流量账本保留语义。修复状态查询失败/挂起会跳过内存保护的问题：独立每 5 秒重读内存，已有保护停止原因直接重试停止；停止确认失败时保留 ACTIVE，不提前发终态。
- 两个 Linux 后端改用 root 所有、0700 普通目录中的固定锁 inode，OpenRC job 使用 0077 umask；不替换或删除已持有锁。OpenRC run-job 在首次启动前注册 SIGTERM/SIGINT，收到停止信号时同步取消并清理独立诊断进程组，保留已有报告与持久任务状态；真实 OpenRC 夹具新增子进程清理、锁释放和停止后重放拒绝回归。
- 本次合并后的 workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁及其 6 项行为测试通过。目标 Rust 回归共 69 项通过：诊断 27、系统 17、PostgreSQL 诊断 4、诊断适配器 8、退役 13；系统测试另有 5 项 Linux/root/systemd 专项按条件忽略。本机为 macOS，未运行真实 OpenRC、systemd 或新增 root 锁验收，后续由最终提交 CI/专用测试机执行；没有把贡献者原始完整测试记录等同于本次合并源的完整验收。

## 2026-10-01：核心与代理业务边界 ADR（独立 PR）

- ADR 0023 固定服务器 core 与 sing-box 业务的所有权，明确管理员与代理用户命名、保留 ID/令牌/旧订阅路径/凭据/授权/流量、暂不改表名，以及网卡总流量与计量 epoch 的语义。用户追加授权的成本、账单网卡配额、周期拨测和插件用户配额覆盖原 MVP 对应排除项，仓库规则同步。
- CI 分层检查继续禁止具体运行时名称，并新增 user/subscription/quota 及复数、蛇形、驼峰引用检查。原生账户和 SQLite 系统 API 仅有按文件、表达式限定的例外；同一行其他业务引用仍失败。原生本地变量和示例 URL 凭据改为 account，没有路由、模型、协议或持久化改变。
- 独立验收文档为 docs/acceptance/proxy-business-boundary.md。当前 core 检查、6 项门禁行为测试、workspace 格式和差异空白检查通过；完整 Rust 测试由本 PR CI 验证。macOS 账户路径例外限定于以 `/Users` 开始的字符串，同文件的 `/api/Users` 业务路由仍被拒绝。业务搬迁与旧订阅的实机兼容验收属于后续独立 PR，本项未声称已完成搬迁。
- 合入 main `21e6a01` 后，保持五项自动 CI 门禁及手动平台验证，新增监督进程回归与终态退役代码通过分层检查。macOS 系统别名注释去除易与代理业务混淆的账户词，运行行为保持不变。
- 最终合入 main `71b8b56`，保留 IP 查询错误分类的独立验证记录。合并后的 core 门禁、6 项行为测试、Python discovery（72 成功、5 项已有条件忽略）、workspace 全 targets Clippy（warnings 为错误）、fmt、三份 workflow 的 actionlint、文档本地链接及差异检查通过；没有新增 Rust 运行行为，未重复完整 workspace 测试。

## 2026-10-01：P0 IP 查询逐条错误分类（独立 PR）

- 七种数据库响应均如实标记同一 `check-place` 查询入口；每条保存目标 IP、尝试时间、耗时、结构化错误类别及可选 HTTP 状态。DNS 和 TLS 根据 source 类型分类，连接失败不被泛化为 DNS；其余包含超时、403/429、非 JSON、字段不匹配、读取失败及响应超限。
- 保留旧 payload、旧错误、零分及 false；前端中文显示分类、入口、IP、时间和耗时，未知字段保持未知。批次未开始的请求不补造逐条尝试时间。同步构建 dist 并保留最新主分支的监控/任务页面。
- 对齐 main `45df3b1` 后，11 项 IP 查询、3 项诊断 PostgreSQL、6 项 foundation 专项测试全部通过；冻结锁文件安装、TypeScript/Vite 构建和桌面/手机实际 dist 夹具通过。初次完整 Rust/PostgreSQL 测试受到测试磁盘耗尽影响，foundation 临时建库失败，补跑成功后仍不记为完整通过；全 workspace 与平台验证交最新提交 CI。独立步骤与边界见 [IP 查询错误分类验收](docs/acceptance/ip-provider-errors.md)。
- 最终整合 main `21e6a01`，保留诊断资源预算、有限流量补报、监控模式、任务页面与退役保护。使用独立 PostgreSQL 再跑上述 20 项专项测试，全部通过且无忽略；panel 全 targets Clippy（warnings 为错误）、workspace fmt 与差异空白检查通过。Bun 1.4.2 冻结锁文件安装及 TypeScript/Vite 构建通过，并重建合并后的 dist；此结果不代表最终提交的完整 workspace 或平台 CI 已通过。
- 此项不修改缓存结构或覆盖语义；失败后保留历史成功结果由下一独立 PR 完成。新增 rustls 类型直接依赖的理由及替代方案记录于 [ADR 0024](docs/adr/0024-ip-provider-error-classification.md)。

## 2026-10-01：专用 Debian 12 测试节点就绪（独立 PR）

- 按明确授权停用选定节点原 xboard-node 业务并禁用自启动，保留配置和身份；447 MiB 内存、约 12 GiB 可用磁盘、已有 2 GiB swap。其余生产代理节点未运行完整硬件测试。
- 建立私有测试面板与独立 PostgreSQL、正常注册的测试 Agent/代理用户/授权，回环绑定和 SSH 隧道连接。公开 TEST_ONLY 签名安装验证保留，实际 GNU 调试构建不作为正式 musl Release 发布。
- Agent 与独立 sing-box 单元健康在线，实读 -500/1000 优先级和初始零重启；凭据、原始日志和环境配置保存在本机私有目录，未进入 Git。独立验收见 docs/acceptance/dedicated-debian12-node.md。默认512MiB完整诊断预算不适合该机，预检应拒绝；完整资源症状与故障场景另项记录。

## 2026-10-01：专用小内存节点完整 NodeQuality OOM 基线（独立 PR）

- 在447MiB专用Debian12节点，用主线541f52d的签名测试Agent提交一次硬件启用、IPv4、低网络流量、关闭上传的完整NodeQuality入口，同步采集cgtop/内核OOM/磁盘/设备状态与面板时间。
- 已复现globalOOM：被杀Geekbench属于诊断单元，anon-rss275192KiB、oom_score_adj500；诊断Result=oom-kill，最终failed且无完整报告。Agent子进程/监督进程和sing-box PID均保留，NRestarts0。SSH恢复后停止诊断，cgroup进程与相关挂载为空。
- 旧API没有专用心跳/指标时间显示，last_seen是“最后消息”；私有DB只读指标时间的最大采样跳跃319.014秒，测试SSH隧道也中断，因此不把此值等同纯心跳中断或停止采集时长。独立验收见docs/acceptance/nodequality-oom-baseline.md；原始证据不进Git，未宣称P0整体验收通过。

## 2026-10-01：CI 会话时间夹具修正

- main 合并后的服务测试夹具已适配 root/0700 锁目录，6 项服务回归及目标 Clippy 通过。后续 CI 暴露会话到期断言的跨秒假设：认证发放时间与确认消息时间可相差一秒；改为认证前后窗口核对，并要求确认消息的绝对到期值与该服务器数据库会话记录一致，保留过期 401 检查。
- 独立 PostgreSQL 会话专项 1 项通过、0 失败、0 忽略，目标 Clippy、fmt、core 门禁和差异检查通过。main `13f2975` 的真实 Reality CI `36766239541` 已通过完整安装、计量、签名拒绝、重装和在线退役；本次最新整合提交的完整回归与 Linux 专项分别继续验证。

- 随后保留贡献者 `98f86d8` 的生产修复：会话到期值与 hello.ack 取同一次签发时间。合并强制 PostgreSQL 会话写锁跨秒回归和认证前后窗口、token/server 数据库一致性检查；该专项 1 项实际通过，panel 全 targets Clippy、workspace fmt 与差异检查通过。Agent 仍提前 60 秒续期，认证 ACK 有 10 秒等待上限，服务端保留绝对到期拒绝。

## 2026-10-01：P0 IP 查询保留成功快照（独立 PR）

- 缓存按 IP 与真实入口分开，每种数据库响应单独保存最新尝试和最后成功快照。失败仅更新状态与错误，不覆盖字段、成功时间或有效期；部分成功只更新对应响应，换 IP 保留旧记录，旧批次不覆盖新批次。旧表名和 payload 保留，0008 迁移明确成功数据，未知字段/时间/分类不补造。
- 页面同时显示当前失败和历史字段、上次成功时间及过期，历史成功不计为当前成功；原零分和 false 不变。刷新 admission 在服务器行锁事务内检查最近尝试与运行租约，异常退出后租约过期可恢复，重复刷新去重。
- 对齐最新 main `e2d898c` 并保留新 Agent 控制界面。在隔离 Debian 12 构建容器通过 workspace fmt、全 targets Clippy、16 项 IP 专项（含五项缓存 PostgreSQL 场景）、四项 diagnostics 与完整 workspace 245 项成功 / 0 失败 / 六项已有真实 systemd/上游运行时条件忽略；新增缓存测试无忽略。独立 PostgreSQL 旧 DDL/旧 payload/0008 实际迁移、Bun 1.4.2 冻结安装、TypeScript/Vite 和最终 dist 桌面/手机历史场景、core gate 及六项行为测试、文档链接与差异检查通过。本提交的平台/Compose CI 单独核对，不由该普通构建容器推断实机通过。
- 数据和兼容语义见 [ADR 0025](docs/adr/0025-ip-provider-cache.md)，独立步骤及边界见 [缓存验收](docs/acceptance/ip-provider-cache.md)。本项没有新增查询入口或修改网络重试策略。

- IP 缓存合并整合 main `75cd846`，保留诊断内存保护、固定锁权限、宿主 ABI 缓存兼容、服务夹具和会话单次签发修复。完整 Rust/PostgreSQL workspace 回归 267 项成功、0 失败、8 项既有 Linux/root/systemd 或上游运行时条件忽略；Clippy --all-targets -D warnings、fmt、core 门禁和差异检查通过。Bun 1.4.2 TypeScript/Vite 重建与提交 dist 一致。此完整回归尚不包含后续 IP 未知字段 PR #44，最终 Linux CI 单独核对。

## 2026-10-01：P0 IP 未知字段显示（独立 PR）

- 专项审查实际复现空字符串/错误类型和 success=false 默认字段被记为成功，归独立 Issue #42，不混入来源适配层 #24。每个已知字段增加语义类型和有效值检查，不能确认的状态/字段保持未知，真实 0/false 和可信原始评分字符串保留。
- 旧缓存原始快照保留，读取过滤不能确认的已知字段并补可选 kind；页面无效值显示未知，历史/过期/未知状态不能冒充当前成功。旧响应包缺失时不追溯编造成功证据。
- 未知字段首轮 CI 的 check 在测试辅助路径的反向迭代编译失败，Rust 测试未执行；作者已修复为 rsplit，保留该修复，未将初轮记为通过。
- fmt、core gate、差异、Bun 1.4.2 冻结安装/TypeScript/Vite 与实际 dist 桌面/手机字段、历史和模拟未启用来源验收通过。Rust/Clippy 和独立 HTTP/PostgreSQL 场景等待隔离编译槽或 CI，结果单独更新，未宣称平台/完整诊断通过。独立步骤见 [未知字段验收](docs/acceptance/ip-quality-unknown.md)。
- 整合 main `07e8f58` 时补齐旧 payload 缺少 kind 的前端校验，代理=0/评分=false 和空白/占位评级不再冒充事实；保留合法 0/false 与未知自定义标签原标量。Bun 1.4.2 冻结安装、4 项字段回归（637 项断言）、TypeScript/Vite 构建通过，覆盖后端全部 55 个字段的前端兼容规则并重建最终 dist；core gate 与差异检查通过。补修后的桌面/手机浏览器场景尚未重跑，Rust 和平台验证随后单独记录。
- 最终正常整合作者 `56f8211` 和 main `b8e5689`，保留缓存、会话签发/夹具、服务保护和宿主 ABI 兼容修复。独立 PostgreSQL 下 20 项 IP 质量 library 测试与 4 项 diagnostics API 测试全部通过，0 失败/忽略；panel 全 targets Clippy（warnings 为错误）、workspace fmt、core 门禁与差异检查通过。Bun 4 项/637 断言及 TypeScript/Vite 再次通过，最终 dist 与重建结果一致；未重复完整 workspace 或实机 NodeQuality/平台 CI，补修后的浏览器桌面/手机场景仍未重跑。

- 未知字段修复后的 `56f8211` CI `36769441248`：check 中全 targets Clippy、Rust/PostgreSQL、真实 systemd 和提交 dist 检查通过，Compose 和两项 musl 也通过；旧基线的 Reality 任务失败另行处理。现保留已合并缓存和最新 main 后复验最终源，不用前一提交结果替代。

- 贡献者提供的同期验收记录：最终 Rust 源保留 main `b8e5689` 后，在独立 Debian 12 构建容器（1.5 GiB/2 CPU、无额外 swap）通过 fmt、Clippy --all-targets -D warnings、20 项 IP 与 4 项 diagnostics 专项、完整 workspace 270 项成功 / 0 失败 / 8 项既有 Linux/root/systemd 或外部运行时条件忽略；新增未知字段测试无忽略，容器无 OOM。最终 TypeScript/Vite 与已有 dist 一致，桌面/手机夹具和 core 门禁通过。平台 CI 仍按最终提交单独核对。 此记录属于贡献者原前端源，不替代合并审查补修后的最终 dist；两侧 Rust/依赖源码完全一致，本地专项证据继续有效。

- 发布前正常合入作者新 head `cbe54ae`：与已验证 `14893f5` 的所有 crates、Cargo 清单/锁文件及工具脚本完全相同，仅补验收记录与已有前端差异。保留双向正常祖先和旧 payload 类型补修，Bun 4 项/637 断言及 TypeScript/Vite 复验通过，最终 dist 与既有构建一致；相同 Rust 源码不重复构建。

## 2026-10-01 心跳与遥测隔离（Issue #17，独立 PR）

- 单一 OS 线程持有 Collector 和硬件补充采集器，watch 缓存 StaticInfo 与已有 TelemetrySample；连接初次、300 秒刷新以及公网/配置变更不再创建 Collector，心跳 uptime 读取缓存。5 秒采集超时保留旧样本，不生成额外阻塞线程，不持有退役 gate。
- 采样缓存、SQLite 持久化与 HTTP 上传分开，保留 1 秒/3 秒默认设置、原 outbox 保留上限与 UUID ACK；时间下限随样本原子保存，ACK 后保留，重启与时钟修正不倒退。
- API 暴露既有 metrics_sampled_at（毫秒），新增仅 heartbeat 更新的可空 last_heartbeat_at（秒）；last_seen 保持最近设备消息语义。界面分别显示三种时间、指标过期和可读历史，未知不补造时间。baseline 读取真实字段，Cookie 和权限保护不变。
- 本机 fmt/core 边界检查、baseline 7 项和边界 6 项通过；Bun 1.4.2 构建及 dist 同步完成；真实 Chromium 回环 6 项验收通过、错误 0。集中 Debian 12 构建容器（1.5 GiB/2 CPU/无 swap）fmt、全 targets clippy、Rust/PostgreSQL 工作区 246 通过/6 原有环境依赖忽略，专项 core 遥测10/阻塞4/面板4通过，Agent/Panel 构建通过、二进制已保存，OOM=false；本机 Python77通过（5环境skip），未在磁盘不足的本机重建 Cargo。
- 初始缓存保留编译期 OS/arch/libc；首个真实采集之前不向面板发送默认 StaticInfo，保留注册时的宿主 ABI，hello/heartbeat/control 继续工作。专项覆盖永久阻塞时真实 20 秒心跳/1 秒 Ping、同缓存重连、不增加采集器、HTTP503补报与ACK、退役 gate、重启时间下限、API 过期与慢采样配置。独立验收见 [telemetry-isolation](docs/acceptance/telemetry-isolation.md)。真实受保护完整验机与取消清理由总任务分别验收。
- 单独提交整合 main b8e5689（含 IP 缓存与会话修复），保留预检和宿主 ABI 修复，新增真实 WebSocket 缓存就绪/双 ABI 测试。本机 fmt/core 边界、Python77（5skip）、Bun/dist、Chromium6项重验通过；最新 HEAD Rust/PostgreSQL 与真实 systemd 交 GitHub CI，上一轮246项与保存二进制只证明 e2d898c 基线版本。

- 合并审查继续正常整合 main `c958ba2`（PR #44），保留双方正常祖先、未知字段过滤和遥测时间类型，重新生成最终 dist。补修独立采集线程退出：异步硬件命令使用 4 秒 / 128 KiB 的受限执行与进程组 guard，退出发取消信号并最多等 1 秒确认清理；同步永久阻塞的 Collector 不 join，不生成替代线程。缓存测试拆为子模块，新增真实 Unix 子树取消、失败/超时后恢复和 CPU 忙循环隔离回归。
- 本轮本机独立 PostgreSQL / macOS 专项 Rust 共 21 项通过、0 失败/忽略：遥测/core 13、真实 WebSocket 心跳与初始 ABI 就绪/usage 重放 3、阻塞采集期间退役与 HTTP503→ACK 补报 1、panel telemetry 4。全 workspace/all-targets Clippy（warnings 为错误）、fmt、core 门禁、差异与 baseline Python 7 项通过，日志前缀 `/tmp/sinan-pr48-`。Rust 验证对应作者 `03f7d30` 加退出补修；随后合入的 `c958ba2` 只修改 IP 质量 Rust 文件，core/SDK/protocol、遥测相关 panel/test 文件与 Cargo 清单/锁完全相同，不重复无交集构建，也不以此前 246 项记录代替本轮验证。
- 最新合并源通过 Bun 1.4.2 冻结安装、TypeScript/Vite 重建，4 项字段测试 / 637 断言，最终 dist 的 Chromium 回环遥测 6 场景全部通过、浏览器错误 0。真实 Linux/systemd 和完整受保护节点负载仍按最终发布 HEAD 的 CI/独立验收核对；本轮 Unix 子树取消通过不冒称已完成平台全验收。

## 确认式诊断取消（Issue #19，独立 PR）

- 完成管理员取消请求持久化、WS 请求 / 清理确认协议、HTTP pending / ACK 恢复、SQLite 持久取消意图与 outbox。请求期间显示“等待设备确认取消”；停止失败、仍有进程或挂载保持待确认重试。旧 Agent 与缺少清理证据的后端明确不支持。
- 准备/下载期间可以取消且不再启动；已进入持久启动检查点的启动先结束再清理确认。任务绑定设备 / UUID / 模块 / 版本和已保存单元，取消不接受任意服务名。普通末尾报告不会提前结束取消状态，已有报告与任务根目录保留。
- 协议、PG + WS + HTTP、SQLite 重启、挂起的签名下载、实际 Agent ↔ 面板及真实 systemd 私有挂载夹具已加入独立验收。Bun 实际 dist 和浏览器桌面/移动状态验收通过；基于 main `75cd846` 的最终本项源码在受限 Debian 12 构建容器 fmt、Clippy 全目标、完整工作区测试通过：271 通过 / 0 失败 / 9 项环境忽略，exit 0 / 未 OOM。Linux core 测试二进制已交付，专用节点 6 个 systemd 夹具结果待记录。
- 代码 `8957f5c` 的 CI `36772176396` 通过 check、compose-smoke、Linux musl 两架构，实际 systemd 串行 6 项通过 / 0 失败 / 0 忽略（2.28 秒）；Reality 安装计量因 Draft 跳过。专用 Debian 12 节点夹具尚未执行，保留待验状态，不以 CI 环境替代。
- 设计见 [ADR 0026](docs/adr/0026-confirmed-diagnostic-cancellation.md)，独立验收见 [取消验收](docs/acceptance/diagnostic-cancellation.md)。不把本项夹具当作完整 NodeQuality / 持续代理流量验收。
- 合并审查统一能力宣告与真实清理条件：Linux/systemd/cgroup v2 三项必须同时满足，缺少控制器证据不宣告确认取消能力。新增回归尚待 Rust 编译槽，保留原签名、预算与不支持 OpenRC 的边界。整合 main `c958ba2` 的缓存/未知字段前端规则后，Bun 1.4.2 冻结安装、4 项/637 断言与 TypeScript/Vite 通过，重建合并 dist；最终后端/平台结果单独记录。
## 2026-10-01：发布草稿查找修正（独立 PR）

- [Issue #52](https://github.com/theLucius7/sinan/issues/52)：真实草稿的 tag 查询返回 404，而已认证 List releases 与按 ID 查询可读。发布器改为每页 100 条、最多 10 页查找精确且唯一的 tag，再按 ID 复核 ID、tag、完整 build SHA；缺失、重复、异常、扫描超限或身份变化均拒绝。原有 CI、资产摘要、签名与发布前后身份门禁保留。
- Fake API 对草稿 tag 查询明确返回 404；新增回归先在旧发布器下失败，再在修复后通过。分页、相似标签、歧义、重复 ID、页数上限及下载期间身份变化均有覆盖。发布专项 22 项全部通过；Python discovery 共 88 项，83 项通过、5 项依既有条件跳过（需要隔离 Linux root）。本次只修改发布工具、测试与文档，未运行 Cargo，也未据此宣称正式 Release 已发布。
- 候选源码 `75cd846`、`agent-v0.3.0` 标签与原有资产保持冻结；本修复须独立审阅，随后从已审阅的工具分支验证同一草稿，正式签名与公开状态另行记录。
- 对真实 GitHub 草稿执行只读验证成功：固定 Release ID、完整 build SHA 和九项资产 ID/摘要/大小与已下载并审核的候选完全一致，未执行签名或发布写操作。

- 合并审查：发布器专项 22 项重新通过，core 门禁与差异检查通过。真实 GitHub 草稿只读查找再次确认 ID、冻结 build SHA 及九项资产名称；没有执行签名或发布写入。

- 继续整合心跳隔离与发布工具修复；Windows 继承 SDK 默认不支持取消确认，恢复线程与 WS 请求共用退役门禁并复核退役标志，新增挂起 HTTP 回复后不得重建取消键的回归。此阶段仅完成源码/前端构建，Rust 最终验证待合入章节与页面拆分后的主线。
## 2026-10-01：诊断章节独立持久化（独立 PR）

- 独立章节表和完整度字段与执行状态分开；保留原整份文本及 r2 历史/恢复。NodeQuality r3 包装器每阶段原子保存章节，缺一章仍可读其他已存部分。
- Agent 离线观察也写 SQLite 章节 outbox，重启继续上传；HTTP 503 不阻止终态回报，版本和设备范围校验避免迟到覆盖。界面分别显示执行状态、完整度、每章预览/完成及缺失章节。
- Python 25 项首轮 20 通过、5 项既有 Linux/root 夹具忽略；本机 fmt、分层检查、差异检查和 TypeScript/Vite 构建通过。远端 Clippy、完整 Rust/PG、Linux 夹具和实际浏览器验收待完成，尚未声称整体验收完成。独立步骤见 [章节保存验收](docs/acceptance/diagnostic-report-sections.md)。

- 合并整合修复 watcher、capture 和退出快照并发发布：固定私有锁序列化版本更新，唯一 0600 临时文件原子替换并同步目录；保留已完成章节和原 ZIP。上游非零退出即使已生成完整报告也保持执行失败，超限章节明确提示截断。新增并发与 exit 0/7 包装器回归；原实现分别触发临时路径冲突、完整章退回预览及 exit 7 被改成 0，修复后 Python 共 28 项，23 通过 / 5 项既有 Linux/root 条件忽略。core 门禁及 6 项行为回归、fmt、actionlint、Bash/Python 语法、差异检查和 Bun 1.4.2 TypeScript/Vite 重建通过；Rust/PostgreSQL 专项待共享构建槽验证。

- 正常合入作者最新 `93356dd` 和 main `c958ba2`，保留缓存、未知字段严格校验及 4 项前端回归（637 项断言），重建 dist。独立 PostgreSQL 下 protocol/SDK/NodeQuality adapter 29 项、Agent 诊断 29 项、章节接口及既有 diagnostics API 6 项，共 64 项通过、0 失败/忽略；覆盖旧 payload/r2、断连重启、HTTP 503 与 ACK 持久化、执行失败但章节完整、迟到章节、内存保护和终态恢复。workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁及文档相对链接通过。未重复完整 workspace 或真实 NodeQuality 硬件压测，Linux/root 包装器 5 项仍须最终 CI 验证。

- 发布前继续正常合入 main `47c066b`（遥测隔离）及 `7a6f104`（草稿发布器），保留遥测线程/心跳时间、严格 IP 类型和全部验收章节。因遥测依赖变化，仅复验 Agent 章节重启/HTTP ACK 2 项与新增心跳迁移下的 PostgreSQL 章节 2 项，4 项全部通过、0 失败/忽略；全 workspace/all-targets Clippy、fmt、core 门禁通过。Bun 4 项/637 断言及 TypeScript/Vite 再次通过并重建最终 dist；发布 Python 22 项通过。草稿查找没有改变 Rust 源，不重复此前 64 项或全工作区测试；最终提交的 CI 与真实负载验收仍单独核对。

## 2026-10-01：P0 服务器 IP 与 NodeQuality 视图拆分（独立 PR）

- Issue #25：独立 IP GET/refresh 与仅包含准备状态/历史的 NodeQuality reports GET，NodeQualityView 去除 IP 查询字段。旧组合 GET 与旧刷新保留兼容汇合层；缓存 schema、旧报告/ID/参数和 当前 r3 签名制品及 r2 历史兼容不改。
- 服务器概况/IP信息/NodeQuality验机独立导航与 hash 页面；ServerIpInfo 展示一个入口下的数据库响应，NodeQuality 只保留验机和报告。IP 查询错误或缓存损坏不阻止新报告页读取历史，浏览切换不创建任务或刷新来源。
- fmt/core gate/差异、Bun 1.4.2 冻结安装/TypeScript/Vite 与最终 dist 桌面/手机导航、历史、IP失败隔离夹具通过。真实 HTTP/PostgreSQL、Clippy 和完整 workspace 待隔离槽或 CI；独立步骤见 [视图拆分验收](docs/acceptance/server-ip-view.md)，没有宣称实机完整诊断通过。

- 视图拆分 `177bfc9` 的 CI `36769525534`：check（Rust/PostgreSQL、全 targets Clippy、systemd、dist）、Compose 与两项 musl 通过，旧基线 Reality 失败。保留新未知字段提交和最新 main 后仍由最终提交 CI 复验。

- 对齐 main `6a583af`：保留章节组件/r3、严格旧字段类型校验和心跳/指标过期展示，只移动 IP 展示与路由。最终提交重新构建 dist，并分别核对最终 CI。

- 视图最终合并源重建通过 TypeScript/Vite、既有四项字段回归（637 断言）和实际 dist 桌面/手机完整路由隔离夹具；额外验证旧 payload 缺 kind 的已知 ASN=false 不冒充事实。章节组件调用和指标过期提示均保留，Rust/平台仍按最终提交 CI 核对。

- 合并审查最终保留 main `6a583af` 与作者最新 `84938ba` 正常祖先；作者新提交与已验证 `e0d6bda` 的全部 Rust/Cargo/CI/工具脚本及 web 源和 dist 完全相同，仅更新两项文档，不重复相同源码构建。完整 locked workspace/all-targets Rust/PostgreSQL 回归 289 项通过、0 失败、8 项既有 Linux/root/systemd 或外部运行时条件忽略；workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁、actionlint 与差异检查通过。Python discovery 83 项通过/5 跳过，NodeQuality 包装器 23 项通过/5 跳过；Bun 4 项/637 断言与 TypeScript/Vite 通过。最终 dist 实际 Chromium 桌面 1280×900/手机 390×844 共 9 组场景通过、页面错误 0，包含严格未知/有效0和false、IP失败隔离、旧报告与独立章节、指标过期与三类时间、独立导航；本轮未测试取消或实机完整诊断，最终 Linux CI 单独核对。

## 2026-10-01：P1 sing-box 业务归位（独立 PR / Issue #26）

- 面板代理节点、代理用户、授权、订阅、用户流量、发布及设备运行时清单移入 plugins/singbox；管理 API 与前端导航切换插件命名空间。系统管理员和代理用户使用不同名称，core 继续管理网卡总流量，epoch 仍标记计数器重置。
- 0012 仅新增启用证据表：当前设备能力、管理员明确启用、已有节点或 singbox 部署才启用；未知能力纯监控机关闭。订阅旧路径永久保留，旧表名和业务 ID/令牌/密钥/授权/历史账本不修改。
- Python 验收驱动 21 项通过、workspace fmt 与 TypeScript/Vite 构建通过。整合 main `6a583af` 后实际 dist 桌面/手机浏览器场景通过，页面错误为零、关闭服务器不发业务请求；独立 PostgreSQL迁移及最终完整 Rust/CI 证据待补；未宣称专用节点或生产迁移通过。设计见 ADR 0030，独立验收见 docs/acceptance/singbox-plugin-business.md。

- 正常保留作者 `b22386f` 与 main `6b63f71`（确认式取消）的双侧祖先，合并取消路由/cancel_supported 和插件 API，重建最终 dist。修复发布扫描与事务之间设备能力消失的竞态：服务器行锁内重新核对插件启用证据，关闭时保留 dirty，不创建会反向启用插件的空部署。补齐 API 文档中的插件路径与有意切换约定，永久 `/sub` 保持。
- 新增真实 SQLx 0012 故障注入、DDL/回填原子回滚与重试/二次启动幂等回归；旧库服务器、节点、用户、授权、部署、状态、批次、用量、设备会话及接入令牌十张表完整 JSON 快照比对，真实旧批次哈希重放去重并拒绝更改 payload。源码已完成，Rust/PostgreSQL 尚未执行，不把测试定义当成通过。Bun 1.4.2 TypeScript/Vite、实际 dist 桌面/手机插件验收、Python 驱动 21 项、core 门禁与差异检查通过；后续日常/完整诊断的业务证据读取将在插件边界内整合。

- Python 验收驱动 21 项通过、workspace fmt 与 TypeScript/Vite 构建通过。整合 main `6a583af` 后实际 dist 桌面/手机浏览器场景通过，页面错误为零、关闭服务器不发业务请求；独立 Debian 12 限制容器最终代码 `b22386f` 的 fmt / 全 targets Clippy -D warnings / 完整 Rust+PG 291通过、0失败、8既有条件ignore，新三项启用/真实旧数据迁移测试无忽略；exit0/OOM=false、容器已移除。首次抽取 Clippy 两项已修并完整复跑，本次 CI 尚单独核对；未宣称专用节点或生产迁移通过。设计见 ADR 0030，独立验收见 docs/acceptance/singbox-plugin-business.md。

- 业务归位 #55 正常整合 main `229becc`，保留已合并 #51 取消与 #56 独立夹具修复。重建实际 dist 后业务桌面/手机和取消浏览器回归、fmt/core 门禁/差异检查通过；291 项完整 Rust 证据仍限定旧 base，本轮 CI 单独核对。

## 2026-10-01 两个诊断入口（Issue #21，独立 PR）

- 新 r4 不可变 runner 将日常检查限制为自有标准库 TCP 探测，最多 4 个已配置启用目标、每 IP 族 4 次、DNS 2 秒/连接 1 秒/任务 90 秒，无硬件/rootfs/测速/上游/公开上传。IP 刷新复用逐源缓存，明确不是节点流媒体证据。
- 日常固定 64MiB/32tasks，完整512MiB/128；不降低256MiB启动预留、2GiB磁盘与128MiB运行保护。服务端完整入口必须管理员确认；正向计量活跃与缺少新计量证据的未知都需要警告确认。时间和证据随任务保存，不用网卡总流量冒充代理流量。
- preflight实际资源/负载/最终预算随Started检查点持久化，environment独立章复用r3补报。日常2章、完整6章；旧r2/r3签名队列/检查点继续恢复收集，不重复运行。新mode独立capability与Linux gate防止旧Agent误ready。
- 本机fmt/core门禁、Python discovery 88（83通过/5环境skip）、daily helper6、Bun/dist与真实Chromium6（0错误）通过。main 6a583af 上的模式源在集中Debian12容器（1.5GiB/2CPU/无swap）通过完整Rust/PostgreSQL293项/0失败/8既有环境ignored、全targets Clippy、fmt与Agent/Panel build，OOM=false；Linux wrapper29/helper6通过。wrapper首轮三个既有exit1期望0夹具需独立PR#56修复，远端临时对齐后验证，不混入本项实现。二进制保存binaries/modes-head；后续整合HEAD CI另核对。本机磁盘不足未从头Cargo。独立验收见 [diagnostic-modes](docs/acceptance/diagnostic-modes.md)，实际小节点保护/持续代理流量/取消矩阵由总任务整合验证。

- 发布前整合 main af43ccf 的独立 IP/NodeQuality 视图，保留拆分API与导航；日常入口改为独立IP刷新接口。fmt/core门禁、Python discovery83通过/5skip、helper6、Bun4项/637断言与TypeScript/Vite重建、最终dist Chromium6场景/0错误再次通过。整合Rust及取消状态兼容交最终HEAD CI，不用先前293项结果代替。

## 2026-10-01：P0 IP 查询入口适配（独立 PR）

- Issue #24：注册真实入口，check-place 是一个旧聚合入口、七种响应视图；新增 AbuseIPDB 官方 v2 CHECK 只读适配，固定 30 天窗口、目标身份与字段契约，Key 仅敏感请求头、无 UA/重试/重定向/verbose/上传。缺私有凭据明确未启用和信息未知，既有成功快照继续保留并标历史。
- 全入口共用四并发/40 秒批次截止、单请求超时/响应上限和 typed DNS/TLS 错误分类，不为每个入口重复预算。IP/provider/database 缓存与旧接口兼容，无新增依赖/迁移，服务器 IP 页按真实入口说明来源与不可用原因。
- 固定 AGPL-3.0 IPQuality 只读审查发现随机 UA、在线 main 引用、统计请求、请求重试和高并发/宿主依赖边界，参数不能直接消除；未执行或打包原版，节点自查明确未启用，流媒体解锁未知。受控修改版本由后续独立 PR 验收，不把此正式 API 项标为节点自查完成。
- 本机 fmt、core 门禁/六项行为测试、差异检查、Bun 1.4.2 冻结安装/TypeScript/Vite、真实桌面/手机浏览器的明确模拟来源/凭据/0false/403429timeout历史/禁用历史/百分比边界通过。真实 HTTP/PostgreSQL、Clippy 和完整 Rust 回归由本 PR 独立 CI 或隔离槽执行，尚未记为通过；官方账户/公网权限与完整诊断压力未验。契约与边界见 [ADR 0027](docs/adr/0027-ip-provider-adapters.md) 与 [入口适配验收](docs/acceptance/ip-provider-adapters.md)。

- 源码接到主线 `6a583af` 与视图拆分，保留报告章节/r3、心跳与严格旧字段兼容；入口适配不修改 NodeQuality 工具链。

- 保留主线旧字段 kind 缺失兼容，正式百分比规则按 abuseipdb-v2 响应区分，旧 payload 不引入新 kind 变体。五项前端字段回归/711 断言覆盖后端 60 个字段及官方整数百分比边界；当前/历史计数明确为数据项，避免把聚合响应数当来源数。
## 确认式取消最终合并审查

- 正常合入 main `af43ccf`（含章节持久化与 IP/报告拆分）并保留作者 `f5d468e` 祖先。修复 Windows 默认不支持路径、Linux/systemd/cgroup v2 能力误报、取消 HTTP/WS 退役门禁；已在途清理结束后不继续新取消任务。取消确认前采集最终章节，取消后迟到章节仍保存但不复活状态，旧整份报告继续标记 legacy；reports GET 和旧组合 GET 同时保留 cancel_supported。PostgreSQL 回归证明已应用 0011 后补入 0010 不丢任务、报告或章节。最终 locked workspace/all-targets 304 项通过、0 失败、9 项条件忽略；workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁和差异检查通过。Bun 1.4.2 冻结依赖下 4 项/637 断言及 TypeScript/Vite 构建通过，最终 JS 为 `index-8GW7pq_B.js`；真实 Chromium 桌面/390px 手机取消专项和原 IP/导航/章节 9 组场景全部通过，页面错误 0。Python discovery 83 项通过/5 跳过。此轮 macOS 本机没有运行新增真实 Linux/root/systemd 取消夹具（9 项条件忽略包含它）或完整 NodeQuality 负载；最终 Linux CI 与专用节点验收另行核对，不以前一作者 CI 替代。

- 已准备日常/完整诊断的插件证据桥：代理部署与正向计量查询由 sing-box 插件拥有，系统诊断只消费 configured / last_positive_at，不把沉默当作空闲，不更改用量账本或为监控机创建配置；接入调用待日常检查 PR 正常合入后完成。

## IP 入口适配最终合并审查

- 正常合入 main `6b63f71`（含确认式取消），保留作者 `cfea748` 祖先及双方全部记录；最终代码审查未发现需要改动的生产缺陷。全程移除真实 `SINAN_ABUSEIPDB_API_KEY`，只使用明确公开的合成 key 与回环 HTTP，未读取或调用真实账户。独立 PostgreSQL 下 IP/provider library 25 项、diagnostics API 7 项及章节/迁移 3 项，共 35 项通过、0 失败/忽略；覆盖敏感 Header、固定路径/参数/无 UA/重试/重定向、身份/类型、403/429/超时、禁用零请求、0/false 与双来源历史及新连接池/租约。Panel 全 targets Clippy（warnings 为错误）、workspace fmt、core 门禁及其 6 项行为回归、差异检查通过。Bun 1.4.2 冻结安装、5 项字段测试/711 断言及 TypeScript/Vite 构建通过，重建 JS `index-C1v7YTay.js`；最终 dist 在实际 Chromium 桌面/390px 手机的入口/缺凭据/0false/错误历史/禁用/百分比边界及确认取消组合场景全部通过、页面错误 0。本轮没有重复完整 workspace 或调用正式公网账户，不宣称配额/权限/节点自查或完整 NodeQuality 压力已验。

- 继续正常合入 main `229becc`（PR #56）：与已验证 `c2b01cc` 相比仅修改 `tools/test-nodequality.py`，所有 Rust/Cargo、生产工具、web 源与 dist 完全相同，未重复无交集 Cargo。包装器 Python 28 项中 23 项通过、5 项既有 Linux/root 条件跳过；core 门禁与差异检查再次通过。真实 Linux 正常退出的生产工具补修仍由独立任务验收，本项不将本机跳过计为通过。

- 业务发布并发复核：候选筛选后设备切换纯监控时，事务内重新检查启用来源，防止空配置意外成为永久legacy部署证据。新增旧候选→能力清除→零发布/保留dirty→明确启用后发布的 PostgreSQL 回归；该新项按后续 CI 单独验证。

- 业务归位最终代码 `29c4df4` / main `229becc` 的 CI36780128478 实际通过：fmt、全 targets Clippy、Rust/PostgreSQL307成功/0失败/9既有条件ignore，四项业务PG验收（含发布启用竞态）全部执行；随后六项真实systemd成功/0失败/0忽略，Compose与musl两架构成功。该PR事件Reality按draft跳过，专用节点/生产迁移未冒称通过；此后仅追加文档证据。

- 业务归位最终整合正常保留作者 `9cc507e` 与 main `8254055`（#51/#54/#57/R5），源码基线 `276bdea`。模式活动读取改用 plugins facade，真实 PG 证明纯监控日常检查 `not_enabled` 且零部署；发布保留锁内筛选和独立语句启用重检。首次整套链接因磁盘不足中断，随后按 target 清单串行完成全部八包 all-targets 覆盖并补统一 workspace library/adapter/runtime，去重325通过/0失败/9既有条件忽略，统一 workspace 全 targets Clippy、fmt、core 门禁/六项行为、actionlint及差异检查通过。Bun五项/711断言、TypeScript/Vite与实际 dist `index-Hx7wA0D0.js` 的桌面/手机插件、模式、取消、provider场景通过，页面错误0；浏览器使用明确 API 夹具。Python83通过/5条件跳过、R5包装器34通过、daily helper7通过。所有 provider 测试移除真实密钥并仅用合成凭据/回环；本机未运行真实Linux/root/systemd、正式账户/配额、上游完整负载或生产迁移，最终CI独立核对。详细证据见 [业务搬迁验收](docs/acceptance/singbox-plugin-business.md)。

## 2026-10-01 两个诊断入口（Issue #21，独立 PR）

- 新 r4 不可变 runner 将日常检查限制为自有标准库 TCP 探测，最多 4 个已配置启用目标、每 IP 族 4 次、DNS 2 秒/连接 1 秒/任务 90 秒，无硬件/rootfs/测速/上游/公开上传。IP 刷新复用逐源缓存，明确不是节点流媒体证据。
- 日常固定 64MiB/32tasks，完整512MiB/128；不降低256MiB启动预留、2GiB磁盘与128MiB运行保护。服务端完整入口必须管理员确认；正向计量活跃与缺少新计量证据的未知都需要警告确认。时间和证据随任务保存，不用网卡总流量冒充代理流量。
- preflight实际资源/负载/最终预算随Started检查点持久化，environment独立章复用r3补报。日常2章、完整6章；旧r2/r3签名队列/检查点继续恢复收集，不重复运行。新mode独立capability与Linux gate防止旧Agent误ready。
- 本机fmt/core门禁、Python discovery 88（83通过/5环境skip）、daily helper6、Bun/dist与真实Chromium6（0错误）通过。main 6a583af 上的模式源在集中Debian12容器（1.5GiB/2CPU/无swap）通过完整Rust/PostgreSQL293项/0失败/8既有环境ignored、全targets Clippy、fmt与Agent/Panel build，OOM=false；Linux wrapper29/helper6通过。wrapper首轮三个既有exit1期望0夹具需独立PR#56修复，远端临时对齐后验证，不混入本项实现。二进制保存binaries/modes-head；后续整合HEAD CI另核对。本机磁盘不足未从头Cargo。独立验收见 [diagnostic-modes](docs/acceptance/diagnostic-modes.md)，实际小节点保护/持续代理流量/取消矩阵由总任务整合验证。

- 发布前整合 main af43ccf 的独立 IP/NodeQuality 视图，保留拆分API与导航；日常入口改为独立IP刷新接口。fmt/core门禁、Python discovery83通过/5skip、helper6、Bun4项/637断言与TypeScript/Vite重建、最终dist Chromium6场景/0错误再次通过。整合Rust及取消状态兼容交最终HEAD CI，不用先前293项结果代替。

- 继续整合 main229becc，保留已合并#51确认取消与#56正常/非零退出夹具；新创建任务返回完整取消字段，cancel_requested同时阻止两种入口。最终dist重建与Chromium7场景（含等待取消和资源章）通过、错误0；fmt/core门禁、Python83通过/5skip与Bun4/637断言复验通过。最终Rust/平台CI另核对。

- main7848268合入独立查询来源适配层后再次整合，保留providers字段与独立IP查询接口；fmt/core门禁、Bun5项/711断言、TypeScript/Vite与dist重建通过。main229becc上的7c784e6已由CI36779254979验证workspace309通过/0失败/9忽略，随后真实systemd6项通过，Compose与AMD/ARM musl/OpenRC也通过；这些记录不替代新来源整合提交的CI。
- 本机fmt/core门禁、Python77（5环境skip）、daily helper6、Bun/dist与真实Chromium6（0错误）通过，wrapper26（6Linux/root跳过）。完整Rust/PostgreSQL/Clippy/Linux wrapper等待集中远端与该PR CI；本机磁盘不足未从头Cargo。独立验收见 [diagnostic-modes](docs/acceptance/diagnostic-modes.md)，实际小节点保护/持续代理流量/取消矩阵由总任务整合验证。


## 2026-10-01：TcpQuality 许可与原生路径（Issue #58）

固定检查 ibsgss/TcpQuality c2295ae 的完整树与 README，未发现明确分发许可。已创建作者授权询问 ibsgss/TcpQuality#27，当前无回复。按用户提供的备选方案采用独立 Rust TCP 建连诊断，不分发、执行或复制上游脚本/rootfs/helper，详见 ADR0029。

本次仅许可审计和决策；原生引擎、固定/签名制品、参数、无上传与宿主修改、共用框架登记及报告均各自验收。没有实际执行上游工具或访问生产节点。

- 最终合并审查正常保留旧作者 `ec7c663`、最新作者 `a374c12` 与 main `c47fc69`（含 #51 确认取消、#54 真实 IP 来源和 #59 许可文档）。修复 DNS 列表前八项同属一个 IP 家族时 `both` 静默漏测另一家族：先按家族选择，仍至多两个地址、每家族四次连接；新增两种顺序回归。portable 端到端夹具明确使用模拟 Linux 服务身份，不放宽生产 Linux 门禁。最终集中 Rust/PostgreSQL 专项 89 项通过、0 失败/忽略（适配器12、Agent诊断37、IP/provider25、diagnostics API10、章节3、真实Agent WS/HTTP/restart2），workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁及六项行为回归、差异检查通过。Bun 1.4.2 五项字段测试/711 断言与 TypeScript/Vite 重建通过，最终 dist 为 `index-CtZ2u8uf.js`；实际 Chromium 桌面与390px手机模式各7场景、确认取消与来源展示通过，页面错误0。Python discovery 83通过/5跳过、daily helper7通过、r4包装器23通过/6项Linux/root条件跳过。所有测试移除真实 `SINAN_ABUSEIPDB_API_KEY`，仅回环与合成凭据；本轮未重复完整 workspace，也未在本机执行真实 Linux/root/systemd、上游完整验机负载或正式 API 账户/配额验收。最终提交 CI 单独核对，R5正常退出契约修复继续独立合并。

## 2026-10-01：固定 NodeQuality 正常退出契约修复

- main `af43ccf` 的 Linux check 发现三项包装器回归失败。完整章节与执行成功仍分开，但此前直接保留上游非零返回漏掉真实固定入口的正常清理 `exit 1`：固定源码 SHA-256 `4e1b25894cadf908ef61fb0d9ce874a75524c6dafc2ea26f0477107288e0c018` 第 455 行，由 `main → post_cleanup` 正常到达。
- 新启动观察器保留上游/许可证原字节，仅确认该精确分支；完整本地报告校验成功后才转换此特例。任意早退 1、真实失败 7、信号清理、清理拒绝、缺报告不转换，可选上传 HTTP 403/传输失败单独警告。原退出值、ZIP、五章、关闭公开上传、取消清理和不可变构建检查都保留。
- 包装器使用独立 r5（r4 留给模式功能）；r2/r3 历史与排队恢复兼容，原 r2 签名资产及冻结候选源码不变。macOS 夹具 32 项全部通过、0 跳过；断网且无 Linux capabilities 的只读测试容器重现旧版 28 项/3 失败，并验证修复版 32 项全部通过、0 跳过。挂载、chroot 和网络全部为合成夹具，不对生产或真实基准执行操作。fmt、core 门禁、Python/Bash 语法及差异检查通过；Rust 编译/Clippy 尚待后续独占槽或最终 CI，模式功能合入后的最终版本将另行复验。

- 正常合入确认式取消、IP 入口及 PR #56 的原始 exit 0/1 夹具；保留纯 exit 0 成功、纯 exit 1 失败及所有原 ZIP/五章/选项/清理断言，固定正常 cleanup 的 exit 1 另作独立场景。夹具补上真实 main 的 EXIT→sig_cleanup 二次清理，macOS 和断网 Linux root 都 33 项全部通过、0 跳过。相同 33 项的反对照中移除正常分支转换触发四项失败；把所有 exit 1 转成功会被早退、信号清理、清理拒绝及原始非零夹具拒绝。已核对 cleanup 夹具与固定 SHA 源码第 440–456 行原字节一致。发布 Python 32 项中 28 通过、4 项现有条件跳过；没有占用 Cargo，模式主线及 r4→r5 历史兼容仍待最终合入。

- 最终正常合入正式模式主线 `2ea4bb9`（含许可文档），保留 DNS 双家族修复。r5 同时内嵌 daily helper 与 full 分支观察器；r4 已签模式任务继续使用自身版本、参数及目标预算恢复，r2/r3 保留旧参数且拒绝 mode 参数，不能用 r5 二进制替代 r4 身份。新任务实际创建为 r5。macOS 和正式主线后的断网 Linux root：包装器 34 项与 daily helper 7 项，共 41 项全部通过、0 跳过；无真实硬件压测或公开上传。独占短槽下适配器 16 项（新增版本接收/恢复 4 项）及 PostgreSQL 模式创建 1 项，共 17 项通过、0 失败/忽略；适配器全 targets Clippy（warnings 为错误）、fmt、core 门禁、actionlint 与差异检查通过，已释放构建槽，不重复完整 workspace。
- 本地双架构 r5 打包成功且 runner 原字节相同，归档 SHA-256 均为 `385a8c42e5a54542544b6459e1246958d0abe8bd860c3e44dd8e1996efbdbcb1`；包内原入口与 AGPL 许可证 SHA-256 和固定值完全一致，daily/observer 都已内嵌、无未替换 marker。当前构建说明与固定旧 r2 草稿候选分开，未签名、发布或修改任何旧 Release；最终整合全 workspace 与实机压力继续独立核对。
## 2026-10-01：原生 TCP 连接工具（独立 PR，尚未接服务）

- 用户授权无上游许可时自行实现；仅新增自有 AGPL Rust 库/二进制，不复制上游代码/目标/rootfs，不注册 panel/Agent 第二插件或改 UI/签名管线。未来目标由已配置 TCP 拨测冻结提供，不修改 ProbeSpec/协议兼容。
- 最多八目标/16 KiB 快照与摘要核对，IPv4/6、count4/8、concurrency1/2；DNS2秒/单连接1秒/间隔250ms，总60秒含排队并预留2秒发布。只连一个同族 SocketAddr、关闭连接且零应用 payload，必须 --no-rank-upload，宿主/测速/未知选项拒绝。
- 有界 JSON 与原子独立章节保留部分结果，明示连接成功率/建连耗时，不冒称包丢失/测速、没有排名，未知不补0。编译期源码 SHA 未提供则 null；固定源码/锁和签名打包及框架注册均为后续独立 PR。
- 本机仅 fmt/locked offline metadata/core 门禁/差异检查；Debian12 1.5GiB/2CPU 隔离槽全源码 touch 后，5896f6d 的真实 IPv4/6/CLI/取消及有界 DNS/并发/截止/输入安全 13 项、fmt、全 targets Clippy（warnings 为错误）、完整 Rust/PostgreSQL 331 项通过/0失败/9既有条件忽略，exit0/OOM=false。ef1c7c9 再补 stdout write/flush 共用2秒截止，13 项与fmt/Clippy再次通过；全量重复运行按协调主动停止，不将331证据移给新SHA。独立 Draft PR #68、milestone1，固定/签名与服务注册仍为后续，最终CI另核对；不宣称服务或真实网络压力验收完成。见 [原生 TCP 验收](docs/acceptance/native-tcp-probe.md)。

- 最终正常保留作者 `0a6b849` 与正式主线 `e3a41ed`，解决进度文档冲突且保留共享诊断服务、中性活动桥、业务插件和 r5。DNS 按家族筛选后至多保留两个有效地址，避免前32项为另一家族时漏测；实际打开输入句柄重验权限/链接数/UID，首次报告写入前拒绝异主目录；运行错误 stderr 也受两秒和总截止限制。16项工具测试（13库/3真实CLI）全部通过、0失败/忽略，回环验证零应用数据。旧截断、移除UID检查、同步阻塞stderr负对照均被回归抓住，恢复修复后再次通过。工具及workspace全targets Clippy（warnings为错误）、fmt/core/actionlint/差异检查通过；截止夹具保留一秒探测并提供两秒原子发布，生产仍60秒/两秒。没有新增依赖、拨打第三方节点、公开上传、签名/发布或冒用旧331项全量证据，最终主线CI继续单独核对。

## 2026-10-01：原生 TCP 固定源码与签名制品（独立 PR）

- 依赖原生引擎 #68，新增固定 Git 对象归档构建与归档内配方执行，Cargo.lock --locked、native musl ELF/CLI/来源验证，外部不可变版本包含完整 SHA。
- 包内二进制 + build-info、许可证、完整源码归档、锁文件和第三方原文使用现有 release/minisign 精确签名契约；release 显式选入第四模块，旧三模块默认不变。
- SDK 默认空辅助文件声明由 core 同时用于签名前检查与下载，含五文件实际安装/缓存篡改/启动前再校验回归；不登记 TCP 插件或修改 UI。
- Python TCP 来源/签名/许可 11 项、旧 Release 32 项、build-script 5 项通过；实际锁定原文库存 35 包 / 2,006,844 bytes。Rust 与真实 musl 和双架构 CI 尚待完成，正式 release 未发布。独立验收见 docs/acceptance/native-tcp-artifacts.md，决策见 ADR 0032。
- 最终正常保留最新作者 `bf56d5e` 与正式主线 `fb79388`，源码基线 `644e785`。多余aux读取删除且准备阶段集合保持一致，五辅助文件下载/缓存篡改/签名前及启动前拒绝与旧适配器兼容保留；显式选入TCP时严格要求双架构及旧三模块完整。独占槽Rust专项90通过/0失败/0忽略（制品14、release6、wire12、诊断/取消/预算/来源40、SDK1、TCP17），workspace全targets Clippy、fmt、core门禁/六项行为、actionlint及差异检查通过。Python最新契约12与模拟发布22项通过，无真实发布。#70的11个前端/门禁哈希原样保留，含暂停完整入口与 `index-OopWuqxH.js`、中性活动桥/r5；未重复无交集浏览器、PG或完整workspace。本机没有Linuxmusl构建、真实systemd、生产节点或正式Release，最终CI继续独立核对。详见 [制品最终验收](docs/acceptance/native-tcp-artifacts.md)。
## 2026-10-01：共用诊断任务服务（Issue #27）

将 NodeQuality 的参数、工具版本和报告规则移到登记插件，创建/能力/签名制品/预算/互斥/结果/历史改为共用服务，保留原 API 和原任务历史。协议新增可空预算，Agent 只收紧既有适配器上限；新任务要求能力握手。新增路由竞争、跨插件互斥、历史/部分报告与预算权限夹具。

代码 bfae951 在限额 Debian 12 容器通过 fmt、全 targets Clippy（warnings 为错误）、完整 locked Rust/PostgreSQL 318 项/0 失败/9 既有条件忽略；容器退出 0、OOM=false。Linux wrapper 29、daily helper 6、core 分层行为 6 项通过。修复并重新验证历史 job={} 正文上传与缺插件字段的 NodeQuality 历史兼容。当前提交的独立 CI、真实 systemd 与双架构构建另行核对；专用节点连接恢复与完整验机总验仍待补。详见独立 [共用诊断服务验收](docs/acceptance/shared-diagnostic-service.md)。

- 最终正常合入作者 `924a8ff` 与正式主线 `2c3c1e5`（含 r5 正常退出契约及 sing-box 插件搬迁）。迁移后的 NodeQuality 保持 r5，活动证据只调用中性 `plugins::runtime_activity_on`，保留纯监控未发布、陈旧能力、缺发布行与代理活动判断。包装器、daily/observer 与前端原字节均与已验证主线一致，本轮不重复无交集包装器或浏览器验收。
- 补齐真实缺少 plugin/resource_budget 字段的 r2 历史 JSON 与原文精确保存、跨插件 queued/running/已过期 cancel_requested 对两条创建入口的互斥，以及 IO 权重不可放宽和预算未知命令字段拒绝。最终相关 Rust/PostgreSQL 68 项通过、0 失败/忽略（协议 12、Agent 诊断/预算 39、共用服务 2、诊断 API 10、章节 3、真实 Agent WS/HTTP/restart/cancel 2），workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁、actionlint 与差异检查通过。完整 workspace 测试尝试在另外 47 项通过/2 项既有条件忽略后因本机链接器磁盘耗尽中止，不记为完整测试通过；只清理本任务失败链接对象及已通过的测试可执行文件。真实 root/systemd 专项、签名发布、硬件压力与最终提交 CI 继续独立核对，未在生产机器压测或公开上传。


## 2026-10-01：NodeQuality 不受控完整任务安全门禁（Issue #28）

- 固定入口审计确认在线 main 依赖、内层公开上传和宿主 swap；分别创建 #65/#66，归整改 milestone。固定提交、摘要、许可缺口与无特权命令模拟证据见 [执行链审计](docs/acceptance/nodequality-chain-audit.md)。
- 插件拒绝新完整任务，共用服务调用登记插件安全 hook；旧排队完整任务保存明确失败原因且不冒充设备已停止。运行任务不重发给无门禁能力的旧 Agent，新 Agent 的适配器在诊断工作目录/工具调用前拒绝完整模式；既有 Started 继续原身份收集/取消。r2–r5 不可变制品不改，日常固定轻量分支与资源预检不降低。
- 界面分别显示日常可用与完整暂停原因，历史关闭上传仅称顶层设置；真实 Chromium 桌面/390px 手机验证日常、403、历史章节与确认取消、离线/重复禁用，页面错误和完整 POST 均为 0。fmt/core 门禁、Bun 五项/711 断言和 dist 构建通过；源码20e27ff在限额Debian12容器通过全 targets Clippy、完整locked Rust/PostgreSQL325项/0失败/9既有条件忽略、专项34和r5包装器34/helper7；Agent/Panel构建退出0、OOM=false。二进制及core测试保存binaries/nodequality-gate-head，源码摘要与日志保存evidence/nodequality-gate-head。
- [ADR0031](docs/adr/0031-nodequality-full-start-gate.md) 与 [独立验收](docs/acceptance/nodequality-full-start-gate.md) 明确旧 Agent 门禁前已领取任务须升级或取消确认。本项仅止血，不关闭 #28/#65/#66，不替换或删减完整能力；完整工具链及用户取舍仍待完成。
- 发布后正常重基已合并框架/插件业务归位/原生引擎的main2574a84，保留主线视图与PluginServer类型，重新构建dist并复验；最终整合HEAD CI单独核对，不以前一冻结源码的325项代替。

## 2026-10-01：完整验机门禁合并审查

- 正常整合插件业务主线 `2c3c1e5`、正式共享服务 `e3a41ed`、正式原生引擎主线 `2574a84` 与最新作者 `75cf69f`，保留新版插件设置、代理用户与独立 IP/诊断路由；重建 dist，完整验机门禁保持启用。
- 修复缺少 plugin/mode 的旧任务绕过面板门禁：排队清理和分发统一采用历史 NodeQuality 归属；分发响应补默认插件名但原 JSON、版本与参数不改写。新增旧排队、运行分发、迟到报告及已登记硬件章节回归；章节可保存但门禁失败原因与未完成状态不被伪造，已有 Started 恢复/取消路径保持原逻辑。Preparing 升级夹具改为主机架构的签名制品地址，实际验证走到适配器门禁且零启动。
- 本机 Python 包装器 34、daily helper 7、Bun 5/711 断言、TypeScript/Vite、fmt、core 门禁、actionlint 与实际 dist Chromium 门禁及插件业务场景通过；完整 POST 0、页面错误 0。冻结整合源码 `e2ccd3f` 的相关 Rust/PostgreSQL 75 项全部通过、0 失败/忽略（适配器 15、Agent 诊断/预算 39、API 13、共用服务 2、章节 3、真实 Agent WS/HTTP restart/cancel/Preparing 门禁 3）；移除旧插件默认归属的负对照使两项回归失败，恢复后三项门禁回归再次通过。workspace 全 targets Clippy（warnings 为错误）通过，最终主线 CI 继续独立核对；本轮仅使用回环和假服务，未重跑无关 accounting 全量。
- 固定上游执行链六个源码摘要与审计表一致；测试移除真实 SINAN_ABUSEIPDB_API_KEY，未运行真实完整验机、正式 API、宿主 swap 或发布操作。
## 2026-10-01：原生 TCP 参数与报告无状态适配器（独立 PR）

- 仅新增依赖 SDK 的 TcpQualityAdapter，不登记第二插件或迁入生命周期/ProbeSpec/UI。版本目录与源码pin、精确version/build-info、五静态签名aux、参数/地区白名单、原字节目标摘要、私有目录/文件、有界IO与64MiB/32tasks/60秒预算均独立校验。
- mandatory --no-rank-upload，不接受测速/宿主/rootfs/上传选项；仅返回ServiceJob，环境章core处理。严格报告目标/参数/source/UTC/实际地址/统计/null语义，最多十独立工具章，坏章不遮住好章，取消/重启前部分结果可继续读取。
- 记录型Privileged与真实磁盘夹具覆盖参数先拒绝、身份失败不准备、预算/固定argv、0与未知、篡改、部分/重复、链接/超限和坏章。881cce5的fresh GitHub CI：13专项、全targets Clippy、完整Rust/PostgreSQL353通过/0失败/9既有条件忽略、六项core真实systemd回归，以及Compose/Agent双musl/TCP制品双arch通过；最终制品依赖HEAD另核。不会将合成报告或现有core回归称为已登记TCP的服务/签名安装验收。见 [TCP适配器验收](docs/acceptance/tcpquality-adapter.md)。

- TCP 适配器最终正常整合最新作者 `972647c` 与正式制品主线 `cbe5558`，保留 nullable 字段必须显式出现、同族 literal 不得伪称家族不可用及原报告/章节完整度规则。prepare 比较可信签名缓存 binary UID，历史收集核对可信私有任务目录、输入、章节/报告及实际句柄 UID，删除旧二进制后仍可重复读取已有部分报告；调用方保持任务根目录及祖先可信。冻结源码 `7a70b11` 的 TCP14+owner1+NodeQuality15 共30项全部通过、0失败/忽略，两项移除 UID/错误依赖旧二进制的负对照被实际回归捕获，恢复后再通过；workspace 全 targets Clippy（warnings为错误）、fmt、core分层6项、actionlint与链接/差异检查通过。core/SDK/native/前端与正式主线原字节一致，未重复完整workspace或冒称TCP已登记/实网验收，已释放构建槽。

### P2 原生 TCP 制品 Debian12 启动修复（独立后续）

#69 整合后独立修复 Bookworm 的 musl-gcc 静态 PIE 启动 SIGSEGV：使用 native cc 与 Rust 自带 musl/CRT（link-self-contained=yes），保留静态 PIE，并新增 Debian12 真实构建、执行、五辅助文件签名 CI。help 去除临时接入状态。永久公开工具源 5e843f0fd9532abe9b7b9a052ef77b45abcfa675、外部版本 0.3.0-<该SHA>-r1 已实际原生执行验证。源码500受限容器完整Rust347/0/9、Clippy/fmt、Python来源签名12/旧Release32/模拟发布22通过；最终5e再次TCP17、Clippy/fmt、实际musl/完整TEST_ONLY bundle通过，exit0/OOM=false。未发布正式Release，后续main功能和最新CI状态需单独核对。详见独立验收 native-tcp-artifacts.md。
