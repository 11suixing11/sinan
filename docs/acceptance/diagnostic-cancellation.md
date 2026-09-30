# 确认式取消独立验收（Issue #19）

本项对应单独 PR，只增加取消链路，不实现报告章节拆分和诊断框架迁移。

## 约定与操作

- 管理员调用 `POST /api/servers/{server_id}/diagnostics/{job_uuid}/cancel`。必须是该服务器已有的任务。支持能力为 `diagnostic:confirmed-cancel`。
- 返回 202 与 `cancel_requested` 表示请求已持久化。界面显示“等待设备确认取消”。重复请求保留第一次请求时间；已确认取消的重复请求返回 200。已自然完成任务返回 409。
- 面板发送完整已知任务的 `diagnostic.cancel.request`，Agent 先持久取消意图，再停止已绑定的诊断单元。不会接受协议中传入任意服务名称。
- Agent 经 `ServiceManager` 停止单元，确认排队任务不再运行、主进程和控制进程消失、整个 cgroup 子树无进程、工作目录下无挂载。已加载单元还需验证私有挂载与整个控制组的停止方式。
- 经设备认证的 `diagnostic.cancel.result` 或 `POST /api/agent/v1/diagnostics/{job_uuid}/cancel-confirmation` 提交确认。只有 `confirmed=true` 进入 `cancelled`。负确认保留请求和错误，下一轮继续处理。
- `GET /api/agent/v1/diagnostics/cancellations` 仅返回当前认证设备的待取消任务，SQL 限制 64 条。设备每 5 秒独立恢复 HTTP 待处理请求和结果 outbox。每次请求/确认网络等待限制 5 秒，不在心跳连接循环内等待停止。
- 下载中取消会中断下载；启动之前再次检查持久意图。已经进入持久启动检查点的请求先结束，再停止和核实，避免在确认后迟到启动。取消不修改原截止时间。
- 断连和 Agent 重启恢复 SQLite 意图及结果，不重新运行诊断。只有 HTTP 确认成功才移除 outbox，旧负确认 ACK 不会移除新的正确认。
- 面板收到末尾自然报告时保留报告，但待取消状态仍等清理确认。确认不会覆盖已有报告。取消不删除任务根目录或章节 sidecar。

旧 Agent、OpenRC 与非 Linux 后端目前不支持同等清理证据，界面禁用取消并显示“不支持确认式取消，请先升级”；接口返回 409。无法证明旧诊断单元已清理时同样保持待确认，不能声称取消成功。

## 自动验收

Rust 使用独立 PostgreSQL 数据库，执行工作区 `cargo fmt --all -- --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked`。设置测试发布信任根：`export SINAN_RELEASE_PUBLIC_KEYS=$(python3 scripts/ci-test-trust.py)`。

| 层 | 独立行为场景 |
| --- | --- |
| 协议 | 请求与确认 envelope 往返；拒绝任意单元字段、缺少显式 confirmed、普通状态假提交 cancelled |
| 面板 / PostgreSQL / WS / HTTP | 管理员授权、旧 Agent 拒绝、请求先持久化后发送、重复请求、设备范围隔离、负确认、过期不结束请求、自然报告竞态、正确认幂等、已有报告保留、取消阻止重复提交 |
| Agent / SQLite / HTTP | 准备阶段重启与重复请求、已取消任务不再启动、停止失败、活动进程、残留挂载、不一致的保存单元、设备或模块不匹配、确认 HTTP 500 后重启重放 |
| 签名下载 | 真实已签名制品下载挂起后收到取消，2 秒内中断，未启动服务，持久确认 |
| 实际 Agent ↔ 面板 | 真 WS / HTTP 与两种数据库：运行中停止失败保持等待和连接活动；Agent 重启后恢复取消、提交清理确认、报告仍可查看、服务仅启动一次 |
| systemd | 独立 UUID 小夹具：未启动任务可确认无残留；实际私有 tmpfs 与子进程运行时不能确认；停止后 cgroup 子进程消失且挂载清理 |

本项继续运行已落地主机预检、内存保护、资源预算测试，因此小内存、磁盘不足、加载状态未知等拒绝行为不能退化。上游 IP 403/429/超时测试随全工作区测试继续运行。完整 NodeQuality 和持续代理流量压测需要专用节点，由独立基线/后续专项记录，不能用本项的小夹具代替。

## 实际前端构建与浏览器

```sh
cd web
bun run build
SINAN_PLAYWRIGHT_MODULE=/absolute/path/to/playwright/index.mjs \
SINAN_CHROME_PATH=/absolute/path/to/chrome \
node tests/confirmed-cancellation.mjs
```

Playwright 与 Chrome 为现有开发工具，不增加生产依赖。测试直接加载已构建 `web/dist`，使用受控 API 响应验证视图：运行中提交请求、等待确认、清理失败继续等待、普通轮询收到正确认、旧 Agent 禁用取消、已产生报告保留、390px 移动宽度无横向溢出且无页面异常。后端真实性由上述数据库与实际 Agent 集成测试单独验证。

## 本次结果

- 本机 Bun 1.4.2 构建成功，构建 JS `index-D49ZhJgy.js`，浏览器桌面 1280px / 移动 390px 全部状态断言通过。
- 受限远端 Debian 12 构建容器：1.5 GiB 内存、禁止额外交换空间、2 CPU、512 PID，独立 PostgreSQL；fmt、Clippy 全目标（warnings 为错误）、完整工作区测试全部通过：271 通过 / 0 失败 / 9 项环境忽略；构建容器 exit 0，未被 OOM 杀死。日志为受控构建目录的 `target/cancellation-{fmt,clippy,test}.log`，基于 main `75cd846` 的本项最终源码。
- 真实 systemd 夹具必须在专用 Debian 12 / cgroup v2 节点串行运行，命令：`core-tests real_systemd_diagnostic_ --ignored --test-threads=1 --nocapture`。本项新增一个确认式取消夹具，当前主线上游五个资源/预检/锁权限夹具继续运行，前缀总计 6 项。未执行之前不计为通过。
- 交付 Linux core 测试二进制 SHA256：`38f0d15713e1863723fe5eb8df0152789385a31e6cc36121b563257ff2e5f6f5`。专用节点结果待记录，不用普通构建容器的忽略结果代替实机验收。

- 代码提交 `8957f5c` 的 [CI 36772176396](https://github.com/theLucius7/sinan/actions/runs/36772176396) 已通过 check、compose-smoke、Linux musl amd64 / arm64；其中实际 systemd 串行 6 项全部通过，0 失败 / 0 忽略，2.28 秒。Reality 安装计量任务因 Draft 条件跳过，不记为通过。此 CI 环境证据不替代待执行的专用 Debian 12 小内存节点验收。
