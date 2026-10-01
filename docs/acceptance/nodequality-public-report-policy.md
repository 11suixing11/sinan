# NodeQuality r7 三处公开报告 POST 策略验收

本项关联 #65，只处理固定 HardwareQuality、IPQuality、NetQuality 源中的三处 `upload.check.place` 报告 POST。NetQuality 的网络和回程章节各调用同一处，因此组合夹具中旧逻辑有四次 POST。默认及显式 `upload_report=false` 禁止这些 POST；显式 `true` 仍保留原隐私、轻量模式条件。非法策略在脚本装载时、`check_bash` 和任何探测之前拒绝。

这不等于禁止所有上传。HardwareQuality 的 `mark.check.place`、Geekbench 自身通信及其他工具请求仍有独立缺口；#65 保持打开。原硬件、IP、网络、回程章节和测试调用保留，不借 `-p` 跳过 CPU/GPU。所有版本完整验机的安全门禁保留；本项不签收或开放完整 NodeQuality，不执行上游基准、不下载新 rootfs、不接受第三方工具许可。

## 实际接线

构建器嵌入 canonical 入口、五份脚本、四份完整许可证、`source-helper.py` 和固定 `report-policy.py`。canonical `source-lock.json` 和物化文件保留原字节。运行器在私有 `.runner` 中物化 helper；原入口的三类脚本请求经现有 curl shim 到 `source-helper.serve`，先核对 canonical 长度和 SHA，再变换 HW/IP/Net 三个已知角色。header/swap 仍返回原文。

策略 helper 只能从 `source-helper.py` 同目录读取，使用 `O_NOFOLLOW | O_NONBLOCK`，要求常规文件、最多 64 KiB 和固定 SHA。读取后的已核验字节在固定命名空间中编译，调用本地纯字节 `transform`；不重新打开路径，不启动外部 patch 进程，不执行上游字符串。原源最多 2 MiB；唯一锚点替换和最终 SHA 都必须匹配，返回值另有字节和 SHA 校验。不提供运行时任意 helper/source 路径覆盖。

修改只增加装载期策略、三处 POST 条件，以及 Net 的局部空 `report_link` 初始化。后者避免 `false` 时继承环境中的陈旧公开链接。保留原版权和完整 AGPL 文本，注释标明修改日期与范围；公开脚本许可不代表 rootfs 中每个工具取得打包授权。

| 角色 | canonical SHA-256 | r7 服务字节 SHA-256 |
| --- | --- | --- |
| hardware.sh | `73e032ef5409e014cca411a71c677a76db19a94ef0a96a73827b41b2059cd86c` | `f7413e8a19eaacce2df70b1ae6c63bab89334bca0d07846badacde5ef6afcb0c` |
| ip.sh | `b30df5a3c2204276c54e99dcc5080b46f8a627667730aee7de63b109b8ecaecf` | `176295d6bc9803d19c1794b73f5c801fe8325f076c60f2114cc943a4b62e68dc` |
| net.sh | `6c40fe1ae40d969255cb63075c94882733b82ba43831341eb1aadeea7b1fbfcd` | `f35836caa8e598f443c7b718daa71211cee393534b03f2f3b3d3062a6a3a90b2` |

策略 helper SHA-256 为 `0c66e702084820e399a16b18b51ba331cd8edd406dd96ede7c2ee84f78c30245`。canonical source-lock SHA-256 保持 `3d20398eeda72654c59b3271fd03b35ca8c0b4e92ee92a054a4a8c432a62723a`。三份已保存、已核验的真实源仅用于静态变换收据；移除上述固定变更后均逐字节还原原文。该收据不证明真实上游工具执行或全部网络副作用已受控。

构建器、runner、release 元数据、适配器和面板当前版本同步到 r7。保留 r2–r6 既有任务历史、Started checkpoint 收集及取消；r4–r7 日常模式兼容。旧不可变制品不改写。

## 独立验证与证据边界

`tools/test-nodequality-report-policy.py` 默认使用私有合成 SHA 和无探测函数；实际 builder、runner、helper 与 curl shim 接线不替换。可选的只读源目录仅用于测试读取已核验固定原文中的 `check_*` 和四个 `run_*` 编排函数体，替换所有探测和 serializer，完整原有效脚本装载流程不执行。旧 serve 未 patch 的负对照和显式 true 必须真正到达自有回环 HTTP recorder；false/default 必须为零 POST，完整 argv、NQENV、CPU/GPU 调用、本地 JSON/ANSI 章节保持一致。隐私、IP 轻量和回程模式分别作条件对照，并检查污染环境不能显示旧链接。

Mac Bash 3 的测试 chroot 替身对 process substitution 使用 stdin 模拟；记录原 argv，但不作为 Linux Bash >=4 的 FD 继承证据。在 Bash >=4 中，该替身保留原 `bash -c "$*"` 调用。真实 Debian guest 的原 FD 路径和有限 chroot 环境继承需单独记录，不能以 Mac 组合结果替代。

helper 失败测试覆盖未知角色、源一字节漂移、重复锚点、错误输出 SHA、超长输出、helper 缺失/篡改/符号链接/FIFO/超长文件，均不能输出可执行脚本或回落在线源。构建测试还验证缺失/篡改 helper 不生成制品，以及已公开 TEST_ONLY 信任根的签名覆盖嵌入 helper；这是测试签名，未经正式签名或发布。

最终 r6 下载流补修 `6d1e731` 已保留；合流到固定主线后的 r6 基线为 `2d63c9652c5ae85793508e3dfe8fcdf3a0e5557b`。host 组合受测点为 `bf0ad14`，移植到该合流点仅解决双方 PROGRESS 追加记录。19 份选定产品、测试、构建与 Cargo 输入 SHA 逐个相同，后续文档回填不改变这些受测字节。

| 验证 | 结果 | 证明范围 |
| --- | --- | --- |
| 合流后的来源专项 | 16 通过，0 失败/跳过，9.710 秒 | 含 r6 有界接收、实际回环超限、r7 helper 接线和测试签名；本机 curl 8.7 的 control 被自身上限截断，不以此替代 r6 guest curl 7.88 的旧行为证据 |
| 合成策略专项 | 6 通过，0 失败/跳过，31.823 秒 | 装载期非法值拒绝、精确 patch/输出校验、helper 文件边界，以及回环 POST 正反对照 |
| 固定原文函数体组合专项 | 2 通过，0 失败/跳过，73.278 秒 | 四个原 caller 的完整 argv、NQENV、原条件、全部 stub 章节；Mac 的 FD 路径使用上述 stdin 模拟 |
| 三份真实源静态收据 | 3 份原/补丁 SHA 匹配，逆变换逐字节相同 | 只调用已核验 production helper 的纯 transform，不执行上游有效脚本 |
| r7 版本切换阶段原 wrapper/daily/release 基线 | wrapper 34、daily 7 通过；release 32 运行，28 通过/4 既有条件跳过 | 对应接入 r6 bounded receive 前的 r7 快照；合流后未重复这些基线，不签收新增来源路径 |
| Rust 版本及 API 专项 | 待根任务记录最终 19 项结果 | 历史 r2–r6、r4–r7 daily、全版本 full 门禁及面板接口 |
| Debian guest 有限环境继承与原 FD 组合 | 待根任务回填独立收据 | 真正 Bash >=4 caller/FD 路径与最小受控 chroot，分别记录，不认证完整 rootfs |

私有 host 收据位于本任务 `nq-inner-report-prototype-20261001/evidence/`：`r7-final-inputs-before.json`、`r7-final-inputs-after.json`、`r7-production-source-receipt-final.json` 和三份 `r7-*-final.log`；仅保存摘要、合成参数和测试结果。根任务另保存 guest 与 Rust 受验身份，不能把不同快照的历史证据改写成同一次运行。

GitHub Actions 按用户要求暂停，不触发或以历史 CI 签收本项。完整工具链、实际完整诊断、公开上传总量为零、宿主副作用和生产部署均未验收。

复核命令只操作合成夹具或既有固定源的受控函数体：

```sh
python3 tools/test-nodequality-sources.py
python3 tools/test-nodequality-report-policy.py
python3 tools/test-nodequality-report-policy.py --readonly-upstream-dir <已核验固定源私有目录>
python3 tools/test-nodequality.py
python3 tools/test-diagnostic-modes.py
```
