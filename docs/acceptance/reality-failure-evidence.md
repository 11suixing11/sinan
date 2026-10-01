# Reality 传输失败证据：独立验收

对应 [Issue #6](https://github.com/theLucius7/sinan/issues/6)，只补定位证据，不宣称修复间歇卡住。配置、环境、密钥、令牌、HTTP 载荷、客户端与服务完整日志仍留在隔离 runner 的私有工作目录，并在清理时删除。

## 已确认的基线

- 固定业务源码 `743955c` 的 [36786401885 attempt1](https://github.com/theLucius7/sinan/actions/runs/36786401885/attempts/1) 在 HUP 后 `resumed-traffic` 只收到 1,103,168/2,097,152 字节，90 秒到期、curl exit28；Agent 与运行时 active/running、Result=success、ExecMainStatus=0。这不能证明实际传输恢复。
- 同源码同失败 job 只重跑一次，[attempt2](https://github.com/theLucius7/sinan/actions/runs/36786401885/attempts/2) 的真实安装、首次传输、Agent 重启、HUP、续传、签名拒绝、重装和在线退役全部通过。总累计字节从3146275到6292550，HUP 原始 epoch 归零而累计值保留。
- 后续整合源码 `75cf69f` 的 [36788887683](https://github.com/theLucius7/sinan/actions/runs/36788887683) 在首次下载90秒收到0字节、exit28。不是同源码对照，不将此差异归因为业务或门禁改动。

旧摘要没有直接夹具 HTTP/TCP/TLS 结果与失败时的资源数据。原因仍未知，Issue 保持开放；重跑成功不删除失败证据。

## 实现及预算

`scripts/e2e-traffic-evidence.py` 是 Python 标准库 helper，端点固定在验收自己的回环 HTTP、SOCKS 与 Reality 监听端口。上传文件路径只交给 curl，不进入摘要；没有可配置目标、重试、UA 修改或外部上传。

- 原 curl `--max-time 90` 不改。进程守护限92秒，只用于 curl 本身不能退出的情况，仍记录超时、返回28；不是把正常传输超时延长至92秒。原2MiB/1MiB载荷精确核对与 shell 失败退出继续保留，`curl_succeeded` 仅表示 curl 退出0，不能替代载荷及后续账本验收。
- 记录最多4条：首次/续传各一次下载、上传；HTTP状态、收发字节数、curl连接/首字节/总耗时、退出码与固定错误类别。curl连接时间指本地 SOCKS 连接，不能当作 Reality 内层TLS握手耗时。HTTP状态0表示尚未收到HTTP状态，越界或无效数字为 null。
- 失败时、停止服务前检查三项回环TCP（每项1秒）、直接HTTP夹具下载（响应头/正文的每次底层读取共享绝对截止，独立短命子进程硬2秒结束并回收、最多2MiB+1字节）、直接TLS夹具握手（总2秒、验证自己生成的证书与示例SNI），总网络预算最多7秒。不会重新发送代理流量，也不改变原失败退出码。
- TLS地址只取本次私有Docker夹具元数据的单一RFC1918 IPv4地址，不在摘要暴露地址、网络名或证书。元数据缺失时记录`not_configured`；未知不会记为通过。
- 宿主仅公布CPU数、一分钟负载乘1000、MemAvailable KiB；客户端和HTTP夹具仅公布进程是否仍存在，不能证明正在处理流量或排除僵尸进程。常驻服务仍沿用固定两单元的ActiveState/SubState/Result/ExecMainStatus白名单；active不等于流量恢复。没有读配置、环境或私有完整日志。
- evidence输入最多16KiB，所有数据在写入和汇入公开摘要时再次严格过滤。私有文件0600、拒绝符号链接；不公开命令、路径、原始异常、curl stderr。诊断或写盘失败不能将原传输失败改成成功。已有失败的清理路径关闭 errexit，继续尝试所有清理并保留原退出码；原成功流程仍会拒绝清理错误。现有CI只上传allowlisted summary。

## 自动验收

```sh
python3 scripts/test-e2e-traffic-evidence.py
python3 scripts/test-e2e-driver.py
python3 scripts/test-e2e-runtime.py
python3 scripts/test-ci-signed-release.py
python3 -m unittest discover -s tests -p 'test_*.py'
bash -n scripts/ci-real-e2e.sh
python3 tools/check-core-boundary.py
cargo fmt --check
git diff --check
```

正常整合作者 `7ce37b9` 和正式主线 `8ef465f` 后，冻结源码 `357eadb` 的专项16项全部通过：保留作者14项真实回环HTTP/TLS、代理90秒、隐私白名单、外层进程硬截止及回收等回归，另验证直接HTTP读取的慢滴响应头/正文截止，以及已有失败28与原成功流程遇到清理错误的两态实际Bash回归。旧代码负对照中，慢滴头/体分别耗时3.48/3.50秒、超过两秒预算，失败清理也把原28覆盖成7；修复后全部重过。验收驱动21、缓存3、CI签名8通过；仓库Python实际运行101项，其中95通过、6项既有条件跳过，共143通过、0失败、6跳过。Python/Bash语法、只读fmt、core/actionlint及链接/差异检查通过，没有运行Rust编译或测试。最终独立提交的完整CI与真实Reality另行核对，不把旧743源码重跑当作本项验收。

## 故障矩阵与限制

本项为测试证据，不改变产品内存/磁盘预检、IP查询403/429/超时、任务取消/重复提交/部分章节、断连补报与持久化。写盘失败保留原传输退出码；HTTP/TLS超时在隔离夹具自动验收。Agent重启/HUP/持续代理流量沿用真实安装场景；小内存与诊断过程总验仍由专用测试节点独立执行。不在生产重跑硬件或修改上游运行时；观测只发生在已经失败之后，不能据此证明失败瞬间或公网压力场景的根因。
