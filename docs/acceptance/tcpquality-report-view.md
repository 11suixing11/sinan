# TCP 报告界面独立验收

范围：服务器 TCP 诊断入口、四项参数白名单、已配置目标地区、历史冻结范围和独立章节展示。依赖面板登记 PR，后端生命周期与制品验证不在本 PR 重写。

真实 Chromium 加 API 夹具验证：桌面 1280px 与手机 390px，零页面错误；每次只发送一条四参数创建请求与一条明确地区 PATCH；取消之后显示等待设备确认并禁用新任务；取消或缺主报告仍可查看已上传的目标章节。未知数值显示未知，真实零成功率保留 0.00%；NodeQuality 历史不会混入 TCP。

检查版本、开始/结束时间、IP 版本、计数/并发和冻结目标范围；明示 TCP 连接成功率并非包丢失/重传/吞吐，不做跨参数排名。目标只取已配置 TCP 拨测并由管理员标注地区，不内置第三方目标。

012ca9f 已完成 Bun frozen install / TypeScript / Vite 和两宽度浏览器验收。在公开主线 9a41fe5 加最终面板登记 fe4ae60 上正常整合为 9dbff20；保留 NodeQuality 完整门禁和插件业务导航，Bun frozen install / 5项711断言 / TypeScript / Vite 全部通过，新 dist 为 index-D5k-FUiH.js。对该实际 dist 的 Chromium1280/390px复验再次通过：页面错误0，每宽度创建请求1与地区PATCH1，零值/未知/部分章节/旧报告过滤/取消屏障全部成立。首次浏览器尝试因 SSH 预览隧道结束连接拒绝，重建隧道后通过；该失败不归因于网页。随后 fa8dca4 修复主报告被旧 partial 章降级与目标未取得误报空配置；新增可独立运行的 web/tests/tcpquality.mjs，对实际 index-CkSqyBT0.js 的1280/390px回归再次通过，零页面错误，完整100%与25ms保留、目标403显示未知且不能创建、离线禁用均成立。旧 dist 被同一完整主报告夹具抓住（exit1）。夹具初次提取运行因缺分号的测试脚本错误失败，098104c 修正后通过，不隐去失败。浏览器使用夹具 API，不宣称真实 Agent、真实 TCP、签名发布、专用节点完整验机或持续代理流量验收完成。

复验命令：提供 SINAN_PLAYWRIGHT_MODULE 和可选 SINAN_CHROME_PATH 后运行 node web/tests/tcpquality.mjs；脚本自行在回环临时端口提供 committed dist，不需要生产面板或节点。

本轮在作者 `233840a` 上补齐成功读取后再轮询失败的验收：诊断接口随后返回 403 时，共用资源 hook 保留了旧 `ready=true`，原 dist 仍允许创建；目标接口随后返回 403 时，也仍显示旧目标数。扩展浏览器夹具在旧 dist 稳定失败（`Denied diagnostic polling reused stale readiness for creation`，exit 1）。TCP 页面现在将当前诊断能力和目标范围标为未知，暂停创建并保留历史报告；成功恢复后重新采用当前状态。没有改变共用 hook、后台任务生命周期或 NodeQuality 完整模式门禁。

修复后的 Bun 5 项/711 断言、TypeScript/Vite（52 个模块）通过，实际生成 `index-DlTWuByG.js` 与 `index-DG2zgVpb.css`。对该 dist 的 Chromium 1280/390px 扩展复验均为零页面错误、每宽度一条四参数创建请求和两条明确地区 PATCH：验证真实零耗时、未知耗时、完整主报告不被旧部分章覆盖、含省略插件字段的 NodeQuality 历史过滤、创建后修改当前地区仍保留已冻结的东亚/IPv6/8次/2并发、确认取消屏障、成功后诊断轮询 403 禁止创建、目标轮询 403 显示范围未知以及离线禁用。`nodequality-chain-gate.mjs` 与 `singbox-business.mjs` 对同一实际 dist 的桌面/手机伴随验证通过，完整模式 POST 为零，业务启用和导航保持正确。

本轮复用与当前 `package.json`/`bun.lock` 完全一致的已有依赖，没有安装依赖；磁盘不足时伴随浏览器第一次启动前的日志重定向失败，空间恢复后实际执行通过。上述浏览器均使用回环 HTTP/API 夹具，没有运行 Cargo、PostgreSQL、真实 Agent、上游探测或生产接口，没有发布签名制品或使用真实密钥。专用节点 TCP 登记、签名安装、断线恢复与真实探测整链继续由对应后端验收和专用节点记录确认；本记录支持已授权的验证后合并，不把实机范围称为完成。

恢复执行时重新核对作者 `233840a` 与保留改动，并重新执行上述验证，不沿用已消失的临时日志。现有依赖的 `package.json`、`bun.lock` 逐字一致；仅恢复官方 Bun 1.4.2 运行时，ZIP 的 SHA256 与 GitHub release asset digest 一致。最终界面将数据流说明改为“不向第三方上传报告”，仍由 Agent 将报告回传本面板。Bun 5 项/711 断言和 TypeScript/Vite 52 模块通过，最终实际 dist 为 `index-hb4JwTbg.js`、`index-DG2zgVpb.css`；Chromium 1280/390px 验证额外确认失败期间历史报告保留及诊断读取恢复后重新采用当前就绪状态。相同 dist 的 NodeQuality 完整门禁和 sing-box 导航夹具通过；作者旧 dist 的负对照仍因轮询失败后允许创建而明确失败。此次日志与文件摘要保存在本任务独立证据目录，最终主线 CI 与专用节点整链仍分别核实。

最终正常合入已包含 #76/#77 的 main `21d8e71ac5e45e09c7a9bed05598b265e1e4c121`，作者 `233840a` 的提交完整保留。合入后逐一核对 41 个已验前端源、测试、锁文件与 dist 的 SHA256，均与刚执行的构建和浏览器验证一致；两侧进度及制品/后端验收记录均保留。此次仅运行与前端变更相称的验证，没有重建 Rust 或启动测试数据库；合并后的正式 GitHub CI 待推送后确认。
