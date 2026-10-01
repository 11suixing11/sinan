# NodeQuality r10 七份静态数据独立验收

关联 #28，基于 #111 的 r9，独立 PR 只固定 IP/Net 实际消费的七份二级静态数据。rootfs 的两个现有归档仍超过当前制品单文件限额，且存在来源/授权缺口；本项没有将它们接入或放宽全局限额，完整门禁保持。

## 已定位的请求与改动

固定 IPQuality `87397e2c3196ec796f5477c83343c2354df601ea` 消费 `ref/iso3166.json`、`ref/dnsbl.list`；固定 NetQuality `d5b99484d51286374d24b892c1b54235dc282148` 消费 `ref/iso3166.json`、`ref/province.json`、`ref/AS_Mapping.txt`、`ref/iperf.json`、`ref/speedtest_cn.json`。此前均经 rawgithub 的 GitHub/CDN 分支读取在线 main。

七文件的 Git tree/blob 身份、长度与实际字节分别核对，再记录 SHA256。清单保留原文件名、来源路径、提交、大小和所属仓库完整 LICENSE；现有入口/脚本/许可证十文件原字节不变。新版共17文件、687,969B，source-lock SHA 为 `16bc414c7f9bf79514b36f9cc3d8b0376a1805ad58f24f8d8be81ddeb8542dd7`。完整摘要列在 [机器可读证据](evidence/nodequality-pinned-data-r10.json)。

`data-policy.py` 被嵌入同一个 runner 并由现有签名制品覆盖。serve 在原报告/swap/依赖策略之后读取有界普通数据文件，核验输入、数据身份、唯一请求锚点和输出摘要，将七个精确 curl 表达式替换为 Bash 内建 `printf '%s'`。原数据经过单引号转义，百分号、换行、单引号、反引号与命令替换只作为数据输出；不执行 eval，不需要外部解码器、chroot 文件路径或临时文件。输出逐字节相同，其余解析、探测、参数与报告逻辑保持。

helper SHA 为 `0115f90f8ce521eab1472d8426b8f8dfbf1fefca427ae0fdfd557b346a8fdab3`。最终 IP 脚本180,863B/SHA `e332b5405ca12fe03ad5f427789c38c92f04bb2b66df05d296060e4cd8483c1e`；Net 脚本179,678B/SHA `99f8a26dabdc09163a165edbb9e200f675011d4f7bef5cfe8b0b729b726e861f`。其它角色不应用数据变换，入口的清理第455行保持。

`IATA_Database` 在固定 IP 源中只有声明与赋值，没有消费点，本项保留其声明，没有下载或打包该 CSV。cookie、广告、赞助内容、动态供应商请求、工具上传和 rootfs 不属于这七个数据请求。仓库 LICENSE 记录不是整个工具链的授权证明，固定测速目标清单也不等于获得对第三方节点探测的许可。没有执行目标探测、供应商查询或完整上游程序。

## 冻结源码与结果

受验源码为 `e95fa7b`，基于 #111 的 `7c30ccd`，最终仅追加文档。r2–r9 历史报告、r4–r10 日常与所有 full 门禁保持；显式保留 adapter 历史 r9 和面板 queued r9 测试，避免版本常量升级后失去旧版本覆盖。

| 检查 | 实际结果 | 范围 |
| --- | --- | --- |
| 新静态数据专项 | macOS6/6；Debian6/6，均无跳过 | 实际 serve 与签名包装接线；七个旧 curl 负对照/新本地输出；含特殊字符的数据不执行命令；每文件缺失/篡改/FIFO/符号链接拒绝；helper、构建器边界；真实固定原文逆还原与语法 |
| macOS 整组 | 122运行，117通过、5条件跳过 | 数据6、依赖11（Bash3跳过完整源语法1）、swap10、来源16、报告6、wrapper34、daily7、release32（隔离Linux root条件跳过4） |
| Rust/API | 19通过、0失败/忽略 | adapter15、panel chain_gate3、真实HTTP/PostgreSQL日常1；Clippy/fmt/core/diff通过，未重跑全部workspace/全部面板 |
| Debian12 Bash5.2.15 | 86运行，85通过、1缺minisign跳过，57.311秒 | 来源16、数据6、依赖11、swap10、固定原文函数体FD组合2、wrapper34、daily7；原探测/serializer为惰性替身 |

来源双架构夹具的旧下载计数20已随17文件清单调整为34；首轮该预期失败日志保留，修正后才记录通过。依赖专项在Mac的语法跳过由Debian同项实测通过补充，不伪称Mac本身支持完整原脚本。

guest独立单元内部持久化读回：MemoryMax256MiB、Swap0、Tasks64、CPUWeight/IOWeight10、OOMScoreAdjust500、PrivateNetwork/PrivateMounts/NoNewPrivileges=yes、KillMode=control-group。峰值71,118,848B/11pids，memory.events max/oom/oom_kill全部0。按真实journal启动时间检查无新OOM；结束无进程、挂载、cgroup，SSH406重启0、同boot、swap0。42份输入前后不变，其中24份仓库输入匹配冻结提交。

guest result/postcheck/receipt SHA 分别为 `76ffe81380c1ecea351d0c969efad1ef33c531089b0283d35c6c013d9db39d55`、`ec5126d30300a9cc14fcc362778a43edaf3a23c7de3d87f36d2e3c49b33df622`、`74114ba4a018dd195fd628e60934c65d4a52218ce27f1593eca407d8150c97e0`。Rust 收据 SHA 为 `c027d01dc1190592703a6629f1d806036bc43dc4798523eac53d5ff493e38a0e`；专属PG55439按PID/启动时刻归属停止，PID不存在、端口关闭。

## 单独发现的失败传递缺口

[Issue #112](https://github.com/theLucius7/sinan/issues/112) 是本轮真实复现的独立问题，尚未在本项修复。固定入口用 `bash <(curl ...)`；供给器拒绝错误数据且stdout为空时，进程替换的失败没有传递给内层Bash，空脚本退出0。外层局部pipefail只能捕获章节命令的非零退出，不能捕获这个异步供给失败。

专用Debian上只执行固定的三份加载函数，curl接真实本地source-helper、chroot替换为Bash桩，篡改本地数据各一个字节：IP/网络/回程三条路径均记录helper错误，章节0字节，却继续下一条语句。没有执行整份上游脚本或公网请求。独立128MiB单元自然退出并回收，SSH未重启；该复现只记录配置预算，不声称做过运行期资源读回。原始结果摘要保存在本项JSON索引。

因此，本项只证明这些静态下载被固定本地数据替代、非法数据被供给层拒绝，**不证明供给失败已使整项任务正确失败**。下一独立修复须先完整取得并核验脚本、确认生产者成功后再执行，覆盖空输出失败、部分输出失败、成功恰好执行一次、真实退出观察器及清理。

## 复核和仍未验收范围

```sh
python3 tools/test-nodequality-data-policy.py --readonly-upstream-dir <已核验17文件的固定来源目录>
python3 tools/test-nodequality-sources.py
python3 tools/test-nodequality-report-policy.py --readonly-upstream-dir <同一目录> WiringTests
python3 tools/test-nodequality-dependency-policy.py --readonly-upstream-dir <同一目录>
```

本项没有运行真实完整工具；小内存/磁盘不足、403/429/超时、Agent重启、面板断连、取消、重复提交、部分报告、持续sing-box负载仍按完整故障矩阵验收。旧 [r8真实日常矩阵](registered-nodequality-daily.md) 不追认为r10完整负载证据。rootfs/二级工具来源与许可、cookies/UA/广告、内层上传和第三方目标授权仍待解决；#28不关闭。四个workflow继续暂停，只登记未来命令，没有正式签署、发布、部署或触发CI。
