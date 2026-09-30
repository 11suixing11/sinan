# 原生 TCP 固定源码与签名制品：独立验收

本项只交付构建、验证与辅助文件通路，依赖原生引擎 PR #68。插件登记、目标配置、探测执行和 UI 分别交付。未签正式 release，未在生产节点运行探测。

## 复现步骤

在干净且包含固定 commit 对象的 Linux amd64 或 arm64 仓库，安装 Rust 对应 musl 目标、musl-tools、Python 3.11+、minisign：

```sh
python3 tools/build-tcp-probe.py amd64 /tmp/sinan-tcp-artifacts --source-commit <完整40位源码SHA>
python3 -m unittest discover -s tests -p test_tcp_probe_artifacts.py
export SINAN_TCP_ARCH=amd64
export SINAN_TCP_SOURCE_COMMIT=<与构建一致的完整40位SHA>
export SINAN_TCP_ARTIFACT_ROOT=/tmp/sinan-tcp-artifacts
python3 -m unittest discover -s tests -p test_tcp_probe_native_bundle.py
```

arm64 使用 arm64 本机及对应参数，不交叉执行二进制。制品位置为 `tcpquality/0.3.0-<SHA>-r1/<arch>`，伴有 SHA256SUMS；二次写同一制品必须失败。

已有全部默认模块先按旧流程完成。明确加入本制品时使用：

```sh
python3 tools/release.py assemble --source <制品根目录> --output <新输出目录> --agent-version <Agent版本> --runtime-version <runtime版本> --tag <Agent标签> --installer <安装器> --tcp-probe-version 0.3.0-<SHA>-r1
```

离线正式签名继续沿用既有 ADR 0017；本项自动验收只有公开 TEST_ONLY 私钥，不产生可供正式安装信任的 release。

## 接受标准与行为测试

- 合法历史 commit 可构建；main/ref、不存在对象、脏仓库、unknown pin 均拒绝。旧固定配方实际从归档执行，当前配方修改不能替换它。
- 实际构建使用 --locked、明确 musl 目标、固定编译期 SHA；二进制静态 ELF 与本机架构一致，version/help/build-info 可执行，help/build-info 不打开诊断工作目录。
- 签名精确覆盖二进制和五辅助文件。改动任一文件拒绝；重新签名但缺源码、unknown 来源、未锁定依赖、锁文件来源冲突仍拒绝。
- Agent 下载全部五辅助文件，之后逐个修改缓存均拒绝复用；未声明、缺文件、prepare 后文件被修改均不会启动服务。旧默认空集合继续通过旧测试。
- 默认 release 仍只需要旧三模块。第四模块是显式选项，不允许因此放宽旧模块完整性要求。
- 两个架构在独立 CI 矩阵构建/运行，错误架构 ELF fixture 拒绝；正式 trust root 加载公开 TEST_ONLY 根必须拒绝。

## 当前证据

原始 10 项及第五许可文件加入后的 11 项 Python 来源/签名契约通过；旧 Release 32 项和 build-script 5 项通过。真实锁定原文收集与验证为 35 个依赖、2,006,844 字节，包含 Unicode 组合许可、Rust 标准库与原生库、系统 musl 通知。最终引擎基线为 `0a6b8499ca663be0177ee3edaf70b89f24b3c854`。叠加最终基线后的脚本、Rust fmt/clippy/workspace、真实 amd64 musl 构建及双架构 CI 待记录；不得把引擎既有测试结果视为本项完成验收。

此项不运行 NodeQuality、网络质量探测或真实 systemd 诊断，资源预算、取消和章节仍由既有框架测试承担。

## main 整合审查阶段

正常保留作者 `00cfa9e` 并合入 main `2c3c1e5`（业务插件归位）。实际 [check job 110129554033](https://github.com/theLucius7/sinan/actions/runs/36786654457/job/110129554033) 在 Clippy 阶段因 `DiagnosticWorker::new` 未使用的辅助文件变量失败；本地删除多余读取，准备阶段仍只读取一次集合，并同时用于下载描述、签名集合验证及启动前重验，没有压制 warning。固定快照配方、locked 原生 musl 目标、完整五文件来源校验和旧三模块默认契约保留。

本阶段 Python 来源/签名10项通过、旧Release32项中28通过/4既有root条件跳过、build-script5项通过，core门禁/六项行为、直接rustfmt、actionlint与差异检查通过。原生 bundle 本机条件不满足，1项明确跳过；作者PR的[双架构原生制品CI](https://github.com/theLucius7/sinan/actions/runs/36786654508)实际成功，但不替代本地补修后最终HEAD的Rust/Clippy验证。共享Cargo槽仍由前项使用，本阶段没有运行Cargo、实际musl构建或启动服务，后续须在正式main包含#68补修后集中验证；未持有正式签名私钥，也未发布Release或登记插件。

继续正常保留作者 `7716fcf`、共用诊断主线 `e3a41ed` 与正式引擎主线 `2574a84`。新增第三方原文为第五辅助文件（共六文件），签名/精确集合与 Agent 缓存、启动前重验夹具同步保留；引擎 journal/engine 与正式 #68 原字节一致，main 只增加 build-info 入口，UID、DNS 家族选择、有界 stderr 和各自回归保留。新版 Python 来源/签名/原文11项通过，直接rustfmt、core门禁及actionlint通过；作者该版[双架构原生CI](https://github.com/theLucius7/sinan/actions/runs/36787615446)实际成功。本地仍未启动Cargo，最终合入后续主线再执行Rust专项及Clippy，不能把前版或作者CI转记为本地最终补修已验。

## 最终主线与专项

正常保留最新作者 `bf56d5e` 与正式主线 `fb79388`（#55/#60/#68/#70），源码验证基线为 `644e785`。作者最终删除了与本地修复相同的多余读取；准备阶段仍使用同一次辅助文件集合，旧适配器默认空集合、签名前检查、下载、缓存重验和启动前重验全部保留。发布仅在显式选入 TCP 时要求它的双架构，同时保持旧三模块完整性。刷新九个变更 Rust/manifest 文件的时间后，独占共享编译槽集中验证：**90 项通过 / 0 失败 / 0 忽略**（Agent 制品14、release6、wire/预算12、诊断/取消/预算与来源40、SDK1、TCP13库与4真实CLI）。覆盖五辅助文件实际下载及逐文件缓存篡改、未声明/缺失/prepare后篡改不启动、旧签名适配器、build-info、UID与DNS回归、有界阻塞stderr、实际停止及零应用payload。workspace全targets Clippy（warnings为错误）、fmt、core门禁及六项行为、actionlint和差异检查通过。

最终源码下 Python 来源/签名/原文/选入契约12项与模拟GitHub发布22项全部通过；仅合成签名和回环夹具，没有真实账户或发布动作。#70证明中11个前端/门禁文件哈希与正式主线完全一致，保留暂停完整入口、日常入口、`index-OopWuqxH.js`、中性活动桥和r5，没有重复无交集浏览器或PG全量。本轮macOS未重新运行完整workspace测试、Linux原生musl构建或真实systemd/生产节点；最终HEAD的双架构制品与正式主线CI继续单独核对，旧作者native CI不能代替此次最终整合证据。没有登记新插件、签正式Release或发布制品。

## Debian 12 启动修复与固定制品验收（独立后续 PR）

#69 已合并；其 main 整合版本没有包含作者在 Bookworm 上发现的 musl 启动修复。旧 musl-gcc linker 产生的静态 PIE 在 --version 就以 SIGSEGV 退出，构建器正确拒绝该产物。相同最小 hello 在该 wrapper 下返回 -11，使用 native cc 与 -Clink-self-contained=yes 返回0。后续修复保留静态 PIE，使用 Rust 自带 musl/CRT；不改 Agent 的既有构建配方。增加独立 Debian12 真构建/执行/签名 CI，Ubuntu 原生 amd64/arm64 验收继续保留。CLI help 只保留稳定功能边界，移除“尚未接入”临时说明。

已公开且实际验证的永久工具源为 [5e843f0fd9532abe9b7b9a052ef77b45abcfa675](https://github.com/theLucius7/sinan/commit/5e843f0fd9532abe9b7b9a052ef77b45abcfa675)，外部版本为 0.3.0-5e843f0fd9532abe9b7b9a052ef77b45abcfa675-r1。制品 binary SHA256 为 dd4b804f290e920637c3164fed0bd6f1b078ca90d26e34d33d2d8eb849beeeba。--version 精确返回 sinan-tcp-probe 0.3.0；--build-info 明确返回同一 source commit、theLucius7/sinan 和 0.3.0。源码 archive、recipe、Cargo.lock、第三方原文及编译输入一致，后续仅合并其他模块或更新验收文档不重定该固定源。

2026-10-01 独占受限 Debian12 构建容器（1536MiB memory/swap、2CPU、pids512、OOMScoreAdjust500）实际记录：

- 源 500a13d990bf21c07a4e111d751693a0803473c1：fmt/core门禁、全targets Clippy（-D warnings）、locked workspace Rust **347通过/0失败/9既有条件忽略**；Python来源/签名12项、旧Release32项、模拟发布22项全部通过；真实 amd64 musl 构建与完整五辅助文件 TEST_ONLY 签名 bundle通过。
- 最终源 5e843f0fd9532abe9b7b9a052ef77b45abcfa675 只进一步改 help 文案，再次 fmt、全targets Clippy、TCP **13库+4真实CLI**、真实 musl 构建/CLI、完整签名 bundle **1项**通过，容器 exit0、OOMKilled=false。
- 永久制品的五辅助文件为 build-info.json、LICENSE、source.tar.gz、Cargo.lock、THIRD_PARTY_NOTICES.txt；完整签名覆盖二进制和全部辅助文件。35个锁定依赖的许可原文及 Rust/musl 通知保留，原文库存约2MiB。
- 最终证据、制品、build-info、日志、SHA256SUMS 与源码标记保存于受控构建机的 evidence/tcp-artifacts-final-5e843f0；原生 binary 与来自明确源500的 core测试 binary 单独保存于 binaries/tcp-artifacts-head。

上述 workspace 全量来自明确的500源码，不替代后续 main 新增功能的验收；本后续 PR 的最新 GitHub 两架构及 Debian12 任务仍须按实际状态核对。没有正式签名/发布，没有执行外部探测或真实 systemd 诊断。所有签名夹具为 TEST_ONLY。

首次后续CI（464f0ff）Ubuntu原生amd64/arm64制品均通过，新增Bookworm任务在构建前Git目录所有权检查返回128。checkout只设置临时HOME的safe.directory，后续容器步骤需对实际GITHUB_WORKSPACE设置精确例外；受限容器复现了dubious ownership，精确设置/workspace后同一固定对象解析通过。工作流只允许本任务检出的目录，不设置全局通配符、不跳过源码对象检查；补修后最新CI另行核对，永久工具源5e未变化。
