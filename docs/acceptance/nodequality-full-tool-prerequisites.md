# 完整执行工具链的剩余条件

2026-10-01 复核以 `3bb9eb56a325f47df7fe6eed97c73ddecbfe7c11` 的 r15 为技术基线，后续 r16 查询修改另有独立验收。此前链审计描述的首次脚本、七份二级参考数据、运行时安装、swap 和部分上传问题已经分别整改；不能把旧描述当作最新实现。当前仍未运行受控完整验机，门禁保持。

用户本轮明确确认没有现成 Geekbench 5 Pro 许可。此前 Tryout 的许可及上传条件不能由管理员的完整验机确认替代。也不能自行删掉 Geekbench、GPU 或原三网测试来宣布原完整能力已经恢复。

| 原测试或制品 | 已核对事实 | 下一项实际条件 |
| --- | --- | --- |
| rootfs | 执行入口仍可下载 v0.0.2 两种 BenchOS 包，没有实际摘要/签名核对；旧压缩包超过现有 256 MiB 制品边界 | 使用可复建、完整来源与许可库存的离线环境，接入签名辅助文件和本地加载；不放宽边界接纳不明旧包 |
| sysbench、fio、内存与网络工具 | 原流程保留，脚本的 AGPL 许可不能覆盖全部二进制 | 固定 Debian snapshot、包版本与摘要，核实签名索引、对应源码和通知；保留原 CPU、内存、磁盘及网络测试 |
| Geekbench CPU/GPU | 现脚本依赖上传后的 Browser URL 提取分数；只加关闭上传参数会使解析失去结果 | 合法工具与许可、精确两个架构身份、真实本地导出样本、完整本地解析；无许可不执行，不虚构得分 |
| Ookla Speedtest | 原代码自动接受 license/GDPR；旧 rootfs ELF 来源不完整 | 实际适用条款与使用场景授权、明确接受记录、合法固定工具身份和本地输出；固定包摘要不等于获得再分发授权 |
| full 资源预算 | 当前默认 MemoryMax=512MiB；原硬件脚本约 950MiB 检查读取宿主内存 | 在合法工具可用后测量完整峰值，定义预算及预检；宿主 MemAvailable 不能证明 cgroup 足够 |

[Geekbench 5 官方 EULA](https://www.primatelabs.com/legal/eula-v5.html)对 Pro 的文档化 CLI、Standalone 和自动化另有允许条件；不能笼统写成所有自动化都禁止，也不能推断普通 Pro 已允许项目公开分发工具。[CLI 文档](https://primatelabs.tenderapp.com/kb/geekbench/geekbench-5-pro-command-line-tool)提供无上传与本地导出选项，[Standalone 文档](https://primatelabs.tenderapp.com/kb/geekbench/geekbench-5-pro-standalone-mode)提供便携方法。尚缺适用许可和真实 5.5.1 CPU/GPU 导出结构，不用猜测样本认证解析。

官方发布者的 Bookworm 包可作为后续身份核对入口：[amd64 1.2.0.84](https://packagecloud.io/ookla/speedtest-cli/packages/debian/bookworm/speedtest_1.2.0.84-1.ea6b6773cf_amd64.deb?distro_version_id=215)摘要 `35e084567a6388631fb10cf01e5e0d6b57a67d34ede2b72ba111b3d9164c8b94`；[arm64 1.2.0.84](https://packagecloud.io/ookla/speedtest-cli/packages/debian/bookworm/speedtest_1.2.0.84-1.ea6b6773cf_arm64.deb?distro_version_id=215)摘要 `98e7de9db3bf181d08bc67e647bcfc71349c8014e387289c08e54e5c55d82f37`。页面限定个人、非商业用途；本轮没有取得完整适用 EULA、下载或执行包，不能证明旧 rootfs 中 ELF 与它相同。

可继续推进可复建开源 rootfs、签名本地加载及许可材料的独立适配，不因没有 Pro 许可停止这些工作。环境至少应覆盖原 Bash/文本工具、curl/CA、jq/bc、网络与路由工具、硬件信息工具、sysbench/fio、mtr/iperf3/stun 和固定源码 NextTrace；GPU 运行库和许可工具分别验证。按 [Debian 签名链](https://manpages.debian.org/bookworm/apt/apt-secure.8.en.html)保留索引、可信 keyring 和来源，并用 [snapshot](https://snapshot.debian.org/)锁定环境。只有制品、权限及完整实机故障矩阵均有证据，才改变阶段签收状态。

本文件是当前缺口复核，不是构建或实机验收证书，不恢复暂停的 CI，不改变生产边界。
