# 真实 Reality 验收驱动

`scripts/e2e-driver.py` 通过真实面板 API 创建一套专用服务器、节点、用户和授权，记录健康部署及精确用量。操作者负责准备 Linux/systemd 设备、执行安装脚本、启动独立客户端、产生流量，以及重启或重载服务。完整人工验收边界仍见 `bash scripts/e2e-real.sh guide`。

驱动仅创建本次随机名称对应的资源，不修改或删除已有业务对象。每个创建步骤立即保存编号；若请求已提交但响应丢失，使用相同私有 state 重试可以按本次唯一名称恢复。不要删除 state 后在同一设备重新开始，也不要同时运行两个使用相同 state 的驱动。

## 准备与安装

需要 Python 3.9 或以上，无第三方 Python 依赖。Linux 和 macOS 可以驱动面板，受管设备仍必须是 Linux/systemd。将面板地址、节点地址和伪装域名放在当前 shell 的私有环境变量中，真实地址、密码、脚本和证据均不得提交。

```sh
E2E_PRIVATE=$(mktemp -d)
chmod 700 "$E2E_PRIVATE"
export SINAN_E2E_ADMIN_PASSWORD_FILE="$E2E_PRIVATE/admin-password"
# 使用安全编辑器写入管理员密码，再执行 chmod 600 "$SINAN_E2E_ADMIN_PASSWORD_FILE"。
# 设置 PANEL_ORIGIN、NODE_PUBLIC_HOST、REALITY_SNI 为本次环境值。
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" prepare \
  --origin "$PANEL_ORIGIN" --public-host "$NODE_PUBLIC_HOST" --sni "$REALITY_SNI"
```

密码也可通过 `SINAN_E2E_ADMIN_PASSWORD` 提供，缺省则隐藏输入。密码和 Cookie 不写入 state、不打印；state、安装脚本、客户端凭据和阶段证据以 0600 保存，所在目录必须是仓库外的 0700 私有目录。

`prepare` 在私有目录保存 `install.sh`；将该文件安全传到专用设备，以 root 执行。脚本包含一次性令牌，应按凭据保管。令牌过期或准备升级时，重新签发并保存安装脚本：

```sh
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" install --refresh
```

当前安装脚本始终选择面板自身版本对应的 Agent，不能通过该接口选择 0.1.0。0.1.0 到 0.2.0 验收需要先准备匹配的旧面板与旧制品，再切换到新面板或明确记录受控的安装 workaround；安装脚本不能因此被当作已验证任意版本升级。

## 健康部署与独立客户端

```sh
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" ready \
  --agent-version 0.2.0 --timeout 600
```

`ready` 要求设备在线、目标版本等于已应用版本、健康为真且无部署错误，可同时核对 Agent 版本。只导出本次专用节点的 `client.json`，本地混合入口固定为 `127.0.0.1:2080`；初次成功保存 `ready` 用量基线，重复执行保留原基线。

如受管设备在容器中监听 20000、宿主将其映射到 28000，使用 `ready --client-port 28000`。这仅修改私有客户端 JSON 的目标端口，保留面板的原始端口，并在 state 记录本次客户端端口。公开地址必须可由独立客户端直达该端口；不要依赖仅代理 HTTPS 的反代来转发 Reality。

在独立客户端使用上游 sing-box 启动此 JSON，经 SOCKS 访问测试端点；同时完成可识别的下载和上传，再等待真实面板入账。自建测试端点应记录客户端实际完成字节数，避免把只有下行增长当作双向验收。

```sh
curl --fail --proxy socks5h://127.0.0.1:2080 https://example.com/
# 经同一 SOCKS 代理向受控 fixture GET 大文件、POST 上传文件。
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" traffic \
  --label first --after ready --min-uplink 1 --min-downlink 1 --timeout 180
```

可以把上下行阈值设为本次实际流量的合理下界。面板统计包含代理协议开销，不要求与 HTTP 文件大小严格相等。脚本仅对本次用户与节点查询，使用任意精度整数检查十进制总量、上下行同时增长且已确认用量没有倒退。

## 暂停、重启和重载

停止独立客户端，关闭本次用户的全部连接。显式设置只读 Agent 状态命令；命令按 `shlex` 分词后直接执行，不经过驱动端 shell。操作者负责命令仅进行只读查询，下面使用本机设备示例：

```sh
export SINAN_E2E_STATUS_COMMAND='sudo sinan-agent status'
# 容器示例：docker exec sinan-acceptance-agent sinan-agent status
# SSH 示例：ssh acceptance-host sudo sinan-agent status
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" verify \
  --label before --interval 35 --timeout 240
```

`verify` 每 35 秒获取新的面板用量和本地 Agent status，连续三次快照覆盖两个完整采样间隔；每次都要求 Agent 已连接、本地应用版本与面板相同、健康为真、`pending_batches=0`。`--interval` 可增加，但不能短于 30 秒；`--timeout` 适用于 CI 和较慢环境。

由操作者记录运行时 MainPID、设备公钥及本地身份文件摘要，重启 Agent，再检查 MainPID 未变。分别在 Agent 恢复和显式重载运行时后执行：

```sh
# 设备：systemctl restart sinan-agent.service
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" verify \
  --label after-agent --unchanged-from before
# 设备：systemctl reload sinan-singbox@main.service
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" verify \
  --label after-runtime --unchanged-from before
```

`--unchanged-from` 在每次采样立即断言已确认上下行与总量原封不动、设备公钥保持不变。仍有连接、重复入账、历史丢失或身份变化都会失败，不能等待这些差异消失后再宣称通过。外部重载时必须保持客户端暂停，避免非托管 HUP 的终值采样窗口影响结论。

恢复同一独立客户端，完成第二批上传和下载：

```sh
python3 scripts/e2e-driver.py --state "$E2E_PRIVATE/state.json" traffic \
  --label resumed --after after-runtime --timeout 180
```

升级后可重新执行 `ready --agent-version 0.2.0`，再用原稳定基线验证身份与已确认用量。设备身份文件、账本和运行时连续性仍应由操作者记录，不由面板公钥相等推断全部已通过。

## 证据与失败处理

每个通过的阶段将证据写入同名 JSON，并保存在 state 的 `checkpoints` 中。证据包括筛选用量、部署版本、设备公钥与版本，以及稳定阶段的本地 outbox 状态；真实证据保持私有。超时或断言失败返回非零，不写通过记录，不自动回滚、删除资源或操作账本。

`python3 scripts/test-e2e-driver.py` 覆盖创建响应丢失后的恢复、不重复创建或修改其他对象、双向增长、重复入账、身份变化、outbox 未清空及两个采样间隔的失败边界。此契约验证不能代替实际 Reality、systemd、升级与客户端流量验收。
