# ADR 0041：跨平台单行 Agent 接入

- 状态：已采纳，2026-10-01。
- 用户要求：服务器接入提供单行命令，自动匹配或自行选择版本；同时支持 Linux、Windows、macOS 和 FreeBSD。

## 入口、版本与平台

接入页提供 Shell 和 PowerShell 入口。Shell 自动检测 Linux GNU/musl、macOS ARM64、FreeBSD AMD64/ARM64；PowerShell 使用本机 Windows AMD64/ARM64，兼容 WOW64 终端。macOS AMD64、32 位系统和其他 CPU 没有现有发布 ABI，不猜测替代制品。自动匹配在执行时按数字排序选择最新稳定版本；显式选版保留精确版本；Linux 可显式选合法预发布版本，原生平台按现有 install-service 的版本约束只提供稳定版。

管理员版本目录与一次性令牌版本目录来自完整且已验签的导入 proof，而不是仅来自当前面板缓存。目录只是候选索引：独立入口仍从固定官方 GitHub tag 下载完整 proof，核对仓库、tag、协议、清单和本机 ABI。面板不能通过修改目录、更换下载源或提供新根使入口执行未签名的 Agent。入口本身按官方 Git blob 身份下载，执行前核对固定 SHA-256；两个生成器均拒绝测试根，并有生成同步检查。PowerShell 模板中的清单解析另做真实 minisign 互操作与拒绝用例，Agent 编译根仍是第二次独立检查。

Linux 优先 musl，GNU 系统可使用本机 GNU；musl 不使用 GNU。旧裸 `amd64`/`arm64` 的 musl 身份继续兼容。完整 Release 中的 install.sh 仍核对签名摘要；Linux 实际使用独立受信 bootstrap 中嵌入、由现有模板与 systemd/OpenRC units 渲染的静态安装器，支持精确目标和 GitHub 已验字节。它与入口一同受固定官方 blob/SHA 保护，避免旧正式 0.3.0 安装器仍要求面板提供 Agent；不修改已发布的任何资产。原生平台下载裸 Agent，依次验证自身、旧缓存、接入，再调用现有 `install-service`，由 launchd、FreeBSD rc.d 或 Windows 计划任务管理。已有配置在切换失败时恢复，已有设备私钥保留；原生 CLI 的版本证明、原子切换与启动失败回滚继续生效。

安装候选还要求处于现有 0.3.0 及更新的安装合同版本线。真实 0.1.0 原 CLI 只有 enroll/run/status；历史 0.2.0 并非统一具备当前所需的验签、缓存预检和 supervisor 命令。协议主版本相同或现在为旧原字节补签 TEST_ONLY metadata 都不能补齐这些能力。已导入且协议兼容的历史 0.1/0.2 返回明确的标准安装合同不支持错误，与制品未导入、协议不兼容及平台不匹配分别报告；原 Release、二进制、身份和状态不删除。直入离线 bootstrap 在验签后也执行相同拒绝，不能绕过版本目录。这条版本线是已登记的支持策略，不证明任意更高版本字节自动实现 CLI：完整证明、下载摘要以及 Agent 独立 verify-installed/verify-cache 调用继续保留。#3 的真实 0.1.0 首装与 0.2.0 标准原地升级验收仍未因此完成。

0.3.0 及更早 Linux Agent 能恢复本地 Preparing，旧 verify-cache 的签名通过不能证明完整启动门禁。已有状态的恢复要求旧 Agent 明确停止并做只读 SQLite 预检，读取真实 WAL、要求既有共享内存并限制期限；完整 Preparing 或未知/损坏状态拒绝且原 JSON 保留。Started 按原版本核验回收，旧 Agent 只接续其确切 r2 及原参数；其他 Started 保留给兼容的新签名 Agent，拒绝不兼容降级。daily 门禁不额外拒绝，但实际执行仍取决于旧适配器兼容能力。bootstrap 与内嵌执行器采用同一标准库守卫，接入前、服务激活前复查；配置解析与停止状态无法确认时拒绝，不自动停服务，也不把单次快照声称为全程原子迁移。

## 按架构导入与直接下载

管理员导入保留完整 proof，仅下载所选平台的兼容制品；局部缓存不改变已签目标目录。单行入口直接从固定官方 GitHub tag 下载本机 Agent，可使用该服务器已配置的独立 HTTPS 镜像，不携带面板接入令牌或设备凭据。下载大小与摘要由独立签名限制，再由 Agent 编译根复验。运行时与配置继续走面板；面板 Agent 下载接口保持 409，遵守主线 ADR 0038 的 GitHub-only 规则，不恢复面板分发 Agent。

## 平台依赖与首次信任

Linux、FreeBSD 缺少依赖时使用系统软件源；Linux 默认软件源没有 minisign 时，使用官方 0.12 静态归档，仅解出本机 CPU 的固定二进制，避免要求额外启用软件源。归档 SHA-256 为 `9a599b48ba6eb7b1e80f12f36b94ceca7c00b7a5173c95c3efc88d9822957e73`，AMD64/ARM64 二进制分别为 `2c74dffcc1c9a5ee55957c60971998ace2b89f22585631594ec2152c588af8db` 与 `cec9f88be8c975af76854a53b4d49c3d257feae38d916edb0d16fb55aacd3000`。macOS 不以 root 运行 Homebrew：缺少固定 Python 时，安装 [Python.org 的 3.13.16 universal2 pkg](https://www.python.org/ftp/python/3.13.16/python-3.13.16-macos11.pkg)，先核 SHA-256 与 Apple pkg 签名；使用系统根证书及受保护的 Python 路径。其固定 SHA-256 为 `30666509020b4da0dd8bc2e773255f34d76b7bb80b66960a928d5f6daa0192d7`。

macOS 和 Windows 自动下载 [minisign 0.12 官方归档](https://github.com/jedisct1/minisign/releases/tag/0.12)，归档及所选二进制均有固定 SHA-256，不信任 PATH 中未知的验证器。macOS 归档为 `89000b19535765f9cffc65a65d64a820f433ef6db8020667f7570e06bf6aac63`，二进制为 `d41cde458303d45c95b00473e2455a7f45f95b550931f1f0cc98ef1f61b2a8ff`；Windows 归档为 `37b600344e20c19314b2e82813db2bfdcc408b77b876f7727889dbd46d539479`，AMD64/ARM64 二进制分别为 `5535be9e4e123831ebe6ef324aafe9dde507015c176191f9e20c3ad60567f9e1` 与 `f39e065e649d5ed7075675accfe0ada234175d63479df650654ec4365d7c4513`。只抽取固定文件名，工具位于 root/管理员保护目录；归档中的其他文件不执行。

首次信任新增这些固定官方依赖来源和源码中的摘要，无新增 Rust/Bun/Python 库依赖。软件许可仍归各上游，仓库不重新分发其二进制。Unix 单行命令先提升执行固定参数化程序，再在 root 私有目录中下载、核对摘要和执行入口；直接运行入口脚本时必须已经是 root，不能将用户可写的已下载文件再次交给 sudo。Windows 命令在普通终端通过 UAC 提升，提升后重新从固定源下载入口；编码参数保留引号与换行的字面值，避免从用户可写文件提升执行。Windows PowerShell 5.1 的中文入口使用 UTF-8 BOM，兼容 PowerShell 7。

## 验证范围

四个平台的版本选择、签名证明、只下载对应目标及一行命令由本地测试覆盖。Linux ARM/OpenRC 的正式根接入可在隔离容器验证。当前已有正式 `agent-v0.3.0` 只提供旧 Linux 架构；本变更没有创建原生签名 Release。Windows/macOS/FreeBSD 的真实机器常驻安装及其正式制品仍需独立发布与实机验收，不能将 Linux 上的 PowerShell 函数验证或浏览器夹具称为原生服务验收。CI 按当前仓库安排保持暂停。
