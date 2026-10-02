# #63 轻量三网周期监控验收

2026-10-02：下列用例和控制器说明保存源码冻结前的准备范围；随后统一验证的实际输入、失败与通过记录见 [集成验收](remaining-issues-20261002.md)。准备时未执行的声明不作为最终状态。

- `crates/protocol/src/tasks/probes.rs`：`crates/protocol/tests/probe_authorization.rs` 的 `historical_probes_remain_readable_without_implicit_permission`、`permission_binds_exact_identity_and_expires_at_the_boundary`。缺省授权暂停，旧结果序列化兼容。
- `crates/agent-core/src/tasks/probes.rs`：`tcp_probe_measures_a_real_listener_and_closed_port`、`selected_ipv6_records_the_actual_family_and_rejects_wrong_family`、`invalid_target_is_unavailable_instead_of_a_loss_measurement`。真实自有回环 IPv4/IPv6、拒绝连接、无授权及错误版本均保留实际/未知语义。
- `crates/agent-core/src/tasks/probes/scheduling_tests.rs`：`slow_probes_do_not_delay_results_and_configuration_changes_cancel_work`、`expired_leases_future_cache_times_and_invalid_authorizations_fail_closed`、`lease_expiry_cancels_work_and_persistence_failure_does_not_stop_the_scheduler`。最多四并发，撤销/到期/配置过期停止，故障不杀调度任务。
- `crates/agent-core/src/state/storage.rs`：`real_sqlite_full_rolls_back_and_retries_without_erasing_durable_samples`、`full_acknowledged_cleanup_retains_rows_until_storage_recovers`；`telemetry/worker/tests.rs`：`full_storage_keeps_worker_alive_and_retries_the_same_snapshot_after_recovery`；`transport/connection/tests/storage.rs`：`full_storage_retains_clock_and_unacked_usage_without_interrupting_authenticated_heartbeat`。真实 SQLite 页上限触发 FULL；旧样本、时间下限和设置保留，状态锁在周期之间释放，结构/协议错误不降级为存储重试，认证后的实际 20 秒心跳和 ACK 重放独立于写入成功。
- `crates/panel/tests/latency_tasks.rs`：`assignment_defaults_keep_wire_compatibility_and_preserve_measurement_identity`、`authorization_metadata_is_scoped_immutable_and_revocation_preserves_history`；既有 `crates/panel/tests/tasks.rs` 覆盖删目标迟到结果、设备隔离、重复内容冲突与完整一天历史。
- `web/tests/probes.test.ts`、`web/tests/carrier-monitoring.mjs`：1440/390 自有回环 fixture；空目标默认未授权、保存来源/范围、三网/地区/实际 IP 版本、失败率及原因、撤销后的旧成功值不冒充当前状态、服务器筛选、请求失败、零外部/未知 API/浏览器错误。浏览器 fixture 只验证 Web API 展示，不代替 Agent 调度证据。

## 专用 Debian 验收控制器（执行前方案）

`tools/carrier-monitoring-acceptance.py` 使用专用 Lima Debian 12 ARM64、2 CPU、1536 MiB、无交换文件机器。必须在最终源码冻结并提交后，从 `tools/prepare-plugin-install-acceptance.py` 新命名空间编译的 GNU Agent/Panel 收据取得二进制；核对源码提交、每个二进制和本控制器摘要。只复用已审核私有 `common.py`/`finish.py` 的限额服务与逐步清理方法，并核对助手摘要。另核对已有可信签名、归档和 ELF 二进制身份收据；不下载新运行时、不安装正式制品、不修改服务模板或系统账号。

示例最终执行参数中的 `<冻结提交>`、`<新构建目录>`、`<运行时签收文件>` 和 `<新验收目录>` 必须由本次实际冻结/构建证据替换，不能复用旧源码二进制。仅在统一最终测试授权后，设置 `SINAN_REMAINING_TEST_SIGNAL=1`，在专用 Guest 中执行：

```sh
SINAN_REMAINING_TEST_SIGNAL=1 /usr/bin/python3 <新构建目录>/src/tools/carrier-monitoring-acceptance.py \
  --built-root <新构建目录> --source-commit <冻结提交> \
  --runtime /home/l7.guest/sinan-joint-load/runtime/sing-box-1.14.2-arm64 \
  --runtime-receipt <运行时签收文件> --output <新验收目录>
```

控制器独立使用 PG 55682、面板 55782、回环代理网关 55882；Agent 128 MiB、独立运行时 96 MiB，服务限制零交换文件、最多 128 任务、只允许 localhost。四个自有不读取 accept 队列的监听端口产生真实 TCP 超时；从 Agent 的 socket inode 和 SYN_SENT 观察最多四并发。运营商标签用于配置与隔离对照，不宣称回环具备真实公网三网线路质量。

逐阶段保留实际心跳、每个 1 KiB 双向代理传输及连接观察：四个慢目标不少于 45 秒、重启保留并重发原样本、断连至少 123 秒并额外观察到期后 12 秒无新样本、恢复后继续心跳、暂停/撤销/删除后迟到结果丢弃与去重。每个不少于 42 秒的在线窗口要求至少两个相差 19 秒的真实心跳、间隔最多 30 秒；每次传输有 3 秒进程预算，任何首失败均保留且使总验收失败。

满盘只填满独立 16 MiB tmpfs，不填 Guest 根盘。观察实际 SQLite FULL/IOERR 日志，至少 45 秒保留心跳/运行时/PID，释放填充文件后要求正常周期恢复、无重启。清理持续执行所有已拥有步骤：停止自己服务、保留 SQLite/PG 私有证据、卸载自己 tmpfs、移除自己临时 unit/运行时目录，检查监听、子进程、OOM、SSH/启动/交换状态不变。失败收据不覆盖、不凭简单重跑签收。

准备时 `carrier-result.json` 与 `cleanup-result.json` 尚未生成；实际执行后的结论以 [集成验收](remaining-issues-20261002.md) 中的独立收据为准。实际 FreeBSD/Windows ICMP 工具/权限以及旧二进制真实升级仍另列未验证范围。使用自有或有记录明确许可的目标，不测试生产，不借此签收完整三网排名。

整合主线后，唯一协议模型为 `spec.monitor={network,region,address_family,authorization}`，授权绑定 `identity` 并使用 `kind`/`enabled`；表单依据/确认只转换为此模型。迁移 0029 仅暂停缺少授权对象的旧记录，已有主线记录保留并由 Rust 严格校验。主线 `?authorization=1` 保留原形状；新授权端点携带三网标签。主线旧离线 Agent 自身 24 小时缓存策略不能由面板更新立即改变，本版 120 秒租期由当前源码实机验证。

## 最终实际执行（2026-10-02）

冻结 `fc4c9019` 的 r5 GNU Agent/Panel 在新 r3 命名空间完成五阶段实机验收，447.82 秒、415 次真实 VLESS 回环代理往返零失败；四并发、重启原样本补报/去重、135.296 秒断连租期、45 秒真实 FULL 与 42 秒正常恢复、撤销/暂停/删除迟到结果全部通过，最终普通 umount 和完整清理通过。完整来源、摘要、首次失败和平台边界见 [集成验收](remaining-issues-20261002.md)。

验收控制器等待 PostgreSQL 真正接受请求，仍使用原 20 秒预算。FULL 阶段先 checkpoint 并固定有界 WAL 快照，使实际新写入在 16 MiB tmpfs 写满后不能借回收旧 WAL 空间绕过故障；正常及异常路径均先释放 reader，再释放填充文件。所有只读查询和备份两端用 `closing` 显式关闭，即使第二次连接失败也先回收第一连接，备份后普通卸载不依赖 Python 垃圾回收。r1 的就绪竞争和 r2 的卸载失败保留为失败，成功来自新 r3 全矩阵。

Linux ICMP 在另一独立命名空间实测 IPv4/IPv6 回环各四个结果，每个真实四次探测、零丢包、有穷延迟，真实 ACK/PG 每个 UUID 一条；45 秒窗口的三次心跳相隔 20 秒，最终清理通过。macOS 双栈另外完成真实 echo 与修复后 15 个用例；FreeBSD/Windows 权限与工具实机仍没有据此签收。
