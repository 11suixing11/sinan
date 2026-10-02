import { time } from '../format'
import { latency, loss, lossLabel, overviewProbe, probeSlots, probeState, probeTone, probeValue } from '../probes'
import type { ProbeField, ProbeOverview, ProbeResult } from '../probes'

function Quality({ label, field, latest, slots, live, compact }: { label: string; field: ProbeField; latest?: ProbeResult; slots: (ProbeResult | undefined)[]; live: boolean; compact?: boolean }) {
  const format = field === 'latency_ms' ? latency : loss
  const current = live ? probeValue(latest, field) : null
  return <div className="d-quality-panel"><div><span>{label}</span><strong className={`d-quality-value d-quality-${probeTone(current, field)}`}>{format(current)}</strong></div>{!compact && <div className="d-quality-bars" aria-label={`${label}近期采样，空缺为灰色`}>{slots.map((point, index) => {
    const value = probeValue(point, field)
    const title = point ? `${time(point.sampled_at / 1000)} · ${point.error != null ? '检测不可用' : `${label} ${format(value)}`}` : '此时段无采样'
    return <span key={index} className={`d-quality-bar d-quality-${probeTone(value, field)}`} title={title} />
  })}</div>}</div>
}

export default function ProbeQuality({ probes, now, unavailable, loading, online, compact = false }: { probes?: ProbeOverview[]; now: number; unavailable: boolean; loading: boolean; online: boolean; compact?: boolean }) {
  const enabled = probes?.filter(item => item.probe.enabled).sort((a, b) => a.probe.name.localeCompare(b.probe.name, 'zh-CN')) ?? []
  if (!enabled.length) return <div className="d-quality-placeholder"><span>延迟 / 丢包</span><span>{unavailable ? '拨测读取失败' : loading ? '正在读取…' : probes?.length ? '拨测已暂停' : '尚未配置拨测'}</span></div>
  return <div className={`d-quality-list ${compact ? 'd-quality-compact' : ''}`}>{enabled.slice(0, compact ? 1 : 3).map(entry => {
    const probe = overviewProbe(entry), results = entry.results
    const latest = results.filter(point => point.sampled_at <= now).reduce<ProbeResult | undefined>((last, point) => !last || point.sampled_at > last.sampled_at ? point : last, undefined)
    const state = probeState(probe, latest, now, unavailable || !online)
    const slots = probeSlots(results, probe, now)
    return <div className="d-quality-row" key={probe.id}>
      <div className="d-quality-heading"><span title={`${probe.target}${probe.port ? `:${probe.port}` : ''} · 每格 ${probe.interval_secs} 秒`}>{probe.carrier ? `${probe.carrier} · ` : ''}{probe.name}</span><small>{state === '最近采样' ? probe.kind === 'icmp' ? 'ICMP' : 'TCP' : state}</small></div>
      <div className="d-quality-grid"><Quality label="延迟" field="latency_ms" compact={compact} latest={latest} slots={slots} live={state === '最近采样'} /><Quality label={lossLabel(probe)} field="loss_percent" compact={compact} latest={latest} slots={slots} live={state === '最近采样'} /></div>
    </div>
  })}{enabled.length > (compact ? 1 : 3) && <small className="d-muted">另有 {enabled.length - (compact ? 1 : 3)} 个目标，详情中查看</small>}</div>
}
