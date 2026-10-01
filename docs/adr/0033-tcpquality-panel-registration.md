# ADR 0033：TCP 诊断登记与配置目标快照

状态：采用。依赖 ADR 0028 的共用诊断服务、0032 的固定签名制品与独立 TCP 参数/报告适配器。

面板将 tcpquality 作为第二个诊断插件登记，Agent 只在 Linux 的受管服务模式下加载适配器；monitor-only 不声明该能力。新任务要求签名、独立章节、共用资源预算、tcpquality 与 native-v1 能力。工具制品版本包含完整源码 SHA，不能用可变分支或在线脚本替代。

参数仅为地区预设、IPv4/IPv6、4/8 次 TCP 连接和 1/2 并发。地区在插件表里标注已启用的 core TCP 拨测目标，不扩展 ProbeSpec，不推断位置，不内置第三方名单。全部已配置目标预设保留未知地区；其他预设仅选明确标注的目标。0 个或超过 8 个目标拒绝。任务冻结目标、地区、原字节 SHA-256、参数、工具版本与无上传/无排名/无测速策略；后续配置修改不会改写历史快照。

共用服务负责创建、预算、同机互斥、上传、取消确认与历史。插件只转换参数并管理地区标签，资源预算为 64 MiB、32 tasks、CPU/IO 权重 10、OOM 分数 500、60 秒。NodeQuality 与 TCP 的 queued/running/cancel_requested 共用一个服务器锁；取消确认前保留屏障和部分章节。

不改既有表名、报告 JSON、NodeQuality API 或历史。新增插件表仅引用 network_probes，删除目标自动清理标签。地区管理接口位于 /api/plugins/tcpquality/servers/...，任务接口仍为共用 /api/servers/.../diagnostics/tcpquality。报告界面另一个 PR 接入，说明 TCP 建连统计不是包丢失、重传或吞吐测试，不作跨参数排名。
