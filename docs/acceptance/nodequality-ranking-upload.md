# NodeQuality r12 硬件百分位上传独立验收

关联 [#65](https://github.com/theLucius7/sinan/issues/65)，基于 #115 的 r11。受验源码为 `2d08c61`；本项只约束 HardwareQuality 的 `mark.check.place` 评分提交，不关闭整个上传审查 Issue，也不开放完整验机。

## 复现与行为

固定 HardwareQuality `06f99880d516bb744afa2948261b9697c79789e2` 的 `get_mark` 先在本地计算 CPU/GPU/内存/磁盘分数及总分，再 POST `{cpu_score,gpu_score,mem_score,disk_score,total_score}` 取得百分位。r7 的三处公开报告 POST 开关没有覆盖它；默认或显式关闭上传仍会提交评分。

r12 在已有报告/swap/依赖策略之后，用 `ranking-policy.py` 修改五个唯一锚点：每次计算先清除旧百分位；本地计算完成后，未允许上传就返回；文本解释“百分位未知：未允许上传本机评分；本地评分已保留。”；JSON 添加 `Benchmark.percentile_upload_allowed`。这个布尔字段只记录许可，不声称查询成功。

数值计算、硬件测试、原请求 URL/参数/10秒超时/响应解析及章节保持。未添加隐私或快速参数，不删 CPU/GPU 测试，也不将未知百分位写成零；原 JSON 序列化将缺失百分位写为 null，文本显示 N/A。请求失败前已清空百分位，因此不会复用旧百分位值。

helper 通过同一 runner 的制品签名覆盖，source-helper 对 helper 普通文件/大小/摘要、输入和最终输出分别检查。原始17文件包、完整许可证与 source-lock 保持，策略说明写入 UPSTREAM.txt。版本升r12，保留r11历史兼容；全部full门禁保持关闭。

| 身份 | SHA256 |
| --- | --- |
| 新 helper | `6f46038c22267108b4572b1f1382a5deb779ecd51d90b0910c2a90f3ef122d59` |
| 输入硬件脚本 | `82b18e8eef943a4acfcfb3724fb6b0526ac1b769d8c3591839e89d50f82f562c` |
| 输出硬件脚本，156,653B | `3590bf56114fa8a2ec8bb7b9249efb2faf10ae52b49566a1084f8f00ab1d05eb` |

## 新专项的证据范围

`tools/test-nodequality-ranking-policy.py` 通过真实 source-helper 得到固定硬件脚本，只提取原 `get_mark`、`show_mark` 与最后一段 Benchmark JSON 序列化。四个评分输入函数用无害固定值代替，bc/jq 是真实工具。curl 的固定原参数先验证，再仅将目标改为测试自有回环 HTTP recorder，其他 URL 拒绝；实际 curl 执行 HTTP 和10秒超时，代理环境被移除。没有执行整个硬件脚本、Geekbench、公开请求或真实跑分。

Debian 的8项专项全部通过：

- 默认及false：旧逻辑各1次真实回环POST，新逻辑0次；四个评分函数各调用一次，组件10/20/30/40及总分100保持；之前的99百分位清空，JSON为null、文本N/A且有中文原因。
- 显式true：旧/新POST载荷相同、恰好1次；本地分数与返回百分位保持，原其余JSON字段保留。
- 403、429、真实curl超时、非JSON、缺失字段：每场景1次请求，没有重试；本地分数和其他JSON仍在，百分位未知且不沿用99。
- 非法上传值：装载期退出2，评分函数、消费者请求和报告输出均未发生。
- 真实源反向还原、固定摘要/唯一锚点/helper边界/非法返回/构建失败；四个原评分公式逐字节保留。

Mac 使用 Bash3，四项真实关联数组场景明确跳过，未以模拟Bash记为通过。Debian使用Bash5并实际执行这四项。专用VM在测试前通过独立有界单元安装 Debian 仓库的 jq、bc 测试依赖；它们不作为新诊断运行时安装步骤或发布制品，工具版本和摘要另存于收据。生产服务未变更。

## 冻结组合与清理

产品冻结于 `2d08c61`，之后 `643522b` 仅修正旧报告测试的逆变换断言：原测试没有剥离新增 ranking 补丁，首次组合6项中1项失败。原失败日志保留，修正后只补跑失败单项和此前未执行的wrapper/daily/release；产品文件没有改变。完整提交及收据摘要见 [机器可读证据](evidence/nodequality-ranking-upload-r12.json)。

| 检查 | 实际结果 |
| --- | --- |
| 本机组合 | 140个唯一用例最终131通过、9条件跳过；共141次执行，另保留上述1次首次失败 |
| 跳过明细 | 新专项4项Bash3关联数组、原依赖语法1项Bash3、release4项Linux root条件；前两类均由Debian实际执行补充 |
| Rust/API | 19通过、0失败/忽略：adapter15、chain_gate3、真实HTTP/PostgreSQL日常1；adapter Clippy、workspace fmt、core及diff通过，未重跑整个workspace |
| Debian12组合 | 104运行、103通过、1缺minisign跳过，80.001秒；新增排名专项8项全部通过 |

Debian组合包含来源16、排名8、加载10、数据6、依赖11、swap10、原文编排/FD组合2、wrapper34、daily7。实际单元属性在运行中读回：MemoryMax256MiB、MemorySwapMax0、TasksMax64、CPUWeight/IOWeight10、OOMScoreAdjust500、PrivateNetwork/PrivateMounts/NoNewPrivileges=yes、KillMode=control-group。峰值70,348,800B/11pids，memory.events max/oom/oom_kill为0；按journal实际起点检查无新内核OOM。

结束单元回收，主/控制PID为0、无任务进程/挂载/cgroup；SSH PID406、重启0、同boot、swap0。46份输入不变，其中28份仓库输入匹配产品冻结提交；不将后续测试断言修正伪记成该guest运行的输入。专属PostgreSQL55439已按自身启动PID与时刻归属停止，确认PID不存在、端口关闭。

Debian result/postcheck/receipt摘要分别为 `198bf09da20aa6673635d16445f6f484a1b7f66e127e3c6308a9247f585210e7`、`ecf9cc28ec8e48ad645ac6ade8d234dead233a700dbad5a34261f5f5cbd862dc`、`b33295792c8dfc571da0252adb6910d6dc8869e89a53f3fe580c5de9d81ad51b`；Rust收据为 `c88e02f70fad8142516567ca2e59a3044105246900ca5c1f3484b2c6aaeedded`。没有把r11最小chroot或旧r8注册设备日常矩阵追认为r12完整验机证据。

## 复核与剩余范围

```sh
python3 tools/test-nodequality-ranking-policy.py --readonly-upstream-dir <已核验17文件的固定来源目录>
python3 tools/test-nodequality-sources.py
```

`SINAN_UPLOAD_REPORT` 的装载期校验来自原报告策略；非法值不得绕过，true也只是允许当前这处请求。Geekbench 自身上传、其他工具流量、rootfs来源及二级工具授权仍未完成审查；没有把一处零POST证明当作全链零上传。完整 NodeQuality、与持续sing-box流量并行的全故障矩阵及正式发布部署仍未验收。四个workflow保持暂停，新增CI命令只供以后恢复时执行。
