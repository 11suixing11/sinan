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
- 签名精确覆盖二进制和四辅助文件。改动任一文件拒绝；重新签名但缺源码、unknown 来源、未锁定依赖、锁文件来源冲突仍拒绝。
- Agent 下载全部四辅助文件，之后逐个修改缓存均拒绝复用；未声明、缺文件、prepare 后文件被修改均不会启动服务。旧默认空集合继续通过旧测试。
- 默认 release 仍只需要旧三模块。第四模块是显式选项，不允许因此放宽旧模块完整性要求。
- 两个架构在独立 CI 矩阵构建/运行，错误架构 ELF fixture 拒绝；正式 trust root 加载公开 TEST_ONLY 根必须拒绝。

## 当前证据

本机初版 10 项 Python 来源/签名契约通过。最终引擎基线为 `0a6b8499ca663be0177ee3edaf70b89f24b3c854`。叠加最终基线后的脚本、Rust fmt/clippy/workspace、真实 amd64 musl 构建及双架构 CI 待记录；不得把引擎既有测试结果视为本项完成验收。

此项不运行 NodeQuality、网络质量探测或真实 systemd 诊断，资源预算、取消和章节仍由既有框架测试承担。

## main 整合审查阶段

正常保留作者 `00cfa9e` 并合入 main `2c3c1e5`（业务插件归位）。实际 [check job 110129554033](https://github.com/theLucius7/sinan/actions/runs/36786654457/job/110129554033) 在 Clippy 阶段因 `DiagnosticWorker::new` 未使用的辅助文件变量失败；本地删除多余读取，准备阶段仍只读取一次集合，并同时用于下载描述、签名集合验证及启动前重验，没有压制 warning。固定快照配方、locked 原生 musl 目标、完整五文件来源校验和旧三模块默认契约保留。

本阶段 Python 来源/签名10项通过、旧Release32项中28通过/4既有root条件跳过、build-script5项通过，core门禁/六项行为、直接rustfmt、actionlint与差异检查通过。原生 bundle 本机条件不满足，1项明确跳过；作者PR的[双架构原生制品CI](https://github.com/theLucius7/sinan/actions/runs/36786654508)实际成功，但不替代本地补修后最终HEAD的Rust/Clippy验证。共享Cargo槽仍由前项使用，本阶段没有运行Cargo、实际musl构建或启动服务，后续须在正式main包含#68补修后集中验证；未持有正式签名私钥，也未发布Release或登记插件。
