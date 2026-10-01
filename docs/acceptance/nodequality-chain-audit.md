# NodeQuality 实际执行依赖审计（Issue #28）

2026-10-01 只读审计。入口固定为 a92fca6c0067df29ddd03fdc2fee6f3000f64545；r2–r5 制品与字节保持不变；r5 仅修正常退出契约，仍使用同一上游执行链。没有运行上游 benchmark，没有在生产执行脚本。新完整任务安全门禁只止血；完整离线工具链、许可证清单与副作用验收仍未完成，不关闭 #28/#65/#66/#82。

## 实际依赖

入口的 raw_file_prefix 仍为 NodeQuality/refs/heads/main。运行时 source part/swap.sh，在 chroot 内执行 part/header.sh，并执行 Hardware.Check.Place、IP.Check.Place、Net.Check.Place。三个入口在审计时实际重定向到对应 xykt 仓库 main，已分别下载固定提交并逐字节比较一致。

| 文件 | 本次固定提交 | SHA256 |
| --- | --- | --- |
| NodeQuality.sh | a92fca6c0067df29ddd03fdc2fee6f3000f64545 | 4e1b25894cadf908ef61fb0d9ce874a75524c6dafc2ea26f0477107288e0c018 |
| part/header.sh | 同上 | d6b1990f815bcdb42ac978941b9edf841556c4861e453c23e9bef41b66e4d03f |
| part/swap.sh | 同上 | 5406da3ab0ff47105f0c06dcbb9fbb34bbb9c5e78c801095c60597b8c6a9d43e |
| HardwareQuality/hardware.sh | 06f99880d516bb744afa2948261b9697c79789e2 | 73e032ef5409e014cca411a71c677a76db19a94ef0a96a73827b41b2059cd86c |
| IPQuality/ip.sh | 87397e2c3196ec796f5477c83343c2354df601ea | b30df5a3c2204276c54e99dcc5080b46f8a627667730aee7de63b109b8ecaecf |
| NetQuality/net.sh | d5b99484d51286374d24b892c1b54235dc282148 | 6c40fe1ae40d969255cb63075c94882733b82ba43831341eb1aadeea7b1fbfcd |

rootfs 下载：
- https://github.com/LloydAsp/NodeQuality/releases/download/v0.0.2/BenchOs.tar.gz：312,475,959 字节；GitHub 发布资产 digest 为 sha256:5f844e73941c3623175c5cdc16b01db34c155d0d1bd9b0cf71f3d72e8b1148e1。
- 同版本 BenchOs-arm.tar.gz：359,657,375 字节；发布 digest 为 sha256:a4dd4e55b129157a02dab437b78e41b5797a7a79a0f0b7febdecab8eb2a312c7。
- v0.0.2 tag 指向 b9967df1797d4079c89b008bb888bc6ef1fb2672。仓库该提交未提供 rootfs 构建配方或包/许可证清单。初次审计没有下载 rootfs；之后已完成 amd64 归档摘要和有界静态盘点，发现预置 Ookla 二进制缺少可核对的来源与再分发证明，见 [rootfs 独立证据](nodequality-rootfs-inventory.md)。ARM 归档已另行完成有界静态盘点，见 [ARM 独立证据](nodequality-rootfs-arm-inventory.md)；资产 digest 不代表签名、完整来源或授权。
- 现行入口下载后直接解压，没有比对摘要或签名。两个 rootfs 压缩包均超当前 256 MiB 单执行文件/辅助文件上限，不能直接塞入既有单文件 runner。不得为了接受不透明包而放宽全局制品限额。

NextTrace：
- https://github.com/nxtrace/NTrace-core/releases/download/v1.3.7/nexttrace_linux_amd64；ARM shim 改成同版本 nexttrace_linux_arm64。
- v1.3.7 tag 为 69588b0d14187ee00f48aa04e5617256148c3ecf。发布 API 未给二进制 digest；本次仅下载、不执行，观察到 amd64 SHA256 cffdcbbb4ed328b9a4e238dde9cb68d4263b3a5769f340d8a55a98fa8e99294c，arm64 为 644d4994b2e949a8adc5b8a4bf9bcb77b1367d3b995d3a6370b671f0ed57fcf0。观察摘要不是上游签名。
- 入口 wget 后 chmod，未校验；内层若探测不到工具还会 curl nxtrace.org/nt | bash。

仍需固定的二级依赖包括 IPQuality 的 iso3166.json、dnsbl.list、cookies.txt、iata-icao.csv；NetQuality 的 AS_Mapping.txt、iso3166.json、province.json、iperf.json、speedtest_cn.json 和 speedtest/stun 二进制。三脚本也会读取 main 广告/赞助数据；这些不应留在产品执行流程。HardwareQuality 有在线 Geekbench 5.5.1 与 main curl-impersonate 下载；依赖缺失时三脚本会运行包管理器安装。

## 副作用与上传

1. **宿主 swap，#66。** 固定入口在宿主 source swap helper。helper 的 MemTotal <1024 MiB 且 MemTotal+SwapTotal <1500 MiB 分支会 dd/mkswap/swapon；r4 自动应答首个 y。隔离无特权 shell 中只覆盖 free、dd、chmod、mkswap、swapon 为记录函数，输入 y，实际记录到 988 MiB 文件与 swapon 命令，没有运行真实修改。HardwareQuality 另有 .gb5_tmp.swap 分支。MemorySwapMax=0/PrivateMounts 不把全局 swapon 变成私有操作。
2. **内层公开上传，#65。** 入口传 -y -o json，没有 -p；三个脚本在 mode_privacy=0 仍 POST upload.check.place（Hardware 使用 HTTP）。host curl shim 只拦 api.nodequality.com，chroot 内 curl 没经过此 shim。-o 只输出文件，不能证明禁上传。Geekbench trial 还有工具自身结果上传。
3. **UA 与网络行为。** IP/Net 生成随机浏览器 UA，并用于页面/非正式入口请求；IP 的 ref/cookies.txt 有 --retry 3。不能以 UA 仿冒、公开页面抽取临时 key 或暴力重试修复查询失败。-L 只跳过 Ookla 三网测速，不禁 iperf 公共测速；目标数据仍来自在线 main。
4. **宿主可见资源。** rootfs 挂载 /proc、/sys、/dev；chroot 不隔离内核/网络。当前 fio 使用工作目录临时文件（256 MiB–2 GiB），此次未发现这一分支直接写原始块设备，不能把 /dev 可写暴露推导成绝对无宿主副作用。

## 许可证缺口

NodeQuality 与三个 xykt 仓库固定提交均提供 AGPL-3.0 全文；NextTrace 固定提交提供 GPL-3.0。脚本自身有许可证，不代表 rootfs 内工具自动获得相同授权。需保留版权、完整许可证、修改记录、对应源码与构建配方，并逐一核对二进制来源。

实际 HardwareQuality 依赖是 Geekbench 5.5.1。Geekbench 5 的官方 EULA 第 2(a) 节要求 Tryout 自动上传结果，第 2(b)/(c) 节的付费许可在其适用范围内允许自行决定上传；第 4(d)/(g) 节分别限制再分发与自动化。尚未取得适用于实际版本、使用者、自动执行和集成分发的授权证明。不能用 Geekbench 6 条款或一枚 Pro key 代替这些证明。

amd64 rootfs 已确认预置静态标记为 Ookla 1.2.0.84 的 ELF，但 dpkg 记录及保存的许可证文本没有给出该工具可核对的包来源或再分发证明。归档文件名未发现 Geekbench，不代表完整执行链不再下载它。Ookla 的精确适用条款仍未核实；三个许可/隐私配置字段均为未知，不能推断已接受或未接受。不打包授权未知的专有二进制，也不替设备管理员自动接受 --accept-license/--accept-gdpr。

[NodeQuality 固定源码](https://github.com/LloydAsp/NodeQuality/tree/a92fca6c0067df29ddd03fdc2fee6f3000f64545)、[rootfs 发布](https://github.com/LloydAsp/NodeQuality/releases/tag/v0.0.2)、[NextTrace 固定许可证](https://github.com/nxtrace/NTrace-core/blob/69588b0d14187ee00f48aa04e5617256148c3ecf/LICENSE)、[HardwareQuality](https://github.com/xykt/HardwareQuality/tree/06f99880d516bb744afa2948261b9697c79789e2)、[IPQuality](https://github.com/xykt/IPQuality/tree/87397e2c3196ec796f5477c83343c2354df601ea)、[NetQuality](https://github.com/xykt/NetQuality/tree/d5b99484d51286374d24b892c1b54235dc282148)、[Geekbench 5 EULA](https://www.primatelabs.com/legal/eula-v5.html)。

## 后续制品流程

无论保留上游还是选择自有有限检查，都必须发布新不可变工具版本。固定依赖清单应包括每个执行/数据文件的来源提交、SHA256、架构、许可证及对应源码；rootfs 如保留必须有可复建配方和包清单。只允许构建时从固定地址获取并核验；运行时只读本地签名制品，不允许在线 main、安装器或更新 fallback。完整文件摘要纳入 release.json，SHA256SUMS 包含 release.json 并由现有发布信任根签名；打包/装载严格检查文件名、大小、摘要和版本，改一个字节必须验签失败。

网络/副作用验收必须运行隔离夹具：所有下载/上传/包管理/swap/内核修改命令可记录并拒绝，固定探测目标白名单可见；禁公开报告与工具自身上传，不使用 browser UA/cookies/key 抽取绕过源限制；403/429/超时明确报告未知。取消、重启、断连、部分章节、持续代理流量仍按独立验收矩阵测试。仅搜索字符串或只拦顶层 curl 不足以宣称完成全链整改。
