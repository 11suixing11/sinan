# TcpQuality 许可独立验收

对应 Issue #58。

- 固定审计 `ibsgss/TcpQuality@c2295ae096437859ce4bbc36f170428fcac47be9` 的完整 git tree、README 和 GitHub 许可接口，没有发现明确分发许可。
- 作者授权询问为 [ibsgss/TcpQuality#27](https://github.com/ibsgss/TcpQuality/issues/27)，2026-10-01 核对零评论；不重复催问，不把未回复当作同意。
- 当前仓库、构建脚本和工具注册未引入该上游任何文件或 rootfs；原生实现使用 Sinan 自有 AGPL-3.0-only 代码，后续执行与制品另行验证。
- 本 PR 仅修改 ADR、验收与进度文档。空白检查及引用/固定提交核对通过；没有运行上游诊断，没有修改生产节点。
- 原生工具、签名打包、参数、无上传、取消/互斥和报告尚未由本 PR 验收，不把许可审计当作接入完成。
