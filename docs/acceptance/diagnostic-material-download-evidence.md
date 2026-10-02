# 诊断材料下载失败证据验收

关联 issue：#141。实现决定见 [ADR 0055](../adr/0055-diagnostic-material-download-evidence.md)。

当前状态：六个源码/请求文件及回归已整合，最终测试 pending。没有执行本轮 collector、factory、下载或构建；不把准备分支、旧真实材料或另一源码的通过结果移作本轮签收。

## 冻结范围

`tools/nodequality-rootfs-collect.py`、`tools/test-nodequality-rootfs-collect.py`、两份 `tools/nodequality-rootfs-request-{amd64,arm64}.json`，以及 `tools/nodequality-rootfs-build.py`、`tools/test-nodequality-rootfs-build.py`。主线 r19/r20/r21 producer helper、原来源锁及许可库存保持字节；保留主线 FIFO 替换拒绝和复制时大小/SHA256 防护。

收集器访问官方固定 Snapshot 只属于材料取得，不代表独立 keyring 信任、builder 审批或第三方许可。工厂继续仅消费本地已认证输入。收集总字节、整体/worker 期限、磁盘预留、签名链、URL 白名单与无自动重试规则保持。

## 最终检查表

| 场景 | 必须观察的结果 | 本轮状态 |
| --- | --- | --- |
| 真实回环 HTTP 403/429 | 单次 worker 请求失败，实际 HTTP 状态与分类保留，worker 回收，无成功正文缓存 | pending |
| 206、非法/截短 Content-Length、空正文及 chunked 超限 | 保存实际响应状态、具体类型/原因/阶段和字节观察，失败不输出成功材料或输入锁 | pending |
| 超时、TERM/HUP | 不增加下载时间预算或自动请求，回收本次进程组；可用 headers/receipt 有界保存，无正文 | pending |
| receipt 缺失或截断 | worker 类型/原因、HTTP 状态明确未知；父层错误单独可见，不推断底层原因 | pending |
| 总字节或磁盘预留不足 | 拒绝本次下载/证据写入，保存失败状态与原因，保持原预留，不删除认证缓存 | pending |
| 最终材料/输入锁/成功收据发布失败 | 本次成功文件撤回，认证缓存与失败证据保留，binding 拒绝不完整目录 | pending |
| 成功文件已存在或被替换，失败收据也无法写入 | 拒绝覆盖/删除未认领或改变身份的文件；回滚未完成明确记录，认证缓存及不完整目录保留 | pending |
| 证据隐私 | Cookie、凭据、query/fragment、控制字符、环境、正文、完整命令参数不进入失败证据 | pending |
| factory 的 FIFO/复制时更换、短写与增长 | 保留主线拒绝行为和逐次摘要/长度检查，结合 prospective capacity，不接受额外或改变身份的材料 | pending |
| factory 容量/取消/清理 | 独立输出身份、原容量和 inode 预留保持，进程与所创建目录实际回收；残留挂载时拒绝递归删除 | pending |

本轮应优先运行两个对应 Python 回归文件，保留第一次失败和必要修补后的重跑结果，逐项记录实际输入摘要、方法数、跳过及清理证据。回环 worker 使用私有测试替换来允许自有服务器；生产 collector 没有测试地址开关，白名单不放宽。mock 的签名/索引/镜像身份仅验证拒绝接口，不宣称真实 Debian 签名或镜像审批。

## 未签收范围

本项不执行第三方硬件或测速工具，不接受许可、打包未授权二进制、重放历史公网故障或修改生产服务。完整 NodeQuality、双架构合法工具闭包、真实 builder 和复建、持续代理业务联合保护仍需另行实际验收；#28/#65/#66/#82 与 full 门禁不因本项小型回归通过而关闭或开放。CI 继续遵从仓库暂停要求。
