# PR #140 最新主线整合与统一验证

本记录对应本聊天对作者 `70bb8416619c299ed8e832eea3bd2f8a5e42e09a` 和已合 PR #138 主线 `10dc9727e0161b64806dee320234ce7505fdefc5` 的正常整合。实现阶段未运行测试、构建或格式检查；全部源码完成后再统一验证。作者的 [剩余 issues 收据](remaining-issues-20261002.md) 继续记录其独立私有执行，本聊天没有重演其中的 Debian、native 制品或正式节点验收。

## 整合与补修

- 保留主线全部监控、DDNS、签名信任、接入保护、退役清理、授权、旧报告与账本。旧 0001–0032 迁移逐字不变；作者分支新增 0029 授权迁移以原 SQL 字节顺延为唯一 0033。真实旧 schema、授权与历史样本保留及重复迁移回归均包含在完整验证中。已有监控/DDNS ADR 0047/0048 保留；新增离线材料、节点查询及运营商监控决策为 [0049](../adr/0049-nodequality-offline-rootfs.md)、[0050](../adr/0050-official-node-ip-query.md)、[0051](../adr/0051-authorized-carrier-monitoring.md)。作者旧私有编号数据库仍须按原收据核对，不能把它当作当前 33 条迁移已经升级的数据库。
- 保留独立 1 秒采样、3 秒实时展示与 60 秒持久确认。整合可恢复 SQLite 写入错误时，确认落盘失败停止本轮旧队列追赶，之后重试；实时消息与未确认身份保持。真实 SQLite FULL 回归核对采样、确认及恢复。
- 周期监控保留精确目标及授权、期限与迟到拒绝，补线路分类、地区/家族及短配置租期。签名 r21 节点 IP 查询只通过精确版本/模式入口，通用新任务仍保留 full 启动门禁；仅新查询路径补专用地址过滤，旧共享 IP 结果与历史原语义不变。
- 固定节点输入和离线材料复制使用有界、不可跟随链接的已打开普通文件；复制与摘要验证使用同一 FD，替换成无写入端 FIFO 不会阻塞。节点查询清除代理、CA 覆盖及 TLS 密钥日志环境，保留私有凭据、固定 HTTPS、无自动重试与绝对截止。原 r19 生产者、17 固定上游来源及其许可证保持原字节；r20/r21 为明确派生候选，所有 full 门禁及精确 Started 回收保持。
- sing-box 各写入口在处理器执行时重新核对同步资源快照和确认修订：刷新 pending、读取失败、已选实体消失或修订变化拒绝陈旧写入，保留草稿；恢复后重新核对服务器、节点、路径及订阅版本，不静默改选。保留统一节点路由、资源详情、运维入口和原全部业务保护。

## 本聊天最终验证

最终 Rust 及测试冻结为 `557cccfcc68f4098452500745d0d52cbe2eed4e1`；后续仅追加本记录和进度，运行输入不变。证据位于本任务耐久目录 `/Users/l7/.local/share/sinan-merge-test/evidence/`。

| 范围 | 真实结果与冻结边界 |
| --- | --- |
| 完整 Rust/PostgreSQL | 86 结果组，680 通过、0 失败、20 条件忽略；`pr140-final-rust-repair1-20261002/proof.json` 与原日志 |
| 独立验证 | macOS umask077 原子写/链接 1 项；全 targets Clippy、fmt、core 边界通过；真实 macOS IPv4/IPv6 回环 ICMP 1 项另计，不改完整组的 20 忽略统计 |
| 前端 | `16dd8b6` 的 Bun 54/1265 断言，两轮强制 TypeScript/Vite 129 模块、21 文件 dist 逐字复现；生成产物提交 `8aa207b`，当前源码、产物和夹具由 `pr140-final-web-20261002/final-head-map.json` 映射到 `557cccf` |
| 实际 Chromium | 30 套当前唯一覆盖为首轮 29 套通过，加修正后 display-data 单套通过；未宣称首轮全过或全部重新执行。新资源快照夹具在 1440/390 共 22 场景，40 次失败/pending 强制提交零写，20 次恢复后各一次预期写；全部为私有回环 API，详 `final-coverage-proof.json` |
| 编译内嵌 | 同 `557cccf` 的 frontend 1 项通过；启用 `rust-embed/debug-embed` 后实际编译 handler 对 21 个 Git 产物的 HTTP GET/HEAD 字节、SHA、长度、MIME、缓存及 nosniff 全匹配，根 index 一致；`pr140-final-embedded-rust-20261002/proof.json` 和 `embedded-asset-proof.json` |
| 相关 Python | 9 组 132 完整方法通过、19 条件方法跳过、1 类初始化跳过、0 子例跳过、0 失败/错误；其中新 5 组 70 通过/9 Linux 条件跳过，不能相加。固定 17 来源共 687969 字节逐锁 SHA 通过，两个真实 FIFO 负例通过；`pr140-python-final-local-20261002/proof.json` 的 `8aa207b` 输入映射到当前未变工具/部署/插件源码 |
| 静态 | 20 个变动 Python AST、bootstrap shell 语法、4 份 workflow actionlint、88 本地链接通过；旧 canonical 与 PowerShell 文件按 Git blob/mode 保留，没有重复旧 299 项 Python 或宣称新 Windows 验收 |

首轮完整 `8aa207b` 为 679 通过、1 失败、20 忽略：既有采样恢复夹具先接到 started 的 watch 通知，再递增独立计数，第三次采集随后阻塞，测试可能没有下一条通知而超时。只在夹具补明确 Notify 同步，保留原 2 秒等待、产品 5 秒采样预算及旧身份/恢复全部断言；完整复验通过。首轮 `pr140-final-rust-20261002` 和原日志保持原失败。

浏览器首轮 display-data 的旧断言漏掉新增线路备注，只修一行精确新显示契约，保留真实零、未知/撤销、权限清除及历史断言；产品和 21 dist 未改变。Python 编排首 actionlint PATH 缺失的环境失败保留，只用已有绝对工具路径补静态，没有重复九组或把环境失败记为通过。

## 未验证与收尾边界

20 个完整组条件忽略仍按原日志要求专用 Linux/root/systemd、固定正式运行时、Pebble 或平台工具。没有重演作者私有 Debian、跨平台正式安装/升级、r20 rootfs 构建、真实提供商凭据/查询、公开 Reality 根因、流媒体、真实 DNS/通知或长期资源压力；局部 helper、签名替身和只读库存不证明完整工具权利、公共上传、副作用或 512 MiB full 足够。新原生 TCP 发行及生命周期收据也不由本聊天重复认证。

自己的 PostgreSQL `127.0.0.1:55432` 与临时内嵌 HTTP 均已停止。四源码 GitHub Actions 继续 `disabled_manually`，未执行不算通过；公开 Release 仍为 `agent-v0.3.0`，本聊天未正式签署、发布、生产部署或评论/关闭其他聊天的 issue。最终 PR head/main 与全部受验输入的逐文件对应由 `pr140-final-delivery-20261002/final-source-map.json` 及实时 `final-main-proof.json` 收尾记录。
