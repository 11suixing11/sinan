# 诊断材料下载失败证据验收

关联 issue：#141。实现决定见 [ADR 0055](../adr/0055-diagnostic-material-download-evidence.md)。

当前状态：六个源码/请求文件已整合，Linux factory 35 项、collector 39 项回归全部通过，零失败、零跳过。#141 的源码与相称覆盖完成，完成推送后关闭。本轮只执行自有回归，没有重新下载公网 Debian 包、构建真实完整 rootfs 或签收完整工具链。

## 冻结范围

`tools/nodequality-rootfs-collect.py`、`tools/test-nodequality-rootfs-collect.py`、两份 `tools/nodequality-rootfs-request-{amd64,arm64}.json`，以及 `tools/nodequality-rootfs-build.py`、`tools/test-nodequality-rootfs-build.py`。主线 r19/r20/r21 producer helper、原来源锁及许可库存保持字节；保留主线 FIFO 替换拒绝和复制时大小/SHA256 防护。

收集器访问官方固定 Snapshot 只属于材料取得，不代表独立 keyring 信任、builder 审批或第三方许可。工厂继续仅消费本地已认证输入。收集总字节、整体/worker 期限、磁盘预留、签名链、URL 白名单与无自动重试规则保持。

## 实际受验输入

2026-10-02，在隔离 Debian 12 amd64 Linux 环境中：

| 专项 | 冻结源码 | 实际结果 |
| --- | --- | --- |
| `tools/test-nodequality-rootfs-build.py` | `9b0707748b6501e2ebf88fc32060e6475399b286`，r1 | 35 项通过，零失败、零跳过 |
| `tools/test-nodequality-rootfs-collect.py` | `89563151556e4e4377428d3b692ea85b1b71f321`，r2 | 39 项通过，零失败、零跳过 |

两份受验源码与文档核对基线 `cb00c6a40a4301609ccb85c07f9fee44284d0ddf` 的六个文件逐个 SHA256 相同；证据绑定这些具体字节：

| 文件 | SHA256 |
| --- | --- |
| `nodequality-rootfs-collect.py` | `0be0fe85b6b28632dcdf6d50a3bcbea5bf4891832f778deebcd9fdd668e6691b` |
| `test-nodequality-rootfs-collect.py` | `1822c54c39689adfba944947694f3d8c3ae1d1c74793390ef3466a526e59a968` |
| `nodequality-rootfs-request-amd64.json` | `353ce52848a94f7534a0f689136cd3c4562913ded501d33e086c1016274e5a7b` |
| `nodequality-rootfs-request-arm64.json` | `61a582a17f843fe5fa4c7330a409ae74960670e0ed496a98df2c876810e1f852` |
| `nodequality-rootfs-build.py` | `c8e1ab9e1e78611e704120a2ba570041733218d2552adf76f4f3db871764cf93` |
| `test-nodequality-rootfs-build.py` | `bd8ca84e42bc140638ad0d4d96d04c045b28cd4e430d578062e906b6e286b248` |

原始记录保留在私有测试命名空间的 `output-r1/rootfs-factory.log`、`output-r1/rootfs-collector.log`、`output-r2/rootfs-collector.log`、对应 `receipt.json` 及 `r2-frozen-input-audit.json`。r1 collector 初次有 5 个环境失败：256 MiB `/tmp` 低于生产 512 MiB 磁盘预留；容量计划多报空间拒绝，TERM/HUP 在请求前拒绝，截短正文及签名不匹配测试也先触发正文空间守卫。父进程的 mock 不会替换真实 worker 子进程的守卫。

r2 仅把测试 `TMPDIR` 指向独占的 `/output/temporary-tests` 磁盘目录，保留 `/tmp` 256 MiB 和生产最低 512 MiB 预留，未改本项六个文件、降低守卫或增加下载/重试预算。冻结审计记录 963 个源文件均未改变，另列两个生成的 `.pyc`。整轮 receipt 因生成文件记录 `source_inputs_unchanged=false`，其 Rust 阶段也有失败；上述通过结果只属于本项两份 Python 回归，不能表示整轮验收全绿。

## 最终检查表

| 场景 | 必须观察的结果 | 本轮状态 |
| --- | --- | --- |
| 真实回环 HTTP 403/429 | 单次 worker 请求失败，实际 HTTP 状态与分类保留，worker 回收，无成功正文缓存 | 通过，Linux 自有回归 |
| 206、非法/截短 Content-Length、空正文及 chunked 超限 | 保存实际响应状态、具体类型/原因/阶段和字节观察，失败不输出成功材料或输入锁 | 通过，Linux 自有回归 |
| 超时、TERM/HUP | 不增加下载时间预算或自动请求，回收本次进程组；可用 headers/receipt 有界保存，无正文 | 通过，Linux 自有回归 |
| receipt 缺失或截断 | worker 类型/原因、HTTP 状态明确未知；父层错误单独可见，不推断底层原因 | 通过，Linux 自有回归 |
| 总字节或磁盘预留不足 | 拒绝本次下载/证据写入，保存失败状态与原因，保持原预留，不删除认证缓存 | 通过，Linux 自有回归 |
| 最终材料/输入锁/成功收据发布失败 | 本次成功文件撤回，认证缓存与失败证据保留，binding 拒绝不完整目录 | 通过，Linux 自有回归 |
| 成功文件已存在或被替换，失败收据也无法写入 | 拒绝覆盖/删除未认领或改变身份的文件；回滚未完成明确记录，认证缓存及不完整目录保留 | 通过，Linux 自有回归 |
| 证据隐私 | Cookie、凭据、query/fragment、控制字符、环境、正文、完整命令参数不进入失败证据 | 通过，Linux 自有回归 |
| factory 的 FIFO/复制时更换、短写与增长 | 保留主线拒绝行为和逐次摘要/长度检查，结合 prospective capacity，不接受额外或改变身份的材料 | 通过，Linux 自有回归 |
| factory 容量/取消/清理 | 独立输出身份、原容量和 inode 预留保持，进程与所创建目录实际回收；残留挂载时拒绝递归删除 | 通过，Linux 自有回归 |

回环 worker 使用私有测试替换来允许自有服务器；生产 collector 没有测试地址开关，白名单不放宽。mock 的签名/索引/镜像身份仅验证拒绝接口，不宣称真实 Debian 签名、第三方分发许可或镜像审批。

## 未签收范围

本项不执行第三方硬件或测速工具，不接受许可、打包未授权二进制、重放历史公网故障或修改生产服务。完整 NodeQuality、双架构合法工具闭包、真实 builder 和复建、持续代理业务联合保护仍需另行实际验收；#28/#65/#66/#82 与 full 门禁不因本项小型回归通过而关闭或开放。CI 继续遵从仓库暂停要求。
