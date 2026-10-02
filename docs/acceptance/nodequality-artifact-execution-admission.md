# NodeQuality 制品自身的完整执行准入

关联 [#28](https://github.com/theLucius7/sinan/issues/28)、[#65](https://github.com/theLucius7/sinan/issues/65)、[#66](https://github.com/theLucius7/sinan/issues/66)、[#82](https://github.com/theLucius7/sinan/issues/82)。本次源码以 `74b403c1365cfee157c2d191bd6255f7bc146aff` 为起点；本文件记录实现与待验范围，不提前记录测试通过。

## 修复的实际入口

既有面板和 Agent 拒绝新完整任务，但 r18 打包的 `nodequality` 仍允许直接传入 `--mode full`；其 host curl shim 还对两个 v0.0.2 rootfs 下载 URL 放行真实网络请求。这两个入口没有跟随产品的完整执行暂停状态。

r19 制品在参数校验后、系统探测及工作目录创建前拒绝显式和默认 `full`。没有环境变量或参数可绕过。日常检查保持；已有任务的报告解析、精确版本收集与取消仍由原生命周期处理，不要求旧制品重新生成、存在新准入清单或运行新 runner。

curl shim 不再为 rootfs 下载提供网络回退。保留的两个架构 nexttrace 历史安装命令也在 chroot shim 执行前拒绝。原 17 个固定官方源码/数据/完整许可证保持，新的拒绝不等于这些源码代表的完整能力已经恢复。

`plugins/nodequality/execution-admission.json` 明确当前制品仅提供日常检查及历史恢复，不允许新完整执行或运行时安装依赖。`source-helper.py` 在打包前要求清单严格保留未知权利和未验状态；把 bool 改为整数、宣称 Ookla 权利已通过、允许下载或完整执行均不能打包。清单完整内嵌于签名覆盖的 runner，可用 `nodequality --execution-admission` 读取。

日常模式可额外使用管理员在节点本机明确配置的正式 IP API；缺配置仍不请求认证来源。该查询不下载运行时代码、rootfs 或工具，不提供完整验机入口。

## 可重复的静态执行库存

新增 `tools/nodequality-execution-inventory.py` 只读取已有的固定 gzip tar，不下载、不展开、不解析许可接受配置、不 chroot、挂载或执行任何成员。先在同一打开文件描述符上核对整份归档摘要，再逐成员读取；归档、解压字节、tar 扩展头、单成员、成员数和可执行文件数均有限制；读取循环检查 240 秒绝对截止，异常阻塞文件 I/O 仍需外层监督。路径越界、重复成员、特殊 tar 成员、错误 ELF 架构、摘要不匹配、超限或输入链接均拒绝。

工具识别 ELF（包括没有可执行 mode 的库）、shebang 脚本及有执行 mode 的其他文件，保存路径、大小、格式、架构和摘要。root/home/etc 以及 `.config`、`.ssh`、`.gnupg` 的配置内容不读；链接不跟随，因此它不是完整依赖图审核。未知来源的 Ookla 即使改文件名，只要仍是普通 ELF，也会进入库存；它的字节身份不能因 dpkg 没有记录而遗漏。

示例只针对已取得并授权做静态审计的本地归档：

```sh
python3 tools/nodequality-execution-inventory.py /PRIVATE/BenchOs.tar.gz \
  --architecture amd64 \
  --sha256 5f844e73941c3623175c5cdc16b01db34c155d0d1bd9b0cf71f3d72e8b1148e1
```

没有事先声明的库存时，输出观察结果并返回 3，明确尚未登记。`--declared-inventory /PRIVATE/declared.json` 要求 schema、架构、归档身份及每个观察可执行文件的路径/大小/摘要/格式/架构精确对应；漏报、多报、重复或字节变化均失败。精确匹配返回 0 仅表示已登记的身份与该静态库存相同，输出的 `full_start_allowed` 和 `rights_verified` 仍为 false。

声明不能自动从待验归档重新生成后当作来源证明；需要独立可信的版本、取得来源、对应源码/构建配方、完整许可证及实际许可授权证据。工具不解释许可证，也不接受归档中的第三方接受配置替代管理员授权。

## 未验边界

amd64 与 arm64 均已有作者独立静态盘点，参见 [amd64](nodequality-rootfs-inventory.md) 和 [arm64](nodequality-rootfs-arm-inventory.md) 文档。本聊天新增的通用库存代码不冒称重演这两份归档；两份公开摘要只是字节身份，尚非上游签名、可复建配方或完整分发许可。

Ookla 1.2.0.84、Geekbench 5.5.1 的实际来源/分发及无人值守权利、工具自身上传、全部动态库与链接目标、全链宿主副作用和故障恢复仍需独立证据。现有 privacy/swap 补修和门禁不能代替这些证明。四个源码 CI 继续暂停；本文件不会关闭 #28/#65/#66/#82，也不会签收、正式发布或部署完整验机。

现有完全私有、无网络/挂载/硬件执行的 collector/政策夹具会在其模板副本中明确去掉新门禁，以继续检查历史报告与原政策。这是测试材料的源码修改；产品制品没有运行时绕过开关。最终验证需另在未改动模板及打包制品上证明新完整执行拒绝、无外部调用和无工作目录写入，再分别记录静态库存、日常查询及历史回收的覆盖结果。
