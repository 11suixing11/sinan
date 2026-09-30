# 开发、构建与验证

所有命令在仓库根目录运行。

## 本地开发与测试

需要 Rust stable、PostgreSQL 16。仓库提交了 `web/dist`，不修改前端时可直接编译面板；修改前端时使用固定版本 **Bun 1.4.2** 管理依赖和运行构建，并同步构建产物。CI 与 Docker 前端构建使用相同版本。

尚未安装 Bun 时，按[官方安装说明](https://bun.com/docs/installation)安装指定版本，并确认当前终端可调用：

```bash
curl -fsSL https://bun.com/install | bash -s "bun-v1.4.2"
export PATH="$HOME/.bun/bin:$PATH"
bun --version  # Must report 1.4.2.
```

如果不使用 Compose 面板，可以单独启动一个开发数据库。在按[部署文档](deploy.md)生成 `.env`、且没有占用 5432 端口的环境中运行：

```bash
# Source only the generated configuration with URL-safe hexadecimal passwords.
set -a
. ./.env
set +a

docker run -d --name sinan-dev-postgres \
  -p 127.0.0.1:5432:5432 \
  -e POSTGRES_DB=sinan -e POSTGRES_USER=sinan \
  -e POSTGRES_PASSWORD="$SINAN_DB_PASSWORD" \
  -v sinan-dev-postgres-data:/var/lib/postgresql/data postgres:16
until docker exec sinan-dev-postgres pg_isready -U sinan -d sinan; do sleep 1; done

export SINAN_DATABASE_URL="postgres://sinan:$SINAN_DB_PASSWORD@127.0.0.1:5432/sinan"
export DATABASE_URL="$SINAN_DATABASE_URL"
export SINAN_LISTEN=127.0.0.1:8080
export SINAN_DATA_DIR="$PWD/data"
cargo run --locked -p sinan-panel
```

数据库名、用户和卷在首次执行时创建，面板启动时自动迁移。重用该容器时运行 `docker start sinan-dev-postgres`，继续使用最初的数据库密码。只使用本地原生 PostgreSQL 时，也可以先创建具有建库权限的开发角色和数据库，再把 `SINAN_DATABASE_URL` 与测试用 `DATABASE_URL` 指向它；sqlx 集成测试会创建独立测试数据库。

另开终端开发前端，Vite 将 API 请求代理到 `127.0.0.1:8080`：

```bash
cd web
bun install --frozen-lockfile
bun run dev
# After editing the frontend:
bun run build
```

提交前运行：

```bash
# DATABASE_URL must allow creating isolated test databases on PostgreSQL 16.
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
(cd web && bun install --frozen-lockfile && bun run build)
git diff --exit-code -- web/dist
```

依赖真实上游二进制的测试默认标记为 ignored，设置 `SINAN_TEST_SINGBOX` 后显式执行，不能把默认 `cargo test` 当作这些专项检查已经通过：

```bash
export SINAN_TEST_SINGBOX=/绝对路径/sing-box
cargo test -p sinan-compiler --test compilation -- --ignored
cargo test -p sinan-adapter-singbox --test runtime -- --ignored
cargo test -p sinan-panel --test reality -- --ignored
```

CI 另外检查分层禁用词、Linux 构建与 Compose 启动持久化。CI 的通过状态需要以远端实际运行结果为准。

## 真实安装与 Reality CI

`real-e2e` 在 Ubuntu 24.04 amd64 runner 上复用本次 musl Agent 制品，按版本与构建脚本摘要缓存固定上游运行时。缓存恢复后仍校验版本、架构、完整构建标签和 SHA-256。Compose 启动面板与 PostgreSQL 后，在干净宿主执行真实安装脚本，并由 systemd 管理 Agent 与独立运行时。

本地客户端经 Reality 向回环夹具下载 2 MiB、上传 1 MiB，检查文件内容、上传响应与真实用户节点用量增量。暂停后连续 70 秒采样稳定、outbox 清空，再验证 Agent 重启不更换运行时 PID、运行时重载不重复入账；恢复流量和同版本重新安装也核对身份与用量连续性。端到端流程及私有 state 驱动见 [真实验收文档](e2e.md)。

CI 公开制品仅包含版本、阶段和精确用量摘要；完整配置、订阅凭据、设备身份、安装令牌和日志不上传。此 job 使用回环 HTTP 路径和本地 TLS 伪装目标，不覆盖外部 CDN、DNS、证书部署或云防火墙配置。

新 job 的实际执行结果见对应提交的 [Actions](https://github.com/theLucius7/sinan/actions)，验收范围只在该 job 通过后成立。运行 0.1.0 到 0.2.0 升级属于真机专项，不能由同版本重装测试推断。
