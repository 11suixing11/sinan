# 运维与验收脚本导航

本目录保留面板运维及已有接入、实机/CI 验收入口。命令在仓库根目录运行；完整参数和执行前提以[部署文档](../docs/deploy.md)、[开发指南](../docs/dev.md)及[真实验收](../docs/e2e.md)为准。

## 面板安装与维护

| 入口 | 用途 |
| --- | --- |
| `panel.py` | Compose 面板安装、状态、日志、自检、备份、升级等管理操作 |
| `init-env.py` | 初始化部署环境配置；已有凭据不能作为临时文件覆盖 |

## 真实验收驱动

| 入口 | 用途 |
| --- | --- |
| `e2e-driver.py`、`e2e-real.sh` | Reality 验收步骤、私有状态和设备操作 |
| `e2e-http-fixture.py`、`e2e-traffic-evidence.py` | 受控流量目标与证据计算 |
| `verify-e2e-runtime.py` | 核验真实运行时来源与能力 |
| `diagnostic-baseline.py` | 诊断基线采样与证据 |

这些驱动会连接配置的设备或启动实际服务，需要专用测试环境；普通代码整理使用本地单元、集成与浏览器夹具验证，不自动运行实机验收。

## CI 包装与签名夹具

`ci-compose-smoke.sh`、`ci-real-e2e.sh`、`ci-openrc-smoke.sh` 包装已有集成场景；`ci-bootstrap-install.py`、`ci-retirement.py` 覆盖安装/退役，`ci-test-trust.py`、`ci-signed-preflight.py`、`ci-signed-release.py` 准备与核验公开 TEST_ONLY 证明。这些文件的存在不表示当前 CI 已启用，工作流仍按仓库协作规则暂停。

`test-*.py` 为相应脚本的本地回归；另有部分回归位于根 `tests/`。构建、制品签名和发布工具见 [tools/README.md](../tools/README.md)。

## 维护约定

- 现有文件路径被文档、工作流及发布流程引用，本轮保留原路径。
- 发布安装模板与生成入口在 `deploy/`，不要复制一份到此目录维护。
- 命令中的真实管理员密码、令牌、数据库信息和验收私有状态不进入 Git。
