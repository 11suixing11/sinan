# 2026-10-03：NodeFlare 看板重新对齐

源码基线为 `943d57f`，参考本地 NodeFlare `88c8a43`。在 main 直接完成，不创建分支，不执行远端 CI、真实通知或生产部署。

## 页面与修复

- 使用当前 NodeFlare 的 App、StatsBar、NodeCard 和样式重新对照：紧凑五项总览（公开四项）、搜索与分组、卡片资源和双列线路质量。其他筛选/排序/表格保留在折叠选项，详情仍可读完整资产和历史。
- 删除大标题、刷新频率/成功时间说明、独立成本大区、冗长脚注、页脚与全屏/暂停入口。页面隐藏时取消及停止请求、恢复前台立即读取，和失效权限清理保持。
- 后台“看板与通知”新增币种与汇率，含当前浏览器展示币种、来源/日期/尝试时间、折叠报价表与显式更新。GET 失败保留报价；POST 返回 200 但无新报价不记成功，POST 返回的新报价不会因后续 GET 失败而丢失；没有自动 POST 或通知写入。
- 剩余价值支持预付多周期，不再限为一个周期；一次性费用有明确未到期日期时保留录入值。缺失价格、到期或汇率保持未知。
- 连续 live 响应不能把新采样回退成旧持久化样本；回退保留的样本按面板最小 15 秒容差老化，访问范围、在线状态与隐藏集合来自新响应。首次读取取消不当作空列表。
- Agent 补充 fuse-overlay/aufs 和严格 Docker thin 设备排除；保留容器根、普通 LVM、独立物理分区及既有 st_dev/Btrfs 去重。不会因目录包含 overlay、aufs、docker 或 snapshotter 就漏掉真实磁盘。

## 本地验证

| 范围 | 结果 |
| --- | --- |
| Bun 单元测试 | 75 通过，1476 断言 |
| TypeScript / Vite | 通过，更新随仓库交付的 dist |
| Agent core 全 targets | 270 通过，8 条件忽略；含 13 项磁盘专项 |
| Agent core Clippy / workspace fmt / core 边界 | 通过 |
| dashboard 浏览器 | 1440、768、390、320 像素通过 |
| server-display 浏览器 | 1440、390、320 像素通过 |
| server-assets 浏览器 | 1440、390 像素通过 |
| display-data 浏览器 | 1440、390、320 像素通过，最终构建补验更新报价后的 GET 失败 |
| telemetry-settings / monitoring / server-operations | 通过，覆盖后台设置、历史保留和公开访问回归 |

浏览器使用实际 dist、本地回环静态服务和全部 API 替身，包含真零值、全部失败与检测不可用的区别、目标授权撤销、历史拒绝、晚到响应、隐藏/公开切换清除、自动/手动读取与超时无重叠。桌面、深浅主题和手机截图已经查看。既有服务器表单的具体写入仅针对内存夹具；没有联系汇率提供方或发送通知。

可复查日志：`/tmp/sinan-dashboard-bun.log`、`/tmp/sinan-multicloud-dashboard-agent-final.log`、`/tmp/sinan-organization-{dashboard,server-display,server-assets,display-data,telemetry-settings,monitoring,server-operations}.log`。截图位于 `/tmp/sinan-organization-screenshots/`，这些本机临时文件不作为跨机器持久证据。

初次沙箱内 esbuild、套接字/子进程受限失败，随后经工具授权在沙箱外运行本地构建与测试；测试使用仓库测试公钥，不使用生产发布根。汇率浏览器首轮因可访问名称包含说明文字而定位失败，修正选择器后通过。没有把这些首轮失败记作通过。最终 UI 审查后仅增加缓存老化、POST 报价保留与剩余价值文案，重跑受影响的 Bun/build/display-data；其他六套不冒称最终逐套重跑。

## 保留边界

- CI 仍暂停；本轮未运行整个 Rust 工作区/PostgreSQL、跨平台或真实设备安装。8 个有条件的 core 测试未执行；真实容器挂载组合、Safari/Firefox 仍需专用环境验证。Vite 原有主块超过 500 kB 的提示保留。
- NodeFlare 自动续期的多周期追赶已在 Sinan 实现；本轮没有更改账本。NodeFlare 新增流量重置时区尚未移植，Sinan 仍按 UTC 账单周期，不能以本轮 UI 更新宣称时区能力已完成。
- 保留源码及构建中的 NodeFlare MIT 许可文件。删除看板许可链接不删除署名和许可。
