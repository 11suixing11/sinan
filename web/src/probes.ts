export type ProbeNetwork = 'other' | 'telecom' | 'unicom' | 'mobile'
export type ProbeMonitoring = { network: ProbeNetwork; region: string; ip_version: 'auto' | 'ipv4' | 'ipv6'; authorization: { basis: 'unconfirmed' | 'owned' | 'permission'; confirmed: boolean; source: string; scope: string; expires_at: number | null } }
export type ProbeIdentity = { kind: 'tcp' | 'icmp'; target: string; port: number | null; address_family: 'any' | 'ipv4' | 'ipv6' }
export type ProbeAuthorization = { kind: 'owned' | 'consent'; source: string; scope: string; enabled: boolean; expires_at: number | null; identity: ProbeIdentity }
export type ProbeMonitor = { network?: ProbeNetwork; region: string; address_family: ProbeIdentity['address_family']; authorization: ProbeAuthorization | null }
export type Probe = { revision?: number | null; task_id?: string; id: string; name: string; kind: 'tcp' | 'icmp'; target: string; port: number | null; interval_secs: number; carrier: string; enabled: boolean; monitor?: ProbeMonitor | null; execution_authorized?: boolean | null }
export type ProbeResult = { id: string; probe_id: string; sampled_at: number; latency_ms: number | null; loss_percent: number; error: string | null; address_family?: 'ipv4' | 'ipv6' | null; attempts?: 4 }
export type ProbeOverview = { server_id: number; probe: Probe; results: ProbeResult[] }
export type ProbeField = 'latency_ms' | 'loss_percent'

export const lossLabel = (probe: Pick<Probe, 'kind'>) => probe.kind === 'tcp' ? '连接失败率' : '丢包率'
export const latency = (value: number | null) => value === null ? '—' : `${value.toFixed(1)} ms`
export const loss = (value: number | null) => value === null ? '—' : `${value.toFixed(1)}%`
export const carrierLabel = (value: string) => (({ telecom: '电信', unicom: '联通', mobile: '移动' } as Record<string, string>)[value] ?? value) || '线路未知'
export const familyLabel = (probe: Probe, result?: ProbeResult) => result?.address_family ? (result.address_family === 'ipv4' ? 'IPv4' : 'IPv6') : probe.monitor?.address_family === 'ipv4' ? 'IPv4（配置）' : probe.monitor?.address_family === 'ipv6' ? 'IPv6（配置）' : '自动（实际版本未知）'
export const networks: [ProbeNetwork, string][] = [['telecom', '电信'], ['unicom', '联通'], ['mobile', '移动'], ['other', '其他线路']]
export const emptyMonitoring = (): ProbeMonitoring => ({ network: 'other', region: '', ip_version: 'auto', authorization: { basis: 'unconfirmed', confirmed: false, source: '', scope: '', expires_at: null } })
export const networkLabel = (probe: Probe) => networks.find(([key]) => key === (probe.monitor?.network ?? probe.carrier))?.[1] ?? '其他线路'
export function monitoringOf(probe: Probe): ProbeMonitoring {
  const monitor = probe.monitor, authorization = monitor?.authorization
  return { network: monitor?.network ?? 'other', region: monitor?.region ?? '', ip_version: monitor?.address_family === 'any' || !monitor ? 'auto' : monitor.address_family,
    authorization: { basis: authorization ? authorization.kind === 'owned' ? 'owned' : 'permission' : 'unconfirmed', confirmed: Boolean(authorization?.enabled && authorizationMatches(probe)), source: authorization?.source ?? '', scope: authorization?.scope ?? '', expires_at: authorization?.expires_at ?? null } }
}
export function withMonitoring(probe: Probe, value: ProbeMonitoring): Probe {
  const address_family = value.ip_version === 'auto' ? 'any' : value.ip_version, authorization = value.authorization
  return { ...probe, monitor: { network: value.network, region: value.region, address_family,
    authorization: authorization.basis === 'unconfirmed' ? null : { kind: authorization.basis === 'owned' ? 'owned' : 'consent', source: authorization.source, scope: authorization.scope, enabled: authorization.confirmed, expires_at: authorization.expires_at,
      identity: { kind: probe.kind, target: probe.target.trim(), port: probe.port, address_family } } } }
}

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
  const monitor = payload.monitor, authorization = monitor?.authorization
  if (!authorization) return { ...payload, enabled: false }
  if (!authorization.source.trim() || !authorization.scope.trim()) return { ...payload, enabled: false, monitor: { ...monitor!, authorization: null } }
  const confirmed = authorization.enabled && authorizationMatches(probe)
  return { ...payload, enabled: payload.enabled && confirmed, monitor: { ...monitor!, authorization: { ...authorization, enabled: confirmed } } }
}

export function authorizationState(probe: Probe, now = Date.now()): string | null {
  const authorization = probe.monitor?.authorization
  if (authorization && !authorizationMatches(probe)) return '当前目标需重新确认授权'
  if (authorization?.expires_at != null && authorization.expires_at * 1000 <= now) return '授权已过期'
  if (authorization && !authorization.enabled) return '授权已撤销'
  if (probe.execution_authorized === false || !authorization && probe.execution_authorized !== true) return '未取得执行授权'
  return null
}

// Legacy Agents also encode an unavailable measurement with an error and a numeric loss placeholder.
export function probeValue(result: ProbeResult | undefined, field: ProbeField): number | null {
  if (!result || result.error != null && result.attempts !== 4) return null
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

// A persisted revision is required for edits and deletion; unavailable/stale reads
// cannot supply execution authority or overwrite a newer administrator's draft.
export const probeRevisionMatches = (draft: { revision?: number | null }, current: { revision?: number | null } | undefined) => Boolean(current && Number.isSafeInteger(draft.revision) && (draft.revision ?? 0) > 0 && current.revision === draft.revision)

export function probeWriteError(probe: Probe, current: Probe | undefined): string {
  if (!probeRevisionMatches(probe, current))
    return '此拨测已不存在或已改变，请刷新后重新确认；当前草稿已保留。'
  return ''
}
