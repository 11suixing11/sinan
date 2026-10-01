# NodeQuality ARM rootfs 静态盘点独立验收

Refs [Issue #82](https://github.com/theLucius7/sinan/issues/82)，补充 [amd64 证据](nodequality-rootfs-inventory.md) 和 [执行链审计](nodequality-chain-audit.md)。本项基于文档分支起点 `61a65651ece3482e03d5121eb097b5d979fe3b96`，仅增加静态证据，不修改产品源码、签名制品、资源预算或完整任务门禁。

## 固定资产与单次取得

2026-10-01（Asia/Taipei）先刷新固定 GitHub asset ID `345687500` 的公开元数据，确认名称、固定 URL、大小和 digest 未变。资产为 [NodeQuality v0.0.2 的 BenchOs-arm.tar.gz](https://github.com/LloydAsp/NodeQuality/releases/download/v0.0.2/BenchOs-arm.tar.gz)，359,657,375 字节，元数据 SHA256 为：

```text
a4dd4e55b129157a02dab437b78e41b5797a7a79a0f0b7febdecab8eb2a312c7
```

刷新后的资产 JSON 摘要为 `d0301d7f75248dafea78b00fab2e0b28f77970924b991e5e1f6cd1e72d12d0b0`。实际单次下载耗时 6.214 秒、HTTP 200、359,657,375 字节；流式 SHA256 及独立重算均匹配上述 digest。开始下载时私有证据目录所在磁盘有 26,235,211,776 字节可用，超过压缩包加 2 GiB 的门槛。下载器以独占尝试记录拒绝重复下载，失败即记录并停止，没有重试。

资产 digest 证明本次取得的字节身份，不是上游签名、可复建配方、对应源码或再分发授权。既有审计记录 v0.0.2 tag 对应 `b9967df1797d4079c89b008bb888bc6ef1fb2672`；本项未重新核实 tag 源码树或取得 rootfs 构建配方。

## 方法与实际安全状态

可信自有 Python 扫描器只在固定分析镜像 `sha256:536cb7ff6e5470d3859742f426a7a9cb989e01d45ca2c8dc7f4012113004b6f7` 中流式遍历 tar；没有解压文件系统、跟随链接、挂载 rootfs、chroot 或执行上游成员。只选读发行标记、dpkg 白名单字段、已知许可目录的普通文本和已知工具 ELF。root/home/.config/.ssh/.gnupg 配置内容不选读；成员索引仅保存路径、类型、长度、模式和链接目标。选中文件索引核对未出现配置路径，七个工具候选均实际为 ELF，未读取非 ELF 工具内容。

扫描容器的实际读回为：非 root `1000:1000`、禁网、只读根与输入、cap-drop ALL、no-new-privileges、512 MiB 内存、memory-swap 同为 512 MiB（无额外 swap）、1 CPU、128 tasks。输入和输出隔离，原始证据目录为 0700；Docker 状态仅保存安全/资源/退出白名单，不保存环境或完整配置。

保持单文件 32 MiB、选中内容合计 64 MiB、解压流 8 GiB、100,000 成员、读取循环 240 秒上限，并加入容器外层 120 秒硬截止；到期先终止已确认归属的容器，finally 中停止和删除。本轮静态扫描 5.798 秒，容器附加运行 6.136 秒，退出 0、未 OOM、未触发硬截止，停止和删除均成功。没有路径越界或读取跳过。120 秒超时路径本轮没有触发，不能把正常清理结果称为超时故障回归通过。

## 实际盘点

| 项目 | 实际结果 |
| --- | --- |
| 系统与架构 | Debian GNU/Linux 12 bookworm；dpkg 架构为 arm64 / all；已知 ELF 为 ELF64、e_machine=183（AArch64） |
| tar 成员 | 30,034：25,232 普通文件、2,645 目录、2,153 符号链接、4 硬链接 |
| 包记录 | 364 个，均为 install ok installed；仅保存 Package/Status/Version/Architecture/Source/License 白名单字段 |
| 许可文本 | 372 个普通许可/版权文件，其中 358 个 copyright 路径；每份内容及摘要保存于私有索引，未提交或再分发归档工具 |
| 有界读取 | 解压流 969,697,280 字节；选中内容 24,367,281 字节；0 越界路径、0 跳过、0 配置成员选读 |
| Geekbench | 成员名称、链接目标和包名未发现该名称；不排除改名代码，不免除 HardwareQuality 在线取得 Geekbench 5.5.1 的缺口 |

许可文本来自 `/usr/share/doc/` 的已知版权/许可文件名、`/usr/share/common-licenses/` 和 `/usr/share/licenses/` 普通文件；不跟随符号链接，也不声称通知覆盖所有包或二进制。保存许可文本不等于完成对应源码、构建来源、分发条件或授权核查。

| 已知 ELF | 字节数 | SHA256 |
| --- | --- | --- |
| `BenchOs/usr/bin/speedtest` | 2,541,880 | `d99fa13293f658b53eaa79fe81f4b210db39fdfc1e9698f33da3f234a6008df7` |
| `BenchOs/usr/local/bin/nexttrace` | 8,978,584 | `644d4994b2e949a8adc5b8a4bf9bcb77b1367d3b995d3a6370b671f0ed57fcf0` |
| `BenchOs/usr/bin/fio` | 1,814,648 | `bad39f4a611738fbe2ba8a9f744a1a6f1618ca03775bd7ed5291cb59dc736db3` |
| `BenchOs/usr/bin/iperf3` | 67,512 | `1c9f122a6b8e8c02d79bca06a88946f72e7542319b679e4cc078062a01690325` |

speedtest ELF 的固定目标二次静态核对确认字节中含 `Speedtest by Ookla` 标记和以 NUL 分隔的 `1.2.0.84` 完整字符串；没有执行 `--version`。该核对复用已验证的归档，只选读这一个已确认的普通 ELF，先验 ELF 头与原盘点摘要，再输出固定布尔标志；容器安全参数与主扫描相同，3.592 秒完成，退出 0、未 OOM，own 容器已停止删除。dpkg 没有 speedtest/ookla 包记录，保存的许可文本语料没有 Geekbench/Primate Labs/Ookla/Speedtest 标记，来源与再分发证明仍缺。归档成员索引显示 `BenchOs/root/.config/ookla/speedtest-cli.json` 为 122 字节普通文件，本项没有选读其内容，不补推许可或隐私接受状态。

nexttrace 的 ELF 摘要与既有固定 v1.3.7 arm64 资产观察值一致；既有源码审计记录其 tag 为 `69588b0d14187ee00f48aa04e5617256148c3ecf`、许可证为 GPL-3.0。相同摘要不是签名，本项未重新取得对应源码、验证构建配方或执行工具。

包记录包括 fio `3.33-3`、iperf3 `3.12-1+deb12u1`、curl `7.88.1-10+deb12u14`、wget `1.21.3-1+deb12u1`。各自 copyright 文件存在；文本 Source 头分别指向 `http://brick.kernel.dk/snaps/`、`http://software.es.net/iperf/`、`https://curl.se/`，wget 文本没有匹配该头。Source 头和包版本只是归档中的声明，不能替代已核对的对应源码或可复建配方。完整许可文本没有改写或用一个标签替代其多组件条件。

## 索引与边界

主盘点证据索引含 386 个文件，独立逐个重新计算 SHA256，0 不匹配；索引自身 SHA256 为：

```text
bdc69eb5b344ee112f64ed1c42b9a426d1b378e33807e5c54a16cb92a314ef72
```

固定 ELF 二次核对另有 8 文件证据索引，独立重算 8/8 全部匹配、0 不匹配，SHA256 为 `c9b344cb3e02e825f29d73fcc799235de2beac1f0c3e5ca8654f2ef8bde29749`；两个索引分别计数，不累加成独立扫描场景数。

私有索引包括资产元数据、单次取得记录、自有扫描器及控制器、实际容器状态、成员/包/选中文件记录和许可文本。归档另按固定资产摘要核对，不纳入公共文档提交。许可文本的摘要正确不代表许可审核完成。


| 自有分析文件 | SHA256 |
| --- | --- |
| `download-arm-once.py` | `e95f2e173ddf977636f4be76921bc65f5493c75fb480bb6c632b6c2449b681fb` |
| `inventory-arm.py` | `764fed0ef156760f919a282387fb07451cedec2cabd17800f7c49d1566ca24cd` |
| `run-inventory-arm.py` | `ddcdcdd198a93acc6fe075dea6c595163d873964091d34d18cc7a7e20f72078d` |
| `elf-identity-arm.py` | `9c12c8b16e2abb9d9ba02d7c6c15d99d1f2f9c7958708ce30de840300a2d737e` |
| `run-elf-identity-arm.py` | `c981c84c9a829a6a0bed78e637ae2b6c632975a516f22e05b5bcaa37e60ddf62` |

本项未验证 ARM 在线执行、运行中上传、上游或新制品签名、对应源码可复建性及分发授权；静态读取不构成这些能力的签收。

四个远端工作流按仓库安排保持暂停，本项未触发、重跑或恢复 CI；未做 Rust/前端编译、诊断负载或真实节点验收；未上传原始归档/配置、未执行归档工具或更改宿主 swap/网络。Geekbench 5.5.1、Ookla、rootfs 可复建来源和全链无上传/无宿主副作用仍待验，#28/#65/#66/#82 保持未解决，`full_ready=false` 及 Agent 完整启动拒绝不变，不签收、正式发布或部署新增诊断能力。
