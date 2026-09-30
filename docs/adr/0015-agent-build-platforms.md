# ADR 0015：扩展 Agent 编译产物的平台

状态：历史扩展决策已采纳；自动构建范围按下方 2026-09-30 的后续决策收敛。

## 背景

用户要求在已有 Linux musl 制品之外，增加 Ubuntu 24.04 glibc、macOS arm64、FreeBSD 13 系列及以上、Windows 的 Agent 编译产物，除 macOS 外均提供 amd64 和 arm64。用户已明确本次范围为“增加可下载的 Agent 编译产物”，因此这是对原 Linux 构建范围的明确扩展，服务部署仍遵循现有 Linux/systemd 架构。

## 决策

- 保留 Linux musl 双架构的原生构建和部署目录。glibc 使用 Ubuntu 24.04 双架构 runner 的动态库，显式关闭静态 CRT，并验证 ELF 架构、动态解释器、`libc.so.6` 依赖和库解析。
- macOS 使用 GitHub 最新 macOS arm64 runner；Windows 使用 Visual Studio 2026 的 x64/arm64 runner、MSVC 目标与静态 CRT；所有目标使用最新 Rust stable。
- FreeBSD 使用 `vmactions/freebsd-vm` 的 13.5 基线及对应架构构建，在同一架构的最新 14、15 系列 VM 内执行同一二进制的 `--version` 和 `--help`。13.5 之前的小版本及更高的未来主版本不宣称已验证。
- 新制品按完整 Rust target 分目录，包含原生二进制和 `SHA256SUMS`，避免 GNU/musl 或不同系统同名文件互相覆盖。Actions 上传名称包含平台、链接方式及架构，保留七天。
- GitHub Actions 工具使用本次核实的最新稳定版：[checkout v7.0.1](https://github.com/actions/checkout/releases/tag/v7.0.1)、[upload-artifact v7.0.1](https://github.com/actions/upload-artifact/releases/tag/v7.0.1)、[setup-python v7.0.0](https://github.com/actions/setup-python/releases/tag/v7.0.0)、rust-cache v2.9.2、setup-bun v2.2.0 和 freebsd-vm v1.5.8。Windows arm64 的 VS2026 runner 标签已有官方文档，但晚于 actionlint 的内置列表，在校验配置中单独列入允许列表。
- Agent 的命令解析与 `--help`、`--version` 可以在所有编译目标启动。`enroll`、`run`、`status` 在非 Linux 平台明确返回部署限制；不增加 launchd、FreeBSD rc.d 或 Windows 服务管理。
- Unix 系统操作与 Unix socket 传输仅在 Unix 目标编译。路径分量检查属于制品通用逻辑，移入 `artifacts`，Windows 无须引入伪造的特权或 socket 实现。
- 构建脚本优先尊重显式 `PROTOC`，FreeBSD 使用构建 VM 安装的 protobuf 编译器。其他目标仍使用已有 vendored 编译器，不新增 Rust 依赖。跟踪 `PROTOC` 变化，确保 Cargo 正确重建。

## 替代方案

直接对所有平台执行现有 Unix 代码会导致 Windows 编译失败。为各平台实现完整安装、权限、身份、IPC 和服务管理超出用户确认的范围。统一从 Linux 交叉编译需要额外 SDK、链接器和模拟器；使用原生 runner 或 VM 可以同时验证架构及实际启动。

## 验证边界

本地检查覆盖 Rust workspace、构建脚本的架构拒绝与制品不可覆盖行为。各系统、架构及 FreeBSD 跨版本执行以对应提交的 Actions 实际结果为准；添加矩阵不能视作远端已经构建成功，也不能视作非 Linux 的设备部署支持。


## 后续决策：2026-09-30 收敛自动构建

用户要求先让 main 的持续集成恢复全绿，再推进真实 Linux/systemd 部署验收。自动构建只保留 Linux musl amd64 和 arm64；glibc 动态版、macOS、Windows 和 FreeBSD 不再阻塞 push 与 pull request 的 CI。保留本 ADR 的历史平台扩展记录和通用构建脚本，不增加非 Linux 部署能力。

Linux 的自动检查和 amd64 构建固定使用 `ubuntu-24.04`，arm64 构建固定使用 `ubuntu-24.04-arm`，避免 `ubuntu-latest` 的基线迁移改变已验收环境。Rust stable 和既有工具版本策略保持原决策。

musl 双架构的 CI 先使用同一源码、目标、工具链和构建脚本，通过 Cargo 环境覆盖恢复旧 release profile（`strip=none`、`lto=false`、`codegen-units=16`），将构建目录隔离到 runner 临时目录；再按仓库的优化 profile 构建正式制品。每次记录两份二进制的精确字节数、缩减百分比到 job summary 与制品中的 `size-comparison.txt`，避免不同提交的二进制差异影响体积对照。

继承工作区中曾加入 `tools/test-freebsd-rust.py` 的主 CI 步骤，但当前仓库并无该测试文件，因此主 CI 不执行此不存在的步骤。原始 workflow 改动已经在仓库外保留，官方工具链安装器保持原样未提交。此次不新建依赖该未提交文件的手动 workflow；后续恢复可选平台时应将安装器与完整测试一并评审。

最新 main 提交 `fc8467d` 的 [CI 36681900051](https://github.com/theLucius7/sinan/actions/runs/36681900051) 中，检查、Compose smoke 和 musl 双架构均已通过；唯一失败是 FreeBSD arm64 在 rustup 安装阶段收到 404，尚未进入 Cargo。此次调整直接收敛到既定 Linux 部署范围。新的完整 CI 结果仍须以本次提交的实际 Actions 运行为准。
