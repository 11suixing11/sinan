export type PolicyGroup = { id: number; name: string; node_ids: number[]; chain_ids: number[]; member_count: number }
export type PackageGroup = { id: number; name: string; monthly_bytes: string | null; reset_day: number; reset_hour: number; reset_minute: number; timezone: string; duration_days: number }
export type Chain = { id: number; name: string; entry_node_id: number; exit_node_id: number; available: boolean }
export type ProxyResourceKind = 'direct' | 'chain'
export type ProxyResourceFilter = 'all' | 'direct' | 'chains'
export type ResourceEndpoint = {
  id: number; name: string; server_id: number; server_name: string; protocol: string; port: number; public_port: number;
  public_host: string; sni: string; enabled: boolean; node_deleted: boolean; server_deleted: boolean;
  plugin_enabled: boolean; online: boolean; desired_revision: number | null; applied_revision: number | null;
  applied_observed_at: number | null;
}
export type ProxyResource = {
  kind: ProxyResourceKind; id: number; name: string; entry: ResourceEndpoint; exit: ResourceEndpoint | null;
  available: boolean; unavailable_reasons: string[]; policy_group_ids: number[]; user_count: number;
  chain_refs: { id: number; name: string; role: 'exit' }[];
}
export type ResourceSnapshot<T> = { data?: T; fresh: boolean; error: string }
export function validatedSnapshot<T>(resource: ResourceSnapshot<unknown>, valid: (value: unknown) => value is T, previous?: T): ResourceSnapshot<T> {
  let data = previous
  let accepted = false
  if (valid(resource.data)) { data = resource.data; accepted = true }
  return { data, fresh: accepted && resource.fresh && !resource.error,
    error: resource.error || (resource.data !== undefined && !accepted ? '面板返回的资源信息格式不完整，请刷新确认。' : '') }
}
const object = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)
const integer = (value: unknown, minimum = 0): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= minimum
const nullableInteger = (value: unknown) => value === null || integer(value)

export function proxyResourceKey(resource: Pick<ProxyResource, 'kind' | 'id'>) { return `${resource.kind}:${resource.id}` }
export function validResourceEndpoint(value: unknown): value is ResourceEndpoint {
  return object(value) && integer(value.id, 1) && integer(value.server_id, 1)
    && ['name', 'server_name', 'protocol', 'public_host', 'sni'].every(key => typeof value[key] === 'string')
    && integer(value.port, 1) && value.port <= 65535 && integer(value.public_port, 1) && value.public_port <= 65535
    && ['enabled', 'node_deleted', 'server_deleted', 'plugin_enabled', 'online'].every(key => typeof value[key] === 'boolean')
    && ['desired_revision', 'applied_revision', 'applied_observed_at'].every(key => nullableInteger(value[key]))
}
export function validProxyResource(value: unknown): value is ProxyResource {
  return object(value) && (value.kind === 'direct' || value.kind === 'chain') && integer(value.id, 1)
    && typeof value.name === 'string' && validResourceEndpoint(value.entry)
    && (value.kind === 'direct' ? value.exit === null && value.entry.id === value.id : validResourceEndpoint(value.exit))
    && typeof value.available === 'boolean' && Array.isArray(value.unavailable_reasons) && value.unavailable_reasons.every(reason => typeof reason === 'string')
    && Array.isArray(value.policy_group_ids) && value.policy_group_ids.every(id => integer(id, 1)) && integer(value.user_count)
    && Array.isArray(value.chain_refs) && value.chain_refs.every(ref => object(ref) && integer(ref.id, 1) && typeof ref.name === 'string' && ref.role === 'exit')
}
export function validProxyResources(value: unknown): value is ProxyResource[] {
  return Array.isArray(value) && value.every(validProxyResource) && new Set(value.map(proxyResourceKey)).size === value.length
}
export function filterProxyResources(resources: ProxyResource[], kind: ProxyResourceFilter, serverId?: number) {
  return resources.filter(resource => (kind === 'all' || resource.kind === (kind === 'chains' ? 'chain' : 'direct'))
    && (serverId === undefined || resource.entry.server_id === serverId || resource.exit?.server_id === serverId))
}
export function proxyResourceCounts(resources: ProxyResource[]) {
  return { total: resources.length, direct: resources.filter(resource => resource.kind === 'direct').length,
    chains: resources.filter(resource => resource.kind === 'chain').length,
    endpoints: new Set(resources.flatMap(resource => [resource.entry, ...(resource.exit ? [resource.exit] : [])]).filter(endpoint => !endpoint.node_deleted && !endpoint.server_deleted).map(endpoint => endpoint.id)).size }
}
export type UserPolicies = { group_ids: number[] }
export type Entitlement = {
  user_id: number; package_group_id: number | null; package_name: string | null; monthly_bytes: string | null;
  reset_day: number | null; reset_hour: number | null; reset_minute: number | null; timezone: string | null;
  starts_at: number | null; expires_at: number | null; cycle_start: number | null; next_reset: number | null;
  used_bytes: string; status: 'unmetered' | 'not_started' | 'active' | 'expired' | 'exhausted'; allowed: boolean;
}
export const statusText = { unmetered: '未分配套餐', not_started: '尚未生效', active: '使用中', expired: '已到期', exhausted: '本期流量已用完' }
export const scheduleText = (p: Pick<PackageGroup, 'reset_day' | 'reset_hour' | 'reset_minute' | 'timezone'>) => `每月 ${p.reset_day} 日 ${String(p.reset_hour).padStart(2, '0')}:${String(p.reset_minute).padStart(2, '0')}（${p.timezone}）`
export function dateText(seconds: number | null, zone?: string | null): string {
  if (seconds === null) return '—'
  const date = new Date(seconds * 1000)
  if (!Number.isFinite(date.getTime())) return '时间不可用'
  try { return date.toLocaleString('zh-CN', { hour12: false, timeZone: zone || undefined }) }
  catch { return `${date.toLocaleString('zh-CN', { hour12: false, timeZone: 'UTC' })}（UTC；浏览器不支持套餐时区）` }
}

export function quotaBytes(amount: string, unit: string): string | null {
  if (!amount.trim()) return null
  if (!/^[1-9][0-9]*$/.test(amount) || !['B', 'GiB'].includes(unit)) throw new Error('每月流量需为正整数，留空表示不限量。')
  const value = BigInt(amount) * (unit === 'GiB' ? 1073741824n : 1n)
  if (value > 18446744073709551615n) throw new Error('每月流量超出支持范围。')
  return value.toString()
}

export function assignmentRequestId(): string {
  const value = crypto.getRandomValues(new Uint8Array(16))
  value[6] = (value[6] & 15) | 64
  value[8] = (value[8] & 63) | 128
  const hex = Array.from(value, byte => byte.toString(16).padStart(2, '0')).join('')
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}
