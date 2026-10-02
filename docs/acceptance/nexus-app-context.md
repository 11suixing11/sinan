# NeXus 精确容器上下文收尾

以 `adbaaafe2fc6b4717606318d0e054034e6db5948` 为基线，完成既定候选的上下文核对后按用户要求收尾提交。私有控制和计划集中修改、独立静态复审后冻结九项输入；本地与远端两程序语法通过，仅执行一次受影响只读观察，输入前后保持。实际结果见[机器记录](evidence/nexus-app-context.json)，前次列表及失败见[上下文补验](nexus-context-recovery.md)。

## 实际结果

SSH 退出 0，2607 ms。前次选中的完整容器 ID、名称、配置镜像与运行状态匹配；本次读取的实际镜像 ID 和非根工作目录前后一致。没有重新列举或替换候选。

该工作目录下的 `package.json` 和 `composer.json` 两次读取均退出 1：各自 Docker 原始错误为文件未找到，stdout 为 0，stderr 分别 148、149 字节并独立保存。只能说明这两个固定路径下未读到文件，不能据此推断整个应用没有 Node 或 PHP。可选 OCI 标签查询退出 0，但投影结构未接受，标签元数据仍未知。没有认证应用技术栈、数据库或当前连接 2，也没有重新导出密钥。

## 控制和交付边界

远端只读取精确容器的六个身份字段、三个可选 OCI 标签及两份清单。没有执行容器进程、应用 bootstrap、SQL、安装或硬件压测，也没有重连测试节点。SSH 每流 64 KiB／总计 25 秒，远端查询共享九秒和 40 KiB 原文预算；所有查询原文、摘要及完整度私有留存。源文件、前次证据、SSH 配置、身份文件、known_hosts 和 agent socket 保持，本地所属进程组清理确认。

Docker 以 stdout 返回文件的 [tar 流](https://docs.docker.com/reference/cli/docker/container/cp/)；读取器只接受内存中的未压缩单个普通文件，JSON 不超过 16 KiB，使用 [TarInfo.offset_data](https://docs.python.org/3.12/library/tarfile.html#tarfile.TarInfo.offset_data) 处理普通 PAX 元数据，没有文件系统提取。由于本次两次文件读取失败，清单解析和运行时声明都保持未知。

本地可用空间快照为 1,151,418,368 字节，仍低于既有 4 GiB 管理预留；当前项目原生打包、传输及构建未开始。旧源码、缓存、target、虚拟机磁盘、失败记录和凭据保持，预留未降低。当前锁仍为 429 总包／418 registry；镜像工具资格及旧 ARM64 快照不认证当前主线制品。

本次收尾属于源码与证据交付。服务器插件安装、当前 Agent 注册、连续代理流量中的资源保护与取消清理故障矩阵，以及完整 NodeQuality 许可／工厂和硬件验机仍待验收；CI 保持暂停，没有新增分项 PR 或正式发布。
