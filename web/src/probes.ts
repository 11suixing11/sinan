export type ProbeAuthorization = { region: string; source: string; scope: 'owned' | 'third_party'; evidence: string; expires_at: number | null }
export type ProbeAuthorizationDraft = { region: string; source: string; scope: '' | ProbeAuthorization['scope']; evidence: string; expires: string }
export type ProbeSpec = { id: string; name: string; kind: 'tcp' | 'icmp'; target: string; port: number | null; interval_secs: number; carrier: string; enabled: boolean }
export type Probe = ProbeSpec & { task_id?: string; revision?: number; authorization?: ProbeAuthorization | null; authorization_state?: 'allowed' | 'missing' | 'expired' }
export type LatencyTask = { id: string; spec: ProbeSpec; authorization?: ProbeAuthorization | null; default_enabled: boolean; server_ids: number[]; revision: number }
export type ProbeResource<T> = { data?: T[]; fresh: boolean; error: string }
export type ProbeWriteSnapshot = { serverId: number; probes: ProbeResource<Probe> }
export type TaskWriteSnapshot = { tasks: ProbeResource<LatencyTask>; servers: ProbeResource<{ id: number }> }
export type ProbeResult = { id: string; probe_id: string; sampled_at: number; latency_ms: number | null; loss_percent: number; error: string | null }
export type ProbeOverview = { server_id: number; probe: Probe; results: ProbeResult[]; authorization_state?: Probe['authorization_state'] }
export type ProbeField = 'latency_ms' | 'loss_percent'

export const probeLeaseNotice = '新 Agent 仅在有效短期许可内检测；断连或刷新失败后停止续期，许可最长 90 秒。保存、暂停或删除配置不代表设备已经立即停止。'
const controls = /[\u0000-\u001f\u007f-\u009f]/
const utf8 = (value: string) => new TextEncoder().encode(value).length
const unixNow = () => Math.floor(Date.now() / 1000)

function localDate(value: number) {
  const date = new Date(value * 1000)
  if (!Number.isFinite(date.getTime())) return 'invalid'
  const pad = (part: number) => String(part).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

export function authorizationDraft(value?: ProbeAuthorization | null): ProbeAuthorizationDraft {
  return { region: value?.region ?? '', source: value?.source ?? '', scope: value?.scope ?? '', evidence: value?.evidence ?? '', expires: value?.expires_at == null ? '' : localDate(value.expires_at) }
}

export function authorizationError(value: ProbeAuthorization | null | undefined, requireAllowed = true, now = unixNow()): string {
  if (value == null) return requireAllowed ? '目标授权待确认，请登记使用依据与同意或管理记录。' : ''
  for (const [key, limit, required] of [['region', 64, false], ['source', 256, true], ['evidence', 512, true]] as const) {
    const text = value[key]
    if (typeof text !== 'string' || controls.test(text) || utf8(text) > limit || (required && !text.trim())) return `授权${key === 'region' ? '地区' : key === 'source' ? '目标来源' : '同意或管理记录'}无效，请检查字节上限与内容。`
  }
  if (!['owned', 'third_party'].includes(value.scope)) return '请明确选择自有目标或第三方已获同意。'
  if (value.expires_at !== null && (!Number.isSafeInteger(value.expires_at) || value.expires_at <= 0)) return '授权截止时间无效。'
  if (requireAllowed && value.expires_at !== null && value.expires_at <= now) return '目标授权已过期，请重新登记有效的同意或管理记录。'
  return ''
}

export function authorizationPayload(draft: ProbeAuthorizationDraft, enabled: boolean, now = unixNow()): ProbeAuthorization | null {
  if (!draft.region && !draft.source && !draft.scope && !draft.evidence && !draft.expires && !enabled) return null
  const expires_at = draft.expires ? new Date(draft.expires).getTime() / 1000 : null
  const value = { region: draft.region, source: draft.source, scope: draft.scope, evidence: draft.evidence, expires_at } as ProbeAuthorization
  const error = authorizationError(value, enabled, now)
  if (error) throw new Error(error)
  return { ...value, region: value.region.trim(), source: value.source.trim(), evidence: value.evidence.trim() }
}

export function probeAuthorizationState(probe: Pick<Probe, 'authorization' | 'authorization_state'>, now = unixNow()) {
  if (Object.hasOwn(probe, 'authorization')) {
    if (probe.authorization?.expires_at != null && probe.authorization.expires_at <= now) return 'expired'
    return probe.authorization && !authorizationError(probe.authorization, true, now) ? 'allowed' : 'missing'
  }
  return ['allowed', 'expired'].includes(probe.authorization_state ?? '') ? probe.authorization_state! : 'missing'
}

export function overviewProbe(entry: ProbeOverview): Probe {
  if (Object.hasOwn(entry.probe, 'authorization') || !Object.hasOwn(entry, 'authorization_state')) return entry.probe
  return { ...entry.probe, authorization_state: entry.authorization_state }
}

export function probeSpec(probe: ProbeSpec): ProbeSpec {
  return { id: probe.id, name: probe.name.trim(), kind: probe.kind, target: probe.target.trim(), port: probe.kind === 'tcp' ? probe.port : null, interval_secs: probe.interval_secs, carrier: probe.carrier.trim(), enabled: probe.enabled }
}

export function probeSpecError(probe: ProbeSpec) {
  if (!probe.name.trim() || controls.test(probe.name) || utf8(probe.name) > 128) return '拨测名称无效。'
  if (!['tcp', 'icmp'].includes(probe.kind) || !probe.target.trim() || !/^[A-Za-z0-9._:-]+$/.test(probe.target.trim()) || utf8(probe.target) > 253) return '拨测目标必须为主机名或 IP 地址。'
  if (!Number.isSafeInteger(probe.interval_secs) || probe.interval_secs < 10 || probe.interval_secs > 3600) return '拨测间隔应为 10–3600 秒的整数。'
  if (probe.kind === 'tcp' && (!Number.isSafeInteger(probe.port) || probe.port! < 1 || probe.port! > 65535)) return '拨测端口应为 1–65535 的整数。'
  if (controls.test(probe.carrier) || utf8(probe.carrier) > 64) return '线路备注无效。'
  if (typeof probe.enabled !== 'boolean') return '拨测启用状态无效。'
  return ''
}

export function probeReadError(...resources: ProbeResource<unknown>[]) {
  const error = resources.find(resource => resource.error)?.error
  if (error) return `最新信息读取失败，暂不能修改；草稿已保留。${error}`
  return resources.some(resource => !resource.fresh || !Array.isArray(resource.data)) ? '正在刷新相关信息，暂不能修改；草稿已保留。' : ''
}

export function initialProbePayloads(drafts: { spec: ProbeSpec; authorization: ProbeAuthorizationDraft }[], now = unixNow()) {
  if (drafts.length > 32) throw new Error('初始拨测目标最多 32 个。')
  return drafts.map(draft => {
    const error = probeSpecError(draft.spec)
    if (error) throw new Error(error)
    return { ...probeSpec(draft.spec), authorization: authorizationPayload(draft.authorization, draft.spec.enabled, now) }
  })
}

function requireRevision(value: number | undefined) {
  if (!Number.isSafeInteger(value) || value! < 1) throw new Error('拨测版本未知，请刷新后重新编辑。')
  return value!
}

export function currentServerProbe(snapshot: ProbeWriteSnapshot, serverId: number, original: Probe) {
  const error = probeReadError(snapshot.probes)
  if (error) throw new Error(error)
  if (snapshot.serverId !== serverId) throw new Error('目标服务器已变化，请重新打开配置。')
  const current = snapshot.probes.data!.find(item => item.id === original.id)
  if (!current || current.task_id || original.task_id || requireRevision(current.revision) !== requireRevision(original.revision) || JSON.stringify(probeSpec(current)) !== JSON.stringify(probeSpec(original))) throw new Error('拨测目标或版本已变化，请取消后重新编辑；草稿已保留。')
  return current
}

export async function saveServerProbe<T>(snapshot: ProbeWriteSnapshot, serverId: number, original: Probe | null, value: ProbeSpec, authorization: ProbeAuthorization | null, writer: (body: ProbeSpec & { authorization: ProbeAuthorization | null; revision?: number }) => Promise<T>) {
  const error = probeReadError(snapshot.probes) || probeSpecError(value) || authorizationError(authorization, value.enabled)
  if (error) throw new Error(error)
  if (snapshot.serverId !== serverId) throw new Error('目标服务器已变化，请重新打开配置。')
  const current = original ? currentServerProbe(snapshot, serverId, original) : null
  if (!current && snapshot.probes.data!.length >= 32) throw new Error('当前服务器的拨测目标已达上限。')
  if (current && (value.id !== current.id || value.kind !== current.kind || value.target.trim() !== current.target || (value.kind === 'tcp' ? value.port : null) !== current.port)) throw new Error('检测方式、目标和端口不可更换，请新建拨测。')
  return writer({ ...probeSpec(value), authorization, ...(current ? { revision: current.revision } : {}) })
}

export async function deleteServerProbe<T>(snapshot: ProbeWriteSnapshot, serverId: number, original: Probe, writer: (id: string, body: { revision: number }) => Promise<T>) {
  const current = currentServerProbe(snapshot, serverId, original)
  return writer(current.id, { revision: requireRevision(current.revision) })
}

function currentTask(snapshot: TaskWriteSnapshot, original: LatencyTask) {
  const error = probeReadError(snapshot.tasks, snapshot.servers)
  if (error) throw new Error(error)
  const current = snapshot.tasks.data!.find(item => item.id === original.id)
  if (!current || requireRevision(current.revision) !== requireRevision(original.revision) || JSON.stringify(probeSpec(current.spec)) !== JSON.stringify(probeSpec(original.spec))) throw new Error('延迟任务目标或版本已变化，请关闭后重新编辑；草稿已保留。')
  return current
}

export async function saveLatencyTask<T>(snapshot: TaskWriteSnapshot, original: LatencyTask | null, value: LatencyTask, authorization: ProbeAuthorization | null, writer: (body: { spec: ProbeSpec; authorization: ProbeAuthorization | null; default_enabled: boolean; server_ids: number[]; revision?: number }) => Promise<T>) {
  const error = probeReadError(snapshot.tasks, snapshot.servers) || probeSpecError(value.spec) || authorizationError(authorization, value.spec.enabled)
  if (error) throw new Error(error)
  const current = original ? currentTask(snapshot, original) : null
  if (!current && snapshot.tasks.data!.length >= 32) throw new Error('统一延迟任务已达上限。')
  if (current && (value.id !== current.id || value.spec.id !== current.spec.id || value.spec.kind !== current.spec.kind || value.spec.target.trim() !== current.spec.target || (value.spec.kind === 'tcp' ? value.spec.port : null) !== current.spec.port)) throw new Error('检测方式、目标和端口不可更换，请新建任务。')
  if (value.server_ids.some(id => !Number.isSafeInteger(id) || id < 1 || !snapshot.servers.data!.some(server => server.id === id)) || new Set(value.server_ids).size !== value.server_ids.length) throw new Error('选中的服务器已不可用，请重新核对分配；草稿已保留。')
  return writer({ spec: probeSpec(value.spec), authorization, default_enabled: value.default_enabled, server_ids: [...value.server_ids], ...(current ? { revision: current.revision } : {}) })
}

export function latencyDraftError(snapshot: TaskWriteSnapshot, original: LatencyTask | null, serverIds: number[]) {
  const error = probeReadError(snapshot.tasks, snapshot.servers)
  if (error) return error
  try { if (original) currentTask(snapshot, original) } catch (error) { return (error as Error).message }
  return serverIds.some(id => !snapshot.servers.data!.some(server => server.id === id)) ? '选中的服务器已不可用，请重新核对分配；草稿已保留。' : ''
}

export async function deleteLatencyTask<T>(snapshot: TaskWriteSnapshot, original: LatencyTask, writer: (id: string, body: { revision: number }) => Promise<T>) {
  const current = currentTask(snapshot, original)
  return writer(current.id, { revision: requireRevision(current.revision) })
}

export const lossLabel = (probe: Pick<Probe, 'kind'>) => probe.kind === 'tcp' ? '连接失败率' : '丢包率'
export const latency = (value: number | null) => value === null ? '—' : `${value.toFixed(1)} ms`
export const loss = (value: number | null) => value === null ? '—' : `${value.toFixed(1)}%`

// Legacy Agents also encode an unavailable measurement with an error and a numeric loss placeholder.
export function probeValue(result: ProbeResult | undefined, field: ProbeField): number | null {
  if (!result || result.error != null) return null
  const value = result[field]
  return typeof value === 'number' && Number.isFinite(value) && value >= 0 && value <= (field === 'loss_percent' ? 100 : 60_000) ? value : null
}

export function probeState(probe: Probe, latest: ProbeResult | undefined, now: number, unavailable = false): string {
  if (unavailable) return '状态未知'
  if (!probe.enabled) return '已暂停'
  const authorization = probeAuthorizationState(probe, Math.floor(now / 1000))
  if (authorization === 'expired') return '目标授权已过期'
  if (authorization !== 'allowed') return '目标授权待确认'
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
