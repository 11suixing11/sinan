# 上游统计协议

`stats.proto` 原样复制自 SagerNet/sing-box 的 `v1.14.2` tag：

- 源码：<https://github.com/SagerNet/sing-box/blob/v1.14.2/experimental/v2rayapi/stats.proto>
- 提交：`af6e64c3b69e6132ebaee0e1a3d24e93903f6709`
- 上游许可证：<https://github.com/SagerNet/sing-box/blob/v1.14.2/LICENSE>

上游 `stats.go` 在初始化时把 gRPC 服务名改为 `v2ray.core.app.stats.command.StatsService`。本适配器保留原始 proto 包名，并在客户端调用中使用实际服务路径。
