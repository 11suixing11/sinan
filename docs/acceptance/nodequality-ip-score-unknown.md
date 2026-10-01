# NodeQuality r13 未知 IP 评分独立验收

关联 [#117](https://github.com/theLucius7/sinan/issues/117)，基于 #116 的 r12。产品冻结于 `66ec04bdac9d006d9067775a2991ce0221852676`；后续 `473d592` 只调整测试夹具的等待上限。本项修复节点固定 IPQuality 脚本的评分解释、显示和 JSON，不替代面板逐源错误、缓存与查询适配验收。

## 复现与修复

固定 IPQuality `87397e2c3196ec796f5477c83343c2354df601ea` 的原始 SHA256 为 `b30df5a3c2204276c54e99dcc5080b46f8a627667730aee7de63b109b8ecaecf`。在专用 Debian 12 提取四个原查询函数，仅将 curl 换成本地错误 JSON、进度函数换成空操作；Scamalytics、AbuseIPDB、IP2Location、IPQS 均返回 `null|LOW`、退出0。没有访问外部服务、执行顶层脚本或硬件负载。

另外，原 IPQS JSON 使用 `ipapi[ipqs]`，不是 `ipqs[score]`；ipapi 的响应字符串未经格式检查就进入 awk 算术。新 `ip-score-policy.py` 在之前各策略之后修改唯一固定锚点：

- 四个数字源必须提供 JSON number、整数且在0至100之间；真实0保留，字符串 `"0"`、null、布尔、越界和小数均未知。
- ipapi 仅接受0至1的十进制比例和已知风险标签；DB-IP只接受low/medium/high。输入经过验证才进入已有算术与等级映射。
- 响应必须是完整且唯一的 JSON object，拒绝明确失败的 success/status/error/errors 包络。非JSON、缺字段和错误类型不会落入“低风险”。
- 文本按既有精简/完整视图显示未知源，避免空评分进入条形图算术。六个 Score 字段缺失时写入真正的 JSON null；有效值保持原字符串表示，IPQS使用自己的评分。

helper 普通文件/大小/摘要、脚本输入与输出均验证；原17文件来源包和许可证保持。升级r13并保留r12历史任务兼容，全部full启动门禁保持关闭。请求表达式、UA、cookies、凭证获取、重试、其他分类字段不属于此修复，不能据此宣称已经完成逐源请求审查或全链无绕过。

| 身份 | SHA256 |
| --- | --- |
| IP score helper | `f5ae90c823d6b6d993c9254369220f6128ac7169f41b557c603ab00f184f245f` |
| r12 IP输入 | `e332b5405ca12fe03ad5f427789c38c92f04bb2b66df05d296060e4cd8483c1e` |
| r13 IP输出，182,869B | `f434c87f920cc9594aca3978b58e28dcaf786c074d70d64475f567068d093c4d` |

## 验证范围

新增 `tools/test-nodequality-ip-score-policy.py` 的10项专项，通过实际 source-helper 获取变换后的固定源码，只执行原六个查询函数、评分显示函数和六行 Score 序列化；curl返回固定响应，jq/bc/awk使用真实工具。原查询URL不会收到请求。专用Debian使用Bash5，本机Bash3的关联数组条件跳过明确计入结果。

专项验证四源旧错误包络低风险的负对照、新未知结果、0/100及各风险阈值、新旧有效输出相同；ipapi合法百分数/DB-IP合法等级保留、非法算术文本不执行；文本未知行符合原精简模式；JSON六源null与有效IPQS零分保留且其他对象字段不变。还检查多JSON/截断输入、类型/范围、唯一锚点、输入输出摘要、helper缺失/篡改/符号链接/FIFO/超限、非法返回和构建失败。

冻结前的初测发现正则 `$` 接受末尾换行，以及测试函数边界包含后续注释，均已修正；短暂 helper 摘要未更新导致的拒绝日志也保留。冻结后的结果与原始复现收据分别记录，不把这些初测失败混成最终通过结果。

## 冻结组合与清理

独立组合结果及清理收据写入 [机器可读证据](evidence/nodequality-ip-score-unknown-r13.json)。本次不运行完整上游引导、rootfs或硬件测试，不改变生产服务。

Debian组合114运行、113通过、1因缺minisign跳过，194.199秒。新增评分专项10项全部通过，另含排名8、加载10、来源16、数据6、依赖11、swap10、原文编排/FD组合2、wrapper34、daily7。

实际单元属性在运行中读回：MemoryMax256MiB、MemorySwapMax0、TasksMax64、CPUWeight/IOWeight10、OOMScoreAdjust500、PrivateNetwork/PrivateMounts/NoNewPrivileges=yes、KillMode=control-group。峰值69,853,184B/11pids，memory.events的max/oom/oom_kill为0；从journal实际单元起点检查无新内核OOM。

结束后单元回收、主/控制PID为0，无任务进程、挂载或cgroup；SSH PID406、重启0、同boot、swap0。48份输入保持，其中30份仓库输入匹配产品冻结提交。Debian没有使用后续测试超时调整，不能把后续版本的测试文件哈希伪记为该运行输入。

Rust/API19通过、0失败/忽略，含adapter15、chain_gate3、真实HTTP/PostgreSQL日常1；adapter Clippy、workspace fmt、core及diff通过，未重跑整个workspace。专属PostgreSQL55439按自身PID和启动时刻停止，确认PID不存在、端口关闭。

本机首次报告编排有一个子场景超过既有20秒期限，未改代码的单项重试也超时。第三次带进度采样显示每秒持续推进，20秒已有70条记录且无stderr；夹具对每个无害probe都启动Python记录器，尚有章节待完成。`473d592` 仅把测试等待上限增为60秒，保留所有输出/参数/上传断言和超时后的进程组清理；产品时限、函数与制品不变。原三次失败记录保留，之后只补跑失败单项及此前未执行的wrapper/daily/release。


本机150个唯一用例最终137通过、13条件跳过，连同三次保留的超时共153次执行。跳过包括评分4项和排名4项Bash3关联数组、依赖1项Bash3语法、release4项Linux root条件；前三类已由Debian实际执行补充。三次失败夹具已无遗留进程。

| 收据 | SHA256 |
| --- | --- |
| Debian result | `f3757fa563980a5962647ba48fd583c14360c7c1b54dfd281d73732ae0eb7c15` |
| Debian postcheck | `93a8ce12d925f9db29ffa1d8b0f37a4ed29cdc6dec8478a9c358fc9166a137e4` |
| Debian receipt | `2d699dc3142735285a5240dd88bdd4f891346bf5d838cdea83171c3d152ce914` |
| Rust/API | `45d38ba9271198f888dfb50201f6df15e31079a285d79c7aebe50616adbe2717` |

## 复核与未验收范围

```sh
python3 tools/test-nodequality-ip-score-policy.py --readonly-upstream-dir <已核验17文件的固定来源目录>
python3 tools/test-nodequality-sources.py
```

当前只验证评分解释，不证明HTTP失败分类、远端正式接口可用、所有非评分字段正确，或完整报告在真实服务下已通过。浏览器UA、cookies、临时密钥/凭证来源、其他上传、二级工具及rootfs来源/许可证继续待审查；完整NodeQuality与持续代理流量的全部故障矩阵、正式签署发布部署均未验收。四个workflow继续暂停，新增CI命令只供统一恢复时执行。
