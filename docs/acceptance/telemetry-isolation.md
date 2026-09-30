# 心跳与遥测隔离的独立验收

关联 Issue #17。本项只处理采集隔离与时间可见性，资源预算、预检和取消接口分别验收。

## 行为与兼容

- 一个 Agent 进程只启动一个 `sinan-telemetry` OS 线程。它持有 Collector 和硬件补充采集器，初始缓存保留编译期 OS/arch/libc，通过 watch 发布 StaticInfo 与已有的 TelemetrySample。连接初次发送、300 秒刷新、公网地址变化与配置应用只读缓存并合并身份和版本，不执行 sysinfo、磁盘或进程采集。
- 一次采集超过 5 秒，最后成功的 UUID、`sampled_at` 和指标保持原样；不启动替代线程。永久卡住的内核读取不会拖住 WebSocket、HTTP 补报或退役控制。退出不 join 永久阻塞的线程，进程退出后由 OS 回收。异步硬件工具采用 4 秒 / 128 KiB 的受限执行，取消时由进程组 guard 清理完整子树；退出最多等 1 秒确认此异步清理，避免退役退出抢先绕过工具析构。同步内核读取超过这个等待上限只记录未确认，不无限等待。恢复采集后才生成新样本。
- 保留默认 1 秒采样、3 秒批量上传及面板设置。采集、持久化、HTTP 上传独立；采样失败不刷新旧数据时间。原有 SQLite 2 小时 / 7200 条 / 64 MiB 保留上限、64 条 / 900 KiB 上传与按 UUID ACK 语义继续使用。
- 保存样本时在同一个 SQLite 事务保存时间下限；ACK 清空 outbox 后下限仍在。升级前已有的 outbox 时间也纳入下限。Agent 重启或负向时钟修正不会让新采样时间倒退。断连补报沿用原 UUID、毫秒时间与指标，面板既有去重和较新样本保护不变。
- `last_seen` 仍表示最近设备消息，单位秒；新增可空 `last_heartbeat_at` 只由 heartbeat 消息更新，单位秒。旧记录不回填。API 暴露既有 `metrics_sampled_at`（毫秒），0 映射为空。指标是否过期按 `max(15, sample_interval*3 + upload_interval*2)` 秒判断。旧 telemetry.metrics 没有采样时间，继续接收指标但将时间标成未知，不拿接收时间代替采集时间。缺失时间显示“尚未记录 / 尚未上报 / 时间未知”；已过期的历史指标保留可读。
- 基线脚本分别保存 `last_device_message_at`、`last_heartbeat_at` 和 `metrics_sampled_at_ms`，不将设备消息假装成心跳，不从不存在的 `collected_at` 字段推断时间。

## 自动验收

需要 PostgreSQL 的测试使用独立测试数据库与 CI 的公开测试信任根，禁止生产库或正式发布信任根。

```sh
cargo fmt --check
python3 tools/check-core-boundary.py
python3 -m unittest discover -s tests -p 'test_*.py'
cargo clippy --all-targets -- -D warnings
cargo test -p sinan-agent-core --lib
cargo test -p sinan-panel --test telemetry
cargo test -p sinan-panel --test end_to_end
```

关键断言：

1. 分别注入永久阻塞的构造器，以及先成功一次、随后永久阻塞的 Source。5 秒后缓存过期，原样本完全一致，只启动一次采集器；退出缓存所有者不会等待卡住的采集。
2. 用真实 TCP/WebSocket 等待 20 秒心跳周期，第二次心跳在 23 秒内到达；采集超时后控制 Ping 在 1 秒内获得 Pong。断连重连继续读取同一个缓存，采集器数仍为 1。
3. 采集卡住期间 HTTP 首次返回 503，随后 ACK；outbox 重试成功、原样本时间未改、退役写锁与托管服务停止仍能完成。重新打开 SQLite 后时间下限不丢失。
4. 面板 API 区分心跳秒和采样毫秒；心跳到达不改变采样时间；在线且过期的旧指标仍可读；60 秒显式采样配置不被默认阈值误判。原压缩重放/延迟样本测试继续通过。
5. 注入一次采集失败和一次超过 5 秒的采集，旧 UUID/时间保持原样；下一次及时成功采集才清除错误并发布新 UUID/时间。Unix 工具取消夹具验证独立于 systemd 的子进程组清理，退出后子进程无法执行延迟写入。同步 CPU 忙循环期间，单线程 Tokio 执行器的计时器仍推进，缓存不改写，退出不等待忙循环。
6. 初始采集未完成时只发 hello/heartbeat，不用编译期默认值覆盖设备注册的宿主 ABI；首个真实快照到达后自动补发 StaticInfo，musl Agent 和 GNU 宿主的两个 ABI 字段保持独立。公网发现和配置变更也遵守缓存就绪条件。

## 前端与实际浏览器

```sh
cd web
bun install --frozen-lockfile
bun run build
bun run preview --port 4175
# 在另一个终端，使用已安装的 Playwright 和 Chromium：
PLAYWRIGHT_MODULE=/path/to/playwright CHROMIUM_PATH=/path/to/chromium \
  node tools/test-telemetry-ui.cjs http://127.0.0.1:4175
```

浏览器脚本从仓库根目录执行。所有 API 被回环夹具替代，不请求真实节点或第三方。验证在线过期历史、毫秒换算、设备消息与心跳时间分开、恢复新鲜、离线历史、未知旧时间，以及无浏览器脚本错误。`web/dist` 必须与此次构建一致。

## 专用 Debian 12 节点复核

在已经完成预检/资源保护的专用节点上执行完整验机，同时记录 `systemd-cgtop`、内核 OOM、磁盘、Agent 线程与面板三个时间字段。确认 `sinan-telemetry` 只有一个；20 秒心跳持续前进；指标卡住时原采样时间停止、15 秒后默认设置显示过期；sing-box 不被杀。面板断连和 Agent 重启后确认 outbox 补报、时间不倒退；取消须按独立取消 PR 验收全部子进程和挂载清理。

本项的卡死注入、WebSocket 和浏览器夹具不等同于实际诊断负载或取消清理验收。不得在未通过资源保护的 447 MiB 节点再次启动默认硬件负载。

## 当前结果

- 本机 fmt、core 边界检查、Python 77 项（5 项环境跳过，含 baseline 7 项和边界 6 项）、Bun 1.4.2 TypeScript/Vite 构建通过。
- 真实 Chromium 回环验收 6 项通过，浏览器错误 0。
- 集中 Debian 12 构建容器（1.5 GiB / 2 CPU / 无 swap）中，fmt、clippy 全 targets、Rust/PostgreSQL 工作区测试通过：246 通过、6 原有环境依赖测试忽略；另有核心遥测 10 项、阻塞 4 项、面板遥测 4 项专项通过。构建容器 OOM=false，Linux Agent/Panel 和 core 测试二进制已保存；这轮二进制基于 e2d898c，未包含之后 main 的预检整改，不用于完整节点负载。专用节点须先整合最新 main，再由总任务记录真实诊断负载复核。
- 已将单独提交 rebase 到 main b8e5689（预检、宿主 ABI、IP 缓存和会话修复均保留），再次通过 fmt/core 边界、Python 77 项（5 环境跳过）、Bun 构建/dist 和真实浏览器 6 项。初始缓存 ABI 就绪用例是这次整合新增；整合 HEAD 的 Rust/PostgreSQL 和真实 systemd 验证交 GitHub CI，结果以该 HEAD 的 check 为准，不将上一轮 246 项结果当作整合验证。

- 合并审查退出补修与作者 `03f7d30` 在本机 macOS / 独立 PostgreSQL 通过 21 项 Rust 专项（core/遥测 13、连接 3、阻塞采集退役/补报 1、panel 遥测 4），0 失败/忽略；workspace/all-targets Clippy、fmt、core 门禁和 baseline Python 7 项通过。随后正常合入 main `c958ba2`；新增主线 Rust 仅 IP 质量相关文件，以上验证对象及 Cargo 输入完全相同，保留专项证据。最终合并前端经 Bun 1.4.2 冻结安装、TypeScript/Vite 重建、字段 4 项/637 断言及 Chromium 遥测 6 场景验证，浏览器错误 0。源码/日志前缀为本轮 `/tmp/sinan-pr48-`；Unix 子树取消实测通过，Linux/systemd 和受保护节点全负载另验。
