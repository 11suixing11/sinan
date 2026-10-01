# 执行进度

## 2026-10-01：开放 issue 第一批（#3、#4、#6、#14、#15）

- 基于 `bb9638b` 审查五项实际剩余缺口。版本选择、类型安全资源预算与常驻 systemd 优先级已有源码，不为已覆盖缺陷制造重复实现；详情见 [第一批处理记录](docs/acceptance/issues-batch-one.md)。
- 修复验收驱动不接受当前默认 `version=latest/tag=null` 的回归，保留私有令牌恢复与显式旧版/目标版选择；人工执行改用 `--version`。显式选择分别说明缺少已签版本、协议不兼容与目标平台没有可安装制品。
- 补齐设备 CDN/同域 TLS origin 分开诊断和完整设备接口要求；注册错误显示 HTTP 状态与限速/授权/路径方向，不输出不可信响应体。补齐 Reality 固定 90 秒预算与超时定位，修正 OpenRC 新诊断和独立 bootstrap 生成来源的过期文档。
- 用户要求批次期间不测试：新增 Rust/Python 回归尚未运行，未执行 Clippy、构建、CI 或实机操作；全部批次完成后统一验证最终整合源码。#6 间歇超时根因仍未知，完整上游联合负载和真实 CDN 策略继续待验，本批不关闭 issue、不发布或部署。

## 2026-10-01：四平台接入后续守卫与并发提交整合

- PR #132 作者同期合入 `dd13a6c` 后，通过独立后续 PR 普通整合实际 main `bb9638b`，完整保留作者负号令牌兼容及原运行输入。补旧正式 0.3 的静止服务/SQLite WAL 只读检查，Preparing 的完整验机及不兼容 Started 降级被拒绝，原状态不改；Started 只接受精确原 r2 与原三个参数。安装器不自动停止现有服务，失败保护旧配置。
- 补目录共享 30 秒绝对截止、PowerShell 五种单引号的双层转义，以及每次验签/Agent 原生调用的实际启动结果。旧退出码为 0 而工具无法执行的真实负例在新实现被拒绝；生成入口与源码一致。
- 冻结运行输入 `0683dd9`：本聊天完整 Rust/PostgreSQL workspace/all-targets 482 通过、0 失败、15 项既有实机条件忽略；macOS umask077 原子文件/链接专项、全 targets Clippy、fmt/core 通过。自己的 PostgreSQL 127.0.0.1:55432 已停止。
- 同份输入的 Python/实际 PowerShell 7.5.3：旧检查点 12 通过、bootstrap 21 通过/3 条件跳过、PowerShell 15 通过/0 跳过、release 29 通过/7 条件跳过，生成一致与 Shell 语法通过。UI/web/dist 与前次本聊天 36 Bun、14 Chromium 所受验输入逐字相同，不把私有 API 替身当作新原生安装。
- 作者的隔离 OpenRC 真实接入证据仍归作者；本聊天未复演原生 Windows PS5.1/UAC/ACL、macOS/FreeBSD 服务安装，也未正式签署、发布、部署。所有 full 门禁保持；CI 继续暂停，未执行不算通过。

## 2026-10-01：四平台接入整合 86e2ef4 与负号令牌回归

- 普通整合主线 `86e2ef4`，保留 PR #127 的统一监控/通知与 PR #128 的 NodeQuality r14；跨平台决策重编号为 [ADR 0041](docs/adr/0041-cross-platform-enrollment.md)，主线 0039/0040 保持原意。相对主线 Core、旧 Agent 更新与监控逻辑不变；接入入口仍只从 GitHub/独立镜像下载 Agent。
- 最终安装验证发现真实随机接入令牌可首字符为 `-`，分离的 `--token` 参数会被解析成选项。修正 Unix 外层、内嵌 Linux 执行器、Unix 原生及 Windows Agent CLI 为 `--token=值`，不调整令牌生成规则或既有正式制品。新增真实 argparse/CLI 边界回归；前次失败日志保留，不计为安装通过。
- 冻结最终 505 份运行/构建/测试输入，收据 SHA-256 `ee1ee1b59e8eafabf87fadb606ede69c24aae9038e00bf63b942ef0795422419`；逐字核对不变。使用公开 TEST_ONLY 编译根与专属 PostgreSQL 重跑完整 Rust workspace/all-targets：482 通过、0 失败、15 项既有条件忽略，64 组；全 targets Clippy、fmt/core/diff 通过。
- 最终 Unix root 29/29、Linux 上 PowerShell 函数/独立验签/清单/下载/恢复 11/11、root 发布工具 36/36 全部通过；模板/生成输出及 Shell 语法检查通过。Unix 固定 blob `05be468326df1c70afed2c23d4763243f8a0e990`，Windows 固定 blob `ec7bb81cbece50114f234e7d4dae96cfc0905433`，均已按 Git 对象与 SHA-256 核对。
- 最终 UI 36 项 Bun 单测、TypeScript/Vite/dist 构建及接入、插件目录、监控、运营、代理业务、看板、服务器展示、NodeQuality、TCPQuality 九组桌面/手机浏览器回归通过；覆盖单行剪贴板、无签名版本不生成令牌和手机布局。受验 dist 逐字保持，另以正式公开根重编最终嵌入前端的面板成功；浏览器使用私有回环 API 替身。
- 隔离 Linux ARM64/OpenRC 容器先移除 curl/minisign，正式公开根面板通过真实 API 生成首字符为 `-` 的令牌；直接执行返回的自动匹配单行命令，以普通用户完成提权、补依赖、独立验签正式 `agent-v0.3.0`、注册并上线。再用显式 `0.3.0/linux-musl-arm64` 命令重复安装，设备公钥与私钥摘要不变。实际受验入口为上列最终 Unix blob；面板只缓存 ARM 三个模块、完整签名目录仍提供 AMD/ARM，Agent 面板下载实际为 409。
- 自有测试面板及两个专属容器已停止/移除；生产面板/PostgreSQL 容器健康且未部署。没有签署或发布新正式 Agent/诊断 Release；Windows/macOS/FreeBSD 对应正式签名制品与原生 ACL/UAC/launchd/rc.d/计划任务实机安装仍待独立发布和验收，macOS 现有 ABI 仅 ARM64。CI 继续按仓库约定暂停，未执行不算通过。下一步审阅后按部署流程升级面板，并由维护者准备对应平台签名发布。


## 2026-10-01：四平台接入整合 33a/824 的阶段验证

- 按本聊天补充要求完成 Shell（Linux/macOS/FreeBSD）与 PowerShell（Windows）入口；执行时检测 OS、CPU/libc，选择最新兼容稳定版，也可指定完整签名 proof 中的版本及 ABI。界面复制为一行，缺少本平台已签版本时禁用生成并说明原因。macOS 目前仅有 ARM64 ABI，Linux/Windows/FreeBSD 支持 AMD64/ARM64；32 位及未知平台拒绝。
- 整合已合入原 PR #114 的主线 `33a3085`，完整保留 `download_source=github` 协商、旧 Agent 空候选保护、制品 URL 约束、镜像与服务器运营功能。Agent 始终从 GitHub/独立镜像匿名下载；面板 Agent 文件接口保持 409。旧正式 0.3.0 的 install.sh 仍校验签名摘要，但实际执行固定 blob 内嵌的受信 Linux 执行器；唯一预下载契约针对实际执行器，不修改既有正式资产。
- Unix 单行固定程序先提权，再在 root 私有目录下载、核对摘要及执行入口；直接非 root 运行入口拒绝，不再特权重读用户可写文件。Windows 在提升后的管理员进程重新下载固定入口，检查 ACL/重解析路径及摘要；原生安装保留身份、旧配置恢复、JSON current 引用与正式根复验。Unix 单文件 300 秒总预算、20 秒/剩余预算读取限制和 Windows 有界流式下载保持。
- `f45ade0` 冻结 504 份运行/构建/测试输入，收据 SHA-256 `2c8434a8300d42e3682ba65a411e8f3aa15f29ac7f1f71bc78a003ac359c6a89`。整合后显式使用公开 TEST_ONLY 编译根的完整 Rust workspace/all-targets 471 通过、0 失败、15 项既有实机条件忽略，61 组；Clippy 全 targets、fmt/core/diff 通过。此前未显式配置 fixture 编译根时三个监督退役夹具失败，改用正确测试根后全量通过；未修改 core 产品或测试去放宽验证。
- `f45ade0` 对应 Unix root 测试 28/28（含实际降权至 UID 65534 的直接入口拒绝）、Linux 上 PowerShell 函数/签名/清单/HTTP/平台/恢复测试 10/10、root 发布工具测试 36/36；两个生成器输出同步。Bun 30/850 断言、TypeScript/Vite 构建及 19 个 dist 一致性通过，接入/运营/制品各桌面与手机共六组 Chromium 通过；最终 UI 相对此前完整看板受验输入仅保留 main 的旧 Agent 迁移提示及相应资源哈希，不冒称重演原生服务。
- 以正式公开根重编面板，在隔离 Linux ARM64/OpenRC 容器清除 curl/minisign，直接执行最终 API 返回的一行命令：普通用户自动提权、准备依赖、独立验签正式 `agent-v0.3.0`、注册并上线；显式 `0.3.0/linux-musl-arm64` 再次安装，设备公钥与私钥摘要不变。最终 Unix blob `a3269772648687109e442b8ea26fa98c64b519f8`，Windows blob `9901b27b65c0e265f718990d33c9ec25f2900e15`；面板只缓存 ARM 三个模块而签名目录仍提供 AMD/ARM，Agent 面板下载实际返回 409。
- 后续 PR #132 提交期间主线合入插件目录与混合链路规划，再普通整合 `8249326`：保留新只读插件目录、删除旧制品页与旧制品浏览器用例，空版本提示改为维护者准备签名发布及实际目录链接；README/运维文档同步。Rust/安装器/构建工具/测试的 405 份输入逐字保持 `f45ade0`，上述 471、28/10/36 及正式根 E2E 不改称重新执行；最终 UI 另完成 36 个 Bun 用例、89 模块 TS/Vite/dist 构建、插件目录/接入/业务/运营四套桌面手机 Chromium，均通过。最终正式根 `cargo build -p sinan-panel` 也通过，确认嵌入最新前端；浏览器只用回环私有 API 替身，既有原生验收缺口不变。
- 专属测试面板、Agent 容器及 PostgreSQL 已停止/移除，生产两容器健康且未部署。没有正式签署或发布新 Agent/诊断 Release；当前正式发布只有 Linux，Windows/macOS/FreeBSD 对应正式制品及实机 ACL/UAC/launchd/rc.d/计划任务安装仍待独立发布与验收。CI 按用户安排继续暂停，未执行不算通过。实现决策见 [ADR 0041](docs/adr/0041-cross-platform-enrollment.md)。


## 2026-10-01：一行接入命令与按服务器架构导入制品

> 以下保留 PR #114 原作者在合入 GitHub-only PR #119 前的历史受测输入。旧正式 0.3.0/OpenRC 成功不能认证最终新下载机制；本聊天未重演作者私有容器。PR #114 合并时的流程明确拒绝旧安装器，彼时要求另发兼容签名 Release；这次四平台扩展改用独立固定 blob 中的受信 Linux 执行器兼容旧正式制品，新的实际验证记于本文件前部的四平台接入整合记录。

- 核对当前 ARM64 部署：旧安装命令依赖预装 `sinan-bootstrap`，`/install.sh` 返回冲突；导入的六个制品同时包含 ARM/AMD。现有完整发布独立 minisign 验签及八项摘要全部匹配，未发现下载物损坏。
- 新接入命令从固定官方 GitHub blob 下载自包含入口，核对 SHA-256 后执行；缺少 curl 时先通过系统软件源准备，入口自动准备 Python/minisign，使用已批准的公开根独立验证已签 Release 与安装器，再下载本机 Agent。`/install.sh` 返回安装描述 JSON，不执行面板动态 shell。生成入口由原有验证工具生成并检查同步，无新增依赖；首次信任流程调整见 ADR 0037。
- 制品导入增加平台架构选项，默认按已接入设备 ABI 选择，无上报时使用面板宿主平台。只下载兼容的所选架构；完整 proof 保留。局部清单仅选择已签路径，同发布可追加架构、复用有效文件、修复缺失或损坏普通文件；未知/重复路径、软链、篡改及跨发布身份冲突继续拒绝。旧完整目录保持兼容，文件逐个原子替换、清单最后公布，下载或验证失败不改变已公布有效集合。
- 最终整合后，真实 PostgreSQL 下 panel 库与 foundation/releases/platform_artifacts/server_setup/agent_updates 共 74 项通过；完整 all-targets Clippy（warnings 为错误）、fmt、core 边界、差异检查通过。root Python bootstrap 11 项全部通过；release 32 项执行、4 项既有条件跳过。Bun 27 项/843 断言通过，TypeScript/Vite 构建通过并同步 dist；接入与制品 Chromium 1440/390 两种宽度通过，覆盖整条命令剪贴板一致、架构选择、错误恢复和手机布局。
- 隔离正式根面板从真实官方 `agent-v0.3.0` 导入仅三项 ARM 制品，没有下载 AMD。导入的 ARM Agent 0.3.0 和 sing-box 1.14.2 均能实际执行。带 init 的隔离 OpenRC 容器移除 curl/minisign 后，直接执行最终 API 返回命令，自动补齐工具、完成正式签名验证、注册并上线；重复安装仍为同一服务器与设备公钥，状态接口 connected=true、面板 online=true。首次无 init 的容器无法回收退出 supervisor 的僵尸进程，重复安装检查改在带 init 的隔离容器完成；未修改产品服务逻辑掩盖测试环境问题。
- 未验证范围：远端 CI（按用户安排继续暂停）、完整 workspace 测试、其他平台及生产升级；未更新生产部署或发布新 Agent/诊断版本。本次实测使用既有已签 Agent 0.3.0，不替代源码 0.3.1 与后续诊断能力的发布验收。下一步审阅并合入本 PR 后按部署流程升级面板。

## 2026-10-01：代理节点内创建多条链路的重新设计

- 用户明确选择统一代理节点页面、直连节点与链路并列管理。基于主线 `0424090` 核对页面、插件、迁移、编译器与既有链路测试，完成 [设计方案](docs/node-chain-design.md)：资源列表/详情、多出口独立监听、事务内批量创建、幂等收据、授权/订阅/计量、共享出口保护、退役资源清理及实施验收。
- 前端与后端独立审查确认现有问题是 Groups 的链路 CRUD 与 Nodes 的普通节点投影错位。旧入口不重复显示为直连，策略组只引用资源；保留旧 ID、凭据、订阅和历史流水，未修改 Agent core、公共协议或运行代码。再次审查补齐新旧节点删除接口共用引用检查、退役链路清理与不级联删除幂等收据。
- 本次交付为完整规划，尚未实施页面、API、数据库迁移或新测试。5 份文档的 92 个本地链接、源码约束、`cargo fmt --all -- --check` 和 `git diff --check` 通过；不将既有测试记录视为目标方案已经实现或双机验收通过。安装脚本等其他进行中任务保留在各自工作树。CI 继续暂停，无发布或生产部署。
- 下一步按设计的统一资源读取与页面归位、原子创建及生命周期、数据库/编译器/浏览器与专用双机验收三个阶段实施；最终验收包含整批失败回滚、并发端口、超时重试、删除后重放、共享出口及旧数据兼容。
- 原受管两跳方案规划三个实施阶段；后续订阅中间跳授权按下述四个阶段扩展，最终验收仍包含整批失败回滚、并发端口、超时重试、删除后重放、共享出口及旧数据兼容。
- 普通整合随后更新的主线 `178aab0`、`cdae763`，保留双方完整进度与服务器运营设置；相关代理节点/链路代码与核对基线逐字相同，设计原文保持。最终差异仍仅 5 份文档，94 个本地链接、fmt 和 diff 检查通过，未复演主线新增 NodeQuality 或服务器运营用例。设计评审提交为 PR #124；本任务记录放在文档前部，避免其他任务追加进度时反复冲突。

### 补充：订阅节点作为真实中间跳

- 用户进一步允许在中间段添加机场等订阅配置，扩展为有序混合路径 `A→订阅X→受管B`、`A→受管M→订阅X→受管B`。代理节点页内管理来源、URL/粘贴/上传解析及逐段选点排序，机场无需 Agent；保留多条独立监听、独立授权和订阅。新增 [来源设计](docs/chain-subscription-sources.md) 与 [ADR 0040](docs/adr/0040-mixed-chains-and-subscriptions.md)。
- 规定四种导入格式、本地转换、秘密输出与有界下载/解析，默认跟随同一明确节点或固定快照。刷新失败、缺失和歧义保留已应用旧版并显式标示，不换点或旁路；源设置 revision/身份 epoch 隔离来源更换。所有路径版本冻结外部节点和受管端点，精确依赖、候选探测及持久恢复屏障控制发布和撤旧，新旧 API 共用完整路径引用保护。
- 对照固定 sing-box 1.14.2 源码并执行九个原生配置 check：HTTP 中间→Reality、显式 DNS 和四段结构被接受；重复 tag/未知字段被拒绝；环、缺失 detour 与 HTTP→Hysteria2 也被接受，因此须独立图及下层承载校验。只执行 check，未启动代理、获取机场订阅或测三/四段流量；这些观察不等于九个功能验收通过。
- 原规划 PR #124 已合入主线；本次扩展独立提交。实施顺序更新为统一资源页面、来源与版本化编译/发布、原子创建与生命周期、数据库/浏览器/真实路径验收四阶段。YAML parser 仅作依赖决策，未修改 Cargo、运行代码或数据库；CI 继续暂停，未发布或部署。
- 工作分支普通整合主线 `a8be958`，保留 #118/#124 的源码与历史审查记录；相对该主线仅改 8 份规则/设计文档。最终三个方向独立审查、122 个本地 Markdown 链接、fmt 与 diff 检查通过；未重跑主线 NodeQuality 用例，也未把九项 check 观察记为新功能通过。

## 2026-10-01：独立服务器看板与双向跳转

- 现有主线已经包含 NodeFlare 来源的展示页，本轮在原实现上完善而非另建重复监测系统。统一入口为 `/#/dashboard`，保留 `/#/overview` 及旧详情书签，后台侧栏和服务器页使用“服务器看板”的明确跳转；保留管理员认证和返回原后台的入口，不开放匿名服务器信息。
- 新增可见页面标题、最近成功读取时间、卡片/表格切换、复合筛选、稳定排序与全屏。两种视图使用同一份服务器和拨测汇总，不逐台请求数据；真实零值、离线、待接入、指标过期、隐藏设备及读取失败分别展示。视图与排序仅保存在浏览器，不修改服务器业务配置。
- 看板专用读取器限制单次请求 12 秒、不重叠请求，暂停同时停止两个数据流，手动读取不恢复定时任务；标签隐藏取消请求，显示后立即刷新。超过 15 秒未更新或暂停/失败时不把旧状态和速率当实时。隔离测试发现浏览器在线标志会误阻止回环面板，已改为依照真实请求结果；没有放开鉴权或公网连接。
- 基于 `de29990`：Bun 单元测试 27 项通过，TypeScript/Vite 构建通过；新看板 Chromium 4 种宽度、既有展示回归 3 种宽度、资产回归 2 种宽度全部通过。包含实际键盘曲线、路由往返、暂停/单次读取、慢请求取消、全屏、读取失败/恢复和认证失效；截图目视核对，core 边界及检查器 6 项通过。
- 本轮只改变前端、构建产物与文档，没有修改 Rust、数据库迁移、Agent、代理插件或诊断执行。未重跑 Rust/PostgreSQL 全工作区，不冒用前序验证数量；未验证 Safari/Firefox、生产数据规模或线上部署。独立验收见 [服务器看板](docs/acceptance/server-dashboard.md)，保持既有 CI/发布暂停。

## 2026-10-01：公网 IP 展示与内网折叠（独立 PR）

- IP 信息页原先为所有地址逐一展开质量卡片，Docker 私网等地址占满列表。后端复用既有公网判定，增加 `public_ip_addresses` / `private_ip_addresses`；前端直接展示公网 IPv4/IPv6，将内网及其他非公网地址合并到默认折叠、可键盘展开的列表，只有内网或尚无地址时禁用公网质量刷新。
- 保留旧 `ip_addresses`、原查询上限、缓存与兼容路由，不修改 Agent 采集或查询入口。更新接口/验收文档，并使用固定 Bun 1.4.2 重建提交的 dist。
- 本地前端 17 项测试、类型检查/构建通过；真实临时 PostgreSQL/回环 HTTP 的地址分类及旧路由/缓存兼容 2 项通过，0 失败/忽略；workspace fmt、panel 全 targets Clippy（warnings 为错误）、core 门禁与差异检查通过。构建后浏览器在 1280/390 宽度通过混合、仅内网、仅公网、空地址、默认折叠、键盘展开、刷新保留展开及无横向溢出检查，均无浏览器错误。
- 验证只使用隔离数据库与模拟 API，未查询外部 IP 服务或部署到生产；完整 workspace 与远端 CI 未重跑，四个 workflow 按既有约定保持暂停。交付修复 PR，后续面板部署即可应用分组展示，无须等待 Agent 签名升级。
- 正常合并主分支 `92800dd`，保留拨测修复及双方进度记录，重新构建 dist 解决生成文件冲突。整合后复验上述 17 项前端测试、2 项数据库/API 检查、桌面/手机交互、fmt/core 门禁及 panel 全 targets Clippy，均通过。

## 2026-10-01：拨测历史归属与工具失败补修

- 审查 #99 时发现同 UUID 修改类型/地址/端口会把旧样本误标为新方向。创建后固定三字段，PATCH 在锁定原行的事务内拒绝变化并提示新建目标；名称、备注、间隔与启用状态仍可改。旧结果、摘要和离线样本身份保留，不清历史；前端编辑锁定三字段。
- Unix ping 完整收发汇总也可能包含本地发送失败。移除隐藏发送错误的 quiet 参数，stderr 工具诊断返回不可用并保留有界原因；真实无响应、无工具诊断的 100% 丢包仍支持。新增权限/发送失败与历史/离线重传回归，保留 TCP 创建任务的冻结原字节判据。
- 受验源码 `d308aee` 在本聊天 macOS ARM64/独立 PostgreSQL 16 完整 Rust 回归 404 通过、0 失败、14 项既有条件忽略；macOS umask077 原子链接专项、workspace 全 targets Clippy（warnings 为错误）、fmt/core 门禁通过。TEST_ONLY 编译根不用于生产发布。
- Bun 1.4.2 冻结安装、17 项/771 断言与 TypeScript/Vite 构建通过，重建 dist。实际 Chromium 展示页 1440/390/320 与深浅主题、原 TCP/NodeQuality/full 门禁、业务与确认取消伴随夹具全部通过；展示读取无写请求/页面错误，目视核对桌面和320px截图。首次 Bun 子命令 PATH 与 Playwright 预设浏览器版本不匹配只属本地环境失败，显式选用已有 runtime/browser 后通过，原失败日志保留。
- #99 作者推进到 `98fd54f` 并先行合并，本项保留其祖先及 #97/#98，另建补修 PR。四个 workflow 保持暂停；未执行跨平台/公网 ICMP、真实现代协议或 ACME 运行时、生产部署与远端 CI，不把作者历史实机结果当作本聊天复演。

## 2026-10-01：补齐 NodeQuality ARM rootfs 静态证据（Issue #82 独立文档 PR）

- [ARM 独立验收](docs/acceptance/nodequality-rootfs-arm-inventory.md)：先刷新固定 asset 345687500，单次取得 BenchOs-arm.tar.gz，实读 359,657,375 字节及 SHA256 与固定元数据一致，无下载重试。禁网、nonroot、只读、512 MiB/1 CPU 容器仅流式分析，保留读取边界并增加外层 120 秒硬截止和 finally 清理；不解压执行或读取原始配置。
- 读回 Debian 12 arm64、364 个已安装包、372 份普通许可/版权文本和 30,034 个 tar 成员。speedtest 与 nexttrace 的 AArch64 ELF 摘要单独记录；前者仍缺可核对的包来源和再分发证明，后者与既有固定 v1.3.7 观察摘要一致，不推定签名或可复建来源。没有名为 Geekbench 的成员/包，不能免除在线 Geekbench 5.5.1。
- 主扫描 5.798 秒、容器退出 0/未 OOM，安全状态读回符合，own 容器 stop/rm 成功。386 文件索引独立重算全部匹配；选中文件索引无配置路径、无非 ELF 工具内容。120 秒硬截止未触发，不记作超时故障回归通过。
- 文档与链接/差异相称检查单独记录；不改产品或制品，不触发 CI、签名、发布或部署。许可文本不等于授权，#28/#65/#66/#82 和 full 启动门禁继续保留。


## 2026-10-01：补齐延迟、丢包展示和 Agent 持续检测

- 服务器展示卡片新增按线路的延迟、ICMP 丢包率或 TCP 连接失败率及近期色条；详情支持按目标切换 1/6/24 小时双曲线。后台“服务器”的刷新按钮旁新增“服务器展示”，跳转 `/#/overview`。无配置、等待采样、暂停、过期、读取失败、真实零值和检测不可用分别展示，不创建默认公网拨测。
- Agent 在 Linux（iputils/BusyBox）、macOS、FreeBSD 使用固定语言环境和完整 ping 收发/均值汇总，处理重复/迟到回复及完整无响应，补齐 macOS IPv6 总时限；Windows 改用 .NET Ping 输出 JSON，避免本地化/编码造成误判。错误记录沿用原协议，前后展示不再将其数字占位视为 100% 实测丢包；TCP 明确标注连接失败率。
- 调度保持最多四路并发，完成的测量立即入账，慢任务不会阻塞整组；按到期顺序选目标、不补跑积压轮次，配置修改/暂停取消已有测量。新增整轮 12 秒期限，并修正持久化缓存使用面板时钟偏移清理。重启、部分确认、过期清理和取消释放退役门锁已覆盖。
- 面板新增认证批量卡片汇总，每目标最多 20 条；新增 0015 索引及按目标 24 小时最多 8641 条历史读取，避免原全局 4096 条截断多目标数据。无新增依赖、匿名接口或生产探测。用法、平台工具需求、测量语义和人工验收见 [服务器展示页](docs/server-display.md)。
- 本地完整 `cargo test` 385 通过、13 条条件忽略；另行运行被忽略的真实 ICMP 回环用例，IPv4/IPv6 均通过。Clippy 全 targets（warnings 为错误）、fmt、core 边界、差异检查通过；真实 PostgreSQL 集成包含登录/设备权限、按服务器隔离、逐目标条数、5000 条历史、重放确认和删除目标。
- Bun 1.4.2 构建通过并同步 dist，17 项前端单测 / 771 断言通过；最终产物的 Chromium 1440/390/320 像素和深浅主题检查覆盖卡片色条、100% 丢包、工具异常空缺、时间/目标切换、刷新错误恢复、展示按钮、登录权限和后台样式。浏览器错误、展示写请求均为零，无整页横向溢出。
- 未验证范围：macOS/FreeBSD/Windows 实机 ICMP、真实公网链路、生产部署、长期驻留和远端 CI；跨平台仅有参数/解析夹具覆盖，不宣称实机验收。CI 按当前用户安排继续暂停，本次提交标注 `[skip ci]`。下一步在专用测试机升级 Agent、配置目标并逐平台核对实际读数。

## 2026-10-01：复刻 NodeFlare 服务器展示界面

- 按用户澄清仅实现服务器信息展示，新增登录后的默认 `/#/overview`、卡片详情 `/#/overview/<ID>` 和后台返回入口。现有后台页面、中文登录、代理业务及服务管理 API 保持；展示样式与深浅主题限定在独立容器内，不新增匿名读取权限。
- 复刻 NodeFlare 的顶部汇总、全宽卡片、进度条、主题和详情布局，接入真实服务器状态、系统指标、网卡计数、资源历史和已有拨测结果。搜索与状态筛选、曲线图例、鼠标/键盘读数、移动布局可用。只读取数据，不创建任务；无成本、配额、地区等数据时不编造。保留来源说明和 MIT 许可，未增加运行依赖。
- 在线与指标新鲜度分开，缺失值不补零，离线/过期/未知时间/刷新失败不计入实时速率。图表保留采样断线与极值；增量重叠读取，每分钟补齐所选窗口。资源范围限定现有两小时，拨测范围保留接口条数限制；错误、未知与空列表分别展示。用原生 SVG 自适应绘图区，窄屏不缩小刻度字体。
- 最终 Bun 1.4.2 / TypeScript / Vite 构建通过并同步 `web/dist`，14 项前端测试、749 断言通过。实际产物在 Chromium 的 1440/390/320 像素、深浅主题下通过搜索筛选、未知/真实零值、历史范围、图表键盘交互、主题保存、403 恢复、拨测读取失败、404、空列表与登录失效检查，浏览器错误和展示页写请求均为零，无整页横向溢出。切回后台的样式及登录权限检查通过；已有 sing-box 业务与 TCP 报告页面的桌面/手机回归通过。
- 临时 PostgreSQL 下真实 HTTP 静态资源集成测试 1 项通过；全 targets Clippy（warnings 为错误）、fmt、core 边界和差异检查通过。本轮没有重跑完整 Rust workspace 或生产 Agent 实机验收，未触发/恢复远端 CI。使用及验证说明见 [服务器展示页](docs/server-display.md)。下一步可在部署升级后核对实际设备数据与所用浏览器。

## 2026-10-01：现代代理协议与自动证书

- 按用户授权完成 Hysteria2、Shadowsocks 2022（AES-128/256）、TUIC v5、AnyTLS、Naive（HTTP/2，含 UDP over TCP）、Snell v6。新增协议模型、0014 兼容迁移、节点 API/界面、独立授权密码、确定性配置与完整 JSON 订阅。旧 Reality 配置字节、UUID、订阅路径/令牌、授权及历史用量保留。链接格式仍仅支持 Reality，混合协议请求显式提示使用 JSON。
- TLS 支持手动 PEM 和 Let's Encrypt 自动申请、续期（HTTP-01/TLS-ALPN-01）。同机提供器合并域名；共享邮箱和验证方式可原子更新，端口冲突整体回滚。证书保存在已有 data/certificates，重载/回滚不删除。列表不回显证书私钥、节点 PSK 或授权密码；手动更新可保留旧证书，替换需成对匹配。
- 适配器增加 TCP/UDP 识别和验证证书的 TLS/QUIC 握手，避免 ACME 异步签发期间提前报告健康。TUIC/HY2 使用 h3；AnyTLS 健康检查遵循实际 ALPN，已修复 TLS-ALPN 验证启用后的握手冲突。首次签发限 240 秒；SDK/core 只增加通用有界健康预算，超时仍恢复旧配置，未越过插件业务边界。
- 本地验证：完整 `cargo test` 376 通过、12 条条件忽略；最终改动另复验编译器 13 项、数据库相关 6 项、对账 10 项（含新增等待超时回滚）及适配器 13 项（含真实累计统计与重载）通过。`cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、core 边界检查通过。前端 9 项测试通过，Bun 1.4.2 构建通过并同步 dist；Python 112 项运行、106 通过、6 条已有条件跳过（临时 minisign 0.12 补齐签名工具）。
- 真实运行时：本地从未修改的固定 sing-box v1.14.2 源码构建，编译器 13 项含原生 check 全部通过。新协议实测覆盖七种配置（SS2022 两种密钥长度）的 TCP/UDP 传输、用户计量隔离及重载撤销；HY2/AnyTLS 验证错误证书域名不会通过健康检查。Naive 使用实际 Cronet 出站，不以配置检查代替连接测试。
- ACME 实测使用临时 Pebble CA/DNS，四种 TLS 协议共享证书，真实执行 HTTP-01 和 TLS-ALPN-01，验证证书持久复用及 60 秒短期证书到期后自动续期和重载恢复；未向公网 CA 请求证书，未修改系统信任库。可复现入口为 `tools/acme-smoke.py`。
- 未验证范围：公网 Let's Encrypt 与真实 DNS/端口权限、长期驻留续期、macOS/FreeBSD/Windows 实机运行、生产部署与最终整合 CI。当前 CI 继续暂停，未触发或恢复，也不声明 main 全绿。使用方式和人工验收见 [协议与证书](docs/proxy-protocols.md)、[ADR 0034](docs/adr/0034-modern-protocols-and-certificates.md)。下一步可在专用测试机升级面板/Agent 后逐项实机验收。
- 推送前正常合并远端 `8657df0`，保留 Agent 独立版本修复及 NodeQuality 证据；仅进度文档存在冲突，双方记录均保留。合并后全 targets Clippy、fmt、core 边界和差异检查通过；升级、状态、退役、服务和对账共 27 项 Rust 定向回归，以及 8 项构建脚本回归全部通过。本次没有重跑完整 workspace、运行时实机或远端 CI。

## 2026-10-01：实测专用虚拟机端口隔离修正

- 独立补验 PR #91 合并审查中的 `guestIPMustBeZero: false`。构建结束后只停止、编辑并重启本任务实例一次，保留磁盘与旧证据；修正模板和实际配置摘要、新 boot 与管理 SSH 身份分别记录在[虚拟机验收](docs/acceptance/p0-dedicated-vm.md)。
- guest 回环/全接口两个有限 HTTP 监听均可在 guest 内访问；宿主三轮六次连接全部拒绝，六次监听检查均无对应端口。夹具正常退出，无进程、监听、准备文件或共享目录残留；UDP 仅启动日志确认关闭，未做线路实测。
- 本项只有文档与实际隔离补验，不修改 Agent/诊断代码，不复用原六夹具来认证新 boot，不签收完整 NodeQuality。新 Agent 联合负载另行记录，远端 CI 继续暂停。

## 2026-10-01：Agent 0.3.1 版本准备（独立 PR）

- 磁盘修复 #93 已合入 `main` 的 `d29fd1c`，该次与最新进程保护的整合验证为 core 185 通过、0 失败、7 项既有实机条件忽略。基于该主线将 Agent 独立版本及 Cargo.lock 更新为 `0.3.1`；面板 workspace 仍为 `0.3.0`，发布文档区分源码候选与已发布的 `agent-v0.3.0`。
- 限额 Linux 容器使用公开 TEST_ONLY 编译根，`cargo test --locked --offline -p sinan-agent` 的 2 项集成测试通过，Agent 构建及实际 `--version` 精确输出 `sinan-agent 0.3.1`；workspace fmt、Agent 全 targets Clippy（warnings 为错误）、core 分层门禁与差异检查通过。测试二进制仅保存在本地验证目录，不作为生产候选发布；未执行多平台或线上升级验收。
- 此次交付范围为源码版本更新 PR，不执行签名、Release 发布或线上部署。正式私钥留在维护者 Mac，当前 VPS 仅执行本地构建检查；四个 GitHub workflow 继续按约定暂停，不以本地结果代替发布 CI。

## 2026-10-01：准备独立 Debian 12 P0 测试虚拟机

- 单独保存 [固定 Lima/VZ 配置](tools/p0-debian12-vm.yaml) 与 [实际隔离及验收范围](docs/acceptance/p0-dedicated-vm.md)。2 CPU、1536 MiB、8 GiB 的 Debian 12 ARM64 guest 已实际启动，固定镜像 341,114,880 字节的 SHA512 校验一致；不共享宿主目录、管理密钥或业务端口，不执行生产硬件压测。
- 实际读回 systemd running、cgroup v2、启动时约 1.3 GiB 可用内存与 6.4 GiB 根盘剩余、无启用 swap。管理 SSH loopback 绑定、重启数 0。安装仅限 guest 编译依赖和校验过摘要的 Rust 1.97.1，构建单元限 1100 MiB/无 swap/128 tasks/一个 Cargo 编译任务。
- 源码冻结 `356350e` 的 ARM64 ELF 实际串行运行六个 systemd 夹具，6 通过、0 失败/忽略、2.30 秒；预算 OOM 仅在诊断 cgroup，结束无残留单元/进程/挂载、无手动补清理。另 16 MiB 私有 tmpfs 实测 ENOSPC 后完整卸载；SSH PID/重启数保持。这些证据不认证后续主线、不证明 Agent 预检整链或持续代理服务。
- 专用 aws-jp0 仍需恢复，完整 NodeQuality 授权与受控执行链门禁保持。真实 systemd 有限夹具、小文件系统、实际 Agent 联合负载分别记录，不把准备 VM 或旧 CI 当阶段总验；本项不恢复暂停的远端 CI。
- 合并审查补齐端口忽略规则的 `guestIPMustBeZero: false`，覆盖回环监听；Lima 2.2.0 配置校验、文档链接、fmt/core 和差异检查通过。原运行证据不转记为修订规则的运行期端口验收，本轮未启动或重启既有 guest，应用后实测单独跟进。

## 2026-10-01：补齐 NodeQuality amd64 rootfs 证据

- 对应 [Issue #82](https://github.com/theLucius7/sinan/issues/82)，单独记录 [归档静态盘点](docs/acceptance/nodequality-rootfs-inventory.md)。单次取得固定发布资产，312,475,959 字节及 SHA256 与发布元数据一致；禁网、只读、512 MiB/1 CPU 分析容器仅流式读取，没有解压执行、挂载或运行诊断。
- 实际读回 Debian 12、375 个已安装包记录和 385 份许可文本；预置 Ookla ELF 有 1.2.0.84 静态标记，却缺对应包来源及可核对的再分发证明。三个精确许可/隐私配置字段均为未知，不公开原始配置。没有发现名为 Geekbench 的归档成员，不能免除内层在线 Geekbench 5.5.1；审计文档改为引用对应主版本的官方条款。
- 原始证据索引 13 文件摘要全部匹配；ARM、完整来源/授权、零上传与宿主零改动仍未验。执行门禁保持，#28/#65/#66/#82 不关闭，源码与制品未改。仓库暂停远端 CI，本项只做相称文档验证，不计为完整执行或专用节点总验。

## 2026-10-01：按用户要求暂时关闭 CI

- 暂停仓库四个 GitHub Actions 工作流并取消正在运行或排队的检查，避免分支 push、PR 和 main 整合反复构建。保留全部工作流文件及既有历史结果，代码与文档工作继续使用相称的本地验证。
- 等所有进行中的任务完成后，再统一恢复工作流、验证最终整合提交；单个 PR 完成不提前恢复。此次被取消及尚未运行的最终提交 CI 保持未验证，不算通过。
- 暂停后合入的 PR #84 在 `ci.yml` 重新纳入全平台检查定义；本轮保留其源码和修复，不擅自恢复工作流。恢复时使用精简自动矩阵还是全平台自动矩阵，待统一确认；第 0 阶段的历史绿色记录不用于认证当前主线。

- 暂停期间逐项复核六阶段交付，更新部署/发布文档中已过时的“r2 旧草稿”描述，并明确签名由维护者本机生成。已公开的冻结 `75cd846` 与当前 r5 源码及后续能力验收分别记录。重新读取归档的真实升级/计量证据并保存脱敏摘要；仅下载原成功 CI 的已有公开验收产物以持久保留，未触发、重跑或恢复 CI。文档部分通过差异及链接检查；发现的版本独立问题另按下节修复。

## 2026-10-01：补齐 Agent 安装、状态与升级的独立版本

- [Issue #85](https://github.com/theLucius7/sinan/issues/85)：Agent 虽已独立声明版本，自动升级、状态快照和原生安装仍误用随面板演进的 core 包版本；两者同为 0.3.0 时掩盖了问题。面板/core 高于 Agent 时会误拒绝有效更新，状态错位还会阻止监督器确认健康候选。
- Agent 入口的自身版本现传入自动升级与原生安装，状态快照使用同一 Runtime 版本；安装目录、签名版本定位和启动确认保持一致。监督器仍按已签候选与 current/pending 版本校验，不把父进程版本当作候选版本；签名验证、失败版本抑制、回退与退役保护保持。
- macOS arm64 / Rust 1.97.1 使用公开 TEST_ONLY 根验证：旧升级比较和状态代码在两个错位升级用例、一个真实状态 socket 用例中失败；修复后升级 8 项、状态 2 项、worker 退役 1 项全部通过。core 与 Agent 全 targets Clippy（warnings 为错误）、fmt、core 分层及差异检查通过。未执行原生系统服务安装、全 workspace 测试或 GitHub CI；最终平台/整合 CI 待所有任务完成后统一恢复，既有正式 Release 保持不变。
- 预编译 `--binary` 打包及 FreeBSD 工作流的预期版本改读 Agent 独立 manifest；构建脚本帮助同步。新增 workspace 9.8.7 / Agent 1.2.3 的真实命令夹具、打包与摘要检查，以及 FreeBSD 内嵌 Python 输出回归；旧代码三项失败，修复后构建脚本全部 8 项通过。架构头检查由既有测试覆盖，本次版本夹具不冒充真实 FreeBSD 二进制。Python/Bash 语法及差异检查通过；本机未安装 actionlint，工作流整体静态检查与平台执行继续待统一验证。
- 正常整合主线 `3f65b42`，保留原生平台修复及其矩阵定义；自动与手动 FreeBSD 两处版本读取均已覆盖。整合后原 11 项及 macOS 严格 umask 原子软链接 1 项通过；另发现上游服务夹具丢失锁目录特判，导致 dead_code 和两项服务回归失败。仅恢复测试替身的 root/0700、准备计数、stat、私有 umask/flock 断言，保留新目录记录，6 项服务回归通过。相关 Rust 去重共 18 项、构建工具 8 项通过，最终定向 Clippy 与 fmt 通过，CI 继续关闭。

## 2026-10-01：合并回归与公共证据复核

- PR #87 原始 `ab2b37c` 的完整本地 Rust/PostgreSQL 回归 371 项通过、0 失败、9 项既有条件忽略；macOS `umask 077` 原子软链接回归及 workspace 全 targets Clippy 通过。保留 #88 独立 Agent 版本与 #86 静态证据后，`8657df0` 的 35 个不同相关 Rust 用例、8 项构建脚本、16 项 Reality 取证与 21 项验收驱动通过，fmt/core/四份 workflow actionlint 通过。误选的两项 PostgreSQL 用例首次因本任务实例已停止无法连接；启动 55432 专用实例后同回归通过，未改源码或放宽测试。最终未重复完整 workspace；本任务实例已停止。
- PR #89 正常保留作者 `be18e4f` 及最新主线，30 项 IP 质量库专项在专用 PostgreSQL/真实回环 HTTP 下通过，workspace 全 targets Clippy、fmt/core、五份受验源摘要和文档链接核对通过。整合只解决进度文档追加冲突，IP 源字节保持；不使用正式账户或把本地结果当作专用节点签收。
- 现有公共摘要回归进一步核对 Actions 注解与白名单 JSON 一致，并确认私密占位字段不进入两者；新夹具在旧 `3f65b42` 上准确检测缺失的传输证据。公开 Release 验签及历史 CI 的适用提交范围下文单独记录，四个工作流继续暂停，不声明当前 main 全绿或各平台实机通过。

## 2026-10-01：Release 发布身份保留整合

- 保留作者 `504bae9`，正常合入正式主线 `2fa405b`。最终发布 PATCH 显式携带已核对标签及完整构建提交，发布前后身份、完整签名、资产摘要和精确主线 CI 门禁保持；当前 TCP 可选模块必须双架构的规则保留。
- 发布24、旧release28、原生来源/签名17、core6，共75项通过/0失败/4既有条件跳过。旧 draft-only 函数在同一标签漂移夹具确实失败，修复通过；五种发布后并发改变仍拒绝报告成功，不声称原子发布或自动回滚。
- 原生三项失败实际来自旧作者对象缺13个TCP源码/配方路径；正常合入最新main后原归档门禁验证通过，旧对象仍拒绝，不放宽来源要求。仅Python测试与源码归档检查，无本机Linux原生构建、正式签名或发布；最终head的两套CI单独跟进。见 [独立验收](docs/acceptance/release-publication-identity.md)。

## 2026-10-01：按前置阶段验收推进整改

- 将 P0 保护服务器、P0 IP 查询、P1 sing-box 边界、P1 共用诊断框架、P2 TCP 接入的顺序写入 AGENTS 与[独立验收状态表](docs/acceptance/ordered-remediation.md)。本项只记录验收与推进条件，不修改菜单、API、Agent 或诊断工具。
- P0 专用节点总验仍未通过。已有基线确认内核 OOM，最近记录的只读 SSH 检查仍在 banner 阶段超时；已记录的 447 MiB 物理内存不足以满足 512 MiB 完整任务预算和 256 MiB 启动预留，恢复后须重新读取实际资源。上游完整执行链的上传、宿主 swap 改动与制品授权仍待整改，继续维持完整任务门禁。
- TCP 后端 #77 与报告页 #78 按独立源码审查及相称验证整合；代码可合入准备，实机能力签收与正式发布/部署仍受前置阶段门禁约束。已有独立 CI、真实 systemd、Reality 和浏览器结果保留；这些证据不替代 NodeQuality 在专用节点持续代理流量下的心跳与取消清理总验，也不证明登记后的原生 TCP 签名安装、实际启动、重连恢复和取消清理整链已实机通过。
- 故障矩阵区分回环服务/数据库、真实 CI systemd、包装器替身和专用节点负载。未执行项不计通过，不以跳过项或恢复前的服务状态证明当前节点健康。恢复后按状态表先完成 P0 再推进后续验收。
- 本轮 66 个文档链接、core 分层、`cargo fmt --check --all` 与差异检查通过；另核对引用的 #54/#55/#74 原 CI 日志，其 Rust 与真实 systemd 合计分别为 314、330、371 项通过、0 失败、9 条件忽略，与表中分开记录的数量一致。本项未编译或测试 Rust、启停服务或执行实机诊断，不把历史 CI 转记为最终整合提交的检查。
- 本轮正常整合 #76 主线 `b152e2a`、#77 主线 `21d8e71` 与 #78 主线 `2bd017a`，保留三项的源码、签名来源验证、登记生命周期和最终 `index-hb4JwTbg.js` 报告页。相对 `2bd017a` 只更改 AGENTS、PROGRESS 与本项验收文档，运行代码、工具、测试和 dist 全部原字节保留；最终整合提交 CI 单独核对，源码合入不补签前置实机验收。

## 2026-10-01：TCP 诊断面板登记（独立 PR）

登记第二诊断插件并由 Linux Agent 加载无状态适配器，共用任务服务负责互斥、预算、上传、确认取消和历史。仅开放地区、IPv4/6、4/8 次连接和1/2并发，冻结已配置 TCP 目标及摘要，地区标签在插件独立表保存；空 PATCH 拒绝、显式 null 才清除。前端另一个独立 PR。

3963c28 组合源码的 PostgreSQL/API 3项、面板参数/目标2项、适配器15项与原生17项全部通过，workspace全targets Clippy和Linux Agent构建通过，限额容器exit0/OOM=false。fe4ae60平铺后显式地区键/API3+unit2、Clippy和Agentbuild再次通过。已正常合入根sing-box插件主线8ef465f；工具版本pin改为版权补齐后公开且实际Bookworm/五aux签名验收通过的b562effcd90f8ae319665fb4ead1807b770ed4d5，1c640d3的新pin/API3+unit2/fmt/Clippy/Agentbuild再次通过，exit0/OOM=false；最终CI另核。没有以夹具替代真实测试机验收，aws-jp0仍待恢复，完整验机工具链仍被安全门禁暂停。见 docs/acceptance/tcpquality-panel-registration.md。


## 2026-10-01：TCP 报告界面（独立 PR）

服务器导航新增 TCP 连接诊断，四项小预设及配置目标地区，按本次冻结范围显示工具版本/时间/参数、连接成功统计和独立章节；未知不补零，取消确认前保留屏障，部分报告可看，不做跨参数排名。主线管理员与 sing-box 插件导航继续保留，NodeQuality 完整门禁不由本 PR 改动。

012ca9f 的 Bun/TypeScript/Vite 与 Chromium1280/390px夹具验收通过，零页面错误、正确创建/地区请求和取消禁用均已核实；最终主线整合后重新构建与浏览器复验另补。验收范围与真实节点待办见 docs/acceptance/tcpquality-report-view.md。

- 在面板 fe4ae60 与公开制品主线 9a41fe5 上重整源代码，NodeQuality门禁及管理员/插件导航保留；9dbff20 的 Bun frozen install、5项/711断言、TypeScript/Vite和最终dist Chromium1280/390px复验均通过，零页面错误、每宽度一条白名单创建/地区PATCH，未知/真实0/部分/旧报告过滤/取消屏障成立。实际dist index-D5k-FUiH.js，独立PR最终CI另跟。

- fa8dca4保留完整主报告统计，部分章仅能补更完整/更多连接样本的结果；目标缺数据显式未知。持久化浏览器夹具在实际index-CkSqyBT0.js的1280/390px通过，完整报告不降级、目标403/离线禁止创建以及原部分/真实0/未知/互斥取消都成立，页面错误0。旧dist被完整结果反对照准确抓住，测试脚本ASI错误修复后实跑通过；没有把这些夹具称为真实节点验收。

- 本轮在作者233840a上修复成功读取后诊断/目标轮询403仍复用旧能力与目标范围的问题：未知期间暂停创建并保留历史；旧dist被新增夹具抓住（exit1）。Bun5项/711断言、TypeScript/Vite52模块及最终实际index-hb4JwTbg.js的Chromium1280/390px全部通过，真实零耗时、冻结地区/参数、部分章不覆盖complete、省略插件的旧NQ报告过滤、确认取消、轮询403禁创建/范围未知、历史保留、读取恢复和离线均成立，报告说明明确为不向第三方上传。相同dist的NQ完整门禁及sing-box业务导航伴随验证通过，完整模式POST为零。复用匹配锁文件的已有依赖，无Cargo/PG/真实节点或生产接口；专用节点TCP登记整链仍未验，验收记录已保留边界。

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

## 正式发布：agent-v0.3.0 已公开，真实面板导入通过

- [agent-v0.3.0](https://github.com/theLucius7/sinan/releases/tag/agent-v0.3.0) 已正式发布，源码固定为 `75cd846f152f61d7b5daa913b31c74579eed3d22`。[该源码的 main CI](https://github.com/theLucius7/sinan/actions/runs/36770621157) 五项通过；[发布工作流 36802878940](https://github.com/theLucius7/sinan/actions/runs/36802878940) 核验维护者本机提供的生产签名、校验全部资产后公开发布。该工作流的构建任务为条件跳过，不持有私钥或执行正式签署。发布后匿名访问以及十项资产的身份、大小、摘要复核通过，原有九项产物未替换，仅追加维护者本机签出的 `SHA256SUMS.minisig`。
- 生产公钥 ID `44B019C8269669B8` 与仓库、Actions 构建变量一致，已编译进发布二进制。使用冻结源码及同一生产根构建隔离面板，从真实公开 Release 完成两次 API 导入和一次浏览器按钮导入：Agent 0.3.0、sing-box 1.14.2、NodeQuality r2 的 amd64/arm64 六项齐全。独立 minisign、完整安装器/制品摘要、ELF 架构、无残留 staging、重复导入幂等、发布身份不变全部通过；实际页面显示六行与签名已验证，导入 POST 返回 200，后续库存读取和截图已核对，测试会话已退出。
- 带口令私钥仍由维护者保存在 Mac 的仓库外，离线保管尚未完成；签署与发布成功不代表离线保管完成。项目与 CI 仅使用公钥和签名，私钥未上传到仓库、服务器及 CI。
- 本轮只读复核匿名公开 Release 身份、十项资产元数据、三份元数据文件的实际摘要及仓库生产公钥下的独立 minisign 验签，确认签名清单中的 release.json 包含上述六项；源 `75cd846` 的五项 main CI 和仅验签发布 workflow 也已核对。未执行下载二进制、访问私钥或重新运行真实导入。
- 导入结果为维护者记录的冻结源码隔离面板验收；本轮未重新发送导入 POST。本次证据只覆盖冻结源码 `75cd846` 的发布和制品导入，不认证后续 main 新增能力，也不补签 P0 专用节点负载、心跳、取消清理或 TCP 整链实机验收。旧成品不包含后续 main 的完整任务暂停门禁；签名认证成品身份与完整性，不能作为上游运行时下载、公开上传或宿主修改已受控的证明。
- [Issue #80](https://github.com/theLucius7/sinan/issues/80) 记录更新草稿正文时观察到的 tag 身份变化；发布器 PATCH 显式携带已验证的 tag 与构建提交，并继续核对发布前后全部身份和资产。发布工具回归 24 项通过，覆盖省略 tag 被重分配，以及 PATCH 后发布 ID、tag、构建提交、Git tag 和资产 ID 被并发修改时拒绝成功。Release 回归 28 项通过、4 项既有 Linux/root 条件跳过；已正常整合 main `2fa405b`，最终提交 CI 单独核对。
- 历史记录：main `be8b792` 合入部署条件检查后，[CI 36746601899](https://github.com/theLucius7/sinan/actions/runs/36746601899) 的构建、检查、Compose 和真实 Reality/计量部分通过，但末尾退役验收误把 systemd 条件跳过的零退出码当作失败。后续修复同时核对条件结果、服务状态、进程与启动时间；本次发布采用上述 `75cd846` 的完整成功门禁。

## 新合入平台能力的整合修复

- 本次合并整合时先通过完整 workspace Rust/PostgreSQL 测试：230 项成功、5 项平台条件忽略；随后整合作者最新队列修复，并增加升级预检、旧进程停机及候选启动三处退出 78 的终止回归。最终升级专项 6 项、系统专项 11 项通过（3 项真实 systemd 在 macOS 忽略），workspace 全 targets Clippy、fmt、前端构建与 actionlint 通过。
- Windows CI 的签名夹具文本写入会将 LF 改为 CRLF，导致实际 Agent 拒绝证明；现改为精确 UTF-8 字节写入。新增模拟 Windows 文本 I/O 的回归，旧实现负对照失败，新实现通过真实 minisign 与 Agent 验证；Python discovery 71 项、66 成功、5 项既有条件忽略。最终 Linux/OpenRC/Reality 与原生服务验收继续以新提交 CI 为准。

- PR #13 的 [CI 36749216636](https://github.com/theLucius7/sinan/actions/runs/36749216636) 五项通过，包含 Reality 443、签名拒绝、重启/HUP 后精确两倍用量与在线退役。之后 main 合入 PR #10；其 [CI 36750812350](https://github.com/theLucius7/sinan/actions/runs/36750812350) 的 musl jobs 在 OpenRC 夹具校验公钥目录所有权时失败，旧提交的成功状态不能认证新源码。
- 历史安排曾将自动 CI 恢复为 check、Compose、musl amd64/arm64 和真实 Reality 验收；当前以文首 CI 暂停要求为准。其余平台的完整构建与服务 smoke 保留在手动 `platforms.yml`。OpenRC 仅将公开测试根复制到容器内受保护目录，不改变宿主源码所有权，也不放宽正式安装器检查。
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

## 交付加固第 3 阶段：实现及正式发布完成，私钥离线保管待完成

- 先采纳 ADR 0017，按用户决定使用 minisign-verify、构建时多个公钥和独立产品版本。Agent 为 0.3.0，面板仍为 0.2.0，协议范围为 1..=1；公钥不由面板或安装脚本向 Agent 下发，生产命令没有运行时换根开关。
- protocol 的完整四行签名、canonical 清单和 metadata 绑定通过 14 项测试；包括正文与 trusted comment 篡改、多根与真实测试根轮换。core 下载、缓存、准备、应用、同 revision、回滚、未完成事务恢复及诊断启动验证实际二进制与签名证明；70 项单元测试通过，1 项真实 Linux/systemd 专项仍由 Linux CI 验证。CLI 和 systemd 预检绑定期望的制品角色及格式，不能用另一种已签制品替换执行目标。
- 新增只读 verify-cache 升级预检，检查已应用、未完成 target/previous、诊断检查点和无数据库 current。主库与活动 WAL 字节保持测试通过；旧未签缓存不能自动认可。GNU mv、systemd ExecStartPre 与实际签名安装路径已由本阶段 Ubuntu 24.04 CI 验收。
- 面板固定官方 GitHub 仓库，从 tag 导入整个签名 Release；校验完整内容后单次发布目录。6 项存储测试通过：失败原子性、篡改拒绝、幂等、组件不可覆盖、兼容 Agent 选择、软链和缺信任根拒绝。HTTP 网络阶段也受并发限额约束；Agent 仍从配置的面板同源下载并独立验签。
- 前端新增 Release 导入和显式 Agent 版本选择，生产 Bun 构建通过；空制品列表不显示已验证徽章。全 workspace fmt、全 targets Clippy（warnings 为错误）与测试通过：145 项成功，4 项依赖真实上游运行时或 Linux/systemd 的专项默认忽略。签名夹具覆盖面板诊断、真实传输、丢 ACK 恢复和 bootstrap/鉴权。
- 最终共享树在隔离 Debian 12 容器使用真实 minisign 0.11 执行 47 项 Python 测试，全部通过且无跳过。审查发现并修复安装器依赖 Python assert 的缺口：现在使用隔离 Python 与显式长度、SHA-256 拒绝逻辑，任何新二进制执行前完成独立校验；优化模式下同长度篡改、超出已签长度的流及下载重定向均拒绝，合法签名安装仍通过。正式面板来源使用 HTTPS，仅明确回环地址允许 HTTP。
- 发布候选流程构建双架构 Agent、运行时及诊断制品，输出 metadata 和 SHA256SUMS，先创建 draft。用户在仓库外本机生成带口令私钥、只提供公钥、本地签署并上传 minisig；CI 不取得生产私钥。正式发布要求全资产验签和对应 main 必需 CI，已知测试根在正式流程中拒绝。本阶段实现时尚无正式公钥、正式签名或正式 Release；最新公钥及发布状态见本文顶部，测试根验收不替代生产签名。
- 发布流程在缓存恢复或新构建后，以归档、ELF 和 Go metadata 检查两种架构、固定源码 revision、工具链及构建标签；检查不执行缓存二进制。此信息用于发现错误产物，不作为独立构建证明。12 项验收驱动、8 项签名 CI 契约、3 项既有缓存契约及 actionlint、Shell/Python 语法检查通过。
- [PR #11](https://github.com/theLucius7/sinan/pull/11) 已合入 main（`20d09ca`）。[PR CI](https://github.com/theLucius7/sinan/actions/runs/36738530095) 的 5 项全部通过，包括真实签名安装、篡改二进制/证明/旧未签缓存拒绝、恢复 systemd 验签器、Reality 双向流量、重启、HUP、精确计量和同版重装。首轮上传/下载为 1,048,821/2,097,454 字节，重载后相同流量累积精确为两倍。使用 TEST_ONLY 根，不能替代正式发布签名。
- 历史记录：[main CI](https://github.com/theLucius7/sinan/actions/runs/36740903057) 的全部 5 项也已通过；当时正式公钥与本地签署仍待用户完成。现已完成正式签署、发布与真实导入，证据及尚未完成的私钥离线保管见上文正式发布记录。

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

## 2026-10-01：修复签名与多平台 CI 的整合失败

以下平台跟进小节记录当时列出的 fork 提交与 CI，不认证当前 main。当前四个工作流按用户要求暂停；历史记录中的下一轮 CI 待所有进行中的任务完成后统一安排，不提前触发或恢复。

- 在 `main` 快进到上游 `45df3b1` 后修复，保留上游签名、设备退役、服务优先级和诊断基线。上游原 PR 已合并，此后只提交针对新基线的修复差异。
- Windows 的已签证明改用原始字节保存，避免文本写入自动转换 CRLF；正式清单与安装器渲染也保留 LF。新增模拟 Windows 文本模式的回归，独立 minisign 验证正确签名并拒绝篡改，不放宽校验边界。
- Windows CI 使用基于 protocol 现有测试支持的 Rust TEST_ONLY 签名 example，避免 cryptography ARM64 wheel 缺失后的 OpenSSL 本机构建；无新增依赖、无正式私钥输入。Linux OpenRC 与 Agent 行为步骤也保留公开失败摘要，便于后续定位。
- 隔离 Alpine 复现普通运行账户执行代理时的 `Permission denied`，确认签名安装器新建的共享父目录受 umask 影响成为 0750。显式设置父目录可遍历权限后，真实 OpenRC 安装、重装、失败恢复、HUP、非 root 低端口能力、独立运行、default runlevel 和 systemd 安装命令契约均通过。此项运行在用户/挂载/进程/网络命名空间和临时根目录内，未安装宿主服务。
- 本地 fmt、全 targets Clippy、完整 Rust/PostgreSQL 回归通过（204 项成功，4 项原有实机专项忽略），实际签名 Agent 的注册、补报、双拨测、命令去重、成功升级及失败恢复全部通过。Python 71 项检查中 66 项通过、5 项需容器 root 的原有专项忽略；构建脚本 5 项、actionlint、Python 语法和 diff 检查通过。
- 下一步：推送修复后核对完整 CI，特别是 Windows 两架构签名升级/原生服务、musl 两架构 OpenRC 及被前置失败阻断的真实 Reality 验收；本地通过不代替远端结果。

### 首轮 CI 跟进：容器内测试信任根归属

- `98287f4` 的主检查、Compose、GNU Agent、四种 Linux 运行时及 macOS 已通过。Windows 双架构通过公开 Rust 签名夹具构建与实际 Agent 签名升级/恢复，原生服务仍在运行；测试依赖和 CRLF 修复已覆盖之前的阻塞步骤。
- 新增公开摘要确认 OpenRC 在 bootstrap 前拒绝源码中的测试公钥：Docker 的只读源码挂载仍属于宿主 runner，不能当作 root 受保护的操作员信任文件。夹具将明确公开的 TEST_ONLY 公钥复制到容器内 `/root` 下的私有临时目录，再按原有 bootstrap 流程验证；不修改生产权限检查。
- 隔离 Alpine 中补跑全部 Python 检查，71 项全部通过、无忽略，包含安装器篡改拒绝、下载边界、重定向拒绝与受保护信任根专项。此前命名空间复现将源码所有者映射为 root，不能替代本轮发现的 Docker 所有权条件，后续另行模拟该条件。
- 随后将临时源码副本设为命名空间内 UID/GID 1001，并只读挂载到 `/src`；新的 root 私有测试根通过严格 bootstrap，完整 OpenRC 服务流程再次通过。Windows AMD64 及 FreeBSD 双架构远端检查已成功，Windows ARM64 常驻服务仍待完成。

### 第二轮跟进：OpenRC 通过与 Windows ARM64 运行时诊断

- `df417a6` 的双架构 OpenRC 与后续 Agent 行为检查已通过，真实 Reality 验收已开始；主检查及 Compose 也通过。此前的签名根所有权阻塞已解除。
- 第一轮 Windows ARM64 在原生运行时对账等待超时；任务已成功启动，现有事件显示它后来被停止，但不能据此确定应用失败原因。详细任务事件挤掉了公开摘要中的 ApplyResult，现将应用结果和最终 Agent 状态放到摘要末尾，事件改用紧凑格式，并只在一次性 CI 夹具为运行时加入 transcript。保留原有账户、启动参数、健康断言及等待期限，继续依据实际错误定位。

### 同步上游后续修复与验收诊断

- 合入上游 `541f52d` 的诊断资源预算、有界流量补报和 CI 修复；统一采用上游的 `ci-fixture-sign`、受保护 OpenRC 公钥目录及共享安装目录权限，删除重复签名 example。保留尚未合入上游的 Windows 签名字节回归、发布文件 LF 写入和原生服务诊断。
- 第二轮 14 项通过，Windows ARM64 仍在首次运行时对账超时，Reality 任务失败但公开接口只有退出码。Reality 验收增加失败行号及公开错误注解，内容仅取现有白名单摘要；用含私有占位字段的状态验证其不会泄漏到注解，未公开安装凭证或完整日志。
- 合并后本地 fmt、全 targets Clippy、完整 Rust/PostgreSQL 回归通过（216 项成功、5 项原有实机专项忽略）；隔离 Alpine 的 71 项 Python 测试全部通过，真实 OpenRC 安装与普通账户遍历权限检查通过。上游 Rust 签名器通过 Windows 换行模拟和独立 minisign 正向/篡改拒绝验证。验收驱动 17 项、运行时缓存 3 项、工作流与脚本语法检查通过。
- 下一步：继续定位 Windows ARM64 和 Reality 的失败，完整远端验收通过前不将此项标为完成。

### Windows ARM64 冷启动时限

- `1ea10e7` 的运行时 transcript 与任务事件确认：计划任务启动后约 28 秒 PowerShell 才开始执行脚本，30 秒健康期限届满即被 Agent 回滚，留给真实代理的启动时间约 2 秒；ApplyResult 连续报告 `runtime failed health check`。
- Windows 运行时健康期限调整为有界 90 秒，服务状态查询允许底层 PowerShell 命令已有的 30 秒期限，避免在较短的外层超时反复取消查询。仍须通过计划任务状态、全部监听和统计 RPC 检查；Linux、macOS、FreeBSD 的时限不变。等待新的 Windows 原生冷启动、重载与独立服务验证。
- 本地 fmt、Clippy 和完整 Rust/PostgreSQL 回归通过（216 项成功、5 项原有实机专项忽略）；Windows 冷启动行为以原生 CI 为准。

### Windows 已通过，修复静态 Agent 的运行时 libc 选择

- [cf93e4a 的 CI](https://github.com/imengying/sinan/actions/runs/36760693539) 中 Windows 双架构完整常驻服务通过，共 14/16 项成功。剩余失败为真实 Reality 安装后等待健康应用及上游新增的 systemd 资源预算专项。
- Reality 公开摘要定位到 `install` 阶段的 `ready` 等待。代码确认静态 Agent 将编译时 musl 同时用于外部运行时下发；验收提供已签旧格式 GNU 运行时，面板会拒绝选择。保留 Agent 自更新使用的 `libc`，新增可选 `runtime_libc`，从系统程序有界 ELF 解释器信息识别宿主，面板下发及 core 签名/缓存预检一致采用宿主 ABI。无法识别与旧字段缺失的行为明确回退，不放宽签名、摘要或平台校验。
- 新增协议兼容、GNU/musl ELF 与截断输入、签名运行时平台拒绝、GNU 宿主上的静态 Agent 下发及篡改拒绝回归；已有 Agent 更新测试同时证明 GNU 宿主仍选择 musl Agent 更新。Ubuntu 四种 Agent 行为任务增加实际宿主 libc 上报断言。主检查为两个真实 systemd 专项补充公开失败摘要与具体预算状态，保留所有限制断言。
- fmt、全 targets Clippy、完整 Rust/PostgreSQL 回归通过（221 项成功、5 项原有实机专项忽略），两个工作流语法与 core 分层检查通过。真实 Reality 和 systemd 资源预算仍须以下一轮 CI 为准。

### 修复 systemd 排队任务被误判失败

- `9cd112a` 的四种 Linux Agent、OpenRC、四种运行时和 Compose 已通过，包含 Ubuntu 上的实际宿主 libc 断言；本地实际签名 Agent 的注册、补报、命令、双拨测、升级和回退也通过。Reality 与原生服务继续执行。
- 主检查公开摘要确认新预算专项在子进程测试首次状态查询得到 `result=success, code=0, status=0` 后提前失败；这是异步启动尚未执行的状态。读取 systemd `Job` 属性，非零待执行 Job 视为运行中，包括排队重启时仍残留上一次终态的情况；没有待执行 Job 的未启动单元仍不能被当作成功。
- 新增排队启动/重启、空值/零值及非法 Job 标识回归，保留 OOM、TasksMax、PID 清理断言。fmt、Clippy、完整 Rust/PostgreSQL 检查通过（222 项成功、5 项原有实机专项忽略）；真实 systemd 验证交由新一轮 CI。

### Reality 全流程与 systemd 预算通过，收敛 Windows 夹具竞态

- [9cd112a 的 Reality 验收](https://github.com/imengying/sinan/actions/runs/36764089656) 已通过，覆盖签名安装、特权端口实际代理流量、重启与重载计量、重复安装、缓存篡改拒绝及在线退役。Windows ARM64、macOS 和 FreeBSD 双架构也通过；Windows AMD64 的常驻服务成功，但无服务升级夹具在读取被替换的 `pending-update.json` 时遇到短暂 `PermissionError`。
- [e8d4e23 的主检查](https://github.com/imengying/sinan/actions/runs/36765073067) 已通过，包含真实 systemd 两项专项。Job 队列识别后，OOM、TasksMax、超时和停止后 PID 清理断言全部通过；其余平台仍在执行。
- Windows 夹具读取更新状态时，对短暂共享冲突进行最多两秒的重读；持续拒绝仍抛出原异常，非 Windows 权限错误及无效 JSON 不重试。新增三项回归验证这些边界；隔离 Alpine 的全部 74 项 Python 检查通过，实际签名 Agent 的完整行为和升级回退流程再次通过。此项仅修改测试，不改变产品权限或文件写入行为。

### 再次同步上游诊断保护与平台校验

- 合入上游 `13f2975`，保留诊断资源预检、独占锁、运行中内存保护、候选升级缓存预检、原生服务独立签名复验、退役保护和 IP 查询错误分类。运行时 ABI、签名字节和 systemd 排队状态统一采用上游实现，删除重复的宿主解析模块与重复回归。
- 当时用户确认保留每次 push/PR 的全平台自动构建；当前已按后续要求暂停，保留的工作流定义包括：Linux GNU/musl 双架构、macOS arm64、Windows 与 FreeBSD 双架构。OpenRC 继续在 musl 任务内验证；保留上游新增的 core 业务边界门禁和真实 systemd 串行执行。
- 完整回归发现上游新增独占锁后，两项外部服务测试仍使用旧的权限夹具。更新夹具记录目录模式及所有者，验证 root/0700 锁目录与 `stat` 查询先于后端启动；不放宽产品权限检查。Reality 摘要回归同步新增参数，验证失败行号、公开注解和白名单摘要一致且不包含私有快照字段。
- 合并后的 fmt、全 targets Clippy、完整 Rust/PostgreSQL 测试及 Agent 构建通过：260 项成功、0 失败，8 项需 root/systemd 或真实外部运行时的专项保持忽略。隔离 Alpine 的 81 项 Python 检查全部通过，真实 OpenRC 安装与恢复夹具通过；验收驱动 21 项、运行时缓存 3 项、构建脚本 5 项及环境初始化 3 项通过。Bun 前端构建与提交的 dist 一致，三份工作流 actionlint 和 core 门禁通过。
- 本次完整合并源的非 Linux 原生服务、真实 systemd 五项专项与 Reality 全流程仍以随后远端 CI 为准；此前成功的独立运行记录不替代新提交的验收。

### macOS 启动前复验与软链接读取权限

- [5a8517e 的完整 CI](https://github.com/imengying/sinan/actions/runs/36767902951) 15/16 项通过：Windows 双架构、FreeBSD 双架构、四种 Linux Agent 与运行时、OpenRC、Compose、完整 Rust/PostgreSQL、真实 systemd 五项专项和 Reality 全流程均成功。唯一失败为 macOS 原生运行时启动，日志在启动前离线签名复验时报告 `Permission denied`。
- macOS 的软链接创建继承 umask，读取链接目标需要链接自身的读取权限。launchd 使用 `umask 027`，root 创建的运行时 `current` 无法由普通运行账户读取。`SystemOps::atomic_symlink` 在 macOS 发布前对临时链接执行 `chmod -h 755`，失败则清理临时链接；目标文件权限、目录隔离、进程 umask 和签名验证保持原有约束。
- 扩展现有原子写入回归，检查软链接可读、目标文件仍为 0640、替换为悬空链接和拒绝覆盖普通文件。自动与手动 macOS 工作流均用 `umask 077` 运行该回归，然后运行完整原生服务、缓存篡改拒绝和恢复验收。
- 本地 fmt、全 targets Clippy、core 门禁、三份 workflow actionlint、构建脚本 5 项及 Agent/协议/编译器/适配器 Rust 回归通过（193 项成功，7 项实机专项忽略）。本轮本机重启后临时 PostgreSQL 环境已清除，面板没有代码变更；完整工作区、真实 systemd、macOS 权限语义与所有平台再次由新提交 CI 验证，尚不记为全部通过。

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
# PR #72 整合复核

保留作者 Bookworm 启动及精确 workspace 信任修复；`b536476` 的 Debian12/amd64/arm64 原生制品 CI 全过。本地修正回环 CLI 测试的非阻塞 socket 读取竞态，TCP 17项单线程通过，TCP 全targets Clippy、fmt、Python来源12/发布22、旧Release28通过/4条件跳过及 core/actionlint 通过。永久源与当前 main 的锁文件区别已明确，生产引擎预算和固定制品未改；最终整合 HEAD 的主线 CI 尚须实时核对。

### P2 原生 TCP 实际 bundled musl 原文补齐（Issue #75，独立 PR）

自带musl/CRT配方使用Rust官方固定commit对应musl1.2.5与安全补丁；旧system1.2.3通知不作为实际libc来源。纳入官方完整版权原文、不可执行Rust证明配方与固定摘要，构建不联网补齐、未知rustc/原文篡改在Cargo前拒绝；签名验证对比固定source与实际rustc，真实重签缺失/篡改仍拒绝。五aux与ABI不变；新的公开工具pin、实际Bookworm及最新CI完成后单独记录，未正式发布。

- 修复后永久公开工具pin b562effcd90f8ae319665fb4ead1807b770ed4d5已实际Bookworm构建/ELF/version/build-info/完整5aux TEST_ONLY签名通过，35锁定依赖与Rust标准库、actual bundledmusl1.2.5、system1.2.3工具通知分别完整记录。fmt/core/15行为与真实重签/旧Release32/模拟发布22通过，exit0/OOMfalse，binary SHA e493d095...，日志evidence/tcp-musl-notices-b562eff。仅Python/库存变动，无重复全workspace；最新独立PR CI待核，未正式发布。
### P1 sing-box 根插件物理目录恢复（独立后续）

合并后 sing-box 面板实现位于 crates/panel/src/plugins/singbox，与用户要求及ADR0023的根 plugins/singbox 不一致。独立后续将13文件 git mv 至 plugins/singbox/panel，以薄的 Rust path 桥保留模块名与接口；逐文件blob SHA一致，无业务/API/数据库/epoch/前端变动。ADR0030与AGENTS明确物理路径。静态fmt/core/差异检查及最新CI分别记录，未重新宣称实机流量完成。详见singbox-plugin-business独立验收。

本轮独立审查逐一确认13个Git blob完全相同，path桥解析全部子模块及publisher嵌套测试，Rust可见性/旧导出保持；Docker COPY plugins与Compose根上下文保留。冻结源码 `7d1bda4` 的19项Rust/PostgreSQL专项全部通过、0失败/忽略（搬迁publisher2、插件业务/迁移4、账本4、业务/旧订阅4、订阅重置2、端口2、真实Agent配置/丢失ACK/重启1），workspace全targets Clippy（warnings为错误）、fmt、core门禁及六项行为、build-script5、runtime-cache3与差异检查通过。独立55432数据库由本任务启动并已停止，未触5432或生产；正常合入正式main `5d908b9` 后业务13文件与桥仍保持已验原字节，新增TCP/构建流程证据由其独立验收负责，最终整合CI继续单独核对。

## 2026-10-01：Reality 间歇传输失败证据（Issue #6）

- 业务源码743955c原CI在HUP后的2MiB下载只收到1,103,168字节，90秒exit28；同源码失败job只重跑一次，attempt2安装/双向流量/Agent重启/HUP/续传/签名拒绝/重装/在线退役全过。后续75cf整合源码另在首次下载90秒0字节失败，不能归因于业务或门禁。追加既有milestone1 Issue #6，保持原因未知，不重复开Issue或推已合并分支。
- 独立标准库helper记录最多4条固定传输的数字/错误类别，保留curl90秒、退出码与原载荷核对；失败清理前最多7秒直接HTTP/TCP/TLS夹具检查、有限宿主资源和进程存在性布尔状态；并行审查发现socket超时不能限制慢滴HTTP，改唯一短命子进程硬2秒结束并回收。沿用常驻单元状态白名单，不读取/上传配置、env、密钥、令牌、证书或完整日志；写入与公开汇总均重新过滤，失败取证不吞失败或重试代理流量。
- 14专项含真实回环HTTP/TLS、卡死夹具预算及恶意摘要隐私回归通过；验收驱动21、运行时缓存3、签名8、Python仓库101项/6既有条件跳过通过，fmt/core/shell/差异检查通过。独立提交CI与实际Reality另行核对，未将同源旧提交重跑当成本项集成验收。见 [独立验收](docs/acceptance/reality-failure-evidence.md)。

- 最终正常保留作者原始 `7db4c1a` 及推进后的 `7ce37b9`，合入正式主线 `8ef465f`，保留原生Bookworm制品修复/CLI夹具及根sing-box插件物理目录。冻结源码 `357eadb` 中HTTP每次底层读取共享绝对截止，作者的唯一短命worker硬2秒截止/回收与进程存在性字段均保留；已有失败的清理继续尝试并保留原28，原成功流程的清理错误仍拒绝。真实负对照捕获慢滴头/体3.48/3.50秒超限和原28被清理7覆盖，恢复后取证16、驱动21、缓存3、签名8全部通过；仓库Python101项运行（95通过/6既有条件跳过），合计143通过、0失败、6跳过。Python/Bash语法、fmt只读检查、core/actionlint及链接/差异检查通过，未运行Rust编译/测试或实际生产Reality/公开网络，完整提交CI继续单独核对，Issue #6原因仍未知。

### PR #76 实际 bundled musl 独立复核

- 保留作者 `08c9de2` 与正式主线 `d1ff2df`。官方 musl1.2.5 归档、193 行 COPYRIGHT、Rust 1.98.1 固定官方 commit 和 97 行配方的摘要及原字节全部核对；配方除两项 2025 iconv 补丁还包含两份 2026 安全补丁，均不改 COPYRIGHT。库存简述只描述 2025 子集，原库存不改写，实际 self-contained libc 与系统构建工具通知分开记录。
- 修复 rustc 身份歧义：旧表达式接受已知/未知双字段和重复已知字段，即使按完整摘要与公开 TEST_ONLY key 真实重签仍被接受；新检查只允许一个精确字段，补充收集前和重签后的实际负向回归。来源/签名 17、旧 release 28（4 既有条件跳过）、模拟发布 22、core 分层 6 项通过，合计 73 通过/0 失败/4 跳过；Python/core/actionlint/链接/差异检查通过。
- 实际原生 CI `36794789931` 三个 job 成功，工作流 head `08c9de2` 显式选择固定工具源 `b562effcd90f8ae319665fb4ead1807b770ed4d5`；每个 job 真实启动及来源/签名 15+原生 bundle 1 项通过。独立下载两个架构的原包，当前严格验证器接受合法历史库存，并将其重新组装公开 TEST_ONLY 签名验证通过；内嵌构建器、旧验证器、收集器及三份库存与 Git b562 逐字一致。不声称旧固定对象含新歧义检查，不改其版本路径或既存制品。本机未运行下载二进制、Cargo/native build、正式签名、发布或生产探测；本地完整整合提交 CI 另行核对。

### PR #77 登记整合复核

冻结 `58868c4` 的 TCP API/目标单元5、core共用诊断生命周期40、TCP适配器与UID15、NodeQuality适配器15，共75项本地通过/0失败/忽略，workspace全targets Clippy及fmt/core通过。核对Linux/monitor-only登记边界、固定b562版本、六文件签名前及启动前校验、60秒/64MiB预算和旧NQ完整门禁；55432专用PG已停止。作者实际370+6 CI与Reality成功分别记录，不将不可执行API签名夹具或通用systemd验收称作新TCP实机完整执行链。正常保留作者提交并合入#76正式主线b152e2a，相关Rust/SQL/锁文件与已验58868c4原字节保持一致；最终主线CI单独跟进。


## 2026-10-01：同步上游与修复 PR #84 合并遗漏

- `main` 快进到上游 `3f65b42`，已包含 PR #84 及最新的 IP 查询缓存、根插件目录、共用诊断服务、TCP 诊断登记与发布修复；保留全平台构建、macOS 软链接权限和 Windows 冷启动等已有修复。
- 恢复合并前上游 `356350e` 的锁目录测试夹具及 umask/flock 断言、Reality 白名单流量证据汇总和被覆盖的进度记录。新增流量证据测试同步传入失败行号，并断言汇总保留该字段；未放宽服务权限、签名验证或公开证据边界。
- 本地 workspace fmt、全 targets Clippy（warnings 为错误）、core 门禁、四份 workflow actionlint 和差异检查通过。Bun 1.4.2 冻结安装、前端构建及 5 项前端测试通过，重建 dist 与上游原字节一致。使用仓库 CI 的公开 TEST_ONLY 编译信任根、回环请求绕过本机代理后，除 panel 外的 Rust workspace 回归 259 项通过、0 失败、8 项既有实机专项忽略；Python 仓库与构建/环境/验收/诊断模式脚本共 169 项通过、0 失败、6 项既有条件跳过。
- 本机没有 PostgreSQL，未重跑 panel 数据库集成测试；root 容器安装、原生 TCP 制品矩阵、真实服务、Reality 全流程及非 Linux 平台也未重新验收。遵循上游 AGENTS 的临时 CI 暂停约定，本次提交使用 `[skip ci]`，保留完整自动构建配置，不将本地结果记为新整合提交的远端 CI 全绿。


## 2026-10-01：P0 IP 显式错误响应确认（Issue #42 独立窄项 PR）

- 根非空/畸形 errors 与 AbuseIPDB 实际 data 响应容器的明确失败或不确定标志不再采纳默认 0 分；沿用字段不匹配分类和已有历史缓存语义。空 errors、真实 0/false 及旧无标志成功兼容；不递归解释 ASN/company 元数据，不增加查询来源、请求或数据库 schema。正式接口仍严格校验目标 IP、公网、版本及已知字段。
- 独立验收见 [IP 响应确认](docs/acceptance/ip-response-confirmation.md)。专属回环 HTTP 与临时 PostgreSQL 先保存 73 分，再分别返回根 errors、data.success=false、data.errors；失败后换池读取仍保留历史 73、成功时间和有效期，其余六库继续成功，新 IP 未知。30 项 IP 专项全部通过、无失败/忽略；workspace fmt、panel 全 targets Clippy（warnings 为错误）与差异检查通过。临时 PostgreSQL 已停止。
- 原解析器字段负对照实际 4 通过/2 预期失败，证明两个错误响应均误返回当前 0；真实 HTTP/PostgreSQL 新历史回归在原解析器下预期失败，恢复修复后通过。源身份与负对照步骤分别记录，不把负例失败当作修复失败或累加场景数。
- 四个远端 workflow 仍 disabled_manually，未触发、重跑或恢复 CI；未重复完整 workspace、浏览器、平台、正式外部源或节点实机总验，不关闭 Issue #42 剩余验收，也不签收、发布或部署新增诊断能力。下一步按整改顺序继续独立实机验收。

## 2026-10-01：独立 P0 诊断 swap 系统调用保护（关联 #66）

- 新 systemd 诊断固定 `NoNewPrivileges=yes`、`SystemCallArchitectures=native`、`SystemCallFilter=~swapon swapoff`、`SystemCallErrorNumber=EPERM`，资源预算五字段保持原样。三次 3 秒/16 KiB 支持探测失败即拒绝；同单元固定 awk 预命令检查 `NoNewPrivs=1`、`Seccomp=2`、至少两层过滤，识别未安装/只有 ABI 过滤的异常。
- 运行中 manager 的 `+SECCOMP` 与 PID 1 无继承过滤必须可验证；OpenRC 新诊断明确拒绝，常驻代理运行时和历史诊断的停止/读取/回收保持。保护只覆盖单元直接 fork/exec 子树，不阻止 D-Bus 或其他宿主 daemon 另开进程，不禁止文件写入，也不解除 full 门禁或修复上游 swap helper，#66 继续开放。
- 本地 `fmt`、core 边界检查、`clippy --locked -p sinan-agent-core --all-targets -- -D warnings` 通过；core 全套在增加 count guard 前通过，最终相关 `system::` 专项 23 passed / 7 ignored、服务集成 6 passed。独立 Debian 12 ARM64 guest 新 swap syscall 夹具 1 passed / 0 failed / 0 ignored，同一新 core 二进制原六有限 systemd 夹具回归 6 passed / 0 failed / 0 ignored；direct/fork/exec 无过滤返回 ENOENT、过滤后 EPERM，NNP=1/Seccomp=2/filters=2，仅 native ABI 负例未运行 payload，swap 表逐字节不变。清理后诊断单元/编译进程/夹具挂载/目录为空，SSH PID 446 重启 0、无 global OOM；不据此声称完整 NodeQuality、实际 Agent 心跳或持续代理流量通过。CI 按临时规则保持暂停。
- 独立步骤与支持边界见 [验收文档](docs/acceptance/diagnostic-swap-syscalls.md)。真实测试仅在获授权的可销毁 guest 对 root-owned 0700 目录下的不存在路径调用 syscall，不创建 swap 文件，不更改宿主 swap。

## 2026-10-01：修复磁盘容量重复统计（独立 PR）

- Docker 宿主的真实根分区与嵌套 overlay 原先各累加一次，容器的根 overlay 与 `/etc/hosts` 等文件绑定挂载也重复统计。Agent 统一筛选容量、已用空间与逐盘列表，排除 Linux 非根 overlay 和单文件挂载，保留容器根盘及独立分区。
- Unix 按文件系统设备标识去重绑定挂载及别名，Btrfs 保留源设备去重；不可读元数据沿用名称/路径回退。Windows 保留原逐盘列表和按名称首条去重的容量规则，避免同一卷多挂载点新增重复计数；不同卷同名或无名的既有歧义待真实卷 ID 的独立实现和原生验收。逐盘 I/O 基线同时包含名称与挂载点。沿用已挂载文件系统容量、保留空间和未知值语义，不新增依赖、协议字段或面板换算规则；边界见 [磁盘容量决策](docs/open-questions.md#磁盘容量容器挂载与文件系统去重)。
- 原提交 `7ef561b` 在 Rust 1.98.1 限额 Linux 容器执行 `cargo test --locked --offline -p sinan-agent-core`，181 项通过、0 失败、6 项既有真实服务条件忽略；包含原新增 8 项磁盘回归。该提交的 workspace fmt、core 全 targets Clippy（warnings 为错误）、core 分层门禁与差异检查通过。后续整合补充 Windows 兼容收窄及同卷名称首条去重/无效/溢出回归，此处原结果不作为补修提交的验证。
- 包含最新主线 swap 保护及 Windows 兼容补修的本地输入 `aa5ab3e`，在 macOS ARM64 使用公开 TEST_ONLY 信任根和专用回环 PostgreSQL 执行完整 Rust workspace 全 targets 回归：389 通过、0 失败、10 项既有实机条件忽略；新增名称去重 helper 与 Unix Collector 回归在本机执行。workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁及额外 macOS `umask 077` 真实软链接权限专项通过。专用 PostgreSQL 已停止；此结果不代替 Linux 条件用例或 Windows 原生验收，最终文档回填不改变受测 Rust 输入。
- 使用相同构建的 Collector 做只读采样：宿主仅保留 `/`、`/boot`、`/boot/efi`，总量 `541018241536` bytes（503.863 GiB），逐字节等于 `df -B1 --output=size` 之和，已用等于逐盘已用之和。另建无目录卷的临时 Debian 12 容器，旧名称累加结果 `1079556669440` bytes，修复后仅 `/` 为 `539778334720` bytes；容器退出 0、无 OOM，采样后已删除。
- 未重跑完整 workspace、面板浏览器或非 Linux 原生实机；未推断容器额外目录卷与不可见 overlay 后端的物理归属。遵循临时约定保留四个 workflow 暂停并使用 `[skip ci]`，不将本地验证称作远端 CI 通过。下一步审查合入后构建并签名新版 Agent，再部署生效；本次仅提交修复 PR，线上 Agent 尚未替换。

- 作者在 Windows 兼容补修前同步主线 `2605dbe` 的输入 `81374df`，另行记录 Linux core 专项 185 通过、0 失败、7 项既有实机条件忽略；fmt、core 全 targets Clippy 和分层门禁通过。此结果不代替 `aa5ab3e` 补修输入的本地完整验证；新版签名、发布与线上升级由对应任务单独记录，CI 保持暂停。

## 2026-10-01：P0 有限联合负载独立验收

- 冻结 `b5289a9` 的 TEST_ONLY Agent **0.3.0**，使用 `--monitor-only` 与真实 WS/HTTP 替身面板；固定正式验签的 sing-box 1.14.2 ARM64 ELF，在修正 Lima 端口规则后的独立 Debian 12 guest 上执行一次联合场景。只借用现有 helper 的回环接收器/传输函数，不运行完整 NodeQuality、上游脚本、安装/升级流程或公网请求；不认证后续 Agent 0.3.1 或主线磁盘遥测等变化。
- 六个独立真实 systemd 夹具一次 6 passed / 0 failed / 0 ignored（2.28 秒），持续 VLESS 每向 1KiB、约 1Hz，113 次全部成功。收到 8 个真实 Heartbeat，baseline 三次跨度 40.0324 秒，连接内最大间隔 20.0334 秒≤30；诊断阶段同一 WS session 实收一次。120 次遥测 ACK 保留 `sampled_at`，正常阶段最大收样年龄 1.5599 秒，断连积压最大 16.4600 秒，未把 telemetry/last_seen 当心跳。
- 实际 Agent PID 767→1951 手动重启，3 个旧未 ACK ID 重放；v1 未另存这些旧 ID 的原时间值，因此保留原事实，单独 7.349 秒 restart/replay-only 补验记录真实 POST/503 前后时间，三个旧 ID 与 sampled_at 逐值相同，不重复代理/core 负载。WS/HTTP 明确断连 10 秒后恢复，完整原始全局间隔保存；原证据另经固定恢复 session 两次心跳/周期判据核证。运行时 PID 765 全程保留、NRestarts=0，SSH PID 406/NRestarts=0。内核五条提示完整归属诊断 64MiB MEMCG OOM；新判据拒绝未归属提示，驻服务/全局无 OOM。
- 清理后无本次 PID/cgroup 子进程、诊断单元、挂载、临时运行时目录或编译进程，根分区可用约 3.716GiB，swap 仍空。最终清理判据拒绝 PID=0 但 cgroup 读取失败的未确认结果。独立脚本的 14 个行为回归、Python 语法、core 分层与差异检查通过；四个 Actions workflow 仍 disabled_manually，没有触发 CI。两次原脚本/结果分别保存，未改写 v1 结果。
- 该证据只证明有限联合负载：monitor-only 不注册诊断/代理适配器，六夹具不能代替管理员取消协议、Agent 诊断 checkpoint/outbox、真实 Panel/PG/UI、生产 Reality 或完整 NodeQuality。全文保留冻结输入、运行时正式验签边界、双 boot 区别、实际统计与缺口，见 [独立验收文档](docs/acceptance/p0-joint-load.md)。P0 总验仍未签收。
- 合并前补修流量 worker 启动异常漏记和停止期间最后一次失败漏判：异常保存明确失败行，停止前必须存活，等待线程后重新核对最终流量及重放记录。本聊天本地行为回归 16 通过；原作者 14 项与私有脚本/结果/receipt 保留原归属，未在 guest 重演，不以新判据追认旧联合验收。正式签名/发布/部署及 CI 均未执行。

## 2026-10-01：sing-box 策略组、套餐周期与两跳链路

- 按用户新增需求在 `plugins/singbox/panel/` 实现策略组、套餐模板/不可变分配快照与独立入口到出口的两跳链路。一个用户可绑定多个策略组和一个当前套餐，单独授权与组授权取并集；旧用户 ID、订阅令牌、节点 UUID 与流水保持。没有套餐仍是显式提示的不限额/不限期兼容模式。界面新增“策略与套餐”，用户详情分别分配可用范围和套餐。详见 [ADR 0035](docs/adr/0035-singbox-policy-package-groups.md) 与 [使用说明](docs/singbox-groups.md)。
- 月度额度按用户跨节点上传/下载汇总，IANA 时区、每月重置日及时分可配置，短月取月末但不漂移。独立有效期按分配时刻计算；模板编辑不追溯影响旧快照，幂等请求重试不延长期限，旧请求不覆盖新套餐。保留完整账本，不通过清空 epoch 或用量实现套餐重置。订阅及服务端发布共享资格判定，周期任务处理耗尽/到期与恢复，不要求管理员打开页面。
- 链路使用未修改的原生 sing-box VLESS + Reality 配置，客户端只获取入口用户凭据；内部 relay 身份不再次计量。两端应用前不提供订阅，循环、嵌套、同机与已授权入口拒绝创建。资格选择及路由选择使用同一数据库快照和时刻，防止周期边界产生无路由的入口；出口删除/退役触发对端撤销。原 core、Agent 协议与签名链未改，不增加生产依赖。
- 首次基于 `61a6565` 的本地 workspace 验证 407 通过、0 失败、11 条件忽略；其中本轮新增 PostgreSQL 13 项、编译器 3 项。另显式执行 1 项本地原生 1.14.2 入口/出口/客户端配置检查通过。全 targets Clippy（warnings 为错误）、格式化、TypeScript/Vite、桌面/手机 Chromium、core 边界及检查器 6 项通过。旧迁移快照补充严格的 `direct_grant=true` 断言后全量回归通过。原生条件检查与默认忽略分别记录，未重复计算为全部环境通过。
- [独立验收与未验证范围](docs/acceptance/singbox-groups.md) 记录隔离 PostgreSQL/资源预算、浏览器夹具、采样跨月及离线停用延迟、迁移后禁止混跑旧发布器的约束。本轮仅源码/PR 交付，不运行生产迁移、不部署、不发布或恢复暂停的 CI。真实双机长连接、离线恢复与生产规模验收仍需单独执行。

- PR #100 创建期间主线合入 `be6bf81`，本轮保留其现代协议、托管证书、拨测及独立展示页。策略组按协议生成 credential，发布/订阅保留双凭据校验；普通节点策略兼容全部已有协议，链路在 API、UI 和编译器限定为 Reality。新增两个 PostgreSQL 交叉用例及一个编译器协议边界用例。迁移改为 `0016`、本决策改为 ADR 0035，解决主线编号碰撞；没有部署过旧草稿迁移。
- 协议整合后重新运行完整 Rust/PostgreSQL：423 通过、0 失败、15 条件忽略；本 PR 专项含 PostgreSQL 15 项、编译器 4 项。另显式执行原生 1.14.2 的三个配置检查均通过，不重复计入 workspace；其余 12 个条件用例未补验。全 targets Clippy、fmt、前端 17 项、TypeScript/Vite、桌面/手机 Chromium、core 边界及检查器 6 项通过。前端跨目录测试的挂载问题修正后正常通过，无放宽断言。生产、真实双机流量和暂停 CI 的边界保持。

- 最终继续保留主线 `92800dd`（#98/#101）的验收工具和拨测修复，解决共用前端构建产物与进度文档冲突；相对 `e05541f`，sing-box 插件、编译器、迁移及前端源码保持。最新整合状态完整重跑 Rust/PostgreSQL：425 通过、0 失败、15 条件忽略；另显式原生配置检查 3 项、前端单元 17 项、桌面/手机 Chromium、fmt/全 targets Clippy、core 边界和检查器均通过。

## 2026-10-01：#100 本聊天最终合流

- 受验源码 `f179cc6` 完整 Rust/PostgreSQL 428 通过、0 失败、15 条件忽略，另 macOS umask077、workspace 全 targets Clippy/fmt/core 通过。保留作者 d93→e055→7f 推进、全部主线进度/开放问题及 #98/#101；最终文档变化不改受验产品/测试输入。初轮47b的425/0/15与作者历史实机证据分别保留。
- 迁移0016/ADR0035解决编号冲突，普通组现代凭据和套餐资格、Reality-only链双端限制、无绕链降级与两侧互补回归保持。Bun17/771、TS/Vite与分组/展示/TCP/业务/NodeQuality/取消六套真实Chromium通过；旧业务fixture新增chains响应，原未知API断言仍在，初次失败日志保留。未执行15项条件实机、真实双机负载、生产迁移/签名/发布/部署或CI；四workflow保持暂停。详见[独立验收](docs/acceptance/singbox-groups.md)。

## 2026-10-01：#100/#102 合入产物同步补修

- 实际 main `ed1d935` 的源码已含公网/内网折叠，HTML 却仍加载旧 `index-BCdIyArN.js`；真实 dist 地址页夹具找不到 private 折叠控件，原失败保留。部分未引用资源还残留合并内容。按冻结 Bun1.4.2/lock 重建并清理本树旧生成文件，恢复源与产物一致。
- TypeScript/Vite 通过，真实 Chromium 地址页1280/390正例通过；重建全部19个文件逐字节等于本聊天受验 `8e1f1f9` 的产物，该输入的分组/地址/展示浏览器已通过。运行源码/迁移不变，无额外Cargo/PG、正式签名、发布部署或CI；四个workflow继续暂停。

## 2026-10-01：补齐服务器资产与账单周期流量

- 按用户追加授权完成地区、展示分组、标签、展示隐藏、金额/币种/费用周期、到期日期、自动顺延记录、网卡额度/统计口径/月重置日/网卡筛选。新增与编辑共用分区表单及快捷跳转，后台列表、详情和服务器展示页接入真实配置；支持地区/分组筛选及标签搜索。
- 新增 0016 迁移与兼容默认值；旧仅名称请求仍可用，PATCH 省略资产时保留现有值，非法字段整体拒绝。金额和字节使用精确字符串，保留免费与未填写的区别，编辑超过 JavaScript 安全整数范围的额度不丢精度。自动顺延按费用周期追平到期记录，并发维护不覆盖编辑；不代供应商付款。
- 使用既有带时间戳遥测，逐网卡保存计数检查点和 UTC 日累计，与遥测确认处于同一事务。首样本建立基线，同批乱序按时间处理，重放与旧采样不重复累加；识别计数回退、可观察重启、网卡变化和采样缺口。每月账单日按 UTC 日历计算，短月取月末，修改统计网卡、口径和重置日会重新汇总保留的历史。到期或超额仅提示状态，不修改服务与代理授权。
- 最终完整 `cargo test` 396 通过、0 失败、13 条既有条件忽略；真实临时 PostgreSQL 覆盖资产鉴权/原子保存/默认值/清空、日期顺延、月末闰年、网卡过滤、重放乱序、大整数及注入写入故障后的事务回滚。旧库迁移快照新增空资产默认值断言，原有身份、凭据、授权和账本不变检查通过。全 targets Clippy（warnings 为错误）、fmt、core 边界和差异检查通过。一次中途终止的运行未计入通过结果，最终完整重跑退出码为 0。
- Bun 1.4.2 / TypeScript / Vite 构建通过并同步 dist，20 项前端测试 / 795 断言通过。最终产物的 Chromium 1440/390 像素覆盖新增、编辑、失败保留与重试、清空、二进制小数额度和最大字节精度、自动顺延条件、地区/分组/标签筛选及隐藏；既有接入流程的两种宽度与展示页 1440/390/320 像素深浅主题回归通过。浏览器错误、非预期接口调用均为零，弹窗无横向溢出；嵌入静态资源的真实 HTTP 测试通过。
- 使用和升级说明见 [服务器资产与流量额度](docs/server-assets.md)、[ADR 0036](docs/adr/0036-server-assets-and-traffic.md)。网卡差值归于后一采样日，跨日断连、升级前和无法识别的重启期间不能恢复精确用量，不承诺与供应商账单一致。未做生产升级、真实多平台流量或长期续期验收；下一步在专用测试机核对出口网卡、基线、断连和月边界。CI 按用户安排继续暂停，本次提交标记 `[skip ci]`。
- 推送前将 `382550f` 正常整合上游 `92800dd`，保留双方进度记录并重新构建前端；核对上游 Agent 0.3.1、磁盘计量、系统保护、IP 与拨测修复的源码原字节保留。最终完整 Rust 回归 416 通过、0 失败、14 条既有条件忽略；全 targets Clippy、fmt、core 边界、差异检查通过。最终 dist 的 20 项前端测试 / 795 断言，以及资产、接入、展示三组桌面/手机浏览器回归通过；新产物为 `index-BshCnV6e.js` 与 `ServerDisplay-B_BGOdU6.js`。本地结果不替代各平台实机或远端 CI，CI 继续暂停。

## 2026-10-01：完善服务器新增配置与接入流程

- 按用户选择参考 NodeFlare 的新增表单与安装引导，改为“配置服务器 → 安装与接入”两步弹窗。分区呈现名称、监控采样/上传间隔、公网地址识别、自动更新和可选初始 TCP/ICMP 拨测，提供实时、均衡、轻量预设及自定义间隔，保留手机布局与键盘操作。成本、标签、地区和流量额度留待后续独立工作。
- 创建接口支持可选 AgentSettings 与最多 32 个初始拨测，同一 PostgreSQL 事务保存，非法参数或拨测存储失败回滚；旧仅名称请求及重命名行为兼容。复用既有协议、设置和拨测表，无新增依赖或迁移，也不创建默认公网目标。
- 新增与详情接入复用安装组件：签名版本选择、命令复制、过期隐藏、失败独立重试与排查说明；可见页面每 3 秒查询实际注册/在线状态。重试命令不重复创建服务器，编辑版本立即隐藏旧命令，状态读取失败不显示旧在线状态，已有设备在线不冒充升级完成。保留可信 bootstrap；Linux 命令与原生平台文档流程分别说明。
- 本地完整 Rust 回归 388 通过、13 条既有条件忽略；新增真实 PostgreSQL 用例覆盖旧客户端默认值、认证、TCP/ICMP 保存、UUID 重建、非法配置、32 条上限、重命名保留设置及中途写入失败回滚。全 targets Clippy（warnings 为错误）、fmt、core 边界与差异检查通过。
- Bun 1.4.2 / TypeScript / Vite 构建通过并同步 dist，17 项前端单测 / 771 断言通过。最终产物 Chromium 1440×1000 与 390×844 覆盖配置校验、失败保留输入、重复提交、命令重试/复制、切换版本、缺少制品、令牌过期、自动注册/上线检测、查询失败恢复、详情继续接入与焦点循环；浏览器错误和额外接口调用均为零，弹窗无横向溢出。已有展示页 1440/390/320 像素回归通过，最终面板静态资源 HTTP 集成测试通过。
- 未执行生产安装、真实设备升级和远端 CI，不将浏览器夹具的上线状态当作实机验收。CI 继续暂停，提交使用 `[skip ci]`；部署后可在测试服务器按 [接入步骤](docs/deploy.md) 检查采样与拨测设置同步。

## PR #103 合并验证与时钟校正（2026-10-01）

在保留作者 `238ce393` 和主线 `412e8fc` 的普通整合提交 `ba3892c6` 上，资产迁移使用 `0017_server_assets.sql`，ADR 使用 0036，保留现代协议、策略组、套餐/两跳链路、固定拨测身份与私有地址折叠。修复墙钟跳变被当成重启而重复累计网卡总量的问题：只有运行时长实际回退确认重启；墙钟/运行时长不一致只标记不完整，单调计数仍取差值。真实 PostgreSQL 回归覆盖向前校时、回拨夹持、明确重启、重放及事务失败回滚。

本聊天在该精确提交完成 workspace/all-targets Rust 与 PostgreSQL：442 通过、0 失败、15 项既有实机或平台条件忽略；另 macOS umask 077 原子写入/链接 1 项通过，fmt、core 分层、workspace 全 targets Clippy 通过。忽略项、生产网卡/账单比对、真实 Agent 联合负载和实机总验仍未验证；GitHub 四个仓库 CI 工作流继续暂停，本项未触发 CI、正式签署、发布或部署。

同一 `ba3892c6` 的前端独立验收：Bun 20 通过/795 断言，TypeScript/Vite 77 模块构建逐字复现 19 个已提交产物；9 个真实 Chromium 夹具全部通过，19 次桌面/手机视口检查、28 张截图。覆盖接入/资产、IP 折叠、策略组、代理业务、展示、TCP、NodeQuality full 门禁和确认式取消；所有写入由私有 API 替身承接，不代表真实面板或 Agent 接入。最终仅追加本段进度，受验 Rust/前端运行输入保持。

## 2026-10-01：NodeQuality 首层五脚本固定来源（关联 #28 独立项）

- r6 构建器从四仓完整提交获取入口、五首层脚本和四份完整 LICENSE，逐文件验证 SHA256/大小后原字节嵌入一个签名 runner；来源/版权信息和许可证不被删改。宿主 curl shim 在原入口真实通路按五个精确 URL 供给本地来源，未知请求拒绝且不能回退在线 main。helper 可独立输出固定清单、打包/校验/供给，不执行源码；有界普通文件读取拒绝 FIFO/符号链接。
- 原硬件/IP/网络/回程参数、章节和历史 r2–r5 收集保留，r4–r6 日常入口保持；面板和 Agent 所有 full 门禁不变，不添加 `-p` 或减少原硬件能力。rootfs、二级工具/数据/二进制许可与所有上传路径仍未完整收敛，#28/#65 保持开放，不据此签收完整 NodeQuality。
- macOS 新来源专项 12、旧 wrapper 34、日常 helper 7 通过；发布契约 32 项运行（28 通过/4 既有条件跳过）。真实私有 shim/runner PATH、单字节篡改/缺源、意外 URL、FIFO、两架构不可变/重建、完整 TEST_ONLY 签名覆盖及篡改拒绝有行为证明；恢复旧 shim 的三个用例出现 16 个预期失败断言/0 异常，证明接线路径不是未使用代码。没有执行上游脚本、rootfs、benchmark 或公网探测。
- 冻结源码的 adapter 诊断 15、panel chain_gate HTTP/PostgreSQL 3、日常接口追加 1 项通过，0 失败/忽略；workspace fmt、adapter 全 targets Clippy（warnings 为错误）、core 分层和差异检查通过，自有 PG55439 已停止。完整 workspace/全部 panel/其它平台未重跑；独立步骤与来源摘要见 [验收文档](docs/acceptance/nodequality-pinned-first-level-sources.md)。
- 专用 Debian 12 ARM64 guest 原单次运行 wrapper34全部通过、来源12运行（11通过/1缺minisign条件跳过）；18输入摘要前后不变，实际测试3.929184秒。清理核验以真实journal monotonic起点取证，OOM/夹具进程/挂载为空、unit inactive/not-found/cgroup不存在、SSH406重启0、同boot、swap0。只证明纯夹具的Linux/root包装/回收；本次限额只有systemd-run配置参数，运行期readback未持久化，不夸大为实机预算或完整验机/代理联合总验。
- 保留已验61a源码快照aca2f62后，重定位到be6bf81形成受验71a778b；本项5Rust/18guest输入SHA逐个不变，SDK诊断接口、签名/runner/helper/build输入无上游差异，仅常驻Adapter默认方法及已有依赖边变化。同一19Rust专项与Clippy/fmt/core再过（收据51a6a036…）。最终固定92800dd形成受验d3ca6756，保留他项PROGRESS，五Rust/18guest/Cargo/trust输入当时仍相同；为覆盖panel/probes新编译输入再过同一19及Clippy/fmt/core（收据7021f9db…），PG已停、PID不存在/端口关闭。重复基线验证不增场景，不认证其它新能力；文档回填形成首个源码点7d4e940，原证据保留。
- 发布前发现 curl 8.4 之前 `--max-filesize` 无法限制未知长度响应，已在同一项中补接收边界：curl第一项`--disable`，stdout经helper最多读2MiB＋1，合法后才O_EXCL/NOFOLLOW创建0600文件，超限不写目标，pipefail保留旧制品/checksum。新版mac来源15全部通过、0失败/跳过，含实际回环chunked/声明超限、严格umask与子进程/server清理；只证明有限流读取，不认证TLS或上游执行。五Rust/Cargo/trust未变，19不重跑；原guest34/12证据只对应旧helper快照。
- 新版来源15在专用Debian12 guest只追加单次运行：14通过/1缺minisign条件跳过，2.513秒；actualcurl7.88未知长度control实际写2,097,153B，新receive两种超限响应均拒绝且无目标。本次限额在单元内部持久化读回（MemoryMax256MiB/Swap0/Tasks64/weights10/OOM500/PrivateNetwork+NNP/KillMode），峰值47,603,712B/7pids、memory.events max/oom/oomkill0。18inputs前后不变，cleanup无OOM/进程/挂载/cgroup残留、SSH406重启0、同boot/swap0；新result1c5a9a51…/postf6ac776f…/index3e74cebe…保留，旧34和19未重复。
- 保持四个 workflow 暂停；未触发 CI、正式签名、发布或部署。后续仍按原整改顺序完成整条 NodeQuality 执行链修复与专用节点验收，不把五首层 pin 等同 rootfs/二级链完成。


## 2026-10-01：#104 文档冲突整合

- 固定合入主线 `412e8fc`，唯一冲突为本文件双方追加记录，均完整保留。NodeQuality、SDK、core、Cargo.lock 与已验 `6d1e731` 输入保持；来源及 guest 验收不重跑，不以此认证主线新增业务。既有 Rust19 对应 `92800dd` 基线，整合后的面板编译输入另随下一项版本验收记录；CI 继续暂停。

## PR #104 本聊天整合复核（2026-10-01）

保留作者 `2d63c965` 与主线资产/流量整合 `00151f47`，精确输入 `e11bf9c`：显式补回 queued r5 的门禁回归，r2–r6 full 继续拒绝；不依赖当前版本常量代替旧版本覆盖。adapter 29、panel diagnostics 13，共 42 项通过/0 失败/忽略，fmt、core、workspace 全 targets Clippy 通过，自己的 PostgreSQL 55432 已停止。这是专项验证，不把 #103 的 442 全量结果改称 r6 版本全量，也未在 guest 重演作者验收。

本聊天独立来源/wrapper/daily/release 在 `df1c1dd` 完成 88 场景，84 通过/4 既有条件跳过（15/34/7/28）；十个官方固定提交文件大小及 SHA256 与清单全部一致，清单 SHA256 `3d20398eeda72654c59b3271fd03b35ca8c0b4e92ee92a054a4a8c432a62723a`。源码只读取未执行；签名测试只用公开 TEST_ONLY key。整合后相关脚本/清单/构建器字节未变。rootfs、二级工具、全部上传与 swap 仍未完整验证，没有签署/发布/部署 r6 或触发暂停中的 CI。

## 2026-10-01：NodeQuality 三处公开报告 POST 策略（关联 #65 独立项）

- r7 将固定 report-policy helper 接入实际 builder、runner 与 source-helper serve；先核 canonical SHA，再固定变换 HW/IP/Net 三处公开报告 POST。默认 false，非法策略在 bootstrap/探测前拒绝；不修改隐私模式或 CPU/GPU 调用、`-o` 本地 JSON/ANSI 和面板采集。Net 增加局部空链接，避免禁止上传时显示继承的旧链接。canonical source-lock/许可证和 r2–r6 旧不可变制品保持，r4–r7 daily 兼容。
- 私有组合夹具实际走嵌入 helper 与 shim；旧未 patch 负对照和 true 到自有回环 recorder，false/default 零 POST。三份既有真实源的静态 production transform 收据均核验 original/patched SHA，逆向移除固定变更后逐字节恢复；不执行有效上游脚本或真实探测。Mac Bash 3 的 stdin 模拟只证明接线；Linux 原 FD 与有限真实 chroot 环境继承在下述独立 guest 组中分别验收。
- 已保留 r6 `6d1e731` 下载流上限补修并在受测 `bf0ad14` 执行 host 来源16、合成策略6、固定原文函数体组合2，全部通过且无跳过；3份原源的 production helper 静态收据可逆还原。固定重基到 r6 合流 `2d63c965` 仅解决双方 PROGRESS 追加记录，19份选定产品/测试/构建/Cargo输入SHA逐项不变，不重复host。此前 r7 版本阶段 wrapper34、daily7、release32（28通过/4既有跳过）另保留原快照归属。
- 最终 Rust19 及 Debian guest 的原 FD/有限 chroot 收据由根任务单独记录，尚不以 Mac stdin 模拟签收 Linux；具体结果与源码身份见 [三处 POST 策略验收](docs/acceptance/nodequality-public-report-policy.md)。四个 workflow 保持暂停；仅登记未来测试命令，不触发/重启 CI，也不正式签名、发布或部署。
- `mark.check.place`、Geekbench 自身上传、完整 rootfs/二级工具执行与授权仍未解决；#65/#28/#66 不关闭，全版本 full 门禁保留，不把三处 POST 禁止宣称为全部零上传或完整验机通过。

- 根复核冻结 `5cb2ed1`，Rust19（adapter15/面板gate3/HTTP-PG日常1）全部通过、0忽略，Clippy/fmt/core/diff通过；源码和Cargo/trust前后不变，专属PG55439按归属停止、PID消失/端口关闭，收据3effc37e…。专用Debian12 Bash5来源16运行（15过/1缺minisign跳过）、Policy4、原函数体FD组合2通过；旧/true四次回环POST，false/default零，97.541秒。独立最小真实chroot的24个stdin策略/参数组合通过，不将它当FD或完整rootfs证据。
- 两个guest单元限额运行期读回；FD/chroot峰值65,818,624B/15,495,168B，0OOM，结束后无进程/挂载/cgroup残留，SSH406重启0、同boot/swap0。21/3输入SHA与产品文件对应不变，收据1c6b1d3b…/96c4cad4…；最终只回填文档，不重复测试。完整负载故障矩阵仍待整条受控执行链就绪，本项不关闭#65或解除full门禁。

## PR #106 本聊天整合复核（2026-10-01）

普通合并最新 `b714629`，保留 #103 资产/周期流量、#104 首层固定和完整进度，在精确输入 `5ab55cc` 补 queued r6 显式门禁用例，不以当前 r7 常量覆盖它。adapter29+panel diagnostics13共42通过/0失败/忽略；fmt、core、workspace全targets Clippy、四workflow actionlint通过，自有PG55432已停。这是r7专项，不把此前442全量改称r7全量。

同一输入纯本地来源16/wrapper34/策略6/daily7/release28通过，共95运行、91通过/4既有条件跳过；官方十文件大小/SHA与固定清单一致。另独立审查在原作者 `fc3bb5a` 完成固定原函数体组合2项和23个边界检查，11份受验产品/测试/构建输入逐字匹配最终整合点；全部上游探测/serializer为替身，只允许自己的回环POST。Mac Bash3 stdin替身不证明Linux process-substitution FD或真实chroot，本聊天未重演作者guest或完整验机。

r7只控制入口及三份固定脚本的公开报告POST，保留隐私、轻量、回程、硬件参数、AGPL原文与修改告知；其他工具上传、二级来源/许可、rootfs及宿主副作用仍未总体验收。r2–r6历史精确版本回收、r4–r7 daily和所有full门禁保持；没有正式签署/发布/部署r7，没有触发暂停中的CI。

## 2026-10-01：NodeQuality 禁止脚本改动 swap（关联 #66 独立项）

- r8 固定 swap-policy 纳入签名 runner：入口不再 source 未调用的旧 helper，清理不调用 swapoff；内层硬件不再分配/格式化/启停/删除临时 swap。低于原950MiB宿主可用内存阈值或无法读取时退出70，入口读取管道原硬件状态，拒绝后不启动后续章节；不借隐私模式删除硬件能力。canonical来源/许可证和原post_cleanup/第455行保持，旧r2–r7报告及r4–r8日常兼容。
- 更正Issue和原审计中的证据边界：固定入口加载旧helper但没有check_swap调用；独立模拟只证明helper自身行为。实际硬件分支先分配文件，swapon失败need_swap仍0，原清理会遗留文件；新无特权负对照用9字节文件证明差异。公开报告开关仍通过原6项组合对照，源码/helper/构建的边界拒绝保留。
- host来源16、swap8（包含既有真实固定源的静态接线）、wrapper34、daily7通过；测试签名单项通过，release32运行/28通过/4既有条件跳过。原型空锚点及合成空函数问题由夹具拒绝后修正，最终来源与swap专项重验通过。Rust和专用guest结果待冻结后回填，没有正式签名/发布/部署/CI。
- 950MiB并非已实测Geekbench预算，未证明默认512MiB cgroup可跑完整硬件；full门禁和#66保持，根文件系统/二级工具、上传与许可和完整联合负载仍待验证。详见[独立验收](docs/acceptance/nodequality-no-swap.md)。

- r8根复核冻结ebf7302：Rust/API19全部通过/0忽略、Clippy/fmt/core/shell/diff通过，PG55439按归属停止且PID/端口已消失；收据c0ba55b1…。专用Debian12单次来源16（15过/1缺minisign跳过）、swap8和Bash5原函数体FD组合2通过，29输入前后不变。实际限额读回、峰值64,737,280B/11pids、0OOM，结束无进程/挂载/cgroup，SSH406重启0/同boot/swap0，guest收据841d30bb…。不运行真实swap、bootstrap或benchmark，不把该有限验收签为完整联合负载。

## 2026-10-01：真实注册 Agent 日常诊断故障矩阵

- 固定 `b0869ef`，专用 Debian12 以普通注册 Agent、真实面板/PG、TEST_ONLY 根验签 r8 和 systemd 完成七项：正常/重复、确认取消、Agent 重启、面板断连、低内存启动拒绝、磁盘不足、运行内存保护。Agent 重启保留原诊断 PID/启动时间；断连时真实 SQLite 保存一章节/一结果并恢复补传；取消/保护停止只留环境章节，状态与完整度独立。
- 真实 sing-box 回环持续1917请求/62,816,256B/0失败，PID与重启数不变；正常和资源故障心跳最大间隔20秒，主动面板断连为46秒，未伪称不中断。结束全部专属服务/PG停止、进程/cgroup/任务挂载清理、SSH406重启0/同boot/swap0；110文件证据索引59a63433…、总收据c85399d9…已复制核验。详见[独立验收](docs/acceptance/registered-nodequality-daily.md)。
- 构建缺缓存、768MiB构建OOM及两次验收脚本缺陷保留失败原始记录；1GiB离线构建成功，修正脚本后只计最终七项通过。产品代码不改；full门禁、完整链许可/副作用及真实完整联合负载仍待验，IP供应商与迁移/TCP证据没有补签。CI、正式签名/发布/生产部署均未执行。

## PR #107 本聊天整合与观察器复核（2026-10-01）

正常保留作者 `b0869ef`，将原堆叠PR改到main后整合 `de299906`，精确受验运行输入 `a083c099` 保留资产、流量、策略组、r6固定来源和r7报告开关；补 queued r7 显式门禁覆盖。adapter29+panel diagnostics13共42通过/0失败/忽略，fmt/core/workspace全targets Clippy/四workflow actionlint通过，自有PG55432已停。Python105运行、101通过/4既有条件跳过（来源16/wrapper34/策略6/swap10/daily7/release28），官方十来源大小/SHA匹配；这不是r8完整workspace或实机总验。

真实DEBUG观察器在Mac Bash3改写PIPESTATUS，旧守卫70可继续章节；GNU Bash5.2.15的旧70会停，两版旧7都继续。补修改局部pipefail并同步固定输出摘要：硬件或来源失败停止后续章节、没有正常完成标记；原EXITcleanup最终码1仍明确失败，不能误称最终码70。原作者guest仅对应旧ebf7302，不追认为新补修。调整可移植负对照后的 `bc7f751` 同一十项swap在Mac Bash3.2和独立GNU Bash5.2.15均全部通过；Bash5初次负对照失败原日志保留，不计通过。运行产品字节与a083不变，仅测试/文档变化。

r2–r7精确历史回收、r4–r8 daily及全部full门禁保持。只移除已定位入口/HardwareQuality swap路径，950MiB只是原宿主阈值而非GB5峰值或cgroup预算；rootfs、二级工具/上传/许可、宿主全副作用和完整故障/负载矩阵仍未验收，没有正式签署、发布、部署r8或触发暂停中的CI。



## 2026-10-01：NodeQuality 禁止运行时安装依赖（关联 #28 独立项）

- r9 使用固定 dependency-policy helper，把三份已 pin 脚本的包管理器、Geekbench、curl-impersonate、NextTrace、speedtest 和 stun 安装路径改为缺项拒绝；顶层 NextTrace 下载改为 rootfs 内执行权限检查。缺少工具列明名称并退出70，-n不能绕过；存在性检查不执行工具，也不证明来源、许可证或版本。硬件字符集函数、探测能力、参数和报告解析保留。
- IP/网络/回程三个章节采用与硬件相同的局部 pipefail，使 tee 不能掩盖失败。固定清理原文和正常结束第455行保留，真实退出观察器区分正常完成与中途拒绝；历史r2–r8报告及r4–r9日常入口保持，所有full门禁仍关闭。
- canonical来源/许可证完整保留；helper、输入、唯一锚点、输出长度和SHA全部核验。只处理已定位的安装路径，rootfs、二级数据、浏览器伪装及工具内上传/授权仍未解决。Debian12原始硬件源码语法通过；Mac Bash3不支持原始语法，完整语法验收使用Debian Bash5。最终专项与资源读回收据另补，不把实现完成当作实机总验。四个仓库工作流继续暂停，不正式签署、发布或部署。

## PR #110 本聊天看板整合复核（2026-10-01）

保留作者 `88bfb836` 并普通合入主线 `d343ae81` 的已验r8和日常验收文档；看板源码及19份产物逐字保持受验版本，Rust/插件/构建输入逐字保持主线。Bun27通过/843断言、TypeScript/Vite构建与已提交dist完全一致，10套真实Chromium均通过：新看板1440/768/390/320、展示页含键盘历史曲线、资产/接入/IP折叠、TCP/业务/策略组/NQ门禁及确认式取消。额外验证未登录仅请求/api/me、禁用localStorage、键盘导航、全屏拒绝、非法ID及零业务写入；截图实际目视。全部API写入由私有替身承接，不代表线上或真实Agent验收。

本项只改前端与文档，不重跑Rust/PG，也不把先前442或42专项称作该看板实机验收。保留旧诊断full门禁、精确账本与签名生命周期；四workflow继续暂停，未正式签署/发布/部署或触发CI。

## 2026-10-01：四平台单行 Agent 接入（整合前检查）

- 按本聊天追加要求实现 Shell（Linux/macOS/FreeBSD）与 PowerShell（Windows）入口、执行时最新兼容稳定版或显式版、实际 OS/CPU/libc 检测及真实签名目录下拉。一行复制、签名未缓存目标可选，native proof目录、升级回滚与首次失败保留身份重试均补齐。
- 隔离 PG 的 lib49/releases21/foundation7/platform2/updates2/setup3 共84项通过；all-targets Clippy、fmt/core检查通过。Python Unix入口19项与发布33项（4既有条件跳过）、PowerShell函数9项、Bun30项/850断言与1440/390接入/资产浏览器回归通过。PowerShell在Linux ARM64上执行，不代替Windows ACL/UAC/计划任务或PS5.1实机测试；macOS/FreeBSD没有真实主机安装验收。
- 此检查对应整合前输入。工作中主线更新到44ba222，AGENTS新增明确Agent二进制必须从GitHub下载、面板不得提供；后续整合保留其原生/镜像及服务器运营改动，入口调整直接GitHub下载并重验。上述结果不追认为整合后的验证，没有触发CI、正式签名/发布或生产部署。



- r9最终独立验收：冻结fc4e49f的host116运行/111通过/5条件跳过，其中新依赖11运行/10通过/1仅因Mac Bash3跳过完整语法；Debian12 Bash5同一新专项11全部通过。guest共80运行/79通过/1缺minisign跳过，45.658秒；真实限额256MiB/Swap0/Tasks64及各隔离属性读回，峰值76,435,456B/11pids，0OOM。结束无进程/挂载/cgroup，SSH406重启0/同boot/swap0；33输入前后相同，22仓库输入与整合后相同，收据5f320012…。
- 正常整合主线8ffcc44为9d99b28，双方PROGRESS均保留，运行产品/测试/构建字节不变。4b35801只补显式queued r8历史门禁，再过Rust19（adapter15/gate3/HTTP-PG日常1）及Clippy/fmt/core/diff，最终收据9c7014087734…。专属PG55439已按归属停止，PID不存在/端口关闭；早期Rust收据与host语法失败日志保留。具体来源/计数/未验证范围见[独立验收](docs/acceptance/nodequality-no-runtime-install.md)，不把有限夹具、日常矩阵或旧收据签为完整NodeQuality负载。CI仍暂停，无正式签署/发布/部署。

### PR #111 合并复核（2026-10-01）

- 保留作者 r9 提交、完整验机门禁与 r2–r8 历史兼容；本聊天未复演作者 Debian guest 或诊断完整执行。
- 刷新隔离树编译输入后，Rust/PostgreSQL 诊断专项 42 项、workspace 全 targets Clippy、fmt 与 core 边界通过；首次缓存结果由这次新执行证据覆盖，不用于最终证明。
- 来源 16、swap 10、实际 Bash5 依赖 11、包装器 34、daily 7 与 release 28 通过（release 4 项既有条件跳过）。报告夹具每次记录重导入测试模块导致原 20 秒预算超时；仅提前分派相同记录函数，回调原字节及全部断言保持，真实固定源报告 6 项在原预算通过。生产源码、摘要和预算不变，失败日志单独保留。
- 证据：本任务耐久目录 pr111-root-rust-fresh-local 和 pr111-113-review-20261001；CI 仍暂停，未执行不算通过，后续供给失败与其他上传问题由独立 PR 处理。

## 2026-10-01：NodeQuality 七份二级静态数据固定（关联 #28 独立项）

- 基于r9独立PR，r10把实际消费的IP国家表/DNSBL、Net国家表/省份表/ASN映射/iperf与speedtest目标表纳入固定提交、大小、SHA和完整源码包；Git blob身份与实际字节逐份核对。IATA变量在固定版本只有声明/赋值，未擅自加入无实际读取的CSV。
- 新data-policy helper在真实serve通路把七个精确curl表达式替换为固定格式、单引号转义的Bash内建printf，保留每个原字节和原解析/探测代码。不需chroot路径、解码器或临时文件；含单引号、命令替换、反引号和百分号的数据不能执行命令。缺失、篡改、FIFO、符号链接或非法输入拒绝，不回退在线main。
- 首轮新专项6全部通过；旧来源夹具中双架构下载计数仍写20，真实增加七文件后为34，已修正预期，保留失败日志。最终回归和Debian资源/清理收据另补。所有完整门禁保留；rootfs、二级工具、cookies/UA/广告、内层上传和目标授权仍待收敛，未发布/部署或启用CI。


- r10冻结e95fa7b完成host122运行/117通过/5条件跳过；新数据专项6全部通过。Rust19（adapter15/gate3/HTTP-PG日常1）和Clippy/fmt/core/diff通过，专属PG55439已停止/PID消失/端口关闭，收据c027d01d…。Debian12共86运行/85通过/1缺minisign跳过，57.311秒；实际限额256MiB/Swap0/Tasks64等读回，峰值71,118,848B/11pids，0OOM；结束无进程/挂载/cgroup，SSH406重启0/同boot/swap0，42输入不变（24仓库输入匹配），收据74114ba4…。
- 独立复现并开Issue #112：真实source-helper拒绝篡改数据，但固定加载器bash进程替换吞掉空输出失败，三个网络分支仍退出0。仅用固定函数体/Bash桩，无公网或上游基准；独立单元已回收。不能把供给拒绝等同整项任务正确失败，错误传递下一项单独修复。详见[静态数据独立验收](docs/acceptance/nodequality-pinned-data.md)及JSON索引，全部full门禁、rootfs/授权缺口与总故障矩阵保持；未正式签署/发布/部署/恢复CI。

### PR #113 合并复核（2026-10-01）

- 正常合入已验证 r9 主线并将 PR 改到 main，保存双方进度；产品输入逐字等同作者 r10，另继承同一报告记录回调优化，原 20 秒预算不变。
- 隔离树刷新编译输入后 Rust/PostgreSQL 诊断专项 42 项、workspace 全 targets Clippy、fmt/core 通过。七份新增数据的官方完整提交、Git tree/blob、原大小及 SHA 全部核对；实际 Bash5 依赖 11、数据 6、swap 10 和真实固定源报告 6 通过。
- 耐久证据 pr113-root-rust-fresh-local、pr111-113-review-20261001；作者 guest 不作为本聊天重演。供给失败吞码另由 #115 处理，完整验机门禁与 CI 暂停继续保持。

### NodeQuality r11：脚本供给失败传播（独立修复 #112）

- 五个加载入口先完整取得有界脚本并等待供给器成功，空结果也拒绝；尾部哨兵保留末尾换行，再按原参数及FD/stdin形状执行。报告头主调用增加失败退出，其余章节沿用既有失败保护，原清理第455行保持。
- 新helper嵌入同一runner、核验输入/输出/自身摘要，版本升为r11并显式保留r10历史兼容。完整入口门禁保持，不新增临时脚本或执行完整上游链。
- 本机初检10项通过，含旧/新负对照、空/部分失败、成功恰好执行一次、供给中取消、真实source-helper校验拒绝、原始主调用/清理/观察器与既有章节保留。首轮测试调用参数拼写错误已修正，日志保留；冻结后的本机、Rust和专用Debian证据另记，尚未以初检签收完整能力。
- 冻结6decf87后，本机132运行/127通过/5条件跳过，Rust/API19通过且Clippy/fmt/core/diff通过；专属PG55439停止、PID不存在、端口关闭。Debian有限组合96运行/95通过/1缺minisign跳过，另有原始chroot_run+生产shim+真实最小chroot30个旧/新对照全部通过。
- 两套单元实际读回256MiB/Swap0/Tasks64等限制，峰值71,622,656B/11pids和21,327,872B/5pids，max/oom/oom_kill均0；结束无遗留进程、挂载和cgroup，SSH406重启0/同boot/swap0。各44份输入不变，26份仓库输入匹配冻结提交。详见[供给失败独立验收](docs/acceptance/nodequality-source-failure.md)和JSON索引；未执行完整上游，全部full门禁和总验收缺口保持，未恢复CI或正式发布部署。

### PR #115 合并复核（2026-10-01）

- 正常保留 r10 主线、作者 r11 以及报告夹具原预算修复，PR base 为 main。供给完成并确认成功后才消费脚本；失败或部分失败不会运行部分正文，成功参数与字节保持。
- 刷新编译输入后的 Rust/PostgreSQL 诊断专项 42 项、workspace 全 targets Clippy、fmt/core 通过。真实固定函数的供给/取消/DEBUG 观察器专项 10 项分别在 Bash3 和实际 Bash5 通过，旧不安全分支为明确负对照，清理失败码与正常标记分别检查。
- 耐久证据 pr115-root-rust-fresh-local、pr115-116-review-20261001；本聊天没有执行作者 guest、上游完整负载或公网上传。CI 暂停、full 门禁及其他待验边界保持。

### NodeQuality r12：硬件百分位上传独立保护（关联 #65）

- 已确认 get_mark 的本地评分计算后另行向 mark.check.place POST 本机分数，既有公开报告开关没有约束该请求。新增固定 ranking-policy：默认/false保留本地CPU/GPU/内存/磁盘评分，在提交前返回；true保留原请求/参数/解析。每次先清除旧百分位，失败不复用旧值；文本说明未知原因，JSON保留null并记录上传许可布尔值。
- 版本升r12，嵌入helper并校验自身及输入/输出摘要，保留r11历史兼容；原始源码/许可证、全版本full门禁不变。代码初检来源16通过，Mac专项8运行/4通过/4因Bash3条件跳过，实际Bash5/HTTP验证待冻结后记录。未执行完整上游或绕过专有工具许可。
- 冻结2d08c61：Debian新增8项全部通过，实际回环HTTP旧/新默认对照、显式true、403/429/10秒超时/非JSON/缺字段及无陈旧百分位；整体104运行/103通过/1缺minisign跳过，80.001秒。实际256MiB/Swap0/Tasks64等读回，峰值70,348,800B/11pids、0OOM；结束无进程/挂载/cgroup，SSH406重启0/同boot/swap0。46输入不变，28仓库输入匹配冻结提交。
- Rust/API19通过及Clippy/fmt/core/diff通过，专属PG55439已停止/PID不存在/端口关闭。本机140唯一用例最终131通过/9条件跳过，另保留旧报告逆变换断言首次1次失败；643522b仅补剥离ranking补丁，单项复测及未完成组通过，产品字节不变。详见[百分位上传独立验收](docs/acceptance/nodequality-ranking-upload.md)及JSON；#65继续保留其他上传缺口，未恢复CI、正式签署/部署或签收完整能力。

### PR #116 合并复核（2026-10-01）

- 正常合入 r11 main 并保留作者 r12 与所有历史兼容，除本聊天进度追加外与独立受验树 82494ac 完全相同。
- 刷新编译输入后的 Rust/PostgreSQL 诊断专项 42 项、workspace 全 targets Clippy、fmt/core 通过。最终 Python/runner 组合含同源复用的真实 Bash5 排名 8 项，共 140 个唯一用例 136 通过、4 个仅 Linux root 安装器条件跳过；初次报告超时另存，回调优化后原 20 秒预算下 6 项通过，未放宽预算。
- 默认和 false 上传时本地分数保留、真实回环 POST 为零；true 保留原载荷，403/429/实际超时/非 JSON/缺字段不重试且不继承旧百分位。耐久证据 pr116-root-rust-fresh-local、pr116-python-final-local、pr115-116-review-20261001。
- 这是受控本地夹具与库验证，不是作者 guest、本机完整上游负载或线上签收；CI 暂停与 full 门禁保持。

## 服务器运营设置、公开看板与 GitHub Agent 下载（2026-10-01）

- 按本轮用户确认补齐服务器隐藏、自动续费记录、Agent 自动更新、离线告警、下载加速与网卡流量矫正的管理入口；移除服务器列表刷新旁的重复看板按钮，继续使用侧栏入口。自动续费沿用到期记录顺延，不向供应商付款。编辑自动更新开关保留已有采样/上传和公网识别设置。
- 新增 `0018_server_operations.sql`：看板/通知设置、离线事件、通知队列、独立流量矫正记录。看板默认私有，可显式公开；专用 `/api/dashboard/…` 接口排除隐藏节点及其历史，匿名响应按白名单隐藏设备公钥、主机名、地址、成本和管理配置，设置收回或节点隐藏后不保留被拒绝的前端快照。后台接口继续鉴权。修复验证中发现的共用读取逻辑影响后台诊断历史的问题，将快照清理限定为看板请求。
- 首次上报后的服务器持续离线达到阈值才触发站内事件；支持启动宽限、单节点关闭、恢复记录与全局开关。Telegram 令牌只写不回显，使用固定官方接口、禁重定向、不记录带密钥的请求错误；持久队列按顺序退避重试，最多 8 次，切换会话或关闭通知取消待发消息。外部发送与数据库提交不能组成原子事务，远端接收后本地提交失败仍可能重复，事件编号可辨认。
- 流量矫正保存周期/网卡范围内的调整量和原因，不改遥测、计数检查点、原始日累计及代理账本。保存期间新增流量保留；重复提交或并发矫正使用记录编号检查；跨周期不继承，修改重置日/网卡会作废旧调整，切回原范围也不复用。
- 按用户「Agent 必须从 GitHub 下载」更正安装和自更新：签名证明与编译时发布根不变，Agent 根据已签仓库/标签/资产名验证 GitHub URL，独立匿名客户端支持 HTTPS 镜像、来源/重定向限制、公网 DNS 固定、长度/摘要校验；镜像不接收设备令牌。bootstrap 在验签后获取 Agent，已签安装器只消费预先下载的文件；旧面板 Agent 下载入口返回拒绝，删除未再使用的面板原生下载模板，更新 OpenRC 离线测试夹具。运行时与配置仍从绑定面板读取。旧安装器或仅接受面板 URL 的 Agent 需要使用新签名 Release 手动升级衔接，不能覆盖旧签名资产。
- 本地完整 `cargo test`：450 通过、0 失败、15 项既有实机/平台条件忽略；真实临时 PostgreSQL 覆盖新迁移、权限、敏感信息过滤、离线去重/恢复、队列退避和并发、流量矫正与既有事务账本回归。之后仅补充「网卡切换再切回不恢复旧调整」断言，该组 4 项复跑通过。全 targets Clippy（warnings 为错误）、fmt、core 边界、差异检查通过。前端最终构建通过，Bun 27 项/843 断言通过；10 套 Chromium 流程通过，覆盖桌面/手机、公开访问撤回、节点开关、流量矫正及原有看板/拨测、接入、IP、代理业务、TCP、验机门禁、取消；新设置与矫正截图已目视。Python 发布/引导测试 41 通过，5 项因需要隔离 root 环境忽略。
- 未验证：真实 Telegram 送达、实际 GitHub/镜像的新签名 Release 下载及跨平台常驻升级、完整 OpenRC/root 安装、多平台实机和既有 15 项条件忽略。没有发布新 Release、生产迁移或部署；GitHub CI 继续暂停，本轮未触发，不宣称 main 全绿。下一步在全部并行任务统一收尾后按既定顺序恢复最终整合 CI，并用专用测试机/测试机器人做发布前验收。

## 统一延迟任务与完整服务器通知（2026-10-01）

- 按用户要求对照本地 NodeFlare，新增后台「延迟检测」页面：TCP/ICMP、目标及端口、间隔、线路备注、搜索/批量选择服务器、统一暂停/恢复/删除及默认分配新服务器。继续复用原 Agent 协议、四次测量、配置轮询、上传和看板曲线；单机独立目标保留，统一任务副本禁止经旧接口覆盖。
- 新增 `0019_latency_tasks.sql`，默认任务与服务器创建原子保存，全部目标合计限制 32 个；批量任务有任一节点超限则整体回滚。任务修订号避免旧表单覆盖新节点分配；变更目标需新 ID，取消分配后的迟到采样不会混入新目标。
- 按用户确认完整补齐通知：总开关、原离线开关、服务器到期提醒、按账单周期的网卡流量提醒、20 条资源规则、指定/全部服务器、窗口平均/每分钟超限、恢复与历史筛选。`0020_notification_rules.sql` 保留旧离线 ID、记录及待发队列，迁移为通用事件；新增提醒去重收据。到期和流量默认关闭，不预设资源规则，不执行付款、停用或命令。
- 资源判断要求完整有效分钟窗口，离线/缺失/无效数据不触发或伪造恢复；流式读取并只保留五项数值，按服务器独立事务处理。流量使用整数比较阈值，兼容矫正、网卡范围、方向及周期；同一提醒不因矫正回落或关闭再启用而重复。
- Telegram 增加话题 ID、纯文本模板、预览和保存后手动测试；限流、长度约束、秘密不回显、不继承代理、不重定向及失败重试保留。替换变量不递归，切换机器人/会话/话题取消旧待发消息；外部接受与本地提交仍不能保证恰好一次。
- 本地完整 `cargo test`：458 通过、0 失败、15 项既有实机条件忽略。收尾增加精确阈值/升级兼容测试及完善已删除节点规则处理后，面板库、新任务/通知/迁移及既有运营专项 58 项全部通过。最终 all-targets Clippy（warnings 为错误）、fmt、core 边界检查通过；新测试使用临时 PostgreSQL 和回环 HTTP，未发送真实 Telegram 消息。
- 前端构建通过，Bun 27 项/843 断言通过；5 套 Chromium 回归通过（新增监控、服务器运营、接入、展示、看板），新增流程覆盖 1440/390/320px 的失败保留、批量选择、暂停删除、模板校验、测试失败重试及事件筛选。收尾布局后复跑监控及运营流程通过，桌面和手机截图已目视，修正弹窗布局和原生输入框样式。提交包含前端产物。
- 文档见 `docs/monitoring.md` 与 ADR 0039。未验证公网 TCP/ICMP、Telegram 实际送达、多平台实机及既有条件忽略；未执行生产迁移、发布或部署。CI 继续暂停，未触发/重跑，不宣称 main 全绿。下一步在全部任务统一收尾后按既定安排恢复整合 CI，并以测试节点及测试机器人完成外部验收。

## main 推送前整合远程更新（2026-10-01）

- 按用户要求在 `main` 普通合入 `origin/main` 的 `a35bdcd`，保留本地监控提交 `d2f04c9` 和远程 NodeQuality r9–r12 历史。唯一冲突为 `PROGRESS.md`；保留远程完整记录，补回此前远程合并遗漏的服务器运营记录及本地监控记录。逐文件核对双方全部非冲突改动与各自提交一致，前端及产物未因合并改变。
- 整合工作树的 `cargo test --locked` 全量 460 通过、0 失败、15 项既有实机条件忽略；真实临时 PostgreSQL 覆盖数据库回归，测试结束后已停止。`cargo fmt --check`、全 targets Clippy（warnings 为错误）、core 边界与差异检查通过。未重复字节不变的前端、Python 或专用节点验收，不据此扩大已有验收结论。
- 只读核对发现 `imengying/sinan` 的四个远程工作流当前为启用状态。本次合并提交保留 `[skip ci]` 跳过自动运行，不修改工作流状态，不触发或重跑远程 CI；远端 CI 仍属未验证。未执行发布、生产迁移或部署。
### NodeQuality r13：未知 IP 评分与 IPQS JSON 修正（#117）

- 已在专用 Debian 12 用固定原函数和无网络错误响应复现：Scamalytics、AbuseIPDB、IP2Location、IPQS 四源均将缺失评分解释为低风险；IPQS JSON还读取错误数组成员。新建 #117，归入整改 milestone。
- 新增固定 ip-score-policy，先验证单一JSON、错误包络、类型/范围，再进入算术；ipapi字符串只接受固定格式，DB-IP只接受已知等级。真实零分和原阈值保留，文本显式未知，六源缺失JSON为null，IPQS改用自己的分数。原请求、完整来源许可证和所有full门禁保持。
- 升r13并保留r12历史兼容，新增10项专项。初测修正正则末尾换行和测试提取边界；本机6通过/4 Bash3条件跳过。冻结后的组合、Debian实际执行和Rust/API验证待独立收据，不将此项视为全链验收。

- 冻结产品66ec04b在Debian12实际运行新增10项全通过，组合114运行/113通过/1缺minisign跳过，194.199秒；256MiB/Swap0/Tasks64等属性实际读回，峰值69,853,184B/11pids，max/oom/oom_kill均0。结束无进程/挂载/cgroup残留，SSH406重启0/同boot/swap0，48份输入保持、30份仓库输入匹配产品冻结。
- Rust/API19及Clippy/fmt/core通过，专属PG55439停止且PID不存在/端口关闭。本机150唯一用例最终137通过/13条件跳过，共153次执行含保留的三次旧报告夹具超时；采样显示20秒持续推进至70条记录，473d592仅将夹具期限20改60秒，保留清理和断言，之后失败单项及未执行组通过，产品字节不变。详见[评分独立验收](docs/acceptance/nodequality-ip-score-unknown.md)和JSON索引。未进行完整验机、正式发布部署或恢复CI；UA/cookies/凭证路径和其他全链缺口保持待审查。

### PR #118 合并复核（2026-10-01）

- 正常合入已包含 #119 的 cdae763 主线、将 PR base 改为 main；保留作者 r13、r2–r12 历史与所有 full 门禁。报告夹具承接相同提前分派，恢复原 20 秒预算；作者曾用 60 秒的历史证据仍单独标明。
- 受验整合输入 27b7849：刷新编译输入后的 Rust/PostgreSQL 诊断专项 42 项、workspace 全 targets Clippy、fmt/core 通过；这不替代 #119 新功能的专项/完整整合验收。
- 115 个唯一 Python 专项 111 通过、4 个 Linux root 安装器条件跳过。实际 Bash5 六源评分专项 10 通过，原函数负对照、未知/null、有效零分、原阈值、IPQS 自身数组和原请求均检查；报告真实原文 6 项在原预算通过，签名/来源与包装器回归通过。
- 耐久证据 pr118-root-rust-fresh-local、pr118-review-20261001；作者 Debian guest 未由本聊天重演，真实源未执行完整负载或公网上传。CI 暂停及发布/部署限制继续保持。

### 插件目录与服务器执行边界（2026-10-01）

- 将“制品”改为只读插件目录，保留旧地址兼容。sing-box、NodeQuality、TCP 连接诊断按插件身份展示介绍，同一插件的版本和架构归入一个条目；Agent 单列为基础组件，未知组件不再误标为代理运行时。移除网页 Release 导入表单，不删除已鉴权的运维供给接口、签名校验或分发保护。
- 目录先选择具体服务器，只跳转、不安装、不启用、不创建任务。新增服务器内插件页，只读取所选服务器并沿用带服务器 ID 的 sing-box 启用接口；启用与安装明确区分。来源只读、读取失败、空服务器列表均不能形成错误启用目标。纯监控概况继续隐藏未启用的代理业务，诊断仍受原有门禁控制。
- 提交前跟进 main 到 44ba222，保留 #119 的公开看板、通知、服务器运营设置和 GitHub Agent 下载，以及 #118 的 NodeQuality 修复；没有恢复面板 Agent 下载、改写后端或数据库迁移。重新构建并同步 web/dist。
- 最终本地 Bun 33 项 / 894 断言、TypeScript/Vite 构建通过。已构建 dist 的 plugin-catalog、singbox-business、server-setup、tcpquality、server-operations 五组 Chromium 桌面/手机回归全部通过；新增检查包括多架构归并、逐版本真实架构、空集合/失败恢复、所选服务器唯一写请求，以及公开看板开启后目录与服务器插件仍需登录。目录截图完成检查，页面无横向溢出。
- 27 个本地文档链接目标、cargo fmt、core boundary 和 git diff 检查通过。最初仅挂载 web 导致既有单元测试读不到 Rust 字段源；新浏览器夹具首次漏写 NodeQuality 的 /reports 读取路径，均修正测试环境/夹具后完整重跑，首次浏览器失败日志保留在 sinan-plugin-catalog-validation。没有通过放宽产品校验或诊断门禁使测试通过。
- 本轮仅进行隔离回环夹具与前端验证，不代表生产 Agent 安装或新诊断能力实机验收；未运行完整 Rust/数据库回归、签署 Release 或部署。CI 按用户安排继续暂停，提交使用 [skip ci]。

### PR #119 旧版 Agent 更新兼容补修（2026-10-01）

- 新 Agent 在更新元数据请求中显式协商 GitHub 下载能力；面板先验证设备身份，缺省、未知或无法解析的能力参数不返回候选，避免已开启自动更新的旧 Agent 按面板地址下载新二进制失败。不根据版本字符串猜测能力，不恢复面板 Agent 二进制分发。
- 固定升级元数据路径保留同一面板、设备凭据、有界响应和原截止；其精确查询例外不放宽制品 URL 的同源、无查询及无片段校验。原签名、平台、稳定版本、失败抑制、退役与下载凭据隔离保持。界面说明旧 Agent 需先通过兼容的新签名接入入口手工迁移。
- 新增真实 API 夹具检查已开启自动更新时旧版及未知请求为空、同一版本的新请求取得签名候选、无效身份优先和已删除设备拒绝；现有升级回环夹具精确核对能力参数及身份传递，额外检查其他查询未被放行。本分工只完成直接 rustfmt、core 边界与 diff 检查，Rust/PostgreSQL 回归及前端构建由最终整合任务执行，尚不记为通过。耐久静态证据 pr119-upgrade-capability-20261001；CI 仍暂停，未操作签名、发布或部署。

## 2026-10-01 PR #124 设计文档整合

- 保留作者 `ff02fbb3` 并普通合入 `44ba2220`；新增范围仅为代理节点内统一创建直连与多条链路的设计，现有实现、旧 ID/凭据、两服务器 Reality、授权及历史计量不变。新页面、API、批量事务和加强删除逻辑仍待实现，未宣称实机验收。
- 本地核对五文档、94 个本地链接及现有节点/链路实现，合并和 diff 检查通过。文档变更未运行 Rust、PostgreSQL 或 CI；四个源码工作流仍暂停，不恢复 CI。

### PR #126 插件目录整合复核（2026-10-01）

- 普通合入已验证的 #114 候选 `04b8008`（含 `6b29061`），保存作者目录与双方进度。目录只读，旧制品地址兼容；不保留全局导入表单。目标架构查询和按架构导入的管理员运维 API、一次性签名接入、旧公开安装器拒绝、GitHub 更新能力协商及所有诊断门禁保持，运维文档按实际接口更新。旧导入页面浏览器夹具随页面移除，由目录回归替代。
- 整合源码 `c73b8a6` 的 Bun 33 项 / 894 断言、TypeScript/Vite 构建通过；19 份已提交 dist 与本次重建逐字相同。11 套仓库 Chromium 回归全部通过，覆盖目录、旧业务、接入、运营、TCP、完整验机门禁、确认取消、IP、资产、展示与看板；另两套独立桌面/手机夹具检查 401 清除一次性命令及通知秘密、无浏览器持久化、旧安装器警告、匿名零管理请求及隐藏/公开撤权后清理。目录两尺寸截图实际目视，页面无横向溢出。
- 28 个本地文档链接通过；Rust/插件/锁/部署与工具输入相对 `04b8008` 逐字不变。本分工未运行 Cargo/PostgreSQL、真实安装或生产接口；浏览器 API 全为私有回环替身，不代表实机签收。耐久证据 `pr126-web-local-20261001`；CI 仍暂停，未签署、发布或部署。

- 继续普通合入 #114 最终 `dfe66cd`：新增的进度和废弃导入页面夹具均正常处理，已受验的前端运行源码、19 份产物和其余 11 套浏览器脚本逐字不变；后端与部署输入完整保持最终 #114。

## 2026-10-01 PR #114 与 GitHub-only 安装升级整合

- 保留作者 `642a92d8`，普通合入已合 #111/#113/#115/#116/#118/#119/#124 的 main；旧作者正式 0.3.0/OpenRC 记录仅对应 GitHub-only 改动前，本聊天没有重演私有容器。根目录 sing-box 业务、诊断共用生命周期、签名与退役保护、流量账本保持。
- 新接入先确认普通安装器文件的完整签名 proof、摘要、唯一预下载契约，以及与 bootstrap 一致的 tag/version/raw/Linux 限制。旧/重复/篡改标识、签名但错配身份及不支持平台拒绝生成命令。下载加速按一次性 token 所属服务器读取并作为独立 shell 参数传入，不把 token 发往 GitHub/镜像；Agent 无本地 payload 也可按完整已签 proof 预下载，运行时仍核对本地已公布集合。
- 自动升级显式协商 `download_source=github`，认证优先；旧/未知/重复查询返回空候选。仅允许固定更新元数据路径携带此参数，普通制品 URL 仍拒绝 query，独立 GitHub 下载及签名/真实版本/回退/失败抑制/退役保护不变。
- 新入口验签后提前拒绝真实公开 `agent-v0.3.0` 旧安装器，真实 minisign 正式公开根验证通过，Agent 请求、安装器执行、面板请求均为零。root 文件归属仅此验证夹具模拟，不声称 macOS 完成 Linux 特权安装。必须由维护者提供新的不可变兼容签名 Release 并安排旧 Agent 可信迁移后，再部署新面板。本聊天未正式签署、发布或部署。
- 本地 Rust/PostgreSQL 完整 workspace/all-targets：`04b8008` 输入 463 通过、0 失败、15 项既有实机条件忽略；另 macOS umask077 原子链接专项通过。全 targets Clippy、fmt、core 边界通过。最终运行源码与该输入逐字相同，后续仅前端夹具/进度文字。此前两轮因新增夹具 deleted_at 类型与旧本地库存断言失败，修正后上述全量通过；旧失败日志保留，不计通过。
- Python bootstrap 11 项通过，4 个 root/独立安装方法在本机未执行；release 28 通过、4 个既有条件跳过；生成入口同步与四工作流 actionlint 静态检查通过。Bun 27/843、TS/Vite 86 模块、19 个 dist 文件复现通过，11 套仓库 Chromium 及两组独立权限负例通过，覆盖按架构导入、一次性命令、手机、旧安装器提示、匿名/隐藏/401清除。旧鉴权 mock 与静态响应头夹具原失败已保留并修复，产品输入未变。浏览器只连接私有 API 替身。
- r2–r13 历史、全部新 full/旧排队 full 门禁、17 来源与所有 helper 摘要不变；来源缓存共 687,969B 精确匹配本轮已核固定官方来源，不冒称再次外网下载。四源码工作流仍 disabled_manually，未执行不算通过；真实安装/跨平台迁移、TCP 完整链、通知与专用节点阶段总验另行签收。测试 PostgreSQL 仅 127.0.0.1:55432，已停止自己的实例。

## 2026-10-01 PR #126 嵌入产物整合验证

- 保留作者 `73828b1` 和已合 #114 的 `33a30854`。`9752b461` 新目录/服务器操作产品输入通过本地 Bun 33/894、TS/Vite、19 文件 dist 复现及 11 套仓库 Chromium、两组独立权限负例；均为私有 API 替身。
- 本聊天在同一冻结输入重新编译并验证真实嵌入前端的 PostgreSQL/HTTP 专项 1 通过，workspace all-targets Clippy、fmt、core 边界通过；自己的 127.0.0.1:55432 PostgreSQL 已停止。Rust/core/插件/锁/安装工具与 #114 已受验主线字节相同，未把 #114 的 463 全量冒称新 dist 的精确二进制全量。CI 继续暂停；正式安装/真实服务器启停/发布部署另验。

## 2026-10-01 PR #129 混合链路设计整合审查

- 正常保留作者 `61a872bc` 并整合已合 #114 主线 `33a30854`；仅调整规则与设计文档，不改运行代码、Cargo 依赖或数据库。混合链路新页面/API/迁移/三四段路径及恢复屏障仍待实施，旧 ID/凭据/账本与单运行时边界保持。
- 并行 ADR 对齐为 bootstrap 0037、服务器运营 0038、延迟通知 0039、混合订阅 0040，引用按各自主题更新。核对当前插件路径、原 API 与本地文档链接，并以官方 SIP002、Mihomo provider 与 YAML parser 文档核对格式/解析依赖的规划依据；没有新增解析依赖。
- 原作者九个原生配置 check 只对应作者历史固定运行时观察，本聊天未重演、未取真实机场订阅或运行代理，不能视为新能力验收。文档 fmt/diff/链接检查通过；CI 继续暂停，无发布或生产部署。

## 2026-10-01 PR #128 Netflix 请求判定与主线整合

- 保留公开作者 `072edf9` 并普通合入 `33a30854`，Netflix 两个原影片仍分别 GET、各 10 秒，不新增重试、UA、cookie 或凭证。传输、HTTP、正文形状失败进入明确失败原因，保留其他媒体、评分和不同任务的历史成功报告；正向 HTML 仍是合成格式契约，不代表真实播放能力。
- 本聊天发现打包漏校验新 Netflix helper：保留换行的合法 Python 篡改仍能生成制品，原无换行夹具只被嵌入格式拒绝。补 `pack()` 的精确 helper 验证与真实回归后，同一负对照以 SHA 不匹配失败且无制品/清单。Netflix helper/变换输入输出摘要保持，补修 source-helper 使用新摘要；作者 Debian 收据单独保留，未由本聊天重演。
- 真实 Bash5 下 Netflix14、来源16、数据6、评分10、报告编排6、包装器34、依赖11、swap10共107项通过；报告保持主线提前分派与原20秒。Bash3 依赖/swap 21项中20通过、1完整上游语法条件跳过。Bootstrap11通过/2条件跳过、Release28通过/4条件跳过；不把跳过记为通过。17固定官方缓存687969B与八helper摘要、r13输入182869B/r14输出186099B精确匹配。
- r2–r14 历史与所有 full 门禁保留，追加旧r13排队门禁夹具，r14 `deploy/bootstrap.sh` 精确再生成并检查通过；core/diff/脚本语法通过。Rust/PostgreSQL由主任务后续验证，本分工没有执行；四源码工作流仍暂停。耐久证据 `pr128-review-20261001`；未运行公网Netflix、上游完整链、真实swap/安装、公网上传、正式签署、发布或部署。

## 2026-10-01 PR #128 最新主线整合验证

- 保留作者 `072edf9e` 与补修 `6c64fae`，普通整合已合 #126/#129 的 main `82493264`。r14 打包前验证 Netflix helper 的精确摘要，合法 Python/末尾换行篡改负对照从产出制品变为明确 SHA mismatch 且零制品；旧真实报告夹具的 early --record 与原 20 秒预算保持。旧 r13 排队 full 显式拒绝，所有 full 门禁及精确已 Started 历史恢复保持。
- 冻结整合 `9040f2c`：本地诊断 Rust/PostgreSQL 专项 42 通过、0 失败、0 忽略；workspace all-targets Clippy、fmt、core、diff 与生成入口同步通过。自己的 PostgreSQL 127.0.0.1:55432 已停止。Python 与双 Bash 专项对应 `6c64fae` 固定源码及耐久证明，未取公网 Netflix、没有完整验机/播放实测或重演作者容器；没有把专项当作工作区全量。四源码 CI 继续暂停，未签署、发布或部署。

- 继续普通合入 #114 最终 `dfe66cd`：新增的进度和废弃导入页面夹具均正常处理，已受验的前端运行源码、19 份产物和其余 11 套浏览器脚本逐字不变；后端与部署输入完整保持最终 #114。

### PR #127 统一延迟任务与通知整合复核（2026-10-01）

- 保留作者 `b98fa5f2`，普通合入插件目录候选 `9752b461`，后者已包含 #114 实际 main `33a308`。导航同时保留只读插件目录、服务器内插件入口、统一延迟任务与告警通知；没有恢复全局 Release 导入表单，已鉴权的维护接口和一次性签名接入仍保留。ADR 编号对齐为 0037 签名接入、0038 服务器运营、0039 延迟与通知，保存双方进度。
- 受验整合源码 `3dea5d9`：Bun 33 项 / 894 断言、TypeScript/Vite 92 模块通过；19 份已提交 dist 与第一次及重复构建逐字相同。12 套仓库 Chromium 回归全通过，覆盖监控、目录、服务器运营、看板与展示、接入、资产、IP、完整验机门禁、TCP、确认取消和代理业务。独立桌面/手机负例检查旧修订 409、目标不可编辑、列表失败禁写、无效间隔零写入、只手动测试通知、429 不自动重试、401 清除秘密与匿名零管理请求，以及隐藏/公开撤权后的历史清理；通知令牌未进入浏览器存储。320 像素监控及 390 像素目录截图完成目视检查。
- 234 份受保护输入与 `9752b461` 逐字相同，包含 Agent/core/协议、NodeQuality、发布及部署工具、签名接入、GitHub-only 下载与升级能力协商；未放宽完整验机或退役保护。254 个本地文档链接、core boundary 和 diff 检查通过。所有浏览器 API 都是私有回环替身，不代表真实拨测、通知送达、生产安装或实机验收。
- 本分工仅完成源码审查和前端验证，未运行 Cargo/PostgreSQL；数据库事务、迁移、通知队列与最终完整 Rust 回归由整合任务另行验证。原独立夹具的定位器/可选 thread_id mock 修正及一次不存在的静态清单路径失败均保留，不计通过。耐久证据 `pr127-review-20261001/integrated` 保存准确源码与原日志；CI 仍暂停，未签署、发布或部署。

## 2026-10-01 PR #127 全量整合验证

- 正常保留 fork 作者 `b98fa5f` 和已合 #126/#128/#129，冻结 `addcdbe` 运行输入：本地完整 Rust/PostgreSQL workspace/all-targets 473 通过、0 失败、15 既有实机条件忽略；macOS umask077 原子链接、workspace all-targets Clippy/fmt/core 通过。自己的 PostgreSQL 127.0.0.1:55432 已停止。
- 已受验监控产品与私有前端输入逐字保持 `3dea5d9`：Bun 33/894、TS/Vite、19 dist 重复复现、12 套仓库与两套独立 Chromium 负例通过；仍未真实 Telegram/多平台/Agent 联合负载，未将本地当作 GitHub CI 或实机签收。
- fork 现有四个工作流 active，逐一固定原 YAML核对仅 push/pull_request/manual 与 agent-v* tag，无 pull_request_target 等额外自动事件；普通 push 最后提交带 [skip ci]，不改变 fork 状态。主仓库四源码工作流继续 disabled_manually，不重跑、恢复或把跳过记为通过。

## 2026-10-01：按五项批次核对开放 issues（第三批）

- 用户要求每五项处理后提交，批次期间不测试，最后统一测试；此安排覆盖旧逐项 PR 和逐阶段测试的交付方式。当前基线 bb9638b，旧会话目录已不存在，使用独立完整 worktree 保留原 checkout。
- #25/#27/#42/#47/#58 的原始源码缺陷已有实现；逐项核对代码与最终回归入口，见 docs/acceptance/issues-batch-three.md。未重复改造、未运行测试，也不据历史测试结果关闭含实机验收条件的 issue。
- 当前完整 NodeQuality 工具链/许可和原生 TCP 整链等缺口继续明确记录；后续批次完成后统一运行整合提交的验证，CI 暂停保持。

## 2026-10-01：开放 issues 第五批

- 核对 #61/#62/#63/#64 已有服务器资产、UTC NIC 周期、统一拨测与插件套餐实现；原 issue 的尚未实现描述不再对应当前源码，具体边界与最终回归入口见 docs/acceptance/issues-batch-five.md。
- #130 引入既有整改分支的显式启用/安全空配置安装与准备错误模型，保留当前监控导航和单服务器筛选；迁移改为 0021 避免版本碰撞。能力不再被当作启用，重试不延期，错误不伪造设备 ACK。
- 本批未测试、未构建 dist、未触发 CI、未部署。最终源/前端统一构建与回归后再记录验证结果，旧分支收据不作为当前整合成功证据。

## 2026-10-01 开放问题整改第二批（#18/#20/#22/#23/#24）

- 基于 `bb9638b` 核查五项原问题。#18 的有界 SQL/outbox、#20 的独立章节、#22 的类型化逐源错误、#23 的 provider 成功缓存、#24 的单一聚合入口与官方凭据 API 已有实现；本批保留这些实现、账本和历史数据，具体覆盖与限制见 [第二批记录](docs/acceptance/issues-batch-2.md)。
- 补修 #20 的独立章节发布：单章损坏 sidecar、类型错误或写入失败不再阻止其后章节保存；继续报告失败章、保留原文件/ZIP，支持后续重试，未成功的完整输出不发布。新增三项待运行 Python 回归。
- 补修 #22/#23 的排队请求时间：入口时间按真实已开始的明细计算，无真实尝试的批次不制造时间，缓存保留此前实际尝试时间；新 IP 保持未知。界面分别显示批次时间与上次真实尝试，新增 PostgreSQL 回归并扩充双 IP 总超时场景。
- 改变包装器输入使用新 NodeQuality r16 身份，保留 r14 与历史版本恢复；bootstrap 源已重新生成。没有生成包、正式签署、发布或部署，完整验机门禁不变。
- 按用户本次“五个完成再提交，最后测试”要求，此批未运行测试、构建或 Clippy，新增和既有专项都留到最终整合验证；前端 dist 也等待最终构建。CI 保持暂停，实机故障与真实来源验收未在本批重演。


## 2026-10-01：Issues 第四批 NodeQuality 源码准备

- 对 #28/#65/#66/#82/#120 逐项复核 main：前四项已有脚本措施，但完整二进制权利、来源、上传和受控取消验收仍不足，详见 `docs/acceptance/nodequality-batch4-readiness.md`；不记为已关闭。
- #120 引入 r17 原生 curl 身份保护，继承子 Bash，禁止 curlrc/额外 config 重新注入浏览器身份，固定输入/输出及 helper 摘要；main 原 Netflix r14 变换及所有许可原文保留。
- 新制品使用独立 r17，保留 r14/r16 的旧任务和日常/报告读取；full 门禁与 CI 暂停保持，不重建历史分叉的 r14/r15。
- 已编写并接入有意义的浏览器身份回归。按用户要求本批未执行测试、build、clippy 或 CI，全部批次完成后统一验证。

## 2026-10-01：Issues 第六批 NodeQuality 公共访问源码准备

- #121/#122：新增独立 r18 public-access policy；在线公共 Cookie 读取/重试、网页临时 key、公共备用 key 与受影响媒体的公共/固定认证路径在执行前明确返回未知，保留原来源、原始代码/许可证和结果列。
- 缺配置逐源写出 `not_attempted`、`credential_not_configured`、`Attempted=false` 与原因，受影响媒体及 ipregistry 不以旧值/空解析冒称成功，其他来源、面板正式 AbuseIPDB 配置及历史缓存保持。
- 对应来源尚无正式节点适配，能力恢复和真实认证验收仍待；不以暂停未授权请求宣称已完成授权接入。完整工具许可和 full 门禁继续保留。
- 旧 r14/r15/r16/r17 收集/取消及日常任务版本兼容；新版本不覆写历史签名制品。已编写固定源请求桩/未知JSON回归，按用户要求未执行测试、build、clippy 或 CI，等待最终统一验证。

- #131 用户权限组分配草稿停用后台轮询，保存后显式读回；与 #130 的显式安装入口一并编写桌面/手机 dist 回归。本批最后三项完成后提交，随后开始最终统一测试。

## 2026-10-01：开放 issues 六批最终统一验证

- 28 项按五批各五项及最后三项分别提交，批次内没有测试/构建；源码已有实现和本轮新增缺口分别记录，具体提交、范围与剩余条件见[最终验证](docs/acceptance/issues-batches-validation.md)。
- 最终本地最新结果：Rust/PostgreSQL 64 目标合计 490 通过/15 条件忽略；全目标完成后唯一原生 TCP 产品故障已修复，该 crate 所有目标 15 库+4 CLI 通过，其余目标输入未改。Python 20 套 333 通过/21 条件跳过，前端 36 项/901 断言、TypeScript/Vite 和 14 套真实 dist 浏览器通过。fmt、warnings deny Clippy、core boundary、diff、两种 bootstrap 一致性通过，自己的回环测试 PostgreSQL 已停止。
- TCP 已开始的原子写盘不再被探测截止中断，worker 独立按探测截止停止、最终排空有界进度保留真实尝试；CLI 与引擎共享绝对进程预算，不增加 60 秒总上限。真实 atomic gate 及旧规则负对照证明故障，不以扩大测试时间求通过；首失败和所有专项重跑日志保留。
- 夹具修复保持真实 ABI、缺少签名制品、授权草稿、安装健康 ACK 和固定源码唯一性判断；嵌入 dist 已同步。独立审查保留旧 NodeQuality 历史版本与 full 门禁。没有正式制品签署/发布/部署、没有关闭尚有实机/授权条件的 issue，四源码 CI 仍暂停。

## 2026-10-01：PR #132 四平台接入与主线前端整合核对

- 本聊天在独立工作树正常保留作者 `dd691656`，普通合入已验证的主线 `86e2ef40`、插件目录/监控任务与通知、ADR 0040 混合链路规划、NodeQuality r14 和脚本截止补修 `f871091`；跨平台接入决策编号调整为 ADR 0041，保留原作者与主线进度记录。
- 前端按 `1313a29` 的实际源码执行：36 个 Bun 用例、901 个断言、TypeScript/Vite 93 模块构建、19 份产物逐字复现，12 套仓库真实 Chromium 和两套独立浏览器负例全部通过。覆盖自动与精确版本、Unix/Windows 系统和 ABI 筛选、缺少兼容制品、失败后清除旧命令、一次性令牌提示、匿名只读、401 清除秘密、目录选择服务器与监控/旧业务入口；全部请求只用私有回环 API 替身，1440/390 布局与可滚动复制动作已检查。
- 普通整合后的前端源码、产物和活跃浏览器脚本仍与上述受验输入逐字相同。相对 `86e2ef40`，332 份既有 Rust/插件/Cargo.lock 输入逐字保留；13 个 Rust 例外仅为面板安装入口、已签版本选择、服务器接入路由及对应测试，Agent/core、协议、compiler、SDK、适配器和所有插件没有例外。66 个本地文档链接、core 边界及差异检查通过。
- Linux 旧 Preparing/full 检查点守卫与 PowerShell Unicode 引号修复由独立脚本审查继续整合，以上前端结果不认证其最终实现；原作者容器与正式根记录没有在本聊天重演。没有运行 Cargo/PostgreSQL 或远端 CI，没有原生平台常驻安装、完整诊断、上传、swap、正式签署/发布或生产部署。CI 继续暂停，NodeQuality full 门禁保留；Windows/macOS/FreeBSD 正式制品和实机验收仍独立待办。

## 2026-10-01 PR #132 PowerShell 字面参数整合补修

- 保留作者 `ae9d6961`。实际定位允许的 HTTPS 镜像路径可含 PowerShell 智能单引号，原 ASCII-only 转义会在外层 payload 赋值时提前执行路径内容。按固定 PowerShell 词法源码将 ASCII 单引号及 U+2018/U+2019/U+201A/U+201B 在参数、外层 payload 两处全部倍写，保留路径原值及单行编码/UAC 后重新下载、Git blob/SHA-256 校验与完整发布验签流程。
- 在本任务私有目录核对官方 PowerShell 7.5.3 macOS ARM64 归档 SHA-256 `f4fac5c72e8c09ba3b6fb8667f21b1d73556047819857fce7883268d02369cde`，与官方摘要文件及 API 一致；固定词法源码为 `b72c7ab1238c2d95b5c9004bca8399b8b3ca88ac`。真实 PowerShell 本地 11 项通过、0 失败、0 跳过；直接编译受验 Rust quote 函数的隔离小夹具，原转义负对照实际以 61 退出，新转义的五类引号/相邻引号/换行/中文/emoji 在六组双层 payload 中逐值恢复。其余用例覆盖真实 TEST_ONLY minisign、完整清单拒绝、历史身份/目录、回环下载预算与镜像匿名规则。
- 另只读下载固定官方 bootstrap Git blob，精确核对嵌入入口原字节；Windows minisign 0.12 官方归档和两架构 PE 文件均实际读回并核对固定摘要，没有执行 Windows 二进制。以上是 macOS PowerShell 7 函数及词法夹具，不是 Windows PowerShell 5.1、UAC、Windows ACL/计划任务或真实 Agent 安装验收；本补修子任务未运行 Cargo/PostgreSQL，也未签署、发布、部署或恢复 CI。

## 2026-10-01 PR #132 Unix 信任与旧 Agent 恢复复核

- 保留作者 `f45ade0` 及普通合入的 `ae9d696`、`dd691656`；旧正式 0.3.0 的 install.sh 保持原字节，只作完整签名证据，实际 Linux 执行器来自独立固定官方 bootstrap 内嵌源码。缓存的正式公开根、真实 minisign 与四文件 proof 已重新核对，旧安装器任意改一字节在 Agent 下载/执行前拒绝；未执行正式二进制或原安装器。
- 修复接入版本目录慢读可续期：仍保留原 30 秒预算，但各次底层读取共用绝对截止。私有回环负对照中，原源码在注入 0.15 秒预算后仍读取约 1.13 秒，新代码约 0.15 秒拒绝且错误不含令牌。原 Mac `/var` 与 `/private/var` 回滚夹具期望已改为精确解析后的路径；首次失败原日志保留。
- 旧 `75cd846` 对本地 Preparing 只核签名，重启后可直接执行，故新增标准库恢复守卫：已有旧 Agent 服务及状态端点须明确停止，只读配置给出的 SQLite 路径（缺省为旧 `/var/lib/sinan/core/state.db`），读取真实 WAL、要求既有 SHM、限制 SQL 期限并检测读取期间变化。完整 Preparing、缺 plugin/mode 的旧完整任务及未知/损坏检查点拒绝，原 JSON 不写。Started 只对旧 Agent 确切支持的 r2/原参数继续回收，其他原版本保留给兼容新签名 Agent，拒绝不兼容降级；daily 门禁放行不等于旧 r2 适配器支持 daily。
- bootstrap 与内嵌 Linux 执行器采用同源守卫，在接入前和激活前再次检查；最后门禁失败恢复原配置、不启动服务。入口不自行停止旧服务，不把单次快照宣称为整个迁移原子，也不保证其他特权操作者不能另行重启服务。已有旧配置严格 TOML 预检需 Python 3.11，SQLite 标准库仅在旧状态存在时使用；无法确认时明确拒绝。
- 受验产品 `3c3d146`：12 个私有 SQLite/WAL、旧缺字段、Started/daily、未知 JSON、服务/Unix socket、并发写入及回滚专项通过；Bootstrap 23 执行中 21 通过/2 root 条件跳过，Release 36 执行中 29 通过/7 Linux root 条件跳过。生成同步、core/diff 检查通过。真实正式 proof 的完整主入口夹具在私有 Preparing 状态下确认零 Agent 下载、零执行器调用，原 DB/JSON 不变；root 归属与管理器状态仅为隔离替身，不冒称 Linux 服务验收。
- 耐久证据 `pr132-review-20261001` 保存初始失败、源码摘要与原日志。此 Unix 分工未运行 Cargo/PostgreSQL、真实安装、外网通知或完整验机；最终前端、PowerShell 引用修复及 Rust 整合另行验证。CI 继续暂停，未签署、发布或部署；作者旧容器验收不替代这份新增恢复门禁的原生实机验收。

## 2026-10-01 PR #132 原生调用退出状态补修

- 实际复现不可执行的 minisign/Agent 文件配合历史 `LASTEXITCODE=0` 会被旧函数当作成功；四行 ED 结构的假签名因验证器未启动而绕过。每次原生调用先清空全局退出码，立即捕获本次 `$?` 和退出码，只接受本次调用确实成功且返回 0；验签工具未启动直接拒绝，真实返回非零的签名拒绝仍可尝试下一个可信根。
- 官方私有 macOS ARM64 PowerShell 7.5.3 的最终完整函数夹具 14 项通过、0 失败、0 跳过。另用补修前精确 `fdc60b8` 入口和新入口作真实双负对照：同一不可执行文件在旧验签与 Agent 函数均被接受，在新函数均拒绝且退出码为 null；同时实际运行坏签名/正确签名、错误根后正确根，以及原生子进程返回 0/7，保留智能单引号回归。
- 同步重生 UTF-8 BOM 入口，新本地 bootstrap SHA-256 为 `425fccaba9de63a4def8a27d95c468338da24a44c29734573bf378a97264acb0`；此前匿名 Git blob 下载核对只对应补修前入口，不能认证这个尚未由本子任务推送的新对象。Windows PS5.1/UAC/ACL/原生服务安装仍待实机验证；本子任务没有 Cargo/PostgreSQL、CI、正式签署、发布或部署操作。

- 上述前端冻结结果之后，继续普通保留 Unix `e8fb3de`（产品 `3c3d146`）与 PowerShell `6cbcf23` 两份独立补修。最终旧状态守卫只放行旧 0.3.0 实际兼容的 r2 Started，其他精确原版本留给兼容 Agent 回收；Preparing full/缺 plugin/mode/损坏状态拒绝，JSON 零写。PowerShell 修复五种引号和真实 spawn 失败后的旧 `$LASTEXITCODE=0` 绕过，签名与 Agent 调用都核对本次调用成功。两分工的 12 项旧状态及 14 项真实 macOS PowerShell7 结果分别记于其冻结证据，不冒称本聊天重复执行或 Windows 原生验收。
- 最终 Linux r14 自包含入口和 PowerShell 正式公开根入口从组合源码重新生成并核对同步；全部前端输入仍与上述 `1313a29` 逐字相同，因此不重复浏览器。最终完整 Rust/PostgreSQL 与组合 Python/PowerShell 回归由主整合任务继续执行；未执行不记通过，CI 仍暂停。

## 2026-10-01 PR #133/#134 节点跳转兼容补修

- #133 新增的服务器内节点/两跳链路链接带 `?server=N` / `?kind=chains`，原 App 仅精确匹配裸路径，真实 Chromium 直达原代码进入不存在页面。有限参数解析现在接到现有节点过滤/新建默认服务器或原两跳链路页签；未知、重复、非正整数及超出安全整数的参数拒绝。不存在或未启用的指定服务器不能悄悄改用其他服务器新建，能力读取失败也禁创建。
- 补修只包含 App、现有 Nodes/Groups 的有限入口和浏览器专项，未增加链路能力、修改数据库或协议。私有回环 API 上 1440/390 真实 Chromium 通过所选服务器、导航、实时路径变更、现有链路编辑及拒绝路径零写入；TypeScript/Vite 通过。完整工作区/CI和真实代理载荷仍另行验证。

## 2026-10-01 PR #134 缺环境重复安装保护复核

- 作者 `4990f1e` 运维 CLI 在既有项目缺少环境文件时，会先生成新数据库/管理员密码，再发现旧状态。真实 CLI 与私有 Docker 命令替身负例分别复现既有容器仍返回成功、孤立数据卷拒绝前已写新凭据；原失败日志保留。
- 补修在 `install` 写新环境前，只读查同一项目的全部容器与卷，任一既有状态立即拒绝并要求恢复原环境，未生成新凭据、未备份或构建启动。空项目正常初始化；原升级先完整备份、失败恢复原容器及旧凭据保留逻辑不变。Python 运维专项 9 通过、0 失败，均为私有命令替身，没有实际 Docker、生产数据、部署或 GitHub CI 验收。

- 同一保护继续覆盖已保留环境、只剩旧 `panel-data` 而无数据库容器的情况：作者仅查询 `postgres-data` 会漏掉此状态，真实 CLI/私有替身负例原代码返回成功；现在检查全部项目卷并要求恢复原服务后备份。新增专项后运维 10 通过、0 失败，不启动实际容器。

### PR #134 部署条件交叉审查补修（2026-10-01）

- 部署条件检查新增独立设备制品验签能力：只声明 sing-box 且面板已有合法签名运行时的旧 Agent，不能再被判为全部条件就绪；实际清单的 `artifact:minisign-v1` 前置保持不变，已有配置和流量仍保留。
- 新增 `signed_runtime_cannot_make_an_agent_without_signature_support_ready`：以公开 TEST_ONLY 根签署惰性运行时夹具，比较部署检查与已认证清单接口；不执行运行时。原未接入/缺制品回归同时保留。
- 独立隔离树以作者 `4990f1e` 为输入完成源码交叉审查、Rust 文件格式、core 边界和 diff 检查；本补修未运行 Cargo/PostgreSQL，新增用例需由最终整合树验证。未启动实际服务、安装、通知、签署正式制品或触发 CI。

### PR #133 TCP 截止与 NodeQuality 独立源码复核

- 固定作者 `57211580` 的原生 TCP 源码还存在就绪 future 绕过已到期计时器的边界：DNS 的一次 poll 延迟返回后，旧 `probe` 会在探测截止之后开始 TCP 连接。独立提取原函数、链接既有 Tokio 库的私有回环负对照实际观察到 1 次截止后调用和 1 条回环连接；补修后同一延迟解析返回下两者均为 0。新增解析后和每次连接前的绝对截止检查，DNS/连接使用截止与局部预算的较早值，迟到就绪结果按超时处理，最后一次迟到尝试保持部分结果，不制造完成或成功延迟。
- 新增 Rust 回归覆盖截止后 DNS 不开始连接、局部 DNS/连接超时不伪造成功及实际章节一致性；本分工未运行 Cargo/PostgreSQL，这些整合测试交由最终统一验证。没有放宽 60 秒总预算、发布预留、并发或尝试次数，也未修改固定 `b562effc` 已分发原生制品；新引擎源码不能冒称历史签名包已变更。
- 本分工在真实 GNU Bash 5.2.15、已核对摘要的 17 份固定官方源码及私有替身下通过 browser policy 8 项、public-access policy 7 项、source-helper 16 项。两个新增 helper 的打包验证和执行顺序、原许可证、受影响来源明确未知、r2–r17 历史及 r18 full 门禁继续保留。这些本地函数/打包夹具与作者完整 Rust/实机证据分别记录，没有执行上游完整验机、公共上传或正式通知。
- 原始反例、补修后回环证据及本地日志保存于本任务耐久目录 `pr133-nodequality-tcp-review-20261001`；四源码 CI 仍暂停，未签署、发布或部署。

## 2026-10-01 PR #133/#134 组合前端验证输入

- 从根整合 `c80b6b1` 建立独立树，保留作者 #133/#134、#135 完整安装保护、已存在的 0021/0022 迁移与新 0023，以及 CLI/节点路径/签名能力补修。只恢复已独立受验 `6c7d71a` 的订阅安全夹具，补充 401 界面凭据清理和匿名零管理请求；冻结后重生 dist 并重新执行组合回归，未把旧独立结果当作此次组合通过。

- 首次组合草稿浏览器专项保留原失败日志：原 5.6 秒停止策略草稿周期轮询、其他用量继续轮询、PUT/读回和更换套餐步骤已推进，但仍定位旧 `.copy-field` 并缺 #134 新只读订阅快照接口而超时。仅更新私有 GET 替身及新地址定位器，保留原全部草稿/读回/零额外业务写断言；产品源码没有因此改变。

## 2026-10-01 PR #133/#134 最终组合前端独立验收

- 冻结根整合 `c80b6b1` 的产品输入，独立测试输入 `5fc797e`：Bun 40 通过、0 失败、922 断言；TypeScript/Vite 104 模块、19 份已提交 dist 两次重建逐字相同，且与 c80 基线产物相同。实际组合 18 套仓库 Chromium 全部通过，包含新的 sing-box 安装状态/每服务器作用域、草稿原 5.6 秒周期保留/其他资源持续轮询/PUT 读回，以及节点参数/部署、有限节点和两跳跳转、统计、订阅每次重新授权/失败清除/401与匿名保护；服务器接入、监控、IP、TCP、完整门禁、确认取消、资产、目录、看板及既有代理业务一起受验。手机节点参数/订阅和统计截图完成目视复核。
- 修正的草稿夹具仅适配只读管理员订阅快照与新地址 DOM，原首次旧定位器失败已保留，不计为通过；产品代码没有为测试改变。运维 CLI 在本次组合再跑 10 通过、0 失败；127 个本地文档链接通过。已存在 0021/0022 迁移原字节保持作者 #134，新的 #133 安装表为 0023。
- 446 份根 c80 的 Rust/插件/锁/部署/工具/CI 文件输入逐字相同、零例外；本分工只新增两份浏览器夹具和进度，19 份 dist 不需变化。所有 API 与 Docker 命令均私有替身，不代表真实已签 runtime 安装、代理载荷、生产备份、通知送达或 Windows/多平台实机验收。本分工未运行 Cargo/PostgreSQL、没有 CI 或远端写入；后续 TCP 期限补修和最终完整工作区由根任务独立冻结验证。


## 2026-10-01 PR #133/#134/#135 最终主线整合验证

- 正常保留 #133 作者 `57211580`、已合入的 #134 主线 `fdbe6687` 与 #135 的旧任务/PowerShell 安装补修，冻结实际组合输入 `35f909729bae537f6b98bd63be826072f6fae175`。既有 0001–0022 迁移逐字保持主线，新 #133 安装准备表编号为 0023；包含节点/链路有限入口、缺环境重复安装拒绝、设备制品验签能力检查和 TCP 解析后/连接前截止补修，未覆盖作者或其他任务工作。
- 该冻结输入完整 Rust/PostgreSQL workspace 全 targets 共 504 通过、0 失败、17 条件忽略。新增两个迟到 DNS/连接回归及设备无验签能力不能显示就绪回归均真实执行通过；macOS umask077 原子替换专项、全 targets warnings-deny Clippy、fmt、core boundary 通过。17 个忽略仍需 Linux/systemd/root、原生 ICMP、正式上游 QUIC/代理运行时及 ACME 环境，不记为通过。自己的 127.0.0.1:55432 PostgreSQL 已停止。
- 前端组合受验输入 `5fc797e` 的 Bun 40 项/922 断言、TypeScript/Vite 104 模块、19 份 dist 两次逐字复现及 18 套真实 Chromium 全通过；当前 web 产品、产物与活跃脚本摘要对应受验输入，API 全为私有替身。组合安装/发布/运维 Python 在 `c80b6b1` 明确通过 88 个方法，其中包含真实 macOS PowerShell 7.5.3 的 15 项；另有 9 方法及一个 setUpClass 条件跳过，后者不计入 unittest 的 Ran，不能直接用 Ran 减全部 skip 计算通过。部署/tools/tests/CI 被覆盖输入逐字映射至最终组合，生成同步、PowerShell parser、shell 语法及四工作流 actionlint 通过。
- NodeQuality `57211580`/TCP 补修 `5cfe2a3` 的 9 套 Python/双 Bash 顶层执行 102 通过、7 条件跳过，含重复版本执行而非 102 个唯一用例；Bash3 public-access 的额外 subtest 跳过原记录单列，不误扣顶层计数。17 个固定官方文件/4 份许可证和 helper 摘要已读回相符；两个合法 Python helper 换行篡改都被打包摘要拒绝。真实原函数截止后 1 条回环连接与补修后 0 的证据保留；最终两个 Rust 回归已由上述完整工作区覆盖。
- 根额外实际执行新的接入 driver 合约 23 通过及受支持 Bash5 的 Netflix 14 通过。首次 Netflix 夹具硬编码系统 Bash3.2，因原上游关联数组不受支持而失败；原失败日志保留，私有 runner 仅切换为既有官方 Bash5.2.15，同源/同预算重跑通过，产品代码没有为测试改变。原前端定位器、独立 Python import harness 等夹具首失败及其明确修正证据继续保留。
- 耐久证据为 `pr133-pr134-combined-root-full-local`、`pr133-final-web-20261001`、`pr133-combined-python-final-local-20261001`、`pr133-nodequality-tcp-review-20261001`、`pr133-final-input-mapping-20261001`、`pr133-root-driver-netflix-final-local`。此前作者 490/15、#135 482/15、#127 473/15 等结果只对应各自冻结源码，不替代本次 504/17 组合输入。最终提交仅追加本节，运行输入仍对应受验冻结版本。
- 四个源码 CI 继续 disabled_manually，未执行不算通过；Windows PS5.1/UAC/ACL、多平台原生常驻安装、真实 QUIC/代理载荷/通知、完整诊断与新 TCP 签名包生命周期、旧业务迁移及阶段故障总验仍独立待验。获取官方上游运行时的只读下载超时，未认证或执行该二进制；相关条件忽略没有假称通过。NQ r18 仅源码准备，r2–r18 的 full 门禁、精确 Started 历史恢复及日常支持保持。公开仍 Agent 0.3.0；源码 Agent 0.3.1 为候选、panel/core 0.3.0。本聊天未签署、发布、部署、恢复 CI 或关闭仍欠实机/权利/正式认证适配条件的 issue。

## 2026-10-01：推送与 issue 关闭前整合

- 按用户要求核对当前 28 项正文的独立验收条件，已完成项目将在最终整合验证和推送后关闭，实际未完成条件继续保留。
- 普通合入主线 `fdbe668`，保留节点配置、统计、订阅与旧 Agent/PowerShell 接入保护；singbox 安装准备错误及修订检查保留，独立迁移从本分支 0021 重编号至 0023，避免与主线数据库版本碰撞。合并两侧进度，bootstrap 与前端 dist 从组合源码重新生成。
- 先前测试收据仅证明 `5721158` 的输入；组合源码仍待重新验证，不用于提前宣称测试或生产部署完成。


- 合并前实时核对发现作者已普通推进到 `196a9b3`（父项 `57211580` 与当时 main `fdbe6687`），故先暂停旧 head 推送并正常保留该作者提交。冲突只涉及双方追加进度、等价迁移编号说明及不同时间生成的前端入口；保留全部进度与新说明，嵌入产物采用当前组合源码对应且已逐字复现的 19 份文件，移除合并带回的两份过期无引用产物。
- 再次按 Git 对象核对：所有 Rust/插件、Cargo、web 源码/产物/活跃测试、部署、Python、脚本和工作流实际字节均仍等于受验 `35f9097`，差异仅 PROGRESS 与迁移说明，因此上述 504/17 及组合前端/Python结果仍适用于最终运行输入。未将作者的合并动作或待验表述当作额外 CI/实机通过，也未代作者关闭 issue。


- 作者随后推进 `897226f`，新增其 `196a9b3` 下 501/17 的独立收据、issue 条件核对及订阅 GET 浏览器夹具；这些作者结果保持明确来源，本聊天没有冒称重演。继续普通保留全部提交，合并作者语义地址定位和本聊天就绪状态等待，原 5.6 秒草稿暂停/其他轮询/PUT读回/套餐与零额外写断言不变。
- 合并夹具输入 `3d334a6` 在当前真实 dist 下 1280/390 Chromium 补跑通过（约14.9秒），其余17套活跃脚本及全部web产品/19dist逐字保持此前受验值；没有因文档和一份测试夹具变化重复Rust全量或重新编译。737份其余跟踪文件与冻结 `35f9097` 逐字相同，完整504/17及Python结果继续对应最终运行输入。新证据 `pr133-author-fixture-final-local` 和主完整目录内 `second-author-final-map.json` 记录差异；作者已由其他任务处理的issue状态不由本聊天重复修改。
## 2026-10-01：最新整合复验与关闭已修复 issue

- 普通整合 `196a9b3` 已推送；全 workspace/all-targets 单轮 501 通过、0 失败、17 实机条件忽略（67 目标），Python 21 套 367 通过/11 条件跳过（实际 PowerShell 15 项全过），前端 40 项/922 断言与 18 套 dist 浏览器通过。自己的测试 PostgreSQL 已停止，源码 CI 暂停保持。
- 按用户要求已附独立范围与验证依据关闭并实际读取 CLOSED：#4/#14/#15/#18/#20/#22/#23/#25/#27/#42/#47/#61/#62/#64/#120/#121/#131。PR #133 仍 draft/open，已无合并冲突；没有正式签署、发布或部署。
- 新增 #42 实际 dist 1440/390 各 31 场景共 62 首轮通过，保持所有产品与 dist 输入不变；已关闭并核对共 17 项，原 28 项剩余 11 项。其余升级、根因、完整工具/授权、正式节点接口、core 地区授权及专用节点首次安装缺口继续开放。最新记录见[最终验证](docs/acceptance/issues-batches-validation.md)，不把已有部分功能当作完整条件满足。


- 再保留作者 `5c6d31b`/`d02f677` 的新增 IP 矩阵、其关闭状态收据与全部进度；独立审查并在本聊天私有回环实际执行原字节夹具：1440/390 共62场景通过，204 HTTP/8刷新POST，浏览器错误/外部请求/未知API为0。它验证规范化面板API输出和明确标注的旧缓存防御；12个raw标签共用归一化失败输出，不冒称重复上游解析或新增负分/超范围矩阵。证据 `pr133-author-ip-confirmation-local` 对应当前 `230af345` 的79源码/19dist及新fixture SHA，未改产品。
- 全部运行输入仍对应504/0/17及组合Python/前端受验源。前述18套组合加新增IP专项合计19套活跃浏览器已覆盖，其中合并改变的授权草稿另在3d输入补跑；未重复其他字节未变的用例。作者17项issue关闭记录只读保留，本聊天未重复关闭/评论、也未恢复CI；剩余实机、权利、正式认证适配和full门禁边界保持。

## 2026-10-02：NodeFlare 服务器展示、上报与通知对齐

- 在 `main` 的 `4b4ee6e` 基线上完成本轮功能，参考本地 NodeFlare `7a7fc0ffe29dddaa21ef11c86e2990d84f3a6cc8`，保留 MIT 来源说明；决策见 [ADR 0047](docs/adr/0047-monitoring-refresh-history-and-channels.md)。没有新增运行依赖、修改工作流、执行生产迁移或扩大诊断实机范围。
- 服务器看板分离轻量状态与静态资料，分别每 3 秒、30 秒读取；旧接口继续 5 秒兼容。加入六档资源历史窗口、实际均值/极值/有效样本数和采样范围，保留无指标与完整缺桶的断线。隐藏页取消请求，恢复后先将过期状态标为待确认；切换节点、窗口、公开范围或撤权时丢弃旧响应，隐藏设备不能被旧请求恢复。费用支持统一币种、周期成本及已知周期/到期日的剩余参考值，缺失价格和汇率保持未知，匿名看板不显示财务数据。详见[展示数据说明](docs/server-display-data.md)。
- 新 Agent 独立执行默认 1 秒采样、3 秒实时上报与 60 秒历史批量写入；新接口协商与能力声明兼容旧严格消息和旧设备。实时接收不删除本地待确认样本，历史与去重收据、精确网卡账本提交后才 ACK；追补批次间释放退役读锁。后台支持每台服务器 15–3600 秒历史写入间隔及全局 1–3650 天保留期，缩短期限需确认；默认 30 天，近两小时原始、七天内分钟、三十天内五分钟、更早小时。聚合采用各指标有效计数、加权均值和极值，旧分钟末值标记局限；独立维护有时间和批次预算，未回填的数据来源不提前删除。详见[遥测验收](docs/acceptance/telemetry-history.md)。
- 新增固定 Frankfurter v2/v1 HTTPS 来源的每日汇率缓存：成功后 24 小时刷新，失败保留真实旧值并一小时重试，管理员手动刷新有 30 秒冷却。数据库租约协调并发和重启，展示来源、数据日期、抓取时间及过期状态；普通读取不请求外网，不使用硬编码外币估值。
- 通知增加通用 JSON、Bark、Discord、Slack、企业微信、钉钉、飞书、ntfy、Gotify 九类 Webhook 预设，与 Telegram 独立排序、重试、取消、测试和记录结果。地址、头部及模板只写不回显，错误脱敏；资源告警采用真实加权窗口和持续最低值，缺测、网卡范围改变或持久水位不足保持未知。钉钉/飞书不生成动态签名，外部接收仍为至少一次投递。详见[通知验收](docs/acceptance/notification-alignment.md)。
- 修复基线合并带入的节点页/统一资源页混用，保留服务器筛选、严格查询参数、统一链路创建和详情；去除构建带回的过期无引用前端产物。原 `0023_runtime_operations.sql` 与已合入安装迁移重号，仅把其原字节追加为 `0031_runtime_operations.sql`，其余既有迁移不重编号；31 个迁移版本唯一。曾手工应用旧分支 0023 运维迁移的环境仍须先备份并核对历史，不能直接改写校验摘要，见 ADR 0043。
- 最终完整 `cargo test --workspace --no-fail-fast`：88 个目标（含文档测试），**599 通过、0 失败、20 条件忽略**；`cargo clippy --workspace --all-targets -- -D warnings`、`cargo fmt --all -- --check`、core 边界与差异检查通过。专用回环 PostgreSQL 18.6 验证迁移、并发、恢复、权限、历史聚合、汇率缓存和通知；HTTP 仅使用私有替身。前两轮原失败保留：旧迁移快照遗漏新列默认值、运维夹具未显式启用插件；分别补准确默认预期和管理员启用调用，保留完整旧字段/凭据/授权/账本相等与所有能力门禁断言后，最终整轮通过。日志为本机 `/tmp/sinan-monitoring-rust-verified.log`、`/tmp/sinan-monitoring-clippy-verified.log`。
- 前端 Bun **49 项通过、987 个断言**；TypeScript/Vite 125 模块通过，19 份 dist 重建逐字相同。15 套相关 Chromium 夹具通过：展示四套、通知三套，以及遥测设置、节点路由、混合链路、新建接入、命令状态、确认取消、NodeQuality 门禁和代理业务；覆盖桌面、390 像素及展示/通知的 320 像素。NodeQuality 夹具原先误把独立 IP 查询完成当作任务列表已读回，现等待实际排队记录，未删除门禁断言。浏览器均为私有回环 API，不代替设备实机；截图位于 `/tmp/sinan-monitoring-screenshots` 和 `/tmp/sinan-operations-screenshots`。最终 Vite 仍提示主后台块约 502.64 kB（gzip 151.23 kB），服务器展示异步块为 60.20 kB（gzip 20.41 kB）；后续按需要单独优化后台拆包。
- 20 项忽略仍需专用 root/systemd、ICMP、固定正式运行时及 ACME 等环境；长期多设备高频负载、真实长断连、多平台常驻、生产历史升级和真实通知渠道送达未验收。独立实时上报需升级 Agent 后生效，旧版继续兼容。本轮仅本地源码提交，不推送、签署、发布或部署；CI 继续暂停且记为未验证。下一步在独立授权的测试环境验证升级、长期采样与实际渠道，不以此次本地结果签收诊断实机能力。
- 本次启动的专用回环 PostgreSQL 已停止；153 个相关本地文档链接检查通过。


## 2026-10-02：开放 issues 与同期 PR 的统一补修

- 用户要求修复开放 issues、检查 PR、完成后一次 PR，中途不测试。本轮实现只阅读与修改源码、编写回归；没有逐项运行测试、构建、fmt、Clippy 或检查器。全部源码整合后才统一验证，结果随后回填。
- 新增精确目标授权的轻量周期拨测：线路/地区/地址家族与实际采样家族，旧配置无授权不执行，目标身份冻结，改向必须重新人工确认；匿名输出剥离授权来源/范围，旧 Agent 收禁用旧格式。断连撤销仅能在同步后获知，已知到期由本地严格执行；不把 24 小时缓存当作即时撤销。
- NodeQuality r19 制品自身在写工作目录/调用上游前拒绝新 full，禁旧 rootfs/nexttrace 在线回退，嵌入严格未准入说明；静态执行库存工具有界只读、不展开或执行，身份匹配不等于许可。新增节点私有正式 Ipregistry/DB-IP self 适配，缺正式授权的媒体仍未尝试/未知，旧报告与 r2–r18 精确历史回收保持。
- Reality 白名单补本地 DNS/预传输/观察进度，不重试或加预算；原地升级驱动校验双端真实版本/新 Agent PID、旧配置版本/独立运行时不变；安装失败直接保存私有证据，不把待发布当健康。公网根因、真实签名升级/安装、完整诊断权利与实机联合负载仍独立待验。
- 同期作者正常合入 PR #136（main 4b4ee6e），本分支普通保存全部作者祖先及运维/命令取消/机场订阅/混合链路。保留原迁移0001–0023原字节，新五项迁移顺延0024–0028；补旧schema真实PG升级/重复执行及历史保留回归。命令退役补已登记进程清理屏障，失败不清身份，Requested/Stopped/Clearing恢复只清理不重执行；源补修239f072普通合入。订阅传输等价与解析器缓存补修一并收尾。
- 四个源码工作流继续 disabled_manually，未恢复/触发 CI，不作正式签署、发布或部署，不以源码合入关闭缺外部证明的 issue。详细范围与最终证据见 [统一验收](docs/acceptance/issues-integration-20261001.md)。

- 统一最终验证已完成：冻结d50ca445完整Rust/PostgreSQL599通过/0失败/20明确条件忽略，80结果组；umask077专项1、全targetsClippy/fmt/core通过。自己PG55432已停止，未动其他实例。首次runtime运维两例因夹具仅声明能力却未显式启用被正确409拒绝，补真实管理员启用和拒绝负例后完整复验通过，未放宽产品门禁。
- 最终web2cdb7ce的45Bun/970断言、两次TypeScript/Vite115模块、19dist逐字复现及22套实际Chromium全过；修正同期主线App/Nodes资源路由/统一直连链路页和概况运维入口。脚本去重308唯一方法中299通过/9方法skip，另1class skip；实际PS7 15/15和正式IP helper12/12包含其中，TLS/API仍为替身。真实双Bash166执行含150完整方法通过/15方法skip/1含10子例skip，不叠加唯一总数，Bash5 90/90；canonical17原文件687969B保持。
- 原失败证据与补验分目录保存：新库存元组/driver循环快照为fixture补正，Unixbootstrap内嵌r18已按r19重生；PS字节保持且actionlint过。耐久证据issues-final-rust-repair1-20261002、issues-final-web-20261002、issues-final-python-summary-20261002按源SHA映射。本轮只创建一个组合PR；仍缺正式API/权利/公网Reality根因/跨平台与实机安装升级/新TCP整链证明的11个issue保持开放，不以本地通过代替签收或恢复CI。


## 2026-10-02：同步上游后继续 Cloudflare DDNS

- 在 main 普通合并上游 `c82fe1e` 的 9 个新提交，保留本地 `8e6c543` 的展示、遥测、汇率和通知功能；未推送或触发 CI。合并覆盖命令退役清理、周期拨测授权、订阅转换及 NodeQuality 源码门禁，双方原验证收据保留来源。
- 上游 0001–0028 迁移逐字保留：运维为 0024，命令/订阅/链路/验证顺延到 0028；本分支新增汇率、遥测历史、通知分别改为 0029、0030、0031，SQL 原字节不变。监控 ADR 改为 0047，保留上游 0046 授权拨测。曾部署本地 `8e6c543` 的非上游编号，或旧分支 0023 运维迁移的数据库，必须先备份并单独核对/协调迁移历史；不能直接启动、改摘要或据此宣称已验证生产升级。
- 合并后独立完整 Rust/PostgreSQL：625 通过、0 失败、20 条件忽略；全 targets Clippy、fmt、core 边界与差异检查通过。测试日志 `/tmp/sinan-monitoring-ddns-upstream-test.log`，Clippy 日志 `/tmp/sinan-monitoring-ddns-upstream-clippy.log`。运行输入不含随后开发中的 DDNS；后续增量必须独立验证。
- 前端 Bun 51 通过、1018 断言；TypeScript/Vite 构建通过，产物从合并源码重生。实际 Chromium 回归节点路由、混合链路、服务器接入、监控配置、命令生命周期 5 套通过，含手机布局，接口均为私有替身。上游其他 Python/实机收据仅保留，没有声称本次重跑。
- 下一步按照用户授权接入 Cloudflare DDNS，继续使用已有 Agent IP 上报。CI 保持暂停，真实 Cloudflare 写入、生产迁移与诊断实机能力均未执行或签收。


## 2026-10-02：Cloudflare DDNS 面板插件

- 在上游同步提交 `dc65301` 后实现用户授权的 DDNS，并按进一步要求做成独立插件。后端位于 `plugins/ddns/panel/`，经既有 plugins 桥注册路由与后台任务；前端位于 `web/src/plugins/ddns/`，提供插件目录、服务器插件页、全局与单服务器入口。复用 `server_plugins` 按服务器显式启停，默认未启用，不要求额外 Agent 制品，不改变 sing-box 插件或诊断准入。
- 复用 Agent 已有静态 IP 消息，只在面板增加通用接收时间；不把旧数据库缓存当作新报告，不给 Agent 下发 Cloudflare 凭据。支持 A/AAAA、泛域名/IDN、TTL、Cloudflare 代理、周期及手动同步；自动选择对应家族有效公网地址，优先保持仍在本轮上报中的上次成功地址。离线、过期、无地址、删除或退役时保留解析。
- 每条规则保存写入后不回显的 API Token；固定 Cloudflare HTTPS，禁代理/重定向，响应限 256 KiB，单轮 20 秒，租约 60 秒，最多 32 规则及两项并发。唯一同名记录需明确接管，重复或 CNAME/NS 冲突停止；PATCH 保留其他字段，创建带规则标记用于丢失回执恢复；无变化不写 DNS，失败退避并保留上次成功。暂停、停用及删除规则不删除远端记录，进行中编辑/停用拒绝。
- 新增 0032 迁移；同步完成后的 0001–0031 字节保持不变，旧业务迁移断言补新接收时间为 NULL，仍完整核对原字段、凭据、授权与账本。参考 IPFlare `19bcf463a3dfdc3d13a9e61dd22bbf1a6fc68c80` 的行为思路并核对官方接口，实现独立编写，不复制参考项目 GPL 源码，无新增依赖。决策、边界及用法见 [ADR 0048](docs/adr/0048-cloudflare-ddns.md) 与 [DDNS 使用说明](docs/ddns.md)。
- 前端 Bun 53 通过、1036 断言；TypeScript/Vite 129 模块通过，21 份 dist 两次重建逐字一致。DDNS、插件目录、节点路由、服务器接入 4 套 Chromium 通过；DDNS 覆盖 1440/390/320 像素、按服务器启用/停用、目录跳转、限定服务器、失败保留草稿、空 Token 编辑、手动同步、暂停及删除，目录含匿名保护。修正表单带提示的标签定位及 320 像素下继承 340 像素最小宽度导致的溢出，没有移除原语义断言；截图在 `/tmp/sinan-monitoring-screenshots`，已目视复核。
- 首次最终插件整轮 Rust 为 637 通过、1 失败、20 条件忽略：既有 `diagnostic_end_to_end::chain_gate` 在等待拒绝回执时超时。保持源码、预算和全部门禁断言原样，独立复查该目标 3/3 通过；不将该超时归因为已确认的产品问题或冒称已修复。原失败日志 `/tmp/sinan-monitoring-ddns-plugin-final.log` 与复查 `/tmp/sinan-monitoring-ddns-diagnostic-recheck.log` 保留，最终完整复验另行记录。
- 第二轮整体验证的该诊断用例通过，但既有订阅来源用例等待任务完成超时，总计仍为 637 通过、1 失败、20 条件忽略；订阅目标独立复查 3/3 通过。检查发现该 HTTP 测试夹具没有启动生产 publisher 的每秒任务调度，只依赖繁忙时可能跳过的 API 唤醒；在 `settled` 中补同频率调度，保留 20 秒截止与全部身份、凭据和历史断言，不改产品调度器。原日志 `/tmp/sinan-monitoring-ddns-plugin-verified.log` 与 `/tmp/sinan-monitoring-ddns-subscription-recheck.log` 保留；夹具缺口是源码确认的条件差异，不据此将两次超时全部归为同一原因。
- 补齐夹具调度后，订阅专项 3/3 通过，最终完整 Rust/PostgreSQL 工作区 638 通过、0 失败、20 条件忽略（93 个结果组）。包含 11 个 DDNS 单元/数据库/提供方替身用例及 2 个 DDNS HTTP 集成用例；先前两项超时用例也在整轮通过。最终日志 `/tmp/sinan-monitoring-ddns-plugin-complete.log`，前端受测产物与 `/tmp/sinan-ddns-dist-manifest.json` 的 21 个摘要全部一致。
- 本轮新增依赖为零，Agent 协议与设备端代码未改。Vite 的主后台块仍有大于 500 kB 的提示（509.50 kB，gzip 153.48 kB），DDNS 为异步独立块（12.67 kB，gzip 4.93 kB），不把该提示记为构建失败。66 个相关本地文档链接检查通过，最终全工作区、全 targets、warnings-deny Clippy 通过（`/tmp/sinan-monitoring-ddns-plugin-complete-clippy.log`），fmt、core 边界与差异检查通过；原有 31 条迁移再次逐字核对不变。
- 本次专用回环 PostgreSQL 55439 已停止，没有操作其他实例。20 项条件忽略仍需 root/systemd、ICMP、正式代理运行时及 ACME 等环境；真实 Cloudflare 凭据、DNS 写入和传播、长期动态 IP 变化及生产历史迁移均未验证。下一步是在单独授权的测试域名验证实际变更与恢复；本轮仅本地 main 源码提交，不推送、发布或部署。CI 继续暂停，未执行部分记为未验证，不据此签收诊断实机能力。
