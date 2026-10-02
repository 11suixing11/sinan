# 订阅 HTTP/H2 转换与解析器升级

日期：2026-10-02。范围：来源格式转换、刷新缓存与不可变历史。本文记录静态依据和新增源码；尚未执行本轮测试、构建或检查器，最终组合提交统一验证。此前来源夹具的通过结果不能覆盖本次补修。

## 固定一手依据

实际面板选用的 sing-box `1.14.2` 对应完整源码对象 `af6e64c3b69e6132ebaee0e1a3d24e93903f6709`。其 [HTTP 客户端实现](https://github.com/SagerNet/sing-box/blob/af6e64c3b69e6132ebaee0e1a3d24e93903f6709/transport/v2rayhttp/client.go)在 `tlsConfig == nil` 时选择 HTTP/1.1，有 TLS 时直接选择 HTTP/2；缺省请求方法为 `PUT`。这一选择依据 TLS 配置，不是仅靠改 ALPN 就能保留 HTTP/1.1 over TLS。

格式对照固定到 Mihomo 完整对象 `88dcbf7f1614a67c3b36b848ee3592dfa92ada36`：

- [VMess 传输分支](https://github.com/MetaCubeX/mihomo/blob/88dcbf7f1614a67c3b36b848ee3592dfa92ada36/adapter/outbound/vmess.go)分别处理 `network: http` 和 `network: h2`；是否套 TLS 与 HTTP 版本分别决定。
- [H2 传输](https://github.com/MetaCubeX/mihomo/blob/88dcbf7f1614a67c3b36b848ee3592dfa92ada36/transport/vmess/h2.go)明确要求 HTTP/2，不回退 HTTP/1.1，允许底层连接没有 TLS。
- [HTTP 伪装传输](https://github.com/MetaCubeX/mihomo/blob/88dcbf7f1614a67c3b36b848ee3592dfa92ada36/transport/vmess/http.go)发送 HTTP/1 请求，缺省方法为 `GET`；上层可先包 TLS。

## 转换边界

原实现将来源的 `h2` 和 `http` 均转换为 sing-box `transport.type: http`，却不核对 TLS：无 TLS 的 H2 会变成 HTTP/1.1，有 TLS 的 HTTP/1 会变成 HTTP/2。解析“支持”不能代表换协议后能连接原服务器。

Mihomo YAML、普通 URI、Base64 URI 与 VMess Base64 JSON 入口现在明确拒绝 `h2` 无 TLS 与 `http` 有 TLS，分别返回固定分类 `unsupported_h2_without_tls` / `unsupported_http_with_tls`，不自动启用 TLS、换协议或丢弃必要参数。Mihomo 明文 HTTP 的缺省或空方法明确保存为 `GET`，显式方法保留；H2 缺省 `PUT` 不改。直接导入的 sing-box JSON 使用其原生 HTTP 参数和 TLS 语义，不按其他格式重新解释。

同一固定源码还确认：Mihomo H2 总是强制 TLS ALPN `h2`，因此转换明确保存该实际值，只接受其原生 `host` / `path` 选项；不将别种传输的 method/headers 偷渡为 H2 功能。Mihomo 明文 HTTP 的精确 `headers.Host` 转为 sing-box 独立的 `transport.host`，保留多个 Host 的随机选择；Host 重复、非精确大小写或与另一 host 语义冲突时拒绝。其他 HTTP 头的多值在源实现中随机挑一项，sing-box 则可能发送全部值，不能等价时返回 `unsupported_random_http_headers`，单值保留。

回归源码覆盖上述拒绝组合、两种 URI 列表、VMess 编码、可等价组合、Mihomo 缺省/显式方法、Host/随机头/H2 ALPN 与选项边界及原生 JSON 参数保留。这些夹具尚未在本轮执行，不能认证真实第三方订阅或服务器可用。

## 解析器、304 和历史

解析器身份由 `sinan-subscriptions-1` 升为 `sinan-subscriptions-2`。排队时立即将同源旧解析器的 queued/running 任务标为 superseded，新请求不复用旧解析器任务；同解析器的重复请求仍共享同一任务。

获取前既有缓存判定同时核对设置 revision、身份 epoch 与成功来源批次的 parser version。解析器升级后即使 URL、凭据和 ETag 没变，也不发送旧 `If-None-Match` / `If-Modified-Since`；没有新正文的 304 返回 `unexpected_not_modified`，不能将旧成功批次当成新解析器成功。成功获取的新正文按新解析器追加不可变来源批次，不能 UPDATE/DELETE 旧批次或旧配置版本。

新增 PostgreSQL 夹具直接插入旧解析器曾产生的无 TLS H2→HTTP 配置，覆盖旧排队/运行任务失效、旧缓存不合法、无正文保留上次成功时间和错误、新正文拒绝节点后旧配置/摘要/解析器身份仍原字节保存。旧已应用链路继续依既有不可变快照读取；本次不回写冻结的链路向量，也不将旧版本升级为新解析器产生的版本。

## 静态下载审查边界

本轮只读审查了来源获取、解压、解析与事务提交：HTTPS、逐跳同源、全部 DNS 地址检查后固定连接、禁用环境代理/自动跳转/Referer、20 秒下载期限、压缩及解压各 2 MiB、四个工作槽、结构/别名预算、秘密错误分类及迟到结果输入核对仍保留。

地址策略仅允许普通全球 IPv6 `2000::/3`，另拒绝 6to4、Teredo/特殊用途及文档段；IPv4 映射/兼容/可转译表示和已知 NAT64 `64:ff9b::/96`、`64:ff9b:1::/48` 均不在允许范围。补充了嵌入回环、私网、metadata 地址的拒绝夹具与普通全球 IPv6 正例，但没有执行网络请求。运营者自定义全球 NAT64/6rd 路由前缀不能只从一个地址识别；本静态审查不证明这类实际路由或跨网设备已签收。

没有使用真实机场 URL、凭据或公网代理，不签署、不发布、不部署；GitHub Actions 继续暂停。
