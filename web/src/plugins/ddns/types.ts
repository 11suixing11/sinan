export type DdnsConfig = {
  name: string; server_id: number; zone_id: string; record_name: string; record_type: 'A' | 'AAAA'
  ttl: number; proxied: boolean; interval_secs: number; enabled: boolean; adopt_existing: boolean
}
export type DdnsRule = {
  id: string; config: DdnsConfig; revision: number; token_configured: boolean; busy: boolean; plugin_enabled: boolean
  server_name: string; candidate_ip: string | null; ip_status: string; ip_received_at: number | null
  last_ip: string | null; last_success_at: number | null; attempted_at: number | null; next_run_at: number
  status: string; error_code: string | null; failures: number
}
const messages: Record<string, string> = {
  plugin_disabled: '此服务器的 DDNS 插件未启用，保留现有解析',
  pending: '等待首次同步', running: '正在同步', updated: '已更新解析', unchanged: '解析一致', waiting: '等待有效地址', error: '同步失败',
  ready: '地址可用', no_public_ip: 'Agent 尚未上报此类型的公网地址', ip_stale: 'IP 报告已过期，等待 Agent 上报',
  server_offline: '服务器离线，保留现有解析', server_retired: '服务器已删除或退役，保留现有解析',
  authentication_failed: '凭据无效或权限不足，请检查 Token 与 Zone 的 DNS 编辑权限',
  zone_mismatch: '域名不属于指定 Zone', zone_inactive: 'Cloudflare Zone 尚未激活',
  record_conflict: 'DNS 记录存在冲突或重复，请先在 Cloudflare 整理', record_not_owned: '已有 DNS 记录，需勾选接管后才能更新',
  rate_limited: 'Cloudflare 请求限流，等待自动重试', provider_rejected: 'Cloudflare 拒绝请求，请检查记录配置与权限',
  resource_missing: 'Zone 或记录不存在，或 Token 无权访问', provider_unavailable: 'Cloudflare 暂时不可用',
  network_error: '连接 Cloudflare 失败，等待重试', request_timeout: '请求超时，下轮将重新核对远端记录',
  invalid_response: 'Cloudflare 返回了无法核对的响应', response_too_large: 'Cloudflare 响应超过大小上限',
  redirect_refused: 'Cloudflare 返回重定向，已停止请求', http_error: 'Cloudflare 请求失败',
  storage_error: '读取服务器状态失败', client_error: '暂时无法初始化同步服务', invalid_configuration: '规则配置无效',
}
export function ddnsMessage(code: string | null | undefined) { return code ? Object.hasOwn(messages, code) ? messages[code] : '状态未知，请稍后刷新' : '' }
export function ddnsWrite(config: DdnsConfig, token: string, revision?: number) {
  return { config: { ...config, ttl: config.proxied ? 1 : config.ttl }, ...(token.trim() ? { api_token: token.trim() } : {}), ...(revision === undefined ? {} : { revision }) }
}
