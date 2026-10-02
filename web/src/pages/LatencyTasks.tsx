import { useRef, useState } from 'react'
import { api } from '../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, Icon, Loading, Modal, PageHeader, Refresh } from '../components'
import { useAction, useResource } from '../hooks'
import { authorizationDraft, authorizationPayload, deleteLatencyTask, latencyDraftError, probeAuthorizationState, probeLeaseNotice, probeReadError, saveLatencyTask } from '../probes'
import type { LatencyTask as Task, ProbeSpec, TaskWriteSnapshot } from '../probes'
import type { Server } from '../types'
import ProbeAuthorizationFields from './ProbeAuthorizationFields'
import ServerPicker from './ServerPicker'
import './server-setup.css'

const empty: Task = { id: '', spec: { id: '00000000-0000-0000-0000-000000000000', name: '', kind: 'tcp', target: '', port: 443, carrier: '', interval_secs: 30, enabled: true }, authorization: null, default_enabled: false, server_ids: [], revision: 0 }

function Editor({ task, servers, getSnapshot, onClose, onSaved }: { task: Task; servers: Server[]; getSnapshot: () => TaskWriteSnapshot; onClose: () => void; onSaved: () => void }) {
  const [value, setValue] = useState(task)
  const [authorization, setAuthorization] = useState(() => authorizationDraft(task.authorization))
  const action = useAction(), editing = Boolean(task.id)
  const change = (part: Partial<ProbeSpec>) => setValue(current => ({ ...current, spec: { ...current.spec, ...part } }))
  const readError = latencyDraftError(getSnapshot(), editing ? task : null, value.server_ids)
  const submit = () => action.run(() => saveLatencyTask(getSnapshot(), editing ? task : null, value, authorizationPayload(authorization, value.spec.enabled), body => api(editing ? `/api/latency-tasks/${task.id}` : '/api/latency-tasks', editing ? 'PATCH' : 'POST', body)), onSaved)
  return <Modal className="monitoring-editor" title={editing ? `编辑延迟任务 · ${task.spec.name}` : '添加延迟任务'} onClose={onClose} busy={action.busy} wide>
    <form onSubmit={event => { event.preventDefault(); void submit() }}><div className="modal-body">
      <ErrorNotice message={readError || action.error} />
      <fieldset disabled={action.busy}>
        <div className="form-grid">
          <Field label="任务名称"><input required pattern=".*\S.*" maxLength={128} value={value.spec.name} onChange={event => change({ name: event.target.value })} placeholder="例如：主站连通性" /></Field>
          <Field label="检测方式"><select disabled={editing} value={value.spec.kind} onChange={event => change({ kind: event.target.value as ProbeSpec['kind'], port: event.target.value === 'tcp' ? 443 : null })}><option value="tcp">TCP 连接</option><option value="icmp">ICMP 回显</option></select></Field>
          <Field label="目标地址" hint="主机名、IPv4 或 IPv6，不含协议和路径。"><input required disabled={editing} maxLength={253} autoCapitalize="none" spellCheck={false} value={value.spec.target} onChange={event => change({ target: event.target.value })} placeholder="probe.example.com" /></Field>
          {value.spec.kind === 'tcp' && <Field label="目标端口"><input required disabled={editing} type="number" min={1} max={65535} step={1} value={value.spec.port ?? ''} onChange={event => change({ port: Number(event.target.value) })} /></Field>}
          <Field label="检测间隔（秒）"><input required type="number" min={10} max={3600} step={1} value={value.spec.interval_secs} onChange={event => change({ interval_secs: Number(event.target.value) })} /></Field>
          <Field label="线路备注"><input maxLength={64} value={value.spec.carrier} onChange={event => change({ carrier: event.target.value })} placeholder="例如：电信 / 联通 / 移动" /></Field>
        </div>
        <p className="helper">{value.spec.kind === 'icmp' ? 'ICMP 测量延迟与丢包；节点需具备检测工具及权限。' : 'TCP 测量连接延迟与连接失败率，每轮尝试四次。'}{editing && '检测方式、目标和端口固定，修改名称与间隔保留已有历史。'}</p>
        <ProbeAuthorizationFields value={authorization} onChange={setAuthorization} />
        <ServerPicker servers={servers} selected={value.server_ids} onChange={server_ids => setValue(current => ({ ...current, server_ids }))} />
        {value.server_ids.some(id => !servers.some(server => server.id === id)) && <button type="button" className="text-button" onClick={() => setValue(current => ({ ...current, server_ids: current.server_ids.filter(id => servers.some(server => server.id === id)) }))}>移除已不可用服务器</button>}
        <label className="server-setup-toggle"><span><strong>启用任务</strong><small>只有有效授权目标才能下发。暂停保留配置与历史，设备停止有短期许可收敛时间。</small></span><input role="switch" type="checkbox" checked={value.spec.enabled} onChange={event => change({ enabled: event.target.checked })} /><span className="server-setup-switch" aria-hidden="true" /></label>
        <label className="server-setup-toggle"><span><strong>默认分配给新服务器</strong><small>仅影响之后新增的服务器；已有服务器按上方选择分配。</small></span><input role="switch" type="checkbox" checked={value.default_enabled} onChange={event => setValue(current => ({ ...current, default_enabled: event.target.checked }))} /><span className="server-setup-switch" aria-hidden="true" /></label>
        <p className="helper">每台服务器的独立拨测与统一任务合计最多 32 个。取消分配后再次分配会开始新的采样历史。离线服务器仍可配置。</p>
      </fieldset>
    </div><footer><button type="button" className="button button-secondary" disabled={action.busy} onClick={onClose}>取消</button><button type="submit" className="button button-primary" disabled={action.busy || Boolean(readError)}>{action.busy ? '正在保存…' : '保存任务'}</button></footer></form>
  </Modal>
}

export default function LatencyTasks() {
  const tasks = useResource<Task[]>('/api/latency-tasks'), servers = useResource<Server[]>('/api/servers')
  const latest = useRef<TaskWriteSnapshot>({ tasks, servers })
  latest.current = { tasks, servers }
  const [editing, setEditing] = useState<Task | null>(null), [removing, setRemoving] = useState<Task | null>(null)
  const [saved, setSaved] = useState(false)
  const action = useAction()
  const readError = probeReadError(tasks, servers)
  const refresh = () => {
    latest.current = { tasks: { ...tasks, fresh: false }, servers: { ...servers, fresh: false } }
    tasks.reload(); servers.reload()
  }
  const completed = () => { setSaved(true); refresh() }
  const toggle = (task: Task) => action.run(() => saveLatencyTask(latest.current, task, { ...task, spec: { ...task.spec, enabled: !task.spec.enabled } }, task.authorization ?? null, body => api(`/api/latency-tasks/${task.id}`, 'PATCH', body)), completed)
  return <>
    <PageHeader eyebrow="持续网络监控" title="延迟检测" description="统一配置已授权的 TCP / ICMP 目标并分配服务器；授权记录只供管理员查看。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={Boolean(readError) || (tasks.data?.length ?? 32) >= 32} onClick={() => { setSaved(false); setEditing(empty) }}><Icon name="plus" size={16} />添加任务</button></PageHeader>
    <ErrorNotice message={readError || action.error} retry={refresh} />
    {saved && <p role="status" className="helper">配置已保存，等待设备刷新短期许可；尚未证明设备已开始或停止检测。</p>}
    <p className="helper">{probeLeaseNotice}</p>
    <section className="panel"><div className="panel-heading"><h2>延迟任务</h2><span className="subtle">{tasks.data?.length ?? 0} / 32 个任务</span></div>
      {!tasks.data && tasks.loading ? <Loading /> : !tasks.data?.length ? <Empty icon="activity" title="尚未配置统一延迟任务" description="登记目标来源和使用依据，再选择服务器。旧未登记目标保留历史，等待补充授权。" /> : <div className="table-wrap"><table><thead><tr><th>任务 / 线路</th><th>检测目标</th><th>间隔</th><th>服务器</th><th>配置状态</th><th>操作</th></tr></thead><tbody>{tasks.data.map(task => {
        const permission = probeAuthorizationState({ authorization: task.authorization })
        const state = readError ? '状态未知' : !task.spec.enabled ? '已暂停' : permission === 'allowed' ? '授权有效，等待设备采样' : permission === 'expired' ? '目标授权已过期' : '目标授权待确认'
        return <tr key={task.id}><td>{task.spec.name}<small>{task.spec.carrier || '未备注线路'}{task.authorization?.region && ` · ${task.authorization.region}`}</small></td><td>{task.spec.kind.toUpperCase()}<small className="mono">{task.spec.target}{task.spec.port && ` · ${task.spec.port}`}</small></td><td>{task.spec.interval_secs} 秒</td><td>{task.server_ids.length} 台<small>{task.default_enabled ? '默认分配新服务器' : '手动分配'}</small></td><td><Badge tone="neutral">{state}</Badge></td><td><div className="monitoring-actions">
          <button className="text-button" disabled={action.busy || Boolean(readError)} onClick={() => { setSaved(false); setEditing(task) }}>编辑</button>
          <button className="text-button" disabled={action.busy || Boolean(readError) || (!task.spec.enabled && permission !== 'allowed')} onClick={() => void toggle(task)}>{task.spec.enabled ? '暂停' : '启用'}</button>
          <button className="text-button danger-text" disabled={action.busy || Boolean(readError)} onClick={() => setRemoving(task)}>删除</button>
        </div></td></tr>
      })}</tbody></table></div>}
    </section>
    {editing && servers.data && <Editor key={editing.id || 'new'} task={editing} servers={servers.data} getSnapshot={() => latest.current} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); completed() }} />}
    {removing && <Confirm title="删除延迟任务" busy={action.busy} error={latencyDraftError(latest.current, removing, []) || action.error} onClose={() => setRemoving(null)} onConfirm={() => void action.run(() => deleteLatencyTask(latest.current, removing, (id, body) => api(`/api/latency-tasks/${id}`, 'DELETE', body)), () => { setRemoving(null); completed() })}>确认删除“{removing.spec.name}”？历史数据不作为新执行授权。设备失去刷新后在最长 90 秒许可内停止，删除接口成功不代表设备立即停止。</Confirm>}
  </>
}
