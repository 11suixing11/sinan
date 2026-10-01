# TCP 诊断面板登记独立验收

范围：参数白名单、配置目标地区、固定快照、第二插件登记和共用诊断生命周期接入。依赖签名制品与无状态适配器；报告界面单独提交。测试夹具中的签名字节只验证面板路由与持久化，不执行工具，不替代真实网络或专用节点压力验收。

验收包括：

- 参数或未批准选项拒绝；没有目标、目标超过 8 个或缺 native-v1 能力时拒绝。管理员会话和目标所属服务器校验。
- 地区独立存储，未标注显示未知；空 PATCH 不清除已有标签，明确 region:null 才清除；旧 ProbeSpec 没有新增字段。仅所选启用目标冻结到任务，原字节摘要、参数、来源版本与资源预算匹配；编辑拨测/地区后历史快照不变。
- 同时重复提交只有一个任务；NodeQuality 和 TCP 双向互斥，过期的取消请求仍保留屏障；HTTP 取消领取与确认走原框架；已保存 TCP 章节和取消状态分别保留，NodeQuality 的原历史视图不混入 TCP。
- Linux Agent 能编译并登记两个适配器；monitor-only 不登记诊断。签名安装、CLI 无上传、停止后残留检查由前序独立制品/适配器和通用框架验收提供，并对最终组合做专项检查。

组合源码 3963c28 在 Debian 12 限额构建容器实测：3 个 PostgreSQL/API 夹具、2 个参数/目标语法单元夹具、TCP 适配器 14 个集成和 1 个单元、原生工具 17 个测试全部通过；workspace 全 targets Clippy（warnings 为错误）与 Linux Agent 编译通过，容器退出 0、OOM=false。首次组合使用的 5e pin 已由版权补齐后的 b562effcd90f8ae319665fb4ead1807b770ed4d5 取代；后者公开且已实际验证 Debian 静态 ELF 与五辅助文件 TEST_ONLY 签名制品。使用测试信任根，未发布正式制品。最终平铺源码 fe4ae60 的空 PATCH、明确 null 清除、3 个 API、2 个单元、workspace Clippy 与 Linux Agent 编译再次全部通过，exit0/OOM=false。已正常合入根目录 sing-box 归位主线；1c640d3（根 sing-box 路径与新 pin 的最终组合）的同一 API3、unit2、fmt、workspace全targets Clippy与Linux Agent构建再次全部通过，exit0/OOM=false；最终独立 PR CI 继续核对。专用 aws-jp0 的 SSH 恢复、持续代理流量和完整验机总验尚未完成，不据此声明通过。
## 本轮登记整合复核

冻结作者 `58868c4` 后，在 macOS 与独立 PostgreSQL `127.0.0.1:55432` 完成 **75 项通过/0失败/0忽略**：TCP API3、参数/配置目标2、共用 core 诊断生命周期40、TCP适配器14及可信UID1、NodeQuality适配器15。覆盖配置快照与摘要、地区缺字段不清除/null清除、跨插件重复与取消确认屏障、部分历史、五辅助文件签名前及启动前重验、绝对截止、Started恢复不重跑、断连回报和取消结果持久化；NodeQuality新完整任务门禁及旧历史精确回收保留。workspace全targets Clippy（warnings为错误）、fmt、core门禁及差异检查通过；本轮仅清理本任务已验证可重建的测试可执行文件，保留其他工作与缓存。测试数据库使用后已停止。

独立实际核对作者[CI检查](https://github.com/theLucius7/sinan/actions/runs/36795237662/job/110157056120)：默认Rust/PostgreSQL370通过、9条件忽略，另有真实systemd6通过；[Reality场景](https://github.com/theLucius7/sinan/actions/runs/36795237662/job/110158114169)也通过。本机没有运行Linux Agent登记编译或systemd。TCP API采用不可执行的TEST_ONLY签名字节；通用systemd、原生制品及Reality各自验证，不能当作登记后TCP真实工具从六文件安装、排队、重连、确认取消到报告上传整链的实机证明。正式制品及专用节点总验仍按前文边界跟进。
