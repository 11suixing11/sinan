import { useState } from 'react'
import { api } from '../api'
import { ErrorNotice, Loading } from '../components'
import { time } from '../format'
import { useAction, useResource } from '../hooks'

type Probe = { id: string; name: string; kind: 'tcp' | 'icmp'; target: string; port: number | null; interval_secs: number; carrier: string; enabled: boolean }
type ProbeResult = { id: string; probe_id: string; sampled_at: number; latency_ms: number | null; loss_percent: number; error: string | null }
type Command = { spec: { id: string; command: string; timeout_secs: number; expires_at: number }; requested_at: number; result: null | { status: 'succeeded' | 'failed' | 'expired' | 'interrupted'; stdout: string; stderr: string; finished_at: number; timed_out: boolean; truncated: boolean } }
const emptyProbe: Probe = { id: '00000000-0000-0000-0000-000000000000', name: '', kind: 'tcp', target: '', port: 443, interval_secs: 30, carrier: '', enabled: true }

export default function AgentTasks({ serverId }: { serverId: number }) {
  const base = `/api/servers/${serverId}`
  const probes = useResource<Probe[]>(`${base}/probes`)
  const results = useResource<ProbeResult[]>(`${base}/probe-results`)
  const commands = useResource<Command[]>(`${base}/commands`)
  const probeAction = useAction(), commandAction = useAction()
  const [probe, setProbe] = useState<Probe>(emptyProbe)
  const [editing, setEditing] = useState(false)
  const [command, setCommand] = useState(''), [seconds, setSeconds] = useState(30), [ttl, setTtl] = useState(300)
  const [selected, setSelected] = useState<string | null>(null)
  const labels = { succeeded: '已完成', failed: '执行失败', expired: '已过期', interrupted: '执行被中断' }
  const selectedResults = results.data?.filter(result => result.probe_id === selected).slice(0, 60)
  return <>
    <section className="panel"><div className="panel-heading"><h2>持续网络拨测</h2><span className="subtle">延迟与丢包率</span></div><div className="panel-body">
      <ErrorNotice message={probes.error || results.error || probeAction.error} />
      <form onSubmit={event => { event.preventDefault(); void probeAction.run(() => api<Probe>(editing ? `${base}/probes/${probe.id}` : `${base}/probes`, editing ? 'PATCH' : 'POST', probe), () => { setProbe(emptyProbe); setEditing(false); probes.reload() }) }}>
        <div className="form-grid"><label>名称<input required maxLength={128} value={probe.name} onChange={event => setProbe({ ...probe, name: event.target.value })} /></label>
          <label>方式<select value={probe.kind} onChange={event => setProbe({ ...probe, kind: event.target.value as Probe['kind'], port: event.target.value === 'tcp' ? 443 : null })}><option value="tcp">TCP 连接</option><option value="icmp">ICMP 回显</option></select></label>
          <label>目标地址<input required maxLength={253} placeholder="主机名或 IP 地址" value={probe.target} onChange={event => setProbe({ ...probe, target: event.target.value })} /></label>
          {probe.kind === 'tcp' && <label>端口<input required type="number" min={1} max={65535} value={probe.port ?? 443} onChange={event => setProbe({ ...probe, port: Number(event.target.value) })} /></label>}
          <label>间隔（秒）<input required type="number" min={10} max={3600} value={probe.interval_secs} onChange={event => setProbe({ ...probe, interval_secs: Number(event.target.value) })} /></label>
          <label>线路备注<input maxLength={64} placeholder="如电信、联通、移动" value={probe.carrier} onChange={event => setProbe({ ...probe, carrier: event.target.value })} /></label></div>
        <button type="submit" className="button button-primary" disabled={probeAction.busy}>{editing ? '保存拨测' : '添加拨测'}</button>{editing && <button type="button" className="button button-secondary" onClick={() => { setEditing(false); setProbe(emptyProbe) }}>取消编辑</button>}
      </form></div>
      {probes.loading && !probes.data ? <Loading /> : !probes.data?.length ? <div className="inline-empty">尚未配置拨测。</div> : <div className="table-wrap"><table><thead><tr><th>名称 / 线路</th><th>目标</th><th>延迟</th><th>丢包率</th><th>最近测量</th><th>操作</th></tr></thead><tbody>{probes.data.map(item => {
        const latest = results.data?.find(result => result.probe_id === item.id)
        return <tr key={item.id}><td><button className="text-button" onClick={() => setSelected(selected === item.id ? null : item.id)}>{item.name}</button><small>{item.carrier || '未备注'} · {item.enabled ? '已启用' : '已暂停'}</small></td><td>{item.kind.toUpperCase()} {item.target}{item.port && `:${item.port}`}</td><td>{latest?.latency_ms == null ? '暂无数据' : `${latest.latency_ms.toFixed(2)} 毫秒`}{latest?.error && <small>{latest.error}</small>}</td><td>{latest ? `${latest.loss_percent.toFixed(0)}%` : '暂无数据'}</td><td>{latest ? time(latest.sampled_at / 1000) : '等待设备测量'}</td><td><button className="text-button" onClick={() => { setProbe(item); setEditing(true) }}>编辑</button> <button className="text-button" disabled={probeAction.busy} onClick={() => void probeAction.run(() => api(`${base}/probes/${item.id}`, 'PATCH', { ...item, enabled: !item.enabled }), probes.reload)}>{item.enabled ? '暂停' : '启用'}</button> <button className="text-button" disabled={probeAction.busy} onClick={() => void probeAction.run(() => api(`${base}/probes/${item.id}`, 'DELETE'), () => { probes.reload(); if (probe.id === item.id) { setEditing(false); setProbe(emptyProbe) } })}>删除</button></td></tr>
      })}</tbody></table></div>}
      {selected && <div className="panel-body"><h3>最近 60 次测量</h3>{selectedResults?.length ? <div className="table-wrap"><table><thead><tr><th>时间</th><th>延迟</th><th>丢包率</th></tr></thead><tbody>{selectedResults.map(item => <tr key={item.id}><td>{time(item.sampled_at / 1000)}</td><td>{item.latency_ms == null ? '暂无数据' : `${item.latency_ms.toFixed(2)} 毫秒`}</td><td>{item.loss_percent}%</td></tr>)}</tbody></table></div> : <p>暂无测量记录。</p>}</div>}
    </section>
    <section className="panel"><div className="panel-heading"><h2>远程命令</h2><span className="subtle">设备服务账号执行</span></div><div className="panel-body"><ErrorNotice message={commands.error || commandAction.error} />
      <form onSubmit={event => { event.preventDefault(); void commandAction.run(() => api(`${base}/commands`, 'POST', { command, timeout_secs: seconds, ttl_secs: ttl }), () => { setCommand(''); commands.reload() }) }}>
        <label>命令<textarea required maxLength={16384} rows={4} value={command} onChange={event => setCommand(event.target.value)} placeholder="输入要在设备上执行的命令" /></label><p className="helper">Unix 使用系统 Shell，Windows 使用 PowerShell。命令可能修改设备；执行中断后不会自动重试。</p>
        <div className="form-grid"><label>执行超时（秒）<input required type="number" min={1} max={600} value={seconds} onChange={event => setSeconds(Number(event.target.value))} /></label><label>设备领取期限（秒）<input required type="number" min={1} max={86400} value={ttl} onChange={event => setTtl(Number(event.target.value))} /></label></div>
        <button className="button button-primary" type="submit" disabled={commandAction.busy}>提交命令</button>
      </form></div><div className="panel-body">{commands.data?.length ? commands.data.map(item => <details key={item.spec.id}><summary>{item.result ? labels[item.result.status] : item.spec.expires_at * 1000 < Date.now() ? '领取已过期' : '等待结果'} · {time(item.requested_at)}</summary><pre className="break-all">{item.spec.command}</pre>{item.result && <><p>{item.result.timed_out && '已达到执行超时。'}{item.result.truncated && '输出超过上限，已截断。'}</p><h4>标准输出</h4><pre className="break-all">{item.result.stdout || '（无输出）'}</pre><h4>错误输出</h4><pre className="break-all">{item.result.stderr || '（无输出）'}</pre></>}</details>) : <p className="subtle">尚未执行远程命令。</p>}</div></section>
  </>
}
