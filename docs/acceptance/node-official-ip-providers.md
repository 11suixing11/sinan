# 节点正式 IP 接口与凭据边界（#24、#122）

r19 日常诊断加入独立节点正式接口；它不是完整 NodeQuality 或流媒体能力的签收。面板现有 check-place 七数据库仍明确标为同一聚合入口，面板 AbuseIPDB 凭据仍只从面板发出请求。节点新增的 Ipregistry 与 DB-IP 来源不会把这两类面板响应改名为节点出口。

## 节点本地配置

仅由节点操作者管理 `/etc/sinan/node-ip-providers.json`；面板、诊断任务参数、任务 JSON、命令行、下载 URL 不传送此文件或密钥。配置目录须为 root 所有且组/其他账户不可写，建议 `0700`；文件须 root 所有、普通单链接文件、权限精确 `0600`，不能为 symlink。读取时检查已打开目录与文件 FD，最多 8192 字节；重复字段、未知源及不合格式配置拒绝。

格式示例（占位值不能用于真实访问）：

```json
{
  "schema": 1,
  "providers": {
    "ipregistry": {
      "enabled": false,
      "operator_owned_credentials": true,
      "api_key": "YOUR_PRIVATE_OPERATOR_KEY"
    },
    "dbip": {
      "enabled": false,
      "operator_owned_credentials": true,
      "api_key": "YOUR_PRIVATE_PAID_OPERATOR_KEY"
    }
  }
}
```

默认不启用；只有操作者有权在此节点使用正式私有凭据，并将对应项设为 `enabled: true` 才进行查询。这里的声明不代表 Sinan 代用户接受服务协议，也不授予数据库或工具再分发权。DB-IP 仅调用带正式私有 key 的 HTTPS 服务，不使用 `free`、网页公开 key 或 HTTP 降级。每次日常诊断每个启用源按所选地址族各查询一次，可能消耗 API 额度；没有配置时逐源 `not_attempted`，不会尝试获取备用 key。

## 实际请求与结果

- [Ipregistry 官方 self 查询](https://www.ipregistry.co/docs/endpoints)的 `https://api.ipregistry.co/?key=...`；仅从执行诊断的节点调用，独立要求所选 IPv4/IPv6，白名单解析用途、组织、代理/Tor/VPN/滥用/威胁布尔字段。字段定义来自[官方响应文档](https://www.ipregistry.co/docs/responses/ip-address-fields)。
- [DB-IP v2 官方文档](https://db-ip.com/api/doc.php)规定 `https://api.db-ip.com/v2/{privateKey}/self` 查询调用者地址；保留国家/地区、用途、组织、威胁等级、代理/爬虫及 ASN 的有限合法字段。官方文档说明加密访问需相应正式服务；403/额度/套餐拒绝不会改用免费或公开材料。

每个来源与地址族由独立短期子进程执行，DNS、TLS、所有读取共用父进程最多 3 秒截止，整段来源查询最多 14 秒预算，超时后 terminate/kill/reap。只允许两个固定 HTTPS host，标准 TLS 验证，无重定向、重试、环境代理或浏览器伪装。原始响应最大 256 KiB；显式失败、地址非公网或地址族不符、错类型、非法字段、重复 JSON 字段和非有限 JSON 均拒绝。真正的 `false` 保留为否，空值/缺失不会补成零或成功。API 自查观察的是该次直连正式接口的节点出口，不保证其他目的地路由同出口。

密钥只在此进程内存及发给其对应官方服务的 TLS 请求中使用，不写日志/报告/任务参数。正式服务错误正文与 URL 不公开，甚至白名单文本字段反射密钥也拒绝。章节只保留来源固定 origin、`node_self`、地址族、本次状态与白名单结果；403/429/超时/传输/schema 错误逐源显示未知，无配置明确未尝试。各次历史诊断报告保持不可变，本次失败不覆盖旧成功报告，不把旧成功冒称本次新查询。

来源明细同时留在节点私有 `node-ip-sources.json` 中；共用 `DiagnosticSection` 只接受固定章节字段，节点明细文件不作为章节字段发送，也不会转成面板 IP 缓存。面板收到且历史保存的章节正文已经包含上述来源、地址族、状态与白名单字段，因此既有章节协议与旧任务 JSON 不变。

Disney+、YouTube Premium、ChatGPT 尚没有可交付的经授权节点认证适配，仍明确 `not_attempted`/未知；不会通过公共 cookies、网页临时 key、固定材料或自动更换 UA 恢复检查。新增 IP 数据不构成这些服务的解锁证据，#122 的媒体授权与专用节点验收仍待。

## 验证范围

`tools/test-nodequality-official-ip.py` 只使用私有 API 替身、保留示例地址和 `TEST_ONLY` 凭据，检查配置权限、重复字段、真实 bool、来源/地址族、403/429/redirect/超量/反射密钥、实际阻塞子进程截止与回收。IPv4/IPv6 夹具使用真实回环 HTTP 字节、socket、子进程与地址族筛选，仅替换固定正式主机的 DNS 和 TLS 包装，不认证第三方 TLS 或实际账户权限；不支持 IPv6 的宿主明确跳过该方法。配置示例与代码不会请求真实第三方接口。按本轮用户要求，源码全部完成后统一运行；本文件不预报已通过。

真实 IPv4/IPv6 出口、操作者实际 API 账户套餐、正式凭据权限、节点重启/断连/取消/持续代理负载仍须在独立授权环境签收。r2–r18 已保存结果与精确版本恢复保留，所有 full 新启动和旧排队门禁继续；不会把日常 API 夹具作为完整工具链授权或完整验机通过。
