# 日常检查与完整验机独立验收（Issue #21）

本项复用已合并的独立章节 PR #49。NodeQuality runner 使用新不可变 r4，r2/r3 制品、已保存队列和重启检查点继续收集；不覆盖旧制品或重复执行旧任务。新建任务需要 Linux、制品验签、独立章节和 `diagnostic:nodequality-modes` 能力，旧 Agent 不显示可运行。

## 行为与边界

- 日常检查：面板按原逐源缓存接口刷新 IP 查询；Agent 仅对该服务器已配置、启用的 TCP 拨测目标探测，按 ID 顺序最多选 4 个。IPv4/IPv6 分开，每个目标每种地址族至多 4 次 TCP 连接，DNS 独立子进程 2 秒、每连接 1 秒，全任务 90 秒。DNS 子进程超时后 terminate/kill/join，不留下重复线程或子进程。无目标和连接失败明确为未知；面板 IP 查询不是节点流媒体解锁证据。
- 轻量分支只执行自有 Python 标准库 helper，不调用上游 NodeQuality、硬件、rootfs、测速、回程或公开上传。固定 64 MiB / 32 tasks；full 固定 512 MiB / 128 tasks，CPUWeight/IOWeight/OOMScoreAdjust 保持现有受保护诊断设置。
- 完整验机：服务端先校验管理员身份，要求 `confirm_full:true`。近一分钟正向代理计量显示活跃警告；已发布代理配置而没有新正增量时显示未知，因为该账本没有零流量采样证据。活跃或未知都要求 `acknowledge_traffic_warning:true`，记录确认时间与流量证据；网卡总流量不能证明代理是否空闲。
- 预检及运行低内存保护保持现有阈值：启动另需 256 MiB 可用内存、2 GiB 工作目录磁盘，运行不足 128 MiB 则保护停止。轻量模式仍可能被小内存或压力节点拒绝，不降低门槛以通过验收。447 MiB 专用节点默认完整验机必须拒绝。
- `Started` 检查点保存实际有效可用内存、磁盘、可用 CPU、一分钟负载与最终 ServiceJob 资源预算。`environment` 独立章随 r3 outbox 持久化与补报；日常预期为 environment/net_quality 两章，完整为 environment 加原五章。缺章不阻止查看其他章节，执行状态与完整度仍分开。

## 自动化验证

测试只使用回环、合成凭据；启动任何验证前移除真实 `SINAN_ABUSEIPDB_API_KEY`，不以自动化夹具证明正式账户权限或配额。

```sh
unset SINAN_ABUSEIPDB_API_KEY
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 tools/check-core-boundary.py
python3 -m unittest discover -s tests -p 'test_*.py'
python3 tools/test-diagnostic-modes.py
sudo python3 tools/test-nodequality.py
cd web
bun install --frozen-lockfile
bun run build
bun run preview --port 4176
# 仓库根目录另一终端：
PLAYWRIGHT_MODULE=/path/to/playwright CHROMIUM_PATH=/path/to/chromium \
  node tools/test-diagnostic-modes-ui.cjs http://127.0.0.1:4176
```

专项覆盖真实回环 TCP 4 次连接、保留端口失败、IPv6 不回退 IPv4、DNS 永久卡死后有界停止、参数/目标上限、空目标未知、轻量分支不调用上游和不创建挂载；Rust 覆盖 profile、模式 capability/平台 gate、确认与持续流量警告、4 目标上限、重复提交，以及重启后同一环境章节补报且不启动服务。

浏览器覆盖完整确认、未知流量二次确认、活跃警告、日常忽略测速/上传选项、IP 源 403 明确显示，以及旧 Agent gate 和手机视图；所有 API 使用回环夹具，浏览器错误需为零，提交 dist 与构建一致。

## 专用 Debian 12 故障矩阵

1. 小内存、磁盘不足：对 full/daily 分别确认预检拒绝并保留原因，服务未启动；不要再次无保护跑满 447 MiB 节点。资源充足夹具才运行完整验机。
2. IP 源 403/429/超时：日常网络章节仍可查看，IP 视图显示源错误与历史成功数据。源缓存/分类沿用对应独立 PR，不能转换成干净或零分。
3. Agent 重启/面板断连：沿用诊断持久化和 r3 章节 outbox；环境采样时间、参数和版本保持不变，已启动诊断不重复启动。恢复后确认章节补报。
4. 取消：复用已合并的独立取消 PR #51，设备确认之前显示等待；确认后核对进程与挂载。此项覆盖等待取消时两入口互斥，实际清理证据仍按取消项验收。
5. 重复提交/部分报告：同机已有 queued/running 拒绝重复；部分报告保持可读，完整度按各模式 expected_sections 计算。
6. 代理持续流量：服务端观察实际计量增量，必须警告并要求明确确认；无新采样时仍显示未知。报告保存启动负载与预算，Agent 心跳和常驻代理存活由隔离/保护 PR 的整合实机验收证明。

## 当前证据

本机不重建 Cargo：fmt/core 边界、Python discovery 88 项（83 通过、5 环境跳过）、轻量 helper 6 项、Bun 1.4.2 构建/dist、真实 Chromium 6 项（错误 0）通过。基于 main 6a583af 的模式源在集中 Debian 12 容器（1.5 GiB / 2 CPU / 无 swap）通过完整 Rust/PostgreSQL 293 项、0 失败、8 项既有运行时环境忽略，以及 fmt、全 targets Clippy 和 Agent/Panel 构建；OOM=false。Linux wrapper 29 项与 helper 6 项通过。wrapper 首轮仅三个既有断言把上游 exit 1 期望为成功；远端临时对齐退出码后通过，最终集成采用独立 PR #56 的正常/非零夹具修复，不混入模式实现。日志为 target/modes-verify-final.log 与 target/modes-finalize-fixture-fix.log，验收二进制另存 binaries/modes-head；这些结果不替代随后整合源码的独立 PR CI。专用节点实机矩阵由总任务记录，不将回环夹具等同实际完整诊断负载。

发布前整合 main 6b63f71（含 #50 视图拆分与 #51 确认取消），保留 cancel_requested 同机互斥、取消字段/能力与现有报告；新建任务返回完整取消状态字段。取消 API/真实 WebSocket 场景按最终 HEAD CI 复验，未重复宣称此前293项已验证这次整合。

main229becc含#56正式退出码夹具，模式专项保留其正常/失败两路径。重建后的真实Chromium7场景通过，新增等待确认取消时两入口均禁用、资源章节仍可查看；fmt/core门禁、Python discovery83/5skip、Bun4/637断言通过。最新Rust和Linux平台按独立PR57最终HEAD CI核对。

7c784e6的CI36779254979核对workspace309通过/0失败/9既有忽略，随后真实root/systemd6项通过（含取消进程和私有挂载清理）；Compose、AMD/ARM musl/OpenRC通过。之后main7848268的查询来源整合保留providers元数据，Bun5项/711断言、fmt/core与重建dist通过；新整合HEAD的CI单独核对。

## 最终合并专项


- 最终合并审查正常保留旧作者 `ec7c663`、最新作者 `a374c12` 与 main `c47fc69`（含 #51 确认取消、#54 真实 IP 来源和 #59 许可文档）。修复 DNS 列表前八项同属一个 IP 家族时 `both` 静默漏测另一家族：先按家族选择，仍至多两个地址、每家族四次连接；新增两种顺序回归。portable 端到端夹具明确使用模拟 Linux 服务身份，不放宽生产 Linux 门禁。最终集中 Rust/PostgreSQL 专项 89 项通过、0 失败/忽略（适配器12、Agent诊断37、IP/provider25、diagnostics API10、章节3、真实Agent WS/HTTP/restart2），workspace 全 targets Clippy（warnings 为错误）、fmt、core 门禁及六项行为回归、差异检查通过。Bun 1.4.2 五项字段测试/711 断言与 TypeScript/Vite 重建通过，最终 dist 为 `index-CtZ2u8uf.js`；实际 Chromium 桌面与390px手机模式各7场景、确认取消与来源展示通过，页面错误0。Python discovery 83通过/5跳过、daily helper7通过、r4包装器23通过/6项Linux/root条件跳过。所有测试移除真实 `SINAN_ABUSEIPDB_API_KEY`，仅回环与合成凭据；本轮未重复完整 workspace，也未在本机执行真实 Linux/root/systemd、上游完整验机负载或正式 API 账户/配额验收。最终提交 CI 单独核对，R5正常退出契约修复继续独立合并。

## 后续包装器退出契约

当前源码默认制品已升级为 r5，同时保留本项 daily/full 与 DNS 双家族修复。r5 只修复固定完整验机入口的正常清理 `exit 1` 特例，日常路径继续使用独立 Python helper，不启动上游、挂载或公开上传。r4 已签队列/检查点保留原版本、模式、目标及预算恢复；r2/r3 仍仅接受旧参数。版本接收/恢复及新任务实际 r5 身份的 Rust/PostgreSQL 17 项通过，断网 Linux root 和 macOS 的包装器 34 + helper 7 共 41 项通过、0 跳过。冻结旧 r2 候选不重打包，完整压力和最后整合 CI 仍分别核对。
