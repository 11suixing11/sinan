# ADR 0015：扩展 Agent 编译产物的平台

状态：已采纳。

## 背景

用户要求在已有 Linux musl 制品之外，增加 Ubuntu 24.04 glibc、macOS arm64、FreeBSD 13 系列及以上、Windows 的 Agent 编译产物，除 macOS 外均提供 amd64 和 arm64。用户已明确本次范围为“增加可下载的 Agent 编译产物”，因此这是对原 Linux 构建范围的明确扩展，服务部署仍限 Linux；后续 OpenRC 服务支持见 [ADR 0016](0016-openrc-services.md)。

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
