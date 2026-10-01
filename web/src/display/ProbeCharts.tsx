import { useState } from 'react'
import { time } from '../format'
import { useResource } from '../hooks'
import { latency, loss, lossLabel, probeState, probeValue } from '../probes'
import type { Probe, ProbeResult } from '../probes'
import Chart from './Chart'
import { Icon } from './Icon'

function History({ id, probe, now, minutes, unavailable }: { id: number; probe: Probe; now: number; minutes: number; unavailable: boolean }) {
  const results = useResource<ProbeResult[]>(`/api/servers/${id}/probe-results?probe_id=${encodeURIComponent(probe.id)}&hours=${minutes / 60}`, 15_000)
  const points = (results.data ?? []).filter(result => result.probe_id === probe.id && result.sampled_at >= now - minutes * 60_000 && result.sampled_at <= now).sort((a, b) => a.sampled_at - b.sampled_at)
  const latest = points[points.length - 1]
  const state = probeState(probe, latest, now, unavailable || Boolean(results.error))
  const props = { from: now - minutes * 60_000, to: now, gap: Math.max(30_000, probe.interval_secs * 3000) }
  return <>
    {results.error && <div className="d-notice d-error" role="alert"><span>拨测结果读取失败，已显示的曲线为历史数据。</span><button onClick={results.reload}>重试</button></div>}
    {results.loading && !results.data && <p className="d-history-loading" role="status">正在读取拨测结果…</p>}
    <div className="d-probe-summary"><span>{probe.carrier && `${probe.carrier} · `}{probe.kind === 'tcp' ? 'TCP 连接' : 'ICMP 回显'} · {probe.target}{probe.port ? `:${probe.port}` : ''}</span><span>{state}{latest && ` · ${time(latest.sampled_at / 1000)}`}</span><strong>{latency(state === '最近采样' ? probeValue(latest, 'latency_ms') : null)} · {lossLabel(probe)} {loss(state === '最近采样' ? probeValue(latest, 'loss_percent') : null)}</strong></div>
    {latest?.error != null && <p className="d-notice">最近一次检测未完成，请在后台查看工具或权限错误。该次结果以空缺显示。</p>}
    <div className="d-charts">
      <Chart {...props} title={`${probe.name} · 延迟`} format={latency} series={[{ label: '往返延迟', color: 'var(--d-info)', points: points.map(point => ({ at: point.sampled_at, value: probeValue(point, 'latency_ms') })) }]} />
      <Chart {...props} title={`${probe.name} · ${lossLabel(probe)}`} maximum={100} format={loss} series={[{ label: lossLabel(probe), color: 'var(--d-warning-bar)', points: points.map(point => ({ at: point.sampled_at, value: probeValue(point, 'loss_percent') })) }]} />
    </div>
  </>
}

export default function ProbeCharts({ id, now, unavailable }: { id: number; now: number; unavailable: boolean }) {
  const definitions = useResource<Probe[]>(`/api/servers/${id}/probes`, 30_000)
  const [minutes, setMinutes] = useState(60)
  const [selected, setSelected] = useState('')
  const probes = definitions.data ?? []
  const probe = probes.find(item => item.id === selected) ?? probes.find(item => item.enabled) ?? probes[0]
  return <section className="d-probes">
    <div className="d-section-heading"><h2><Icon name="network" size={17} />延迟与丢包</h2><div className="d-segmented" role="group" aria-label="拨测时间范围">{[[60, '1 小时'], [360, '6 小时'], [1440, '24 小时']].map(([value, label]) => <button key={value} aria-pressed={minutes === value} onClick={() => setMinutes(Number(value))}>{label}</button>)}</div></div>
    {definitions.error && <div className="d-notice d-error" role="alert"><span>拨测配置读取失败。</span><button onClick={definitions.reload}>重试</button></div>}
    {definitions.loading && !definitions.data ? <div className="d-chart-empty" role="status">正在读取拨测配置…</div> : definitions.error && !definitions.data ? <div className="d-chart-empty d-glass">暂时无法读取拨测配置</div> : !probe ? <div className="d-chart-empty d-glass">尚无已配置的拨测项目<a className="d-button" href={`#/servers/${id}`}>前往后台配置拨测</a></div> : <>
      <label className="d-probe-select">拨测目标<select aria-label="拨测目标" value={probe.id} onChange={event => setSelected(event.target.value)}>{probes.map(item => <option key={item.id} value={item.id}>{item.carrier ? `${item.carrier} · ` : ''}{item.name}{item.enabled ? '' : '（已暂停）'}</option>)}</select></label>
      <History key={`${probe.id}/${minutes}`} id={id} probe={probe} now={now} minutes={minutes} unavailable={unavailable || Boolean(definitions.error)} />
      <p className="d-footnote">由服务器向配置的目标发起检测，每轮 4 次。ICMP 显示回显丢包率，TCP 显示连接失败率；延迟为成功响应的平均值。无响应为 100%，检测失败或缺少采样保留空缺。</p>
    </>}
  </section>
}
