# 托管验收工具与主线 API 合同

关联 [Issue #156](https://github.com/theLucius7/sinan/issues/156)，基线为已合并主线 `b9a9c725fdb92dd19e836a63820463dd1b176891`。本步骤修复验收工具的实际请求；面板旧 API、数据库及代理用户授权模型保持其当前合同。

原工具在来源创建／刷新、job 轮询、链路创建／重放、版本应用及退役时，仍请求保留的旧路径。当前有序合同分别位于 `/api/plugins/sing-box/ordered-subscription-sources`、`ordered-subscription-source-jobs`、`ordered-proxy-resources` 和 `chains/ordered-batch`。工具使用明确的四个路由常量，来源和链路资源 ID 仍是数值；请求、外部节点、节点版本、来源 revision 和 job 仍使用各自 UUID，策略组继续使用数值 `chain_ids`。

新增合同场景实际调用 driver 的创建与原子重放、inline 刷新与 job 轮询、follow／pinned 版本应用及失败来源、退役与删除后重放。测试使用明确的 TEST_ONLY 响应，不向数据库写设备回执。工具中的原生路径证明不创建周期拨测 `ProbeSpec` 或执行租约；当前已有周期拨测工具保留自身来源、地区和许可合同。

实现、合同测试代码及文档全部完成后冻结 11 份输入，冻结收据 SHA256 `f9dbde316ee4a72abdc4260306e917ba4bcbc61021f3602b3e18bfdf77c4a872`。三个控制程序只编译语法成功，冻结输入在集中执行后逐字及稳定身份保持。最初冻结工具将读取会改变的 atime 纳入相等判断，执行前触发 AssertionError；修正为设备、inode、大小、mtime、ctime、mode、uid，保留原失败记录，没有据此声称源码曾改变。

驱动合同首次 23 项执行：22 通过，新增 setup 夹具遗漏服务器响应的数值 `id` 导致 1 错误。原失败日志 SHA256 `6fceeffcae8b299f5588db05f43d29a55fa47a21ef85c72a5caf7762ff0aed08` 保留；只补齐夹具 `id`，其它 10 份冻结输入保持，重新冻结受影响输入后仅补验失败项，通过日志 SHA256 `c346f1228fe88f4ab3cb336ffa610dc3a34ec7c7bb7d05df7a1c222f2d928881`。最终有效去重 23 项通过、0 跳过；未重复 Rust、前端及不受影响工具验收。

单次只读 SSH 仅到已有跳板，继承有效的正常 agent、保存私有原始 stderr 并绑定 config／key／socket。实际连接成功，退出码 0，耗时 2.461 秒，stderr 0 字节，绑定保持且自有进程组收尾确认；收据 SHA256 `90a541b3543d41d1ef46a4cec70e2d748d356c6ba5edac955326a1ecc4dc4e5c`，840 字节。没有改远端、连接测试节点或安装服务，因此原测试节点失败原因仍未知。

测试节点连通、当前原生制品、真实注册 Agent 故障矩阵、三设备整链、完整 NodeQuality 许可／工厂及实机验收分别待验，合同回归与跳板成功不能替代它们。CI 保持暂停。
