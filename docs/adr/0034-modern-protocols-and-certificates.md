# ADR 0034：现代代理协议与托管证书

状态：已接受（用户明确授权，2026-10-01）。

## 决策

在现有 sing-box 插件内增加 Hysteria2、Shadowsocks 2022（AES-128/256-GCM）、TUIC v5、AnyTLS、Naive（HTTP/2）和 Snell v6；保留 VLESS + Reality。固定的 sing-box 1.14.2 已提供对应入站，不修改上游源码、不增加运行时实例或 SSM API。此授权覆盖原 MVP 对这些协议的排除。

编译器使用可序列化的协议枚举；旧节点和旧部署快照缺少新字段时仍解释为 Reality。节点协议创建后不可更改。每个授权独立生成凭据，保留原 UUID、统计名称、订阅令牌和历史账本；Shadowsocks 与 Snell 使用节点密钥和授权密钥。配置始终确定性排序，订阅只读取已成功应用的快照。

TLS 协议同时支持手动证书和 ACME 自动申请、续期。ACME 使用 sing-box 的 certificate_provider，首批提供 Let's Encrypt 的 HTTP-01 和 TLS-ALPN-01；管理员配置证书域名、联系邮箱及验证方式。同一服务器的自动证书共用一个提供器，邮箱和验证方式必须一致，域名排序去重；编辑已有自动证书时共享设置原子更新到该服务器所有自动证书节点，避免无法逐个修改的问题。证书保存在运行时现有 data 目录的 certificates 子目录，独立于 revisions，因此升级与回滚不删除证书。验证端口不得与 TCP 代理监听冲突；不自动修改 DNS、防火墙或系统端口权限。

手动私钥、节点密钥和授权密码不从节点列表返回。订阅只包含当前代理用户所需的客户端凭据，绝不包含服务端 TLS 私钥、ACME 账户配置或其他授权。完整 sing-box JSON 是所有协议的共同订阅格式；无法无损表达的链接订阅明确报错并引导使用 JSON，不能静默漏掉节点或编造 Snell URI。

适配器识别各协议的 TCP/UDP 监听，服务健康仍要求进程和本地统计接口可用；真实认证与计量另用本地运行时集成测试验证。核心不识别代理协议、证书或代理用户业务。自动证书申请失败必须使部署失败，不将仅有监听端口视为证书就绪。

## 依赖与验证

复用现有 serde、base64、rand、rustls 及 sing-box ACME 能力，不引入独立 ACME 守护进程。适配器增加 tokio-rustls、webpki-roots（工作区已有传递依赖）及 quinn，用实际 TLS/QUIC 握手校验证书链、域名和有效期。仅读取证书文件会依赖运行时私有存储格式且不能证明服务已加载证书；额外启动 OpenSSL 子进程不适合所有平台，因此不采用。QUIC 仅作有超时的本地握手，不转发流量。SDK 提供通用健康检查预算，core 将适配器建议限制到最多 300 秒，其他操作的超时保持原策略。自动签发等待最多 240 秒，失败沿用部署回滚。

验证包括旧配置字节兼容、新协议编译与凭据隔离、数据库迁移和授权事务、实际运行时配置检查与流量统计、证书配置和持久路径、前端构建。公网 CA 签发、真实 DNS 以及各操作系统实机部署不能由本地配置检查代替；未运行项在 PROGRESS 中单独记录。

依据：[sing-box 入站文档](https://sing-box.sagernet.org/configuration/inbound/)、[ACME 提供器](https://sing-box.sagernet.org/configuration/shared/certificate-provider/acme/)、固定 v1.14.2 源码。
