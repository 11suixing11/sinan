# NodeQuality amd64 rootfs 静态盘点独立验收

关联 [Issue #82](https://github.com/theLucius7/sinan/issues/82)，补充 [执行依赖审计](nodequality-chain-audit.md)。本项记录已取得的归档证据与授权缺口，不修改执行器、资源预算、完整任务门禁或制品。源码合入不代表完整 NodeQuality 已恢复或实机验收通过。

## 固定输入与方法

2026-10-01（Asia/Taipei）从 [NodeQuality v0.0.2 发布](https://github.com/LloydAsp/NodeQuality/releases/tag/v0.0.2) 单次取得 amd64 `BenchOs.tar.gz`，发布资产 ID 为 `345686702`，大小为 312,475,959 字节。实读大小及 SHA256 与发布 API 元数据一致：

```text
5f844e73941c3623175c5cdc16b01db34c155d0d1bd9b0cf71f3d72e8b1148e1
```

在隔离分析容器中流式读取 tar；没有解压完整 rootfs、跟随归档链接、挂载、chroot 或执行任何成员。分析容器禁网、只读根与输入、移除全部 capabilities、启用 no-new-privileges，限制 1 CPU、512 MiB 内存、无 swap、128 tasks。单文件 32 MiB、所有选中文件 64 MiB、解压流 8 GiB、成员 100,000 个为读取上限；读取循环检查 240 秒截止，但没有独立进程墙钟硬超时。绝对路径与 `..` 成员拒绝读取。

实际扫描 5.255 秒，容器退出 0、未 OOM；读取解压流 874,588,160 字节，选中文件 12,857,194 字节，没有越界路径或跳过项。所有分析容器已删除。摘要和包/许可证清单保存在私有证据目录；原始工具、配置和实例信息没有提交或再分发。此分析没有诊断负载，不属于生产硬件压测或节点验收。

## 结果

| 项目 | 实际读回 |
| --- | --- |
| 系统 | Debian 12 bookworm，amd64 |
| tar 成员 | 30,087：25,252 普通文件、2,659 目录、2,172 符号链接、4 硬链接 |
| dpkg | 375 个已安装包记录 |
| 许可文本 | 保存 385 个普通 copyright/license 文件及其摘要；文本存在不代表已完成逐包源码与分发条件核对 |
| Ookla | `BenchOs/usr/bin/speedtest`，2,613,400 字节，ELF64/x86-64；静态字符串含 `Speedtest by Ookla` 与 `1.2.0.84`，未执行 `--version` |
| Ookla 来源 | 无对应 dpkg speedtest/ookla 包记录；保存的许可文本未出现对应工具标记，尚无可核对的来源、构建配方与再分发证明 |
| Geekbench | 成员名称、链接目标和包名未发现该名称；不排除改名代码，也不免除 HardwareQuality 在线下载 Geekbench 5.5.1 的缺口 |

Ookla ELF SHA256：

```text
31f1124c5ab8acdae6b9fe1741e704df420f9f2e7d429679fabe62075453c051
```

归档还含 122 字节的 `BenchOs/root/.config/ookla/speedtest-cli.json`，SHA256 为 `c781800a71b9fcf6d7d48e23dd4dcb79bfead81f6ca7d65f20ebac6f523f97ff`。只检查所有层级的精确键名，只有 JSON 布尔值或精确的 `true`/`false` 字符串才构成已知结果：`LicenseAccepted`、`GDPR`、`PrivacyAccepted` 均为未知。第一项的值不能识别为布尔状态，后两项未找到精确键；没有从其他字段推断接受状态，也没有公布原始配置或实例标识。

## 独立验收与限制

- 重新计算保存的归档 SHA256，仍与上面的固定资产一致；原始证据索引的 13 个文件全部摘要匹配，0 不匹配。索引自身 SHA256 为 `d1285e90659cbb8fc2592bbacf2c4484e83f3942ca8ec725369cff86ca607039`，许可字段摘要文件为 `8b658bfb60f5e55d2557ae310eececc063595071b8e978026be59d50c584d2ad`。索引验证不能代替 385 份许可文本的逐项授权审核。
- 文档改动只允许上述白名单事实、归档内部工具路径与公开上游地址；检查相对链接及差异，不提交原始归档、二进制、配置、密钥或私有节点信息。远端 CI 按仓库当前暂停安排保持未运行；没有把以前的绿色 CI 计到本项。
- ARM rootfs 已在独立项完成单次取得及受限静态盘点，见 [ARM 证据](nodequality-rootfs-arm-inventory.md)。发布 API 的 digest 与本地一致只能说明该次取得的字节身份，不能替代上游签名、可复建配方、对应源码或分发授权。
- 完整执行链仍需固定全部脚本、数据与辅助工具、核实许可，并实测禁上传、禁宿主 swap/网络改动与取消清理。Geekbench Tryout 上传要求及授权范围以实际版本的 [Geekbench 5 EULA](https://www.primatelabs.com/legal/eula-v5.html) 为依据，不使用其他主版本条款替代。
- #28/#65/#66/#82 保持未解决，`full_ready=false` 和 Agent 启动拒绝保持；本项不签收完整 NodeQuality、持续代理流量联合负载或后续 TCP 实机能力。
