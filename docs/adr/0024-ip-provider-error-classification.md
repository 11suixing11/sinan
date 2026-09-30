# ADR 0024：IP 查询错误的类型与来源

- 状态：已确定。
- 对应任务：逐源分类错误，Issue #22。

## 问题

reqwest 的 `is_connect()` 同时覆盖 DNS、TCP 连接和 TLS。把所有连接错误写成 DNS 会误导排查；七种数据库响应都来自同一个 check-place 入口，也不能写成七个独立服务商。

## 决策

保留既有请求次数、超时、并发、响应字节上限和重定向策略。为每种数据库响应增加 provider、目标 IP、实际开始尝试的时间、耗时、错误类别和可选 HTTP 状态；provider 统一为 `check-place`，database 保留响应形状名称。保留旧 `error` 中文文本及旧 JSON payload 的反序列化兼容。

只在 IP 查询的 Client 中使用实现 reqwest DNS trait 的薄适配器，调用既有 Tokio `lookup_host` 的系统 getaddrinfo/threadpool 路径，把 DNS 失败封装成专用错误类型。遍历 reqwest 的 source 链时 downcast 该类型，准确识别 DNS；没有这项类型证据的连接错误保留为 connect。

面板增加 `rustls` 0.23 的直接依赖，禁用其默认 features，仅用于 downcast TLS 错误类型。它已经是 reqwest/sqlx 的锁定传递依赖，此次不新增网络后端、加密 provider 或传递包。io::Error 包装的 rustls 错误也检查其 get_ref。无法取得 TLS 类型证据的传输故障保留连接或未分类错误，不根据域名、请求 scheme 或错误文字猜测。

对 403、429、其他 HTTP 状态、超时、非 JSON、已知字段缺失、响应读取和大小限制分别记录类别。批次总超时结束时，记录已开始请求的实际耗时；尚未开始的请求标为 not_attempted，不能伪造逐源请求时间。缺字段保持未知，不推导零分或 false。

## 替代方案

按错误文字做通用模糊匹配容易把连接故障误认成 DNS/TLS。hyper-util 的具体 ConnectError 没有从公开模块导出，且 DNS 标签仍是私有字段；增加多项仅用于关联类型的依赖不比薄 DNS 适配器更清楚。通过第二次 DNS 查询或关闭 TLS 验证来排查会改变真实请求语义，本次不采用。

## 验证边界

使用受控 loopback HTTP/TCP 源和 reqwest 的可注入 resolver 验证所有类别，不依赖第三方服务临时可用。生产入口可用性和失败后保留历史成功结果属于独立验收与后续缓存 PR。
