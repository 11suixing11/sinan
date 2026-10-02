# 托管路径的独立 Linux 环境与启动前闭包

本步骤承接[私有面板信任与验收入口](managed-agent-paths.md)。源码基线为 `cd9bec762c1b445ace4be5cf49f3ee0624e2a48f`；旧 265 项 Agent-core 和 47 项工具合同结果只认证其当时的输入，不能认证本步新清单逻辑、Linux 原生重建或三设备实际运行。

## 集中修改

静态对照确认 [Issue #149](https://github.com/theLucius7/sinan/issues/149)：控制器遗漏普通 Agent CLI 必需的 `--panel`，且令牌被拆成两个 argv，无法可靠处理首字符为 `-` 的值。现使用已经绑定的面板 origin 和 `--panel=<origin>`、`--token=<token>`；私有描述文件的附加 URL、二进制或 argv 不参与选择注册目标。配置文件中的 `panel_url` 不替代 CLI 参数。

真实驱动在构造控制器、面板客户端或网络 helper 之前完成纯清单闭包：规范非空 UUID、规范 HTTPS origin、三方 run/source/origin、私有 owned root 与 marker、证据目录边界，以及角色地址和固定产品端口。证据目录可以尚未创建，也可为失败范围重放保留已有私有目录。X 和订阅客户端使用驱动宿主的网络命名空间，明确记录为 `driver_host`；地址别名不构成另一个客户端设备的隔离证明。

修改期间不运行测试或构建；新增合同代码先完整写好，独立只读复核后再冻结统一验收。原始失败材料保留，最终 gate 发现问题时先停止测试集中修复，仅补验受影响范围。

## 独立环境预算

拟准备的专用 Debian 12 ARM64 VM 配置为单独的 Lima 目录、2 CPU、2560 MiB 内存与 8 GiB 虚拟磁盘，要求不挂载宿主目录、不转发宿主代理凭据、不暴露业务端口。缓存镜像重新核对固定 SHA512；旧 P0 VM 及其 root、附加输出盘和所有材料保持，不扩大旧预算或清空旧材料重试。用户随后要求立即结束任务，本步收尾期间不创建或启动新 VM。

宿主另外保留 4 GiB 磁盘管理空间，本步骤新增实际分配预算最多 5 GiB；旧缓存只读借用的压缩副本最多 512 MiB。旧 VM 确认没有活动 Sinan 任务后可以正常停机，以释放内存，保持可恢复的磁盘和配置。新 VM 的动态宿主管理保护只停止本次所属实例，不能停止其他服务或以未观察到的进程退出推断清理成功。

工具链与 registry 材料在新目录预置，不复用旧 target。不复制 Cargo 认证／配置，也不信任旧解包源：新 cargo-home 只预置 registry 索引与固定锁对应的 `.crate`，由 Cargo 重新校验和展开，缺项则明确拒绝。原生准备仍要求目标文件系统初始可用至少 4 GiB、1280 MiB 可用内存及 16384 inode，执行时 MemoryMax=1 GiB、零 swap、128 tasks 和 900 秒期限；旧盘上已取得的资源拒绝不变。

## 真实服务预置顺序

1. 完整冻结源和实际 web/dist 一起复制到新 guest；执行普通离线 native prepare，成功后才使用它生成的唯一 `run_id`。prepare output 自带私有 ownership marker，可作为本次 `run_root`。
2. A/M/B 使用不同根目录、machine-id、PID／挂载／网络命名空间及真实 systemd；各自保留产品 `18085/18086` 控制与统计端口，A 入口 20011／20012，M/B 20001。fixture 为驱动宿主的自有私网地址，B 到目标不经 SNAT。
3. 初次 Agent 使用明确的测试离线预置：同一签名 Agent 原始字节、完整旁置 release/checksum/minisign 证明、正式服务模板与标准目录。TEST_ONLY inert installer 不能用作安装器，Linux 的 `install-service` 不提供此替代。运行时不手启、不预造配置，仍由实际面板启用插件、Agent 下载验签及应用。
4. 专用 PostgreSQL database、socket、数据目录和身份全部绑定 run_id。0700 run_root 可由专用数据库服务账户持有，以支持其遍历；只在本次自有 socket 安排 root→postgres 的受限认证，不改变其他数据库。面板单元只读取绑定的私有 EnvFile，制品原样放在 `data_dir/artifacts`。
5. 原生面板正常 HTTP 服务由自有 TLS 反向代理提供同一 HTTPS origin、WSS、下载和 `/health`。Agent 通过本地 `panel_ca_file` 正常验证；runtime 另在隔离设备里正常信任面板／fixture CA，不关闭验证，不把 Agent 的 CA 字段冒称运行时信任。
6. 实际 PID/starttime、namespace/root/machine-id、native SHA、unit/EnvFile 和成功准备收据写入控制器清单。只有实际限额生效、inspect 绑定通过后才调用真实登录、服务器注册、插件安装、节点、用户、链路与计量 API。

## 证据边界

本步整套源码和合同修改完成后冻结 782 份功能输入，SHA256 `95c66f7511ffd7fb393f7d21e490280db6019e409ae05bfd0df8ffe2b8a82c3e`。最终集中 gate：准备／控制器 12 项、API 驱动 19 项，共 31 个不同合同方法通过，0 失败／跳过；功能源保持冻结，差异检查通过。旧 Rust、协议、前端、运行时及 helper 输入未改，没有重复相应测试、构建或浏览器。计数和私有日志摘要见 [机器收据](managed-agent-environment-local.json)。

用户要求立即结束任务后，本轮只完成当前源码收尾，不创建新 VM、不运行 native 构建／签名或三设备联测。只读缓存副本保留；补充 registry 的私有编排因索引名检查失败停止，部分归档保持且没有用于构建，不写成准备成功。旧 VM 没有停机或扩容，旧 target 和其它材料没有删除。

配置闭包合同、VM 启动、native 编译、签名准备、实际注册和整组托管传输分别记账；任何一项不能替代下一项。没有实际三设备传输、精确回执、逐跳、无旁路、计量和清理证据时，11 场景保持未验。完整 NodeQuality、许可、工厂身份及正式发布／生产部署仍未完成，整体目标没有签收。CI 继续暂停，按用户结束要求停止持续推进。
