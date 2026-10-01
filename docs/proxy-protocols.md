# 代理协议与证书

在“sing-box 插件 → 代理节点”创建节点，选择协议并授权给代理用户。使用新增协议前需升级面板和 Agent。每台服务器仍运行一份完整配置。无授权节点不监听端口，也不申请证书；变更按原有 5 秒窗口合并发布。

| 协议 | 节点监听 | 证书与凭据 |
|---|---|---|
| VLESS + Reality | TCP | 保留原有 X25519、UUID 与 short ID，不需要申请证书 |
| Hysteria2 | UDP | TLS 证书，每个授权独立密码 |
| Shadowsocks 2022 | TCP + UDP | AES-128-GCM 或 AES-256-GCM，节点密钥与授权密钥自动生成 |
| TUIC v5 | UDP | TLS 证书，每个授权独立 UUID 和密码，ALPN 为 h3 |
| AnyTLS | TCP | TLS 证书，每个授权独立密码 |
| Naive | TCP（HTTP/2） | TLS 证书，统计名称作为用户名，独立密码；客户端开启 UDP over TCP |
| Snell v6 | TCP | v6 多用户模式，节点 PSK 与每个授权的 userkey 自动生成 |

所有协议使用同一用户账本，统计名称仍为 `u{用户ID}_n{节点ID}`。撤销后重新授权会更换凭据；旧的客户端需更新订阅。协议与 Shadowsocks 加密方法创建后不可更改，需要变更时创建新节点。

## TLS 证书

Hysteria2、TUIC、AnyTLS、Naive 的“证书域名”必须是证书覆盖的 DNS 名。

- 自动：选择 Let's Encrypt，填写联系邮箱和验证方式。HTTP-01 要求域名解析到服务器、TCP 80 可从公网访问且未被占用；TLS-ALPN-01 对 TCP 443 有相同要求。两种方式均不支持通配符申请。同一服务器共用邮箱与验证方式；编辑现有自动证书时这两项原子同步到该服务器的其他自动证书节点，冲突时整个修改撤回。
- 手动：同时粘贴 PEM 证书链和私钥。面板检查格式和密钥匹配；部署时实际握手检查域名、信任及有效期。编辑时两项均省略会保留原证书。手动证书需自行续期并重新保存。

自动证书由运行时持续维护，保存在其 `data/certificates` 目录，配置版本切换和回滚都保留。该目录包含账户密钥和证书私钥，应按现有服务账户权限备份，不得公开。面板列表不回显任何私钥；客户端订阅只包含必要的公共证书和自身授权凭据。

初次签发最多等待 240 秒；健康检查要完成真实 TLS 或 QUIC 握手，同时检查服务状态和统计接口。证书未就绪、域名不匹配或过期都不会使新版本进入订阅，失败沿用既有回滚。TLS-ALPN-01 的 TCP 443 不能同时作为托管 TCP 节点端口；使用 TCP 443 节点时请选择 HTTP-01。UDP 443 不与 TCP 验证端口冲突。

管理员自行准备 DNS、网络放行和服务绑定低端口所需的系统权限；本功能不修改 DNS、防火墙或系统安全设置。已有 Linux systemd/OpenRC 服务带绑定低端口能力；其他系统仍需核对实际服务账户权限。本轮未接入 DNS-01 或 DNS 服务商令牌。

## 客户端订阅

新增协议统一使用 `?format=singbox`，界面默认复制此格式；需要支持对应协议的 sing-box 客户端，建议与固定运行时 1.14.2 对齐。Naive 客户端还需包含 `with_naive_outbound` 及对应平台 Cronet 支持，Snell 客户端必须支持 v6。服务端支持不代表所有客户端或操作系统发行包都支持全部出站。

原有链接订阅和订阅令牌保持不变；`?format=links` 仅支持 Reality。混合订阅含新增协议时返回 409，提示切换 JSON，不静默省略节点、不生成非标准 Snell 链接。手动证书的公共链随 JSON 下发，客户端不启用 `insecure`。订阅仍仅含已经成功应用且健康的版本及当前有效授权。

## 本地验证

Rust、Bun 常规检查外，可使用固定运行时执行实测：

```sh
SINAN_TEST_SINGBOX=/tmp/sing-box cargo test -p sinan-compiler -- --include-ignored
SINAN_TEST_SINGBOX=/tmp/sing-box cargo test -p sinan-panel --test protocol_runtime -- --ignored --nocapture
```

运行时需包含 `with_quic,with_acme,with_v2ray_api,with_utls,with_naive_outbound`；本地环境需 OpenSSL 生成一次性证书。第二项通过实际客户端验证新协议的 TCP/UDP 流量、用户计量、证书域名拒绝及重载撤销。所有测试只使用临时凭据和回环端口。

实际 ACME 测试使用 [Let's Encrypt Pebble](https://github.com/letsencrypt/pebble)：在独立目录编译其 `cmd/pebble` 与 `cmd/pebble-challtestsrv` 到 `bin/`，然后运行：

```sh
python3 tools/acme-smoke.py --pebble-source /tmp/pebble --runtime /tmp/sing-box
```

脚本隔离测试 CA/DNS、执行真实 HTTP-01 与 TLS-ALPN-01、检查证书持久复用，以及短期证书过期后重载自动续期；不请求公网证书、不安装系统信任根。测试证书只存临时目录。此测试不替代真实 DNS、公网 Let's Encrypt、长期驻留续期与多平台实机验收；运行结果及未验证范围见 PROGRESS。
