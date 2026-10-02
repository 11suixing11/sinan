import { time } from '../format'
import { authorizationState, familyLabel, latency, loss, lossLabel, networkLabel, probeSlots, probeState, probeTone, probeValue } from '../probes'
import type { ProbeField, ProbeOverview, ProbeResult } from '../probes'

function Quality({ label, field, latest, slots, live, compact }: { label: string; field: ProbeField; latest?: ProbeResult; slots: (ProbeResult | undefined)[]; live: boolean; compact?: boolean }) {
  const format = field === 'latency_ms' ? latency : loss
  const current = live ? probeValue(latest, field) : null
  return <div className="d-quality-panel"><div><span>{label}</span><strong className={`d-quality-value d-quality-${probeTone(current, field)}`}>{format(current)}</strong></div>{!compact && <div className="d-quality-bars" aria-label={`${label}近期采样，空缺为灰色`}>{slots.map((point, index) => {
    const value = probeValue(point, field)
    const title = point ? `${time(point.sampled_at / 1000)} · ${point.error != null && point.attempts !== 4 ? '检测不可用' : `${label} ${format(value)}${point.error ? ` · ${point.error}` : ''}`}` : '此时段无采样'
    return <span key={index} className={`d-quality-bar d-quality-${probeTone(value, field)}`} title={title} />
  })}</div>}</div>
}

export default function ProbeQuality({ probes, now, unavailable, loading, online, compact = false }: { probes?: ProbeOverview[]; now: number; unavailable: boolean; loading: boolean; online: boolean; compact?: boolean }) {
  const enabled = probes?.filter(item => item.probe.enabled).sort((a, b) => a.probe.name.localeCompare(b.probe.name, 'zh-CN')) ?? []
  if (!enabled.length) return <div className="d-quality-placeholder"><span>延迟 / 丢包</span><span>{unavailable ? '拨测读取失败' : loading ? '正在读取…' : probes?.length ? probes.some(item => authorizationState(item.probe, now)) ? '目标未授权或已到期' : '拨测已暂停' : '尚未配置拨测'}</span></div>
  const rows = enabled.slice(0, compact ? 1 : 3).map(({ probe, results }) => {
    const latest = results.filter(point => point.sampled_at <= now).reduce<ProbeResult | undefined>((last, point) => !last || point.sampled_at > last.sampled_at ? point : last, undefined)
    const state = probeState(probe, latest, now, unavailable || !online)
    const label = [probe.monitor?.network && probe.monitor.network !== 'other' ? `${networkLabel(probe)} · ${probe.monitor.region}` : '', probe.carrier, probe.name].filter(Boolean).join(' · ')
    const title = `${label} · ${probe.target}${probe.port ? `:${probe.port}` : ''} · ${familyLabel(probe, latest)} · ${state}`
    return { probe, latest, state, label, title, slots: probeSlots(results, probe, now) }
  })
  if (compact) return <div className="d-quality-list d-quality-compact">{rows.map(({ probe, latest, state, label, title, slots }) => <div className="d-quality-row" key={probe.id}>
    <div className="d-quality-heading"><span title={title}>{label}</span><small>{state === '最近采样' ? probe.kind === 'icmp' ? 'ICMP' : 'TCP' : state}</small></div>
    <div className="d-quality-grid"><Quality label="延迟" field="latency_ms" compact latest={latest} slots={slots} live={state === '最近采样'} /><Quality label={lossLabel(probe)} field="loss_percent" compact latest={latest} slots={slots} live={state === '最近采样'} /></div>
  </div>)}</div>
  const lossHeading = rows.every(row => row.probe.kind === 'icmp') ? '丢包' : rows.every(row => row.probe.kind === 'tcp') ? '连接失败率' : '丢包 / 连接失败率'
  return <div className="d-quality-list">
    <div className="d-quality-grid">{(['latency_ms', 'loss_percent'] as const).map(field => <div className="d-quality-panel d-quality-carriers" key={field}>
      <div>{field === 'latency_ms' ? '延迟' : lossHeading}</div>
      {rows.map(({ probe, latest, state, label, title, slots }) => {
        const format = field === 'latency_ms' ? latency : loss
        const value = state === '最近采样' ? probeValue(latest, field) : null
        return <div className="d-carrier-quality" key={probe.id}>
          <div className="d-carrier-reading" title={`${title} · ${field === 'latency_ms' ? '延迟' : lossLabel(probe)}`}><span>{label}</span><strong className={`d-quality-value d-quality-${probeTone(value, field)}`}>{format(value)}</strong></div>
          <div className="d-quality-bars" aria-label={`${label}近期${field === 'latency_ms' ? '延迟' : lossLabel(probe)}`}>{slots.map((point, index) => {
            const value = probeValue(point, field)
            const reading = point ? `${time(point.sampled_at / 1000)} · ${point.error != null && point.attempts !== 4 ? '检测不可用' : `${format(value)}${point.error ? ` · ${point.error}` : ''}`}` : '此时段无采样'
            return <span key={index} className={`d-quality-bar d-quality-${probeTone(value, field)}`} title={reading} />
          })}</div>
          {state !== '最近采样' && <small className="d-quality-state">{state}</small>}
        </div>
      })}
    </div>)}</div>
    {enabled.length > 3 && <small className="d-muted">另有 {enabled.length - 3} 个目标</small>}
  </div>
}
