# sing-box 双端插件与安装流程验收

本次在统一分支 `remediation/all-diagnostics-20261001` 交付。面板插件管理节点、代理用户、链路、授权、订阅和套餐；设备上的普通 Agent/适配器负责签名下载、安装、配置对账及独立运行时健康确认。采用 Sinan 内的面板插件与设备插件，没有对接额外外部面板产品。操作说明见[安装与控制流程](../singbox-installation.md)，业务范围见 [ADR 0061](../adr/0061-singbox-plugin-lifecycle.md)。

## 实现与本地验证

设备声明能力不再自动启用业务。管理员明确启用会安排首次发布；重复启用不延期待办，也不产生重复部署。缺制品、平台不匹配或验签准备失败与设备应用结果分别保存，不把面板准备成功当作设备已安装。仅当前目标版本的错误影响当前安装状态；已认证心跳能恢复丢失应用回执后的健康确认。

界面顺序为接入服务器 → 启用并安装 → 创建节点/两跳链路 → 创建代理用户并授权 → 等待设备应用。链路归在代理节点分类内，按入口或出口服务器筛选，展示两端服务器、目标/已应用版本与状态。创建链路只保存拓扑，不自动授权。旧响应、查询失败、相等但仍 pending 的版本均不会显示已应用；这不认证公网可达。策略组授权草稿不再被五秒轮询覆盖。旧用户 ID、凭据、订阅 URL、授权和历史表名未改。

冻结 `e5d03c38ab0e46e2759beff26effb18418db8483` 的面板 Rust/PostgreSQL 专项 80 通过、0 失败、1 个明确实机条件忽略，面板 all-targets Clippy 通过。覆盖首次空配置、重复启用、缺签名制品、监控/离线与旧错误版本、真实认证心跳恢复、业务迁移、策略/套餐、订阅与计量。该普通测试中的实机用例未执行；此前旧客户端实机记录见[单独验收](imported-subscription-runtime.md)，不能将其算作本次执行。专属 PostgreSQL 55439 已停，PID 不存在、端口关闭，412 个受验源码输入未变。

`fa030155` 仅修正测试的 Rust 格式、导航 link 定位和文档措辞；fmt/core 分层检查/diff 通过。真实 dist 的桌面/手机业务、授权草稿跨 5.6 秒等待、服务器接入及策略/套餐/链路创建测试通过。随后 `081d383` 补两端状态和筛选并重新构建；`fbfbeb3` 的业务及 groups 界面测试通过，dist 摘要单独记录。它们没有修改设备/面板 Rust 或迁移输入；不能把后来的前端构建冒称为 guest 中运行的二进制。

原格式检查失败、旧 groups 按钮定位失败，以及新增双端成功状态的 strict locator 失败均保留；对应格式/link/双元素断言修正后补验通过，产品语义未为通过测试而放宽。

后续统一分支整合保留主线 `0019_latency_tasks.sql`、`0020_notification_rules.sql`，把本支未正式发布的安装准备表迁移后移为 `0021_singbox_installation.sql`，SQL 原字节保持。上面与下方的冻结源码及专用节点收据实际执行的是旧 `0019_singbox_installation.sql`，没有执行新编号组合；新整合版本须另补全部迁移验证，历史结果和证据 JSON 不改写。

## 专用 Debian 12 的实际安装与恢复

专用 ARM64 guest 为 Debian 12、2 CPU、1536 MiB 内存、8 GiB 磁盘、无 swap、systemd/cgroup v2。冻结源码 `fa030155b3daf5a1829dfcd75e1a4f77239c6daa`，归档 SHA-256 `251302cfd9949839a786b529f19e4cc2eea6bc4d67e50be53584d57a97af599c`。离线受限编译 Agent/Panel 20.24 秒通过，1 GiB/Swap0/Tasks128/CPU与IO10/OOM500/PrivateNetwork/PrivateTmp；内存上限事件 1751，但 oom/oom_kill 均 0，不能称零上限事件。采样最小可用内存 566398976 B、磁盘 2238713856 B。

首次准备因没有 minisign 在创建业务服务/账户前拒绝，记录仍在。仅补 Debian minisign 0.11-1（一个新包，无升级）；第二次准备发现本地 PG 端口先于数据库就绪及 Debian userdel 自动删除空同名组的测试脚本问题，Agent/Panel 尚未启动。停止 PG、核对无残留后，在新 r3 命名空间修正这两项；不复用旧数据库或失败结果，不重新编译产品。

最终有限安装验收运行 3 分 24 秒，实际退出 0，阶段和清理均通过：

1. 普通 Agent 真实接入并声明 singbox 与签名能力，42 秒内仍未启用、无部署、无安装文件或运行时监听，记录真实心跳。
2. 管理员启用得到 queued；发布唯一 rev 1 空配置，重复启用不延期。没有签名制品时显示具体失败，applied 为 0，运行时未启动。
3. 使用已知真实 sing-box 1.14.2 ARM64 统计版，SHA-256 `fee83ca8457c94449dd04aa17a51830cbc9b449a4dda290e995d3366188e0302`。公开 TEST_ONLY fixture 签署并独立核验完整制品；正式 publication 明确拒绝该测试根。经仓库 `ci-signed-release.panel_tree` 写入私有制品库后，真实 Agent 通过面板 HTTP 下载、验签、安装和对账，文件/签名证明摘要一致，实际 verify-installed 通过。
4. 真实 systemd sing-box 服务以独立非 root 用户运行；配置可读、二进制可执行、私有 Agent 配置不可读。实际 ready，target=applied=1；没有代理入站，进程唯一监听是 127.0.0.1:18085 stats。
5. 重复启用保持部署数和版本不变；Agent 真重启、面板真停 10.123 秒后，运行时 PID/启动时间/UID/cgroup 都保持，控制面恢复后连续真实心跳与 ready。三个心跳窗口均取得 3 次真实心跳，最大间隔 20 秒。
6. 先停 Agent，再停独立运行时、Panel、PG。临时模板/单元、公开安装树以及本次创建的账户/组清理；相关进程、挂载、填充 cgroup 和三个端口均无残留，PG 的 PID/socket/lock 消失。私有 PG 数据、源码和脱敏账本作为证据保留。SSH PID406、启动时间和重启0、boot、swap 前后不变；运行期 cgroup 和新增内核 OOM 为零。

fixture 模板保留原 User/Group、capability、NoNewPrivileges、CPUWeight1000 和 OOMScoreAdjust-500；只替换专属路径，并因验签 Agent 放在私有 0700 目录而给只读 ExecStartPre 加 `+`，另加 128 MiB/Swap0/Tasks64/超时预算。三个测试服务和运行时设 IPAddressDeny=any/IPAddressAllow=localhost 并读回。此记录不认证生产安装器原字节或所有网络沙箱行为。

664 份归档源码和 7 份运行脚本摘要结束后均未变；4 个测试单元及运行时的 MainPID/ControlPID 为 0、FragmentPath/ControlGroup 为空。117 条限定单元日志均成功解析，未见 BPF/IP firewall 不支持或加载失败提示；没有输出原始日志。

脱敏结果、二进制/制品/源码摘要、故障记录、资源和清理索引见[证据 JSON](evidence/singbox-plugin-installation.json)。

## 仍未签收的范围

公网 GitHub 制品导入 API、外部面板、生产部署、真实双机公网链路与持续代理业务没有在本次空配置安装验收中执行。完整 NodeQuality 的第三方工具/rootfs/许可证/上传门禁及联合负载矩阵、TcpQuality 注册执行全链路和整改总验继续保持待验；不恢复暂停的 CI，不创建单项 PR，不宣称整个目标已完成。

最新主线整合安装表编号为 `0023_singbox_installation.sql`，保留主线0021节点设置/0022统计索引；上文旧0019/0021是各自历史冻结输入，不作为新组合已验收证据。SQLx迁移记录与校验不自动改写，旧编号已应用的数据库需单独制定保留数据的升级路径；未知状态不能按空白库处理。
