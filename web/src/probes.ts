export type ProbeNetwork = 'other' | 'telecom' | 'unicom' | 'mobile'
export type ProbeMonitoring = { network: ProbeNetwork; region: string; ip_version: 'auto' | 'ipv4' | 'ipv6'; authorization: { basis: 'unconfirmed' | 'owned' | 'permission'; confirmed: boolean; source: string; scope: string; expires_at: number | null } }
export type Probe = { task_id?: string; id: string; name: string; kind: 'tcp' | 'icmp'; target: string; port: number | null; interval_secs: number; carrier: string; enabled: boolean; monitoring?: ProbeMonitoring }
export type ProbeResult = { id: string; probe_id: string; sampled_at: number; latency_ms: number | null; loss_percent: number; error: string | null; ip_version?: 4 | 6; attempts?: 4 }
export type ProbeOverview = { server_id: number; probe: Probe; results: ProbeResult[] }
export type ProbeField = 'latency_ms' | 'loss_percent'

export const lossLabel = (probe: Pick<Probe, 'kind'>) => probe.kind === 'tcp' ? '连接失败率' : '丢包率'
export const latency = (value: number | null) => value === null ? '—' : `${value.toFixed(1)} ms`
export const loss = (value: number | null) => value === null ? '—' : `${value.toFixed(1)}%`
export const networks: [ProbeNetwork, string][] = [['telecom', '电信'], ['unicom', '联通'], ['mobile', '移动'], ['other', '其他线路']]
export const emptyMonitoring = (): ProbeMonitoring => ({ network: 'other', region: '', ip_version: 'auto', authorization: { basis: 'unconfirmed', confirmed: false, source: '', scope: '', expires_at: null } })
export const networkLabel = (probe: Probe) => networks.find(([key]) => key === (probe.monitoring?.network ?? 'other'))?.[1] ?? '其他线路'
export const familyLabel = (probe: Probe, result?: ProbeResult) => result?.ip_version ? `IPv${result.ip_version}` : probe.monitoring?.ip_version === 'ipv4' ? 'IPv4（配置）' : probe.monitoring?.ip_version === 'ipv6' ? 'IPv6（配置）' : '自动（实际版本未知）'
export function authorizationState(probe: Probe, now = Date.now()): string | null {
  const authorization = probe.monitoring?.authorization
  if (!authorization?.confirmed || !['owned', 'permission'].includes(authorization.basis)) return '目标未授权'
  if (authorization.expires_at != null && authorization.expires_at * 1000 <= now) return '目标授权已到期'
  return null
}

// Legacy Agents also encode an unavailable measurement with an error and a numeric loss placeholder.
export function probeValue(result: ProbeResult | undefined, field: ProbeField): number | null {
  if (!result || (result.error != null && result.attempts !== 4)) return null
  const value = result[field]
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= (field === 'loss_percent' ? 100 : 60_000) ? value : null
}

export function probeState(probe: Probe, latest: ProbeResult | undefined, now: number, unavailable = false): string {
  if (unavailable) return '状态未知'
  const authorization = authorizationState(probe, now)
  if (authorization) return authorization
  if (!probe.enabled) return '已暂停'
  if (!latest) return '等待采样'
  if (latest.sampled_at > now || now - latest.sampled_at > Math.max(60_000, probe.interval_secs * 3000)) return '采样已过期'
  if (latest.error != null && latest.attempts !== 4) return '检测不可用'
  return '最近采样'
}

export function probeTone(value: number | null, field: ProbeField) {
  if (value === null) return 'empty'
  const thresholds = field === 'latency_ms' ? [60, 100, 160, 200] : [1, 3, 6, 9]
  return ['good', 'fair', 'warning', 'poor', 'danger'][thresholds.findIndex(threshold => value <= threshold)] ?? 'danger'
}

export function probeSlots(results: ProbeResult[], probe: Probe, now: number, count = 20) {
  const width = Math.max(10, probe.interval_secs) * 1000
  const end = Math.ceil(now / width) * width
  const slots: (ProbeResult | undefined)[] = Array(count).fill(undefined)
  for (const point of results) {
    if (point.probe_id !== probe.id || !Number.isFinite(point.sampled_at) || point.sampled_at > now) continue
    const index = Math.min(count - 1, Math.floor((point.sampled_at - (end - count * width)) / width))
    if (index < 0) continue
    if (!slots[index] || point.sampled_at > slots[index]!.sampled_at) slots[index] = point
  }
  return slots
}
