# 真实 Agent 托管路径：信任与验收入口

本步骤承接[有序路径编译、版本发布及 native 验收](ordered-paths.md)，增加正常的私有面板 CA 信任和真实设备验收入口。旧 native 结果使用了变换后的编译配置，不是原样产品包、真实设备回执或实际面板计量的证明。本步骤不把它升级为托管验收通过。

## 产品变化

Agent 本地配置可设置 `panel_ca_file`，默认未设置时保持原有公共证书根。HTTPS 注册、面板制品下载、WSS 认证连接和退役确认使用相同附加信任；证书链、有效期、主机名、面板 origin 和制品签名继续分别核对。独立 GitHub 下载和公网发现不采用该私有 CA。管理员通过本地配置选择信任文件，面板不能下发新的信任根，环境变量也不能关闭验证。

信任文件是绝对普通 PEM 路径，拒绝路径中的符号链接、超限文件、超限证书、私钥及非证书材料，加载失败在产生注册身份前拒绝。配置示例见 [agent-private-panel.example.toml](../../deploy/agent-private-panel.example.toml)，决策见 [ADR 0074](../adr/0074-private-panel-certificate-authorities.md)。本地私有 CA 不替代正式制品发布公钥。

## 三个入口及证据边界

| 工具 | 执行内容 | 不足以证明的事项 |
| --- | --- | --- |
| `tools/prepare-managed-paths-linux.py` | 验证完整冻结源，在原生 GNU/Linux 上有界离线重建 Agent/panel，固定真实 native runtime，重新生成 TEST_ONLY 签名归档，独立验签并由刚构建的 Agent 验证 release | 没有登记设备或运行用户流量；`prepared` 不等于托管路径通过 |
| `tools/managed-paths-controller.py` | 只在预置专用的三套 systemd/PID/挂载/网络环境执行普通注册、服务启停及有界只读证据查询 | 不负责创建虚拟机，不改变业务数据库，不生成 ACK、检查点或成功回执 |
| `tools/test-managed-paths-linux.py`、`tools/managed-paths-fixtures.py` | 真实管理员 API 创建服务器、安装插件、建立节点/用户/路径/授权；实际订阅客户端和外部 X 传输，读回设备实例、发布向量、探测、屏障及账本 | 工具合同测试、准备阶段和选定失败场景补验均不能冒称整组实机成功 |

所有业务写入走实际 HTTP API，版本推进只由产品后台发布器和真实 Agent 执行。控制器没有任意 shell、SQL 修改、直接 publisher 或确认设备接口。公共订阅使用真实返回的字节，不改端口、认证或配置来使场景通过。网络 helper 仅拥有它创建的 X、客户端和自有目标，不启动产品 A/M/B 服务。

## 环境与资源准入

专用 Linux 节点使用 cgroup v2 和 systemd。A/M/B 必须各有不同的根文件系统视图、machine-id、PID、挂载与网络命名空间；每套环境正常使用 `sinan-agent.service`、`sinan-singbox@main.service` 及产品固定控制/统计端口，不能在同一服务管理命名空间给产品改端口。多个容器共用内核只能签收本节的功能闭环，不能视为三台独立硬件或完整硬件压测。

预置环境在宿主私有 `run_root` 和每个设备 `/etc/sinan-managed-test-run` 保存相同的规范 UUID。控制器绑定 init PID/starttime、命名空间 inode、根目录设备/inode、machine-id、面板 unit 文件身份和两个 native 二进制哈希；实际执行前固定命名空间及根目录 FD，防止 PID 复用指向其他设备。所有本地描述文件为私有普通文件，真实秘密不提交。

原生构建使用新的输出和 target 目录，前置要求各目标文件系统至少 4 GiB 可用、16384 可用 inode，以及 1280 MiB 可用内存。构建单元使用 MemoryMax=1 GiB、MemorySwapMax=0、TasksMax=128、PrivateNetwork、900 秒期限和整个 cgroup 停止；动态保护保留 1536 MiB 磁盘、256 MiB 内存及 inode。拒绝、原日志、收尾结果分开保存。旧材料不能为本次构建而删除，旧虚拟机和此前工厂预算也不隐式扩大。

2026-10-02 的既有专用 Debian 12 容量盘点：根盘可用 589557760 字节，附加盘可用 1444077568 字节，均低于新构建的准入要求。整步冻结后在新的私有目录复制 782 份完整输入，真实准备入口先核对全部身份，再以 `managed_build_disk_reserve_rejected` 拒绝：输出／target 目标盘可用 1431252992 字节，可用内存 1296416768 字节也低于 1280 MiB 初始要求。没有创建 target、bin 或成功收据，所属构建 unit 为 not-found、MainPID=0；旧 runtime 摘要不变，旧材料和本次失败证据保留。首次宿主编排因挂载点由 root 所有，在复制前拒绝；仅新建并授予本次私有目录，没有改变挂载点所有权或旧目录。新设备环境、Linux 重建、原样签名包托管与完整矩阵仍待验，不能用资源拒绝或已有 native 回环结果代替。

## 准备和运行契约

先冻结整步的功能源，记录 `schema:1`、HEAD、`files:{relative_path:{sha256,size}}`，覆盖 Cargo、各 crate、插件、脚本、工具、deploy 模板及前端源；生成 dist 另外保存身份。把该源复制到独立 Linux 构建目录，保持冻结清单原字节，通过外部记录的清单 SHA-256 验证。准备入口参数如下；这里的路径均是操作者准备的本地绝对路径。

```text
python3 tools/prepare-managed-paths-linux.py \
  --source-root /owned/source \
  --frozen-inputs /owned/frozen-inputs.json \
  --frozen-inputs-sha256 <recorded-sha256> \
  --target aarch64-unknown-linux-gnu \
  --toolchain-dir /owned/toolchain \
  --cargo-home /owned/cargo \
  --target-dir /owned/new-target \
  --runtime-binary /owned/sing-box \
  --runtime-sha256 <recorded-runtime-sha256> \
  --output-dir /owned/new-artifacts \
  --dedicated-test-node
```

构建 cache 必须在专用构建范围中，无未审批 Cargo config；本入口不安装包或联网取缺失依赖。只使用仓库公开的 `TEST_ONLY.key` 与对应公钥，正式发布验证明确拒绝该根。归档只包含实际验证的架构，不把一份二进制伪装成两种架构。`prepared-artifacts.json` 绑定源码、锁、工具链、二进制、runtime 版本/能力、归档及签名。运行中的 Agent 安装验证和面板实际选择/下载仍须另外取得设备证据。

预置环境另保存私有控制器 manifest，至少包括 `schema/run_id/run_root/test_only/dedicated/source_identity`、对应 `prepared-artifacts.json`、两二进制路径/哈希/大小、三个角色的固定设备路径与隔离身份、面板 unit 路径/哈希、origin、data_dir、私有 environment_file/environment_sha256 及本次专用 PostgreSQL socket/database。unit 必须只使用这一份普通 `NAME=value` 的环境文件，不含 shell 引号、转义或空白值。控制器私下核对声明及实际运行环境中的数据库、数据目录与公开 origin，不能让测试 unit 指向其他业务；不展示数据库连接串、订阅／认证 URL 或密码。inspect 仅返回无凭据的公开 `panel_origin` 用于绑定。数据库名固定为 `sinan_managed_<去除连字符的run_id>`；数据只读查询设置五秒 statement timeout，限定本次 ID 及返回行数，截断证据不能通过。

控制器逐次接受以下有界 stdin JSON，固定参数 `--manifest /owned/environment.json`，返回同一 schema/run_id/operation 的 `{ok,facts}`。只允许 inspect、enroll、agent_stop/start/restart、runtime_restart、panel_stop/start、device_snapshot、panel_evidence、restore_all 和 cleanup。注册描述文件在本次私有根内，保存 `{run_id,role,token}`，不在普通日志展示令牌。

```json
{"schema":1,"run_id":"00000000-0000-4000-8000-000000000000","operation":"device_snapshot","role":"A","arguments":{}}
```

运行清单绑定面板 HTTPS origin/CA/管理员描述文件、角色私有地址与 SNI/设备配置副本、控制器及 helper 的源哈希和 manifest、TEST_ONLY release、冻结源码身份及证据目录。实际路径探测访问同一个面板 HTTPS `/health`。如果测试 CA 也用于 runtime 的原生 HTTPS 探测，只在该隔离环境的 unit 中配置正常 CA 信任，不关闭证书验证，不修改宿主公共根。

网络 helper 的 `prepare --plan /owned/network-plan.json --work-dir /owned/new-fixtures --dedicated-test-node` 创建本次 X 和自有目标的私有材料。plan 固定 `schema:1/test_only:true`、与控制器相同的规范非空 run_id、native_binary/native_sha256、`addresses:{fixture,A,M,B,client}`、`ports:{x,handshake,tcp_echo,udp_echo,https}` 及 `managed_ports:{A,M,B}`；地址仅允许自有隔离 IPv4，A 可列两个真实入口端口，M/B 可以在不同地址使用同一个端口。由 controller 读回的 runtime.pid 为宿主 PID，guest_pid 为容器内 PID；helper 身份使用宿主 PID/starttime/netns inode，不能混用。

本组节点的 SNI 使用自有 `reality.test`，通过现有节点创建 API 的 `settings.reality.handshake_server/handshake_port` 指向 helper 的自有握手服务；不借用第三方网站。正常场景先创建入口再将已有入口用于原子路径批次，原子失败另覆盖新入口事务回滚。该设置来自真实节点参数，不在生成后的产品包或订阅中做替换。

## 集中验收矩阵

全组包含三段和四段路径、原子批量失败与原键重放、同身份来源 follow/pinned 更新和解析失败保留、候选失败保留旧代、切换后整向量复验/屏障/旧凭据清理、Agent/内部 runtime 重启、面板断连及 outbox 补传、X 停止且无直接回退、授权撤销/恢复、引用保护及退役。实际 TCP、UDP 和 HTTPS 逐跳观察与入口单次计量分别记录；设备 pending outbox 只有 opaque 统计名称，用户/节点归属从面板实际账本读取。

各状态等待最多 240 秒，整组最多 1800 秒，命令和网络调用另有限额。失败先恢复产品控制连接，再清理本次 helper 子树；产品 cleanup 只停止本次所属设备单元，核对 MainPID 和 cgroup，不删除数据库或证据。未确认清理不能记通过。支持选定场景补验时，结果明确标为部分验收。

运行驱动和 helper 的宿主 systemd 单元须实际设置 MemoryMax≤512 MiB、MemorySwapMax=0、TasksMax≤96 及稍长于整组期限的 RuntimeMaxSec，并使用 KillMode=control-group。控制器 inspect 读取实际 cgroup 文件核对前三项，未实施限额时在业务写入前拒绝；这些限额只约束验收工具，产品服务另由各设备 systemd 管理。

## 本步集中验收结果

最终功能输入 782 份，SHA256 `05d5131356cb2aed5489639ba29dee7aebaf7b00cb717f85d947634b415c9768`，源基线 `06c2404a2b0ebdc6faccfd18af05b64dd8042f6f`。修改及测试代码集中完成后才开始格式化、冻结和验收；首轮失败与补验的私有日志摘要见 [机器收据](managed-agent-paths-local.json)。

| 范围 | 最终结果及边界 |
| --- | --- |
| Agent-core 全 targets | macOS ARM64 7 个 targets，265 通过、0 失败、8 项既有条件忽略；不是 Linux systemd 专项或工作区全测试通过 |
| 私有面板 CA | 真实 HTTPS 注册／下载、WSS 设备认证、清除身份后的退役回执，以及未知根、错误主机名、过期证书、旧配置、文件预算和 FIFO 替换通过；不扩展独立公网客户端的信任 |
| 工具合同 | 准备／控制器／进程所有权 11 项、网络 helper 21 项、API 驱动 15 项，共 47 个有效不同方法通过；合同不证明真实设备矩阵 |
| 静态门禁 | 全工作区 all-targets Clippy（warnings 为错误）、fmt、core 分层、差异检查通过；前端源与 dist 未改，未重复 Bun、构建或浏览器 |
| 专用 Debian 12 | 冻结输入与原 runtime 身份核对后，磁盘准入真实拒绝；没有构建、签名或三设备运行，材料保留 |

首次 Rust 在编译前因测试声明与锁中版本不符拒绝；改为既有 `tokio-rustls 0.26.6`，不更新包版本。准备合同首次在 macOS 上遇到 TERM 后孤儿僵尸进程组的 SIGKILL EPERM；修复为观察自有非僵尸成员、有界确认退出，并分别保留原失败和清理失败。两项收齐后停止测试集中修复，仅补失败及受影响范围；已过 helper／API 驱动四份源身份保持，原日志不覆盖。

本步源码和相称本地验收完成，原样产品包、真实 Agent 回执、实际面板账本及 11 个托管场景尚未执行。它不包含完整 NodeQuality、正式发布或生产部署，不解除 Geekbench/Ookla 及工厂身份/材料门禁，不恢复暂停的 CI。

自然跨过 probe/barrier 期限后的真实过期重放另记为待验；11 个既定功能场景的总通过不包含该证明。信号取消只有实际发生且清理得到确认才单独记录，否则同样未验。
