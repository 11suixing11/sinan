# 诊断资源预算独立验收

对应第 1 步的 systemd 资源预算项及 Issue #14。诊断适配器返回类型安全的 ServiceJob 预算，core 通过 Privileged/ServiceManager 边界将预算传给 systemd-run。NodeQuality 和缺少预算字段的旧持久任务均采用下表的默认值。

| 属性 | 默认值 | 输入约束 |
| --- | --- | --- |
| MemoryMax | 536870912 字节（512 MiB） | 正整数，排除 systemd 无界值 u64::MAX |
| TasksMax | 128 | 正整数，排除 u32::MAX |
| CPUWeight | 10 | 1–10000 |
| IOWeight | 10 | 1–10000 |
| OOMScoreAdjust | 500 | 0–1000；诊断不能获得负值保护 |
| MemorySwapMax | 0 | core 固定禁止诊断使用 swap，不开放参数 |

权重只在相应 cgroup 控制器下争抢资源时生效，不是固定 CPU/I/O 限速。MemoryMax/TasksMax 限制整个诊断服务的子进程；原有 KillMode=control-group 和 PrivateMounts 保留。上游程序在预算内无法完成时可以失败，不保证跑分完整。

## 自动验收

在普通开发机运行：

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

完整 workspace 测试需要按现有开发流程配置独立 PostgreSQL 数据库和公开 TEST_ONLY 信任根。以下为本项的独立判定条件：

- SDK 构造和反序列化拒绝零上限、无限值、越界权重、负诊断 OOM 值及非数字字符串；显式错误值不能静默回落到默认预算。
- NodeQuality prepare 返回默认预算。命令捕获测试用默认值和另一组自定义值启动服务，检查每个 systemd 属性只出现一次且位于 `--` 之前；形似属性的程序参数保留在 `--` 之后，不能覆盖预算。
- 自定义预算 JSON 持久化往返保持原值。没有新增字段的旧 SQLite checkpoint 在 Agent 重启后仍被观察，不重复启动；最终报告保留并回传。

## Linux/systemd 行为验收

只在专用测试机执行，需要 Linux cgroup v2、systemd 系统管理器、Python 3 和 root。现有 Linux CI 已显式运行以下两个 ignored 专项：

```sh
cargo test -p sinan-agent-core --lib --no-run --message-format=json > /tmp/sinan-core-tests.jsonl
diagnostic_test_binary=$(python3 -c 'import json; print(next(item["executable"] for item in (json.loads(line) for line in open("/tmp/sinan-core-tests.jsonl")) if item.get("reason") == "compiler-artifact" and item.get("executable") and item["target"]["name"] == "sinan_agent_core"))')
sudo "$diagnostic_test_binary" real_systemd_diagnostic_ --ignored
```

判定条件：

1. `real_systemd_diagnostic_jobs_survive_manager_recreation_and_enforce_timeout` 读回真实单元的五项预算和 MemorySwapMax，逐个核对值；原有成功、失败、管理器重建和超时行为仍通过。
2. `real_systemd_diagnostic_resource_budget_restricts_memory_and_children` 将单元限制为 64 MiB/8 个任务。分配 256 MiB 的 Python 夹具必须失败且 systemd Result 为 oom-kill；创建子进程的夹具必须在达到 8 个任务以前收到 EAGAIN，然后回收子进程并成功退出。停止后 MainPID/ControlPID 均为 0。

这些夹具只运行有限的本地内存分配和最多 64 次派生尝试，不执行完整 NodeQuality、网络测试或上传。实际流水线结果须对应本 PR 最终提交，不能用测试配置代替执行证据。

## 兼容及验收边界

本项的资源预算强制执行仅覆盖 systemd。当前 `SystemServiceManager::start_diagnostic_job` 要求 systemd 及可验证的 swap 系统调用保护，OpenRC 新诊断明确拒绝，不能降级为仅输出预算警告后继续执行。旧 OpenRC 任务的观察和停止实现继续保留，以便恢复与清理；其他原生平台未新增诊断服务支持。ServiceJob 中持久化的有限默认值不代表 OpenRC 已获得同等的内存、任务数、CPU/I/O 权重或 OOM 限制，不能用每进程 rlimit 代替 cgroup 限制。

新字段只影响此版本启动的新单元。升级期间已经运行的旧单元继续按已有 systemd 属性运行，默认反序列化用于恢复和观察，不会追溯设置属性或重跑任务。

本项不新增可用内存/磁盘/负载预检，不调整常驻服务，也不实现诊断取消和章节完整度。小内存的真实完整验机、持续业务流量下的心跳/运行时存活、取消后的挂载清理需在后续保护项完成后联合验收。查询源的 403/429/超时和磁盘不足属于独立修复项，不以本项资源夹具认定已通过。

上文自动检查与真实夹具结果属于原资源预算提交，后续联合负载和日常链路记录见 [整改顺序](ordered-remediation.md)。2026-10-01 五项 issue 批次只审查已有预算与修正文档，未运行测试；最终整合测试及完整上游负载分别记录，不能沿用旧提交的结果认证当前代码。
