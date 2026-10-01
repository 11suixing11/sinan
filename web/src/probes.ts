export type ProbeIdentity = { kind: 'tcp' | 'icmp'; target: string; port: number | null; address_family: 'any' | 'ipv4' | 'ipv6' }
export type ProbeAuthorization = { kind: 'owned' | 'consent'; source: string; scope: string; enabled: boolean; expires_at: number | null; identity: ProbeIdentity }
export type ProbeMonitor = { region: string; address_family: ProbeIdentity['address_family']; authorization: ProbeAuthorization | null }
export type Probe = { task_id?: string; id: string; name: string; kind: 'tcp' | 'icmp'; target: string; port: number | null; interval_secs: number; carrier: string; enabled: boolean; monitor?: ProbeMonitor | null; execution_authorized?: boolean | null }
export type ProbeResult = { id: string; probe_id: string; sampled_at: number; latency_ms: number | null; loss_percent: number; error: string | null; address_family?: 'ipv4' | 'ipv6' | null }
export type ProbeOverview = { server_id: number; probe: Probe; results: ProbeResult[] }
export type ProbeField = 'latency_ms' | 'loss_percent'

export const lossLabel = (probe: Pick<Probe, 'kind'>) => probe.kind === 'tcp' ? '连接失败率' : '丢包率'
export const latency = (value: number | null) => value === null ? '—' : `${value.toFixed(1)} ms`
export const loss = (value: number | null) => value === null ? '—' : `${value.toFixed(1)}%`
export const carrierLabel = (value: string) => (({ telecom: '电信', unicom: '联通', mobile: '移动' } as Record<string, string>)[value] ?? value) || '线路未知'
export const familyLabel = (value?: ProbeIdentity['address_family']) => ({ any: '自动家族', ipv4: 'IPv4', ipv6: 'IPv6' }[value ?? 'any'])

export const probeIdentity = (probe: Probe): ProbeIdentity => ({ kind: probe.kind, target: probe.target.trim(), port: probe.port, address_family: probe.monitor?.address_family ?? 'any' })
const sameIdentity = (left: ProbeIdentity, right: ProbeIdentity) => left.kind === right.kind && left.target === right.target && left.port === right.port && left.address_family === right.address_family
export const authorizationMatches = (probe: Probe) => Boolean(probe.monitor?.authorization && sameIdentity(probe.monitor.authorization.identity, probeIdentity(probe)))

export function changeProbe(probe: Probe, part: Partial<Probe>): Probe {
  const changed = { ...probe, ...part }
  if (sameIdentity(probeIdentity(probe), probeIdentity(changed)) || !changed.monitor?.authorization) return changed
  return { ...changed, monitor: { ...changed.monitor, authorization: { ...changed.monitor.authorization, enabled: false } } }
}

export function bindProbeAuthorization(probe: Probe): Probe {
  const { execution_authorized: _derived, ...payload } = probe
  const monitor = payload.monitor
  return monitor?.authorization ? { ...payload, monitor: { ...monitor, authorization: { ...monitor.authorization,
    enabled: monitor.authorization.enabled && authorizationMatches(probe) } } } : payload
}

export function authorizationState(probe: Probe, now = Date.now()): string | null {
  const authorization = probe.monitor?.authorization
  if (authorization && !authorizationMatches(probe)) return '当前目标需重新确认授权'
  if (authorization?.expires_at != null && authorization.expires_at * 1000 <= now) return '授权已过期'
  if (authorization && !authorization.enabled) return '授权已撤销'
  if (probe.execution_authorized === false || probe.monitor === null) return '未取得执行授权'
  return null
}

// Legacy Agents also encode an unavailable measurement with an error and a numeric loss placeholder.
export function probeValue(result: ProbeResult | undefined, field: ProbeField): number | null {
  if (!result || result.error != null) return null
  const value = result[field]
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= (field === 'loss_percent' ? 100 : 60_000) ? value : null
}

export function probeState(probe: Probe, latest: ProbeResult | undefined, now: number, unavailable = false): string {
  if (unavailable) return '状态未知'
  if (!probe.enabled) return '已暂停'
  const authorization = authorizationState(probe, now)
  if (authorization) return authorization
  if (!latest) return '等待采样'
  if (latest.sampled_at > now || now - latest.sampled_at > Math.max(60_000, probe.interval_secs * 3000)) return '采样已过期'
  if (latest.error != null) return '检测不可用'
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
