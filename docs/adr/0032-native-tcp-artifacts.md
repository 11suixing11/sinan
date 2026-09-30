# ADR 0032：原生 TCP 工具的固定源码与完整签名制品

状态：已接受；制品入口独立于插件登记、执行参数和界面。

## 约束

用户要求固定提交、锁定依赖、校验与签名，不执行在线 main。上游 TcpQuality 尚无分发授权，采用本仓库 AGPL-3.0-only 的原生连接工具，不复制或运行上游脚本、rootfs、targets。工具只建立 TCP 连接；此项不增加探测、上传或宿主修改能力。

## 决策

工具内版本为 `sinan-tcp-probe 0.3.0`。制品角色为 `tcpquality`，二进制名为 `sinan-tcp-probe`；外部版本为 `0.3.0-<40 位小写 Git SHA>-r1`，每个源码变化产生不同不可变路径。源码提交必须是仓库中存在的 commit 对象，不接受分支、标签、unknown 或空值。它可为当前发布树的祖先，避免后续插件常量登记造成循环。

构建器先验证干净仓库，再 `git archive` 指定对象到私有临时目录，执行该归档内的构建配方，以 `Cargo.lock --locked` 构建本机架构 musl 目标。不使用当前工作树替换固定源码。构建前后均验证归档、源码文件和仓库，编译环境嵌入固定 SHA；提供非法编译期 SHA 会编译失败，未提供 SHA 的开发构建可运行但不能打包。

制品包含二进制和四个辅助文件：`build-info.json`、`LICENSE`、`source.tar.gz`、`Cargo.lock`。build-info 记录版本、完整 SHA、目标、rustc、locked 标志和各文件 SHA256。源归档保留 Git 的完整 commit 注释，并必须包含工具及构建配方；独立锁文件、许可证必须与归档逐字一致。编译器验证原生架构的静态 ELF、无动态解释器或依赖，并实际执行 `--version`、`--help` 和无网络的 `--build-info`。不在错误架构上执行。

沿用 release.json、SHA256SUMS 与 minisign 契约，辅助文件全部记录大小与 SHA256。SDK 新增默认空的 `DiagnosticAdapter::auxiliary_files()`；core 的签名检查和下载描述使用同一次读取的集合，保持精确集合及缓存再校验，旧适配器行为不变。

默认旧三模块 assemble 及发布流程保持原行为。新制品仅显式 `--tcp-probe-version` 加入。CI 在 amd64/arm64 原生 runner 上构建，仅使用公开 TEST_ONLY 签名夹具，拒绝将其作为正式信任根。本项不持有正式私钥、不创建或发布 release，生产发布接入仍须由后续登记项选择已验收的源码版本。

## 验收

见 [独立验收](../acceptance/native-tcp-artifacts.md)。制品协议同时拒绝签名后篡改，以及重新签名但来源字段缺失、unknown、未锁依赖、锁文件与源码不一致的包；签名不能替代来源完整性。
