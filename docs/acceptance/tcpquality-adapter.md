# 原生 TCP 无状态适配器独立验收

此项仅实现 `sinan-adapter-tcpquality` 的参数转换、制品身份检查与报告/章节解析。工作区依赖只有 adapter-sdk；不登记 Agent/面板第二插件、不搬入创建/互斥/持久化/取消生命周期、不增加 UI 或 ProbeSpec 字段。原生引擎、固定源码和签名制品分别由 PR #68/#69 交付。

## 参数与身份边界

- plugin_name=tcpquality，binary_name=sinan-tcp-probe，额外 capability 为 `diagnostic:tcpquality-native-v1`。静态辅助文件为 build-info.json、LICENSE、source.tar.gz、Cargo.lock、THIRD_PARTY_NOTICES.txt；核心签名安装/复验负责这五个文件的哈希、大小及信任根，制品构建器负责许可原文库存。
- 制品版本只接受 `0.3.0-<40位小写源码SHA>-r1`，版本目录和二进制名称必须一致。通过 Privileged.execute_bounded 分别核对精确 `sinan-tcp-probe 0.3.0` 与无网络 `--build-info` 三字段；version/source_repo/source_commit 必须对应本仓库与固定 SHA。缺失/null/错误 pin、失败、截断、超时、额外字段拒绝，不产生可启动 ServiceJob。
- 业务 options 仅 ip_version=4/6、count=4/8（默认4）、concurrency=1/2（默认1）、targets、target_digest；另允许 core 的 environment_section=true/false，不传入工具 CLI。拒绝命令、URL、上传、测速、rootfs、任意地区选项。
- 快照 schema=1、1–8个不重复 UUID、原 UTF-8 ≤16 KiB、SHA256 逐字节核对。目标主机仅合法单播 IPv4/IPv6 或 ASCII hostname，端口1–65535，名称/运营商长度及控制字符受限。region 为 null 或 east_asia/southeast_asia/europe/americas/other；configured 是面板筛选预设，不能当作实际地区，不从 IP 推断地区。
- 工作目录按诊断 UUID 隔离，所有路径绝对且无扩展/遍历；检查祖先、普通私有目录与输入。prepare 以 core 同一特权执行者写入的可信签名缓存二进制 UID 为参照，拒绝异主工作目录与输入。通过 Privileged 创建700目录、写600快照，已有输入不同或已有部分输出时拒绝重复准备，不覆盖已保存结果。没有执行网络测试的 prepare 分支，身份命令只使用固定两个参数。
- ServiceJob 始终包含 `--no-rank-upload`，预算64 MiB/32 tasks/CPUWeight10/IOWeight10/OOM500，不设CPUQuota，运行时限取 core 传入剩余秒数与60秒的较小值。每项操作有两秒上限，准备也受总剩余预算限制。后续注册计划须限定60秒，core继续按任务到期时间收紧；引擎 parameters.total_timeout_ms 固定60000，外层更早停止的报告保持部分状态。

## 报告与章节边界

result.json 和每份章节最多64 KiB，仅接收普通私有文件；拒绝 symlink/hardlink、超限、无效UTF-8、非JSON。校验冻结目标/digest、IP/参数、engine/version/source pin、无上传/排名/测速、UTC起止与逐次时间、实际地址与端口、完成状态以及基于样本重算的计数/成功率/建连耗时。未尝试的成功率为null，没有成功的延迟为null；真实0%或成功样本0毫秒保留，不能把失败补成0延迟。

固定读取 scope、summary 和至多八个目标章节，共至多10章，不枚举任意文件。各章节检查名称、revision、采集时间、内容与独立完整度；坏/缺/超限/链接章节跳过，其他部分仍可读。总章节采集预算五秒，截止时返回已经取得的章节。report_url始终None。环境章由core单独采集，不挤入工具十章预算；新适配器实例可以继续读取取消/重启前已保存的部分结果，即使旧二进制已不存在也可收集。历史收集以 core 管理的可信私有任务目录 UID 为参照，核对已保存输入原字节、章节目录、报告路径及实际打开句柄的 UID，不写生命周期状态或清理历史文件。调用方必须保持整个任务根目录及祖先可信；本适配器的 UID 一致性检查不构成针对恶意祖先或整体异主任务根目录的沙箱。

## 独立验证

```sh
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
cargo fmt --all --check
cargo test --locked -p sinan-adapter-tcpquality
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

| 场景 | 证据 |
|---|---|
| 白名单、摘要、UUID/路径、超限/重复目标 | 参数拒绝先于任何 Privileged 调用 |
| pin/仓库/版本错误、null、截断、身份超时 | 不创建工作目录、不返回 ServiceJob |
| 正常IPv4/IPv6、地区缺失与五种标签、core环境开关 | 冻结原字节，必需 no-rank-upload，64MiB/32tasks，短剩余时限保留 |
| 部分/重启读取、真实0与未知 | 删除旧二进制后同一已保存报告和章节仍可由新实例读取；0/false有明确语义，失败延迟保持null |
| 报告参数/目标/源SHA/时钟/安全字段/统计篡改 | 严格拒绝，原文件保留；DNS/family终结保持未知，取消/worker_failed不可伪称完整 |
| 坏主报告或一个坏章节 | 好章节仍返回，未知章节不扩范围，最多10工具章 |
| 普通文件、symlink/hardlink、公开权限、超大文件及 UID 不一致 | 真实私有文件系统夹具核对可信 UID 与不同 UID，拒绝路径/文件 owner 不一致和无界内容 |
| 已保存结果、旧快照不同 | 重复prepare拒绝，不覆盖部分报告 |

这些测试使用记录型 Privileged、合成身份/报告和真实私有文件系统，明确不是真实签名归档安装或 native TCP 网络/服务测试。原生引擎真实回环/实际进程取消由 PR #68 验证；签名制品/许可证库存由 PR #69 验证；同机NodeQuality互斥、确认取消、压力与Panel/Agent完整接入由后续独立PR验证。

本机只fmt、locked offline metadata/core门禁和差异检查，没有从头编译。源码 `881cce5c48d0a65f4ab6eeb83d04e52a2d6b8ae2` 的 [GitHub check](https://github.com/theLucius7/sinan/actions/runs/36789086624/job/110137429669) 已通过：13项适配器测试、全 targets Clippy（warnings视为错误）、完整 Rust/PostgreSQL 353通过/0失败/9既有条件忽略，随后专门执行的六项真实systemd回归全部通过。Compose、Agent双架构musl与TCP制品双架构检查也已通过；Reality与最终制品依赖更新的HEAD另核对。真实systemd回归验证现有core框架，不能当作此尚未登记适配器的完整服务验收。
