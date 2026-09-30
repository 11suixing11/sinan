# 诊断启动预检与运行中内存保护独立验收

对应第 1 步启动预检项及 Issue #16，依赖资源预算 PR #30。所有资源采集经过 Privileged；服务枚举、启动和停止经过 ServiceManager。拒绝原因通过现有失败结果上传，包含中文原因、测量值和可操作建议。

## 默认策略

| 检查 | 条件 | 不满足时行为 |
| --- | --- | --- |
| 启动内存 | 有效可用内存不少于 MemoryMax + 256 MiB | 拒绝启动，提示释放内存或增加内存 |
| 工作目录磁盘 | 可用空间不少于 2 GiB | 拒绝启动，提示清理磁盘或更换目录 |
| 启动负载 | 一分钟负载不超过可用 CPU 数 × 1.5 | 拒绝启动，提示等待其他任务结束 |
| 同机诊断 | 无其他活动或过渡中的诊断单元 | 拒绝启动，提示等待或停止原任务 |
| 运行内存 | 有效可用内存不少于 128 MiB | 每 5 秒轮询检查，低于阈值主动停止整个诊断服务 |
| 资源不可读 | 资源、单位或服务状态无法确定 | 启动前拒绝；运行中无法确认内存时保护停止 |

512 MiB 默认预算需要至少 768 MiB 有效可用内存才能启动。这里的有效可用内存取 Linux `/proc/meminfo` 的 MemAvailable 和当前 cgroup 及所有可见祖先硬限制剩余量的最小值。cgroup 剩余量为 memory.max 减 memory.current，不推测可回收缓存；读取失败不回落到仅看宿主内存。内存探针要求 cgroup v2。

磁盘使用固定 `stat -f -c '%a %S'` 参数查询真实工作目录所在文件系统，兼容 GNU/BusyBox，输出限制 4096 字节。资源文件单个最多读取 64 KiB；资源和冲突探针分别限制 5 秒。已启动任务等待面板 HTTP 或服务状态查询时仍继续每 5 秒观察内存；状态查询失败、超时或挂起不跳过保护。低内存或内存不可读时先尝试停止，再确认服务已无活动进程；确认查询失败时保留 ACTIVE 和已持久化的停止原因，继续重试，不提前回传终态。工作目录拒绝上层目录跳转和符号链接祖先。

systemd 冲突检查列出所有 `sinan-diagnostic-*.service` 活动/过渡单元，包括不在当前 Agent SQLite ACTIVE 中的服务；已完成的 active/exited 不算运行冲突。OpenRC 检查其持久任务目录和运行状态；非 Linux 后端不宣称支持安全诊断。

两个 Linux 后端共用 `/run/sinan-diagnostic/lock` 的非阻塞 flock；固定父目录经检查确为 root 所有、0700 的普通目录，OpenRC job 同样使用 0077 umask，普通系统账户不能打开锁文件或抢占锁。启动准备不重建、替换或删除已有锁 inode，防止两个 Agent 同时通过预检后的启动竞态。持锁的诊断程序结束或服务被停止后锁自动释放；保留空锁文件，不能在其他任务持锁时删除它。锁属于服务进程，不依赖 Agent 存活。systemd 的资源硬限制仍按 PR #30 生效；OpenRC 的原有预算限制不足仍须单独补足，不将 OpenRC 预检等同于 systemd 硬限制。

## 独立自动验收

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

完整测试需要独立 PostgreSQL 和公开 TEST_ONLY 信任根，按现有开发/CI 流程设置。新增行为判定：

1. 注入宿主小内存、cgroup 小内存、磁盘不足、高负载、无效负载、无法读取资源/服务和同机其他任务，均产生具体失败原因，服务启动次数为 0。
2. 恰好达到阈值可以启动；自身单元不误判为另一个任务。Agent 重启后继续观察原单元，启动次数不增加。
3. 面板断连时，低内存或无法读取内存仍会主动停止；确认无活动进程后保留已有报告并持久化 Failed，等待恢复连接上传。
4. 停止失败或仍有活动进程时保留 ACTIVE 和保护停止原因，不发终态确认；Agent 重启及内存恢复后仍继续完成停止。
5. SQLite 写失败不阻止内存保护动作；存储恢复后仍能收集已有报告，不假报成功。
6. 让面板 HTTP 请求挂起，再降低内存，必须在下一保护节拍停止并留下失败报告，不能等面板超时才停。
7. 原有安装/准备消耗仍计入绝对截止时间，不因预检、重启或面板重连延长运行时间。
8. 状态查询失败或挂起时降低可用内存，停止动作必须在状态查询超时前发生；确认查询仍失败时保留 ACTIVE，重启及内存恢复后继续重试原保护停止，报告只在确认停止后回传。


本轮整合 main `e2d898c` 后，在 macOS 运行 workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁和 6 项门禁行为测试，全部通过。目标 Rust 测试共 69 项通过、0 项失败：诊断 27、系统 17、PostgreSQL 诊断 4、诊断适配器 8、退役 13；系统另有 5 项 Linux/root/systemd 专项按条件忽略。

OpenRC 停止信号清理的新增真实夹具检查父子进程结束、共享锁可重新取得、停止后的延迟子程序不执行，以及持久任务不可重放。本机仅完成该 Python 夹具的语法检查、Unix 同步取消命令并杀后代的现有 Rust 回归，以及共享信号监听构造器的 Clippy；没有运行 Linux/OpenRC 的真实服务停止流程。该新增 smoke、root 锁 inode/权限专项和真实 systemd 专项仍待最终提交 CI 或专用 Linux 测试机执行。

## 专用 Linux/systemd 验收

只在专用测试机运行以下专项；需要 root、flock、Python 3、cgroup v2 及 systemd 系统管理器。CI 已串行运行同一组测试，避免验收夹具争同一个独占锁。

```sh
cargo test -p sinan-agent-core --lib --no-run --message-format=json > /tmp/sinan-core-tests.jsonl
diagnostic_test_binary=$(python3 -c 'import json; print(next(item["executable"] for item in (json.loads(line) for line in open("/tmp/sinan-core-tests.jsonl")) if item.get("reason") == "compiler-artifact" and item.get("executable") and item["target"]["name"] == "sinan_agent_core"))')
sudo "$diagnostic_test_binary" real_systemd_diagnostic_ --ignored --test-threads=1
```

新专项读取真实内存、CPU 和工作目录资源，再启动一个持锁的有限 shell/sleep 夹具。服务列表必须识别它，第二个服务必须被独占锁拒绝，第二个 payload 的文件标记不得出现；停止第一个服务后确认没有活动进程。原资源预算的内存 OOM、任务数上限、属性读回、管理器重建和超时专项继续通过。

资源策略自动测试使用可注入的明确模拟数据，不根据开发机剩余资源决定通过与否。最终代码整合主线 21e6a01 后，在受限 Debian 12 构建容器（1.5 GiB 内存、2 CPU、禁止 swap）运行 fmt、Clippy 和完整 cargo test --locked：243 项通过，0 项失败，7 项忽略；包含独立 PostgreSQL、HTTP/WebSocket 及面板完整 e2e，构建容器无 OOM。7 项忽略为 4 项真实 systemd 和 3 项既有外部运行时测试；systemd 专项二进制另交专用测试节点执行。真实 systemd 专项和完整 NodeQuality 压测以对应 CI/专用节点实际结果为准，不将夹具通过视为完整验机。管理员取消、报告章节完整度和报告中的负载元数据由各自独立 PR 完成。
