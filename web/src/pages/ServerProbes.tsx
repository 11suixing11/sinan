import { useRef, useState } from 'react'
import { api } from '../api'
import { ErrorNotice, Loading, Refresh } from '../components'
import { time } from '../format'
import { useAction, useResource } from '../hooks'
import { authorizationDraft, authorizationPayload, currentServerProbe, deleteServerProbe, latency, loss, lossLabel, probeAuthorizationState, probeLeaseNotice, probeReadError, probeState, probeValue, saveServerProbe } from '../probes'
import type { Probe, ProbeResult, ProbeWriteSnapshot } from '../probes'
import ProbeAuthorizationFields from './ProbeAuthorizationFields'

const emptyProbe: Probe = { id: '00000000-0000-0000-0000-000000000000', name: '', kind: 'tcp', target: '', port: 443, interval_secs: 30, carrier: '', enabled: true, authorization: null }

export default function ServerProbes({ serverId }: { serverId: number }) {
  const base = `/api/servers/${serverId}`
  const probes = useResource<Probe[]>(`${base}/probes`)
  const results = useResource<ProbeResult[]>(`${base}/probe-results`)
  const latest = useRef<ProbeWriteSnapshot>({ serverId, probes })
  latest.current = { serverId, probes }
  const action = useAction()
  const [probe, setProbe] = useState<Probe>(emptyProbe)
  const [origin, setOrigin] = useState<{ serverId: number; probe: Probe | null }>({ serverId, probe: null })
  const [authorization, setAuthorization] = useState(() => authorizationDraft())
  const [saved, setSaved] = useState(false)
  const [selected, setSelected] = useState<string | null>(null)
  const history = useResource<ProbeResult[]>(selected ? `${base}/probe-results?probe_id=${encodeURIComponent(selected)}` : null, 15_000)
  const editing = Boolean(origin.probe)
  let readError = probeReadError(probes)
  if (!readError && origin.serverId !== serverId) readError = '目标服务器已变化，请取消旧草稿后重新配置。'
  if (!readError && origin.probe) {
    try { currentServerProbe(latest.current, origin.serverId, origin.probe) } catch (error) { readError = (error as Error).message }
  }
  const reset = () => { setProbe(emptyProbe); setAuthorization(authorizationDraft()); setOrigin({ serverId, probe: null }) }
  const refresh = () => {
    latest.current = { serverId, probes: { ...probes, fresh: false } }
    probes.reload(); results.reload(); history.reload()
  }
  const completed = () => { setSaved(true); refresh() }
  const toggle = (item: Probe) => action.run(() => saveServerProbe(latest.current, serverId, item, { ...item, enabled: !item.enabled }, item.authorization ?? null, body => api(`${base}/probes/${item.id}`, 'PATCH', body)), completed)
  return <section className="panel">
    <div className="panel-heading"><h2>持续网络拨测</h2><div className="monitoring-actions"><Refresh onClick={refresh} /><a className="button button-secondary button-small" href="#/latency">配置延迟检测</a></div></div>
    <div className="panel-body">
      <ErrorNotice message={readError || results.error || action.error} retry={refresh} />
      {saved && <p role="status" className="helper">配置已保存，等待设备刷新短期许可；尚未证明设备已开始或停止检测。</p>}
      <form onSubmit={event => { event.preventDefault(); void action.run(() => saveServerProbe(latest.current, origin.serverId, origin.probe, probe, authorizationPayload(authorization, probe.enabled), body => api<Probe>(editing ? `${base}/probes/${probe.id}` : `${base}/probes`, editing ? 'PATCH' : 'POST', body)), () => { reset(); completed() }) }}>
        <fieldset disabled={action.busy}><div className="form-grid">
          <label>名称<input required maxLength={128} value={probe.name} onChange={event => setProbe({ ...probe, name: event.target.value })} /></label>
          <label>方式<select disabled={editing} value={probe.kind} onChange={event => setProbe({ ...probe, kind: event.target.value as Probe['kind'], port: event.target.value === 'tcp' ? 443 : null })}><option value="tcp">TCP 连接</option><option value="icmp">ICMP 回显</option></select></label>
          <label>目标地址<input required disabled={editing} maxLength={253} placeholder="主机名或 IP 地址" value={probe.target} onChange={event => setProbe({ ...probe, target: event.target.value })} /></label>
          {probe.kind === 'tcp' && <label>端口<input required disabled={editing} type="number" min={1} max={65535} step={1} value={probe.port ?? 443} onChange={event => setProbe({ ...probe, port: Number(event.target.value) })} /></label>}
          <label>间隔（秒）<input required type="number" min={10} max={3600} step={1} value={probe.interval_secs} onChange={event => setProbe({ ...probe, interval_secs: Number(event.target.value) })} /></label>
          <label>线路备注<input maxLength={64} placeholder="如电信、联通、移动" value={probe.carrier} onChange={event => setProbe({ ...probe, carrier: event.target.value })} /></label>
        </div>
        {editing && <p className="helper">方式、目标地址和端口创建后不可修改。更换目标请新建拨测，以保留历史归属。</p>}
        <ProbeAuthorizationFields value={authorization} onChange={setAuthorization} />
        <label><input type="checkbox" checked={probe.enabled} onChange={event => setProbe({ ...probe, enabled: event.target.checked })} />启用拨测</label>
        </fieldset>
        <button type="submit" className="button button-primary" disabled={action.busy || Boolean(readError)}>{editing ? '保存拨测' : '添加拨测'}</button>
        {(editing || origin.serverId !== serverId) && <button type="button" className="button button-secondary" disabled={action.busy} onClick={reset}>取消编辑</button>}
      </form>
      <p className="helper">{probeLeaseNotice}</p>
    </div>
    {probes.loading && !probes.data ? <Loading /> : !probes.data?.length ? <div className="inline-empty">{probes.error ? '拨测列表读取失败。' : '尚未配置拨测。'}</div> : <div className="table-wrap"><table><thead><tr><th>名称 / 线路</th><th>目标</th><th>延迟</th><th>丢包 / 连接失败率</th><th>最近测量</th><th>操作</th></tr></thead><tbody>{probes.data.map(item => {
      const latestResult = results.data?.find(result => result.probe_id === item.id)
      const state = probeState(item, latestResult, Date.now(), !probes.fresh || !results.fresh)
      const current = state === '最近采样' ? latestResult : undefined
      const permission = probeAuthorizationState(item)
      return <tr key={item.id}><td><button className="text-button" onClick={() => setSelected(selected === item.id ? null : item.id)}>{item.name}</button><small>{item.carrier || '未备注'} · {item.enabled ? '配置启用' : '配置暂停'}</small></td><td>{item.kind.toUpperCase()} {item.target}{item.port && `:${item.port}`}</td><td>{latency(probeValue(current, 'latency_ms'))}{latestResult?.error && <small>{latestResult.error}</small>}</td><td>{loss(probeValue(current, 'loss_percent'))}<small>{lossLabel(item)} · {state}</small></td><td>{latestResult ? time(latestResult.sampled_at / 1000) : '等待设备测量'}</td><td>{item.task_id ? <a href="#/latency">统一任务管理</a> : <>
        <button className="text-button" disabled={action.busy || Boolean(probeReadError(probes))} onClick={() => { setSaved(false); setProbe(item); setAuthorization(authorizationDraft(item.authorization)); setOrigin({ serverId, probe: item }) }}>编辑</button>
        <button className="text-button" disabled={action.busy || Boolean(probeReadError(probes)) || (!item.enabled && permission !== 'allowed')} onClick={() => void toggle(item)}>{item.enabled ? '暂停' : '启用'}</button>
        <button className="text-button" disabled={action.busy || Boolean(probeReadError(probes))} onClick={() => void action.run(() => deleteServerProbe(latest.current, serverId, item, (id, body) => api(`${base}/probes/${id}`, 'DELETE', body)), () => { if (origin.probe?.id === item.id) reset(); completed() })}>删除</button>
      </>}</td></tr>
    })}</tbody></table></div>}
    {selected && <div className="panel-body"><h3>最近 60 次历史测量</h3><p className="helper">保留历史结果不代表当前目标仍获授权或设备正在检测。</p><ErrorNotice message={history.error} />{history.data?.length ? <div className="table-wrap"><table><thead><tr><th>时间</th><th>延迟</th><th>丢包 / 连接失败率</th></tr></thead><tbody>{history.data.slice(0, 60).map(item => <tr key={item.id}><td>{time(item.sampled_at / 1000)}</td><td>{latency(probeValue(item, 'latency_ms'))}</td><td>{loss(probeValue(item, 'loss_percent'))}{item.error != null && <small>检测不可用</small>}</td></tr>)}</tbody></table></div> : <p>{history.error ? '历史结果暂不可读取。' : '暂无测量记录。'}</p>}</div>}
  </section>
}
