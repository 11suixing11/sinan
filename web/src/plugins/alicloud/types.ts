export type Target = { bandwidth_mbps: number; charge_type: 'PayByTraffic' | 'PayByBandwidth' }
export type Snapshot = Target & { kind: string; cloud_id: string; region: string; public_ip: string; resource_charge_type: string; status: string }
export type Bill = { month: string; queried_at: number; usage_micro_gb: number | null; rows: { instance_id: string; region: string; product_type: string; billing_item: string; usage: string; unit: string; amount: string; currency: string }[] }
export type Account = { id: string; name: string; site: 'china' | 'international'; enabled: boolean; auto_enabled: boolean; limit_gb: number; revision: number; next_run_at: number; error_code: string | null; bill: Bill | null; traffic_error: string | null; traffic: { queried_at: number; mainland_bytes: string; overseas_bytes: string; regions: { region: string; bytes: string }[] } | null }
export type Resource = { id: string; account_id: string; name: string; kind: 'ecs' | 'eip'; region: string; cloud_id: string; auto_enabled: boolean; cap_mbps: number; revision: number; snapshot: Snapshot | null; checked_at: number | null; error_code: string | null }
export type Operation = { id: string; resource_id: string; before_state: Snapshot; target: Target; source: string; billing_cycle: string | null; status: string; created_at: number; updated_at: number; expires_at: number; error_code: string | null; request_id: string | null }
export type Overview = { accounts: Account[]; resources: Resource[]; operations: Operation[] }
export const charge = (value: string) => value === 'PayByTraffic' ? '按流量计费' : value === 'PayByBandwidth' ? '按带宽计费' : '未知计费方式'
export const time = (value: number | null) => value ? new Date(value * 1000).toLocaleString('zh-CN', { hour12: false }) : '尚未查询'
const states: Record<string, string> = { preview: '等待确认', queued: '等待执行', running: '执行中', uncertain: '结果待核对', succeeded: '已核对完成', failed: '未执行', cancelled: '已取消', dismissed: '已人工结束跟踪' }
const messages: Record<string, string> = {
  authentication_failed: '访问密钥无效或缺少权限', rate_limited: '云服务限流，稍后重试', resource_not_found: '未找到指定地域和标识的资源',
  unsupported_resource: '仅支持 ECS 固定公网 IP 与按量付费的独立 EIP；不操作共享带宽包', resource_busy: '云资源正在变更或状态不支持调整',
  billing_incomplete: '账单数据不完整，自动控制暂停', state_changed: '云资源或本地配置已变化，请重新预览', policy_inactive: '策略已关闭或账单数据不可用于控制',
  request_timeout: '云接口超时，等待核对结果', network_error: '云接口连接失败', response_error: '云接口暂时不可用', invalid_response: '云接口响应与预期不符',
  provider_rejected: '云服务拒绝请求，请检查权限和资源限制', awaiting_confirmation: '尚未核对到目标状态，系统只读回结果，不重复提交',
}
export const status = (value: string) => Object.hasOwn(states, value) ? states[value] : '状态未知'
export const message = (value: string | null) => value ? Object.hasOwn(messages, value) ? messages[value] : '云接口暂时不可用' : ''
export function accountWrite(account: Pick<Account, 'name' | 'site' | 'enabled' | 'auto_enabled' | 'limit_gb'>, key: string, secret: string, revision?: number) {
  return { name: account.name.trim(), site: account.site, enabled: account.enabled, auto_enabled: account.auto_enabled, limit_gb: account.limit_gb, ...(revision === undefined ? {} : { revision }), ...(key.trim() || secret.trim() ? { access_key_id: key.trim(), access_key_secret: secret.trim() } : {}) }
}
export function billUsable(account: Account, now = Date.now() / 1000) {
  const month = new Date((now + 8 * 3600) * 1000).toISOString().slice(0, 7), bill = account.bill
  return Boolean(account.enabled && !account.error_code && bill && bill.month === month && bill.usage_micro_gb !== null && bill.queried_at <= now && bill.queried_at + 900 >= now)
}
