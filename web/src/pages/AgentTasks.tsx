import { useState } from 'react'
import { api } from '../api'
import { ErrorNotice, Loading } from '../components'
import { time } from '../format'
import { useAction, useResource } from '../hooks'
import { emptyMonitoring, familyLabel, latency, loss, lossLabel, networkLabel, probeState, probeValue } from '../probes'
import type { Probe, ProbeResult } from '../probes'
import CommandsPanel from './CommandsPanel'
import ProbeMonitoringFields from './ProbeMonitoringFields'

const emptyProbe: Probe = { id: '00000000-0000-0000-0000-000000000000', name: '', kind: 'tcp', target: '', port: 443, interval_secs: 30, carrier: '', enabled: true, monitoring: emptyMonitoring() }

export default function AgentTasks({ serverId, commandsEnabled }: { serverId: number; commandsEnabled: boolean }) {
  const base = `/api/servers/${serverId}`
  const probes = useResource<Probe[]>(`${base}/probes`)
  const results = useResource<ProbeResult[]>(`${base}/probe-results`)
  const probeAction = useAction()
  const [probe, setProbe] = useState<Probe>(emptyProbe)
  const [editing, setEditing] = useState(false)
  const [selected, setSelected] = useState<string | null>(null)
  const selectedHistory = useResource<ProbeResult[]>(selected ? `${base}/probe-results?probe_id=${encodeURIComponent(selected)}` : null, 15_000)
  const selectedResults = selectedHistory.data?.slice(0, 60)
  return <>
    <section className="panel"><div className="panel-heading"><h2>持续网络拨测</h2><a className="button button-secondary button-small" href="#/latency">配置延迟检测</a></div><div className="panel-body">
      <ErrorNotice message={probes.error || results.error || probeAction.error} />
      <form onSubmit={event => { event.preventDefault(); void probeAction.run(() => api<Probe>(editing ? `${base}/probes/${probe.id}` : `${base}/probes`, editing ? 'PATCH' : 'POST', probe), () => { setProbe(emptyProbe); setEditing(false); probes.reload() }) }}>
        <div className="form-grid"><label>名称<input required maxLength={128} value={probe.name} onChange={event => setProbe({ ...probe, name: event.target.value })} /></label>
          <label>方式<select disabled={editing} value={probe.kind} onChange={event => setProbe({ ...probe, kind: event.target.value as Probe['kind'], port: event.target.value === 'tcp' ? 443 : null })}><option value="tcp">TCP 连接</option><option value="icmp">ICMP 回显</option></select></label>
          <label>目标地址<input required disabled={editing} maxLength={253} placeholder="主机名或 IP 地址" value={probe.target} onChange={event => setProbe({ ...probe, target: event.target.value })} /></label>
          {probe.kind === 'tcp' && <label>端口<input required disabled={editing} type="number" min={1} max={65535} value={probe.port ?? 443} onChange={event => setProbe({ ...probe, port: Number(event.target.value) })} /></label>}
          <label>间隔（秒）<input required type="number" min={10} max={3600} value={probe.interval_secs} onChange={event => setProbe({ ...probe, interval_secs: Number(event.target.value) })} /></label>
          <label>线路备注<input maxLength={64} placeholder="如电信、联通、移动" value={probe.carrier} onChange={event => setProbe({ ...probe, carrier: event.target.value })} /></label></div>
        <ProbeMonitoringFields value={probe.monitoring} onChange={monitoring => setProbe({ ...probe, monitoring })} editing={editing} />
        {editing && <p className="helper">方式、目标地址和端口创建后不可修改。更换目标请新建拨测，以保留历史归属。</p>}
        <button type="submit" className="button button-primary" disabled={probeAction.busy}>{editing ? '保存拨测' : '添加拨测'}</button>{editing && <button type="button" className="button button-secondary" onClick={() => { setEditing(false); setProbe(emptyProbe) }}>取消编辑</button>}
      </form></div>
      {probes.loading && !probes.data ? <Loading /> : !probes.data?.length ? <div className="inline-empty">尚未配置拨测。</div> : <div className="table-wrap"><table><thead><tr><th>名称 / 线路</th><th>目标</th><th>延迟</th><th>丢包 / 连接失败率</th><th>最近测量</th><th>操作</th></tr></thead><tbody>{probes.data.map(item => {
        const latest = results.data?.find(result => result.probe_id === item.id)
        const current = probeState(item, latest, Date.now(), Boolean(results.error)) === '最近采样' ? latest : undefined
        return <tr key={item.id}><td><button className="text-button" onClick={() => setSelected(selected === item.id ? null : item.id)}>{item.name}</button><small>{networkLabel(item)} · {item.monitoring?.region || '地区未配置'} · {item.carrier || '未备注'}</small></td><td>{item.kind.toUpperCase()} {item.target}{item.port && `:${item.port}`}<small>{familyLabel(item, latest)}</small></td><td>{latency(probeValue(current, 'latency_ms'))}{latest?.error && <small>{latest.error}</small>}</td><td>{loss(probeValue(current, 'loss_percent'))}<small>{lossLabel(item)} · {probeState(item, latest, Date.now(), Boolean(results.error))}</small></td><td>{latest ? time(latest.sampled_at / 1000) : '等待设备测量'}</td><td>{item.task_id ? <a href="#/latency">统一任务管理</a> : <><button className="text-button" onClick={() => { setProbe(item); setEditing(true) }}>编辑</button> <button className="text-button" disabled={probeAction.busy} onClick={() => void probeAction.run(() => api(`${base}/probes/${item.id}`, 'PATCH', { ...item, enabled: !item.enabled }), probes.reload)}>{item.enabled ? '暂停' : '启用'}</button> <button className="text-button" disabled={probeAction.busy} onClick={() => void probeAction.run(() => api(`${base}/probes/${item.id}`, 'DELETE'), () => { probes.reload(); if (probe.id === item.id) { setEditing(false); setProbe(emptyProbe) } })}>删除</button></>}</td></tr>
      })}</tbody></table></div>}
      {selected && <div className="panel-body"><h3>最近 60 次测量</h3><ErrorNotice message={selectedHistory.error} />{selectedResults?.length ? <div className="table-wrap"><table><thead><tr><th>时间 / 网络版本</th><th>延迟</th><th>丢包 / 连接失败率</th></tr></thead><tbody>{selectedResults.map(item => <tr key={item.id}><td>{time(item.sampled_at / 1000)}<small>{item.ip_version ? `IPv${item.ip_version}` : '实际版本未知'}</small></td><td>{latency(probeValue(item, 'latency_ms'))}</td><td>{loss(probeValue(item, 'loss_percent'))}{item.error != null && <small>{item.error}</small>}</td></tr>)}</tbody></table></div> : <p>暂无测量记录。</p>}</div>}
    </section>
    <CommandsPanel serverId={serverId} enabled={commandsEnabled} />
  </>
}
