# NodeQuality 第四批源码修复与未完成条件

2026-10-01，基于 main `bb9638b`。本批对应 #28、#65、#66、#82、#120；只将能以源码完成的 #120 变更提交，其他四项保留真实缺口。按用户要求，本批不执行测试、构建或 CI，等待全部批次完成后统一验证。以下是源码边界，不能当作全链实机验收。

| Issue | 当前已在 main 的措施 | 本批结果和仍待完成的条件 |
| --- | --- | --- |
| #28 | r6 固定入口、五份第一层脚本和许可证；r9 禁运行时安装；r10 固定七份参考数据 | 未完成：rootfs、实际全部二进制/二级材料的对应源码、构建配方、版本/摘要与许可证明；不能用入口 pin 或完整任务门禁代替 |
| #65 | r7 三份脚本公共报告 POST 和 r12 百分位上传由既有 upload 选项控制 | 未完成：Geekbench 5.5.1 等工具自身的许可和上传要求，实际工具链无未授权网络上传证据；脚本门禁不能授权或阻断工具自身行为 |
| #66 | r8 不加载无调用的 swap helper，取消入口 swapoff，移除 HardwareQuality 临时 swap 分配路径并保留内存拒绝 | 源码措施已有；完整任务取消、挂载及其他工具宿主副作用的受控故障验收未完成，不能据静态变换声称全链实机无残留 |
| #82 | amd64、arm64 官方 rootfs 静态盘点和 ELF 摘要已有 | 未完成：预置 Ookla 的来源/再分发证明，rootfs 对应源码/可复建配方、逐工具权利证明及完整运行验收。API digest 和许可证库存均不能代替授权 |
| #120 | main r14 Netflix 请求有效性保护；浏览器身份整改仅在未合入分支 | 本批新增 r17，实际固定 IP/Net 的 curl 走原生默认身份，继承子 Bash，curlrc 不注入浏览器头。先保留未知/失败；仍需统一回归验证 |

`browser-policy.py` 接在原 main `netflix-policy.py` 后，原 Netflix helper、固定来源和许可证字节不改。r17 的输入/输出摘要独立计算，runner、构建入口、发布版本、面板和 Agent 一致使用新身份。Agent 保留 r14/r16 的旧任务、日常模式及报告读取支持。历史 r14 与另分支 r14/r15 内容曾分叉，本批不会把已有版本换内容重新分发。

浏览器包装器保留请求方法、正文、非浏览器请求头及 curl 返回码，移除明确 User-Agent、Sec-CH-UA、Sec-Fetch 选项；`-q` 为原生 curl 首参，显式额外 config 与 header 文件拒绝。它没有扩大到任意脚本/工具的网络沙箱，也没有消除公共 cookies、网页临时 key 或所有 provider 解析问题；这些由 #121/#122 和其他查询项目单独处理。

#28/#65/#66/#82 的剩余内容需要操作者提供适配实际版本和集成方式的工具来源/权利证明，以及专用环境中的网络、命令、取消与资源故障证据。当前仓库材料不能据此推断上游违法，也无法替操作者取得许可。本批不下载 rootfs、不执行或接受专有工具条款、不修改全局 swap、不部署。`full_ready=false`、Agent 拒绝新的 full 与 CI 暂停保持。

已有静态证据分别见 [amd64 库存](nodequality-rootfs-inventory.md)、[arm64 库存](nodequality-rootfs-arm-inventory.md)、[报告上传](nodequality-public-report-policy.md)、[swap 变换](nodequality-no-swap.md) 与 [执行链审计](nodequality-chain-audit.md)。本批新增浏览器身份回归入口为 `tools/test-nodequality-browser-policy.py`；统一测试必须覆盖实际固定源、子 Bash、curlrc、原请求内容和 403/429/超时不换身份重试，当前未执行。
