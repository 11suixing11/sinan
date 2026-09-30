# NodeQuality 完整任务安全门禁独立验收

关联 Issue #28/#65/#66。本项只停止不受控完整任务的新执行；执行链整改和用户对完整能力的选择仍待完成。没有运行上游 benchmark，也没有在生产修改 swap。

## 可重复检查

1. 适配器专项测试覆盖 r2/r3/r4/r5 完整任务，上传设置 true/false 都在任何特权目录或工具调用之前拒绝。日常任务按签名版本验证后准备；历史完整报告仍能按原版本收集。
2. 独立 PostgreSQL 的 `diagnostics::chain_gate` 覆盖旧/共用创建 API 返回 409，视图的 `full_ready=false` 与明确原因；排队完整任务失败但 `agent_completed=false`，迟到报告仍可保存，日常任务可继续创建。旧 Agent 不收到运行完整任务重发，声明新门禁能力的 Agent 可收集已有检查点。
3. 真实 Agent WebSocket/HTTP 的持久化重启夹具将 Started 任务保存为旧 r2/r3 身份，确认重启只回收报告或确认取消，启动次数保持一次。另注入门禁前已领取的 Preparing 检查点，新 Agent 不启动单元、不创建诊断工作目录并回报明确失败。章节、旧缺插件字段与部分报告夹具继续通过。
4. `web/tests/nodequality-chain-gate.mjs` 在真实 Chromium 加载提交的 dist，覆盖桌面及 390px 手机：完整按钮和确认框禁用、原因可见、日常可提交；403 明确显示；重复任务禁用；历史硬件章节可看；旧未确认停止任务可请求取消并等待设备确认；离线状态禁用、无横向溢出、无完整 POST。
5. Python 包装器只使用隔离的假上游夹具，证明 r4/r5 退出及章节契约未改动。真实 Linux/root 条件测试需在专用 Debian 容器执行，本机跳过不算通过。

推荐命令：

```sh
cargo fmt --all -- --check
python3 tools/check-core-boundary.py
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 tools/test-nodequality.py
python3 tools/test-diagnostic-modes.py
cd web && bun run build && bun test
node tests/nodequality-chain-gate.mjs
```

数据库使用独立测试库，发布信任根使用 `scripts/ci-test-trust.py` 的测试根，正式 API 凭据不进入测试。浏览器参数及运行环境见脚本文件。

## 当前证据与边界

本机 fmt、分层门禁、Bun 五项/711 断言、TypeScript/Vite 与提交 dist 的真实 Chromium 场景通过，页面错误 0，完整请求 0。冻结源码20e27ff在专用Debian12容器（1536MiB/2CPU/无swap）通过fmt/core、全targets Clippy、完整locked Rust/PostgreSQL325项/0失败/9既有条件忽略、适配器及PG/真实Agent专项34、r5包装器34与daily helper7；Agent/Panel构建退出0、OOM=false。二进制/源码摘要保存在binaries/nodequality-gate-head，原日志在evidence/nodequality-gate-head。此后正常重基main2574a84保留插件业务与原生引擎，最终整合HEAD由独立CI验证，不以前一冻结源码替代。

已领取任务的旧 Agent 必须升级或取得取消确认；面板过滤不撤回门禁前返回的 HTTP。新 Agent 拒绝 Preparing 再执行，已有 Started 则继续收集。日常检查仍要求 256MiB 启动预留与 2GiB 磁盘，未降低低内存预检；447MiB 专用节点不能用本项绕过完整验机保护。原上游完整运行的网络零上传、宿主零改动、rootfs 全部授权与持续代理流量压力尚未验，不计为本项完成。
