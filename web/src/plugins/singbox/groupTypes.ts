export type PolicyGroup = { id: number; name: string; node_ids: number[]; chain_ids: number[]; member_count: number }
export type PackageGroup = { id: number; name: string; monthly_bytes: string | null; reset_day: number; reset_hour: number; reset_minute: number; timezone: string; duration_days: number }
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
