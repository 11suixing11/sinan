# 司南 Sinan

自托管的中文服务器与代理节点控制面板。面板保存期望配置；Linux Agent 主动连接，负责对账、应用恢复、系统遥测和按用户计量。代理运行时由独立 systemd 服务管理，Agent 重启时继续提供服务。

提供 VLESS + Reality、用户授权、订阅、部署状态与流量汇总。运行时固定上游 sing-box 1.14.2，保留官方默认构建标签并启用统计 API。NodeQuality 支持按需诊断，公开报告上传默认关闭。

先按[部署文档](docs/deploy.md)创建私有 `.env`，再在仓库根目录启动：

```sh
docker compose --project-name sinan --env-file .env \
  -f deploy/docker-compose.yml up -d --build --wait
```

面板默认在 `http://127.0.0.1:8080`。远端接入需要可达的 HTTPS 地址；导入匹配架构的 Agent 与运行时后，通过面板生成一次性安装命令。

- [部署、制品导入、节点接入与升级](docs/deploy.md)
- [开发、测试和 CI](docs/dev.md)
- [真实 Reality 验收与阶段证据](docs/e2e.md)
- [HTTP API](docs/api.md) / [设备协议](docs/protocol.md)
- [架构决策](docs/adr/0001-declarative-snapshots.md) / [问题与选择](docs/open-questions.md)
- [执行计划](docs/PLAN.md) / [验证进度](PROGRESS.md)

MVP 仅支持 Linux/systemd、单运行时和单管理员，范围见架构决策。许可证：AGPL-3.0-only。
