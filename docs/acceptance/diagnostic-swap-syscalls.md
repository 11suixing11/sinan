# 诊断单元 swap 系统调用保护

关联 [Issue #66](https://github.com/theLucius7/sinan/issues/66)。本项只保护 systemd 诊断单元及其直接 fork/exec 子树的 `swapon`、`swapoff`，不修复上游 swap helper，不关闭 #66，不解除完整 NodeQuality 门禁。

## 固定限制与保守拒绝

所有新 systemd 诊断单元固定设置：

```ini
NoNewPrivileges=yes
SystemCallArchitectures=native
SystemCallFilter=~swapon swapoff
SystemCallErrorNumber=EPERM
ExecStartPre=/usr/bin/awk 'BEGIN { while ((getline line)>0) { split(line,f,":"); if(f[1]=="NoNewPrivs") n=f[2]+0; if(f[1]=="Seccomp") s=f[2]+0; if(f[1]=="Seccomp_filters") c=f[2]+0; } exit !(n==1 && s==2 && c>=2) }' /proc/self/status
```

四项限制均位于 `systemd-run` 的程序分隔符之前；诊断参数不能覆盖它们。五个资源预算字段及默认值保持原样，`MemorySwapMax=0` 继续限制该 cgroup 使用 swap，与禁止更改宿主 swap 的系统调用分别生效。

启动前通过 `Privileged::execute_bounded` 顺序查询运行中 manager 的 `Features`、PID 1 的名称与 seccomp 状态，以及内核的 `actions_avail`。每次最多 3 秒、16 KiB 输出；执行失败、超时、截断、未知字段都拒绝创建诊断单元。必须确认 `+SECCOMP`、PID 1 为 systemd、`Seccomp=0`、`Seccomp_filters=0`，内核支持 `errno` 动作。只看客户端 `systemctl --version` 无法证明当前 manager 能力。

上述条件仅证明具备支持条件。[systemd 252 实现](https://github.com/systemd/systemd/blob/v252/src/shared/seccomp-util.c)还允许 `SYSTEMD_SECCOMP=0` 或内核探测失败时跳过过滤。因此同单元 `ExecStartPre` 在启动诊断程序之前检查其自身 `NoNewPrivs=1`、`Seccomp=2` 和 `Seccomp_filters≥2`（native ABI 与 swap deny 至少各一层过滤）；过滤没有安装时检查失败，任务转为失败，诊断程序不执行。预命令不使用 shell、Python、联网或新的常驻 helper。PID 1 已继承过滤的环境保守拒绝，防止预命令将旧过滤误当成新限制。属性读回和 `Seccomp=2` 都不能单独证明某一个 syscall 被拒绝，具体拒绝结果须由下文真实负对照测试证明。安装检查针对本次固定配置识别缺失/仅一层过滤，不构成任意未来 systemd 变体的形式证明；验收仅证明实际受测 manager/内核组合。

支持边界收紧为可验证的 systemd 系统 manager；带继承过滤的容器、缺失 `Seccomp_filters` 字段的旧内核、非 systemd PID 1、缺少 awk 或不支持所需 transient 属性的 manager 均不能执行新诊断。OpenRC 新诊断明确拒绝，不降级为无过滤运行；OpenRC 代理运行时与其他常驻服务管理、旧诊断状态读取、停止和回收继续保留。旧 Agent 和升级前已运行的单元不会自动获得新限制。

[systemd 252 官方文档](https://github.com/systemd/systemd/blob/v252/man/systemd.exec.xml)建议配合 `SystemCallArchitectures=native`，防止兼容 ABI 绕过本机 ABI 的过滤；本项固定此属性，非 native ABI 工具不能执行。[内核 seccomp 文档](https://docs.kernel.org/userspace-api/seccomp_filter.html)说明过滤可由 fork/exec 子树继承。保护不覆盖诊断请求 D-Bus、systemd、cron 或其他宿主 daemon 另建的进程；不阻止 `dd`、`mkswap` 或其他文件写入，不提供完整宿主沙箱，也不解决在线依赖、内部上传和工具分发许可。

## 独立验收

只允许在获授权的可销毁 Linux/root/systemd guest 运行真实 syscall 夹具。不得在生产机器或远程宿主执行，不创建 swap 文件、不挂载、不执行上游 NodeQuality。

```sh
cargo fmt --all --check
python3 tools/check-core-boundary.py
cargo clippy --locked -p sinan-agent-core --all-targets -- -D warnings
cargo test --locked -p sinan-agent-core
```

真实夹具位于 `crates/agent-core/src/system/syscall_protection/acceptance.rs`，使用 `ServiceManager::start_job` 创建受控单元；Python ctypes 仅对 UUID 私有 root-owned 0700 目录内永不创建的路径调用两个 swap API。命令需要额外显式 guest 开关，常规测试忽略该项：

```sh
sudo env SINAN_SWAP_TEST_DISPOSABLE_GUEST=1 \
  <本次源码构建的 core 测试二进制> \
  system::syscall_protection::tests::acceptance::real_systemd_swap_syscalls_are_denied_before_payload_and_inherited \
  --exact --ignored --nocapture
```

通过条件：

1. 无过滤对照的 direct/fork/exec 均返回 `ENOENT`，证明 root 能进入不存在路径查找，并非无权限导致本来就返回 `EPERM`。
2. 经本次实际启动接口创建的单元，在 direct/fork/exec 中均返回 `EPERM`，`NoNewPrivs=1`、`Seccomp=2`，过滤计数至少为 2。
3. 读回固定属性与准确 swap deny 列表、native ABI、errno=1；实际读回用于核对配置，不代替第 2 项。
4. 独立 transient 负例只设置 native ABI 过滤，省略 swap deny，但使用同一个预命令；任务失败且 payload marker 不存在。此夹具模拟未安装过滤的结果，没有修改 PID 1 环境，也不声称实测了 `SYSTEMD_SECCOMP=0` manager。
5. `/proc/swaps` 前后逐字节相同，永不存在目标路径；停止并 reset-failed 本次单元，删除本次 UUID 目录。

模拟回归覆盖每一步探测的失败/超时/截断/未知、继承过滤与重复状态字段、OpenRC 拒绝和固定属性不能被诊断参数覆盖。真实 systemd 夹具的结果与精确源码/二进制摘要在完成后单独记录。GitHub Actions 按当前仓库规则暂停，不触发、重跑或恢复；本项验收不代表完整 NodeQuality 或全部 P0 实机验收通过。

## 本次证据

2026-10-01，在独立 Debian 12 ARM64 guest 完成下列验收：

| 项目 | 结果与证据层级 |
| --- | --- |
| 本地 fmt / core 边界 / core Clippy 全目标 | 通过；静态与编译检查 |
| 最终相关 core `system::` / 服务集成 | 23 passed、7 ignored / 6 passed；模拟与有界执行夹具，忽略项不算通过 |
| 新 swap syscall 真实专项 | 1 passed、0 failed、0 ignored；systemd 252.39-1~deb12u2、Linux 6.1.0-50-cloud-arm64 |
| 原六个有限真实 systemd 夹具回归 | 同一新二进制 6 passed、0 failed、0 ignored（2.31 秒）；锁、预算、同机互斥、queued 状态、重建/超时、取消与私有挂载清理 |
| direct / fork / exec | 无过滤两个 syscall 均 errno=2；保护单元均 errno=1，NNP=1、Seccomp=2、filters=2 |
| 仅 native ABI 的负例 | 同一 awk 预命令拒绝，payload marker 不存在；未修改 manager 全局环境 |
| 现场清理与驻服务 | 诊断 active 单元、编译进程、夹具挂载、swap 测试目录均空；swap 表前后相同；SSH MainPID=446、NRestarts=0，无 global OOM |
| 未验证 | amd64 实际 syscall、其他 manager/内核组合、旧 Agent/既运行单元；完整 NodeQuality、驻 Agent 心跳与代理持续业务流量不在本项验收内；CI 仍暂停 |

冻结受测代码/夹具提交 `ab13f5e`，源码 archive 为 1,039,560 字节，SHA-256 `d975fe6b8a6f8a3e1c6b4622397d805f484c8d651e35c5cb815d9f2f0e5536bd`。ARM64 core 测试二进制为 50,673,336 字节，SHA-256 `0d3a7647345277b20fe9774a2ccade9bae3643fd221d80cf2a7aa92687b79026`。验收后回填本文及 PROGRESS，并重基到后续 main。受测输入 `Cargo.toml`、`Cargo.lock`、core、SDK、protocol 与测试信任根脚本均保持受测提交字节不变；主线面板等无关变化未重复验证。

私有 guest 证据保留在 `/home/l7.guest/sinan-swap-protection/evidence/`：`build-identity.json`、`build.log`、`build.jsonl`、`swap-syscalls.log`、`systemd-regression.log`、`postcheck.json`、`source-ab13f5e.tar.gz`。保存二进制为同任务目录 `core-tests-ab13f5e`；公开文档只记录脱敏状态和摘要，不提交环境原始日志。
