# 原生 TCP 工具独立验收

此 PR 仅增加 Sinan 自有 AGPL-3.0-only Rust 库和 `sinan-tcp-probe` 二进制，不注册第二个诊断插件、修改面板/UI、迁移表、接入 Agent 服务或打包签名。用户已授权在上游无许可证时自行实现；没有复制 TcpQuality 的代码、目标表、helper 或 rootfs。上游[许可证询问 #27](https://github.com/ibsgss/TcpQuality/issues/27) 未获授权前不分发其制品；本工具不声称与其 SYN、大包、路由或测速方法等价。

## 输入与参数

- 绝对、普通、私有工作目录和目录内普通私有快照文件；拒绝 symlink、输入 hardlink、已有输出与路径穿越。Unix 之外不能确认私有权限，明确拒绝运行。
- 目标 JSON 不超过 16 KiB，schema=1，1–8 个唯一 UUID 目标；仅名称、目标主机、端口、运营商标签和可空地区。未知字段、非法地址、零端口、控制字符或重复 ID 拒绝。未来面板只能从本服务器已有 enabled TCP 拨测提供冻结目标，本项没有任意 URL/命令/API。
- `--target-digest` 必须等于实际快照 SHA256；读取后在库内部保留不可变快照，后续输入修改不改变本轮范围。地区/运营商仅为管理员标签，空地区保持未指定。
- 必须 `--no-rank-upload`；只接受 IP 版本 4/6、count 4/8、concurrency 1/2（默认 4/1）。拒绝 `--allow-speedtest-staged`、`--no-rootfs`、测速、上传与未知/重复选项。
- DNS 2 秒、单 connect 1 秒、样本间隔 250 ms、总 60 秒含初始化报告/排队；其中预留 2 秒发布最终报告。最多 64 次连接，共享 1/2 个目标执行槽；最多一次解析，限制返回地址数，选择一个同族 SocketAddr 后只连该地址，不让 hostname 触发自动循环拨号。

## 输出与语义

只建立并立即关闭 TCP，零应用 payload；没有 HTTP、UA、报告/排名上传、raw socket、测速、子进程、依赖安装、网络或宿主配置改动。系统 TCP 握手仍产生协议流量，本工具不把连接次数等同原始发包数。

报告为有界 64 KiB JSON，记录 UTC 毫秒起止、参数、目标快照摘要、实际配置目标/解析地址、逐次错误/耗时、engine/version/source_commit。成功率分母是已尝试连接；没有尝试或 DNS/地址族不可用保持 null，没有成功样本的建连耗时为 null。0% 成功率只表示已尝试的连接全部失败，不能称为包丢失率、干净或零延迟；不排名或横向比较不同参数。

`result.json` 和 `sections/tcp_scope.json`、`tcp_summary.json`、每个 `tcp_target_<UUID>.json` 通过私有临时文件原子发布。章节含 name/text/complete/revision/collected_at，与执行结果分开；每次进度保留部分章节。网络探测结束不代表网络健康；全部探测完成 exit 0，截止后保存部分报告 exit 1，参数拒绝 exit 2。工具不持久化 job/outbox 或自行重启；外层框架负责互斥、预检、资源预算与确认取消。

源码 commit 由编译期 `SINAN_NATIVE_TCP_SOURCE_COMMIT` 记录；未嵌入时显示 null，不能称为固定或已签名制品。自身固定源码、Cargo.lock、许可证/对应源码与离线签名打包另 PR 完成；框架确认 NodeQuality 不退化后才另 PR 注册工具。

## 独立验证

```sh
export SINAN_RELEASE_PUBLIC_KEYS="$(python3 scripts/ci-test-trust.py)"
export SINAN_NATIVE_TCP_SOURCE_COMMIT="$(git rev-parse HEAD)"
cargo fmt --all --check
cargo test --locked -p sinan-tcp-probe
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

测试仅用回环监听和合成 resolver/连接夹具，不拨打公共第三方节点：

| 场景 | 验证 |
|---|---|
| 真实 IPv4/IPv6 | 四次成功，监听端每次收到 EOF/零应用 payload，地址/参数/时间正确 |
| 真正拒绝连接 | 有效 0% 成功率，所有 RTT 为 null，不伪造零延迟 |
| DNS 错误/超时、族不匹配 | 一次解析、零连接、成功率/RTT 未知 |
| 多 DNS 地址、count8、concurrency1/2 | 只使用一个匹配 SocketAddr，连接数与峰值有界 |
| 排队截止/部分报告 | 排队也消耗总预算，未开始目标没有计数或评分，已保存章节仍可读 |
| 外层 future 取消、实际 CLI 进程停止 | 活跃连接结束，后续连接不继续；既有部分章节保留 |
| 非法/重复/未知参数、输入超限、摘要不匹配 | 网络请求前拒绝；不改已有报告、不逃逸私有目录 |
| 真实 CLI | bounded JSON 与本地文件一致，禁用上传标志必需，拒绝宿主/测速选项 |

本机只做 fmt、locked offline metadata/core 门禁和差异检查，不在磁盘不足机器从头编译。Rust/真实 TCP/CLI、Clippy 与全工作区由受限隔离 Debian 12 和最终 CI 验证，结果补记。不把未启用工具视为完整 TcpQuality 接入，也不声明已验真实第三方网络、排名、Agent/面板重连、完整 NodeQuality 或持续代理压力。HTTP 403/429 不属于纯 TCP 工具协议，未来制品下载/框架适配另验。
