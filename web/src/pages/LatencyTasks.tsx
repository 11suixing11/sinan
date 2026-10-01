import { useState } from 'react'
import { api } from '../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, Icon, Loading, Modal, PageHeader, Refresh } from '../components'
import { useAction, useResource } from '../hooks'
import type { Probe } from '../probes'
import type { Server } from '../types'
import ServerPicker from './ServerPicker'
import './server-setup.css'

type Task = { id: string; spec: Probe; default_enabled: boolean; server_ids: number[]; revision: number }
const empty: Task = { id: '', spec: { id: '00000000-0000-0000-0000-000000000000', name: '', kind: 'tcp', target: '', port: 443, carrier: '', interval_secs: 30, enabled: true }, default_enabled: false, server_ids: [], revision: 0 }

function Editor({ task, servers, onClose, onSaved }: { task: Task; servers: Server[]; onClose: () => void; onSaved: () => void }) {
  const [value, setValue] = useState(task)
  const action = useAction(), editing = Boolean(task.id)
  const change = (part: Partial<Probe>) => setValue(current => ({ ...current, spec: { ...current.spec, ...part } }))
  return <Modal className="monitoring-editor" title={editing ? `编辑延迟任务 · ${task.spec.name}` : '添加延迟任务'} onClose={onClose} busy={action.busy} wide><form onSubmit={event => { event.preventDefault(); void action.run(() => api(editing ? `/api/latency-tasks/${task.id}` : '/api/latency-tasks', editing ? 'PATCH' : 'POST', { spec: value.spec, default_enabled: value.default_enabled, server_ids: value.server_ids, ...(editing ? { revision: value.revision } : {}) }), onSaved) }}><div className="modal-body"><ErrorNotice message={action.error} /><fieldset disabled={action.busy}>
    <div className="form-grid"><Field label="任务名称"><input required pattern=".*\S.*" maxLength={128} value={value.spec.name} onChange={event => change({ name: event.target.value })} placeholder="例如：主站连通性" /></Field><Field label="检测方式"><select disabled={editing} value={value.spec.kind} onChange={event => change({ kind: event.target.value as Probe['kind'], port: event.target.value === 'tcp' ? 443 : null })}><option value="tcp">TCP 连接</option><option value="icmp">ICMP 回显</option></select></Field><Field label="目标地址" hint="主机名、IPv4 或 IPv6，不含协议和路径。"><input required disabled={editing} maxLength={253} autoCapitalize="none" spellCheck={false} value={value.spec.target} onChange={event => change({ target: event.target.value })} placeholder="probe.example.com" /></Field>{value.spec.kind === 'tcp' && <Field label="目标端口"><input required disabled={editing} type="number" min={1} max={65535} step={1} value={value.spec.port ?? ''} onChange={event => change({ port: Number(event.target.value) })} /></Field>}<Field label="检测间隔（秒）"><input required type="number" min={10} max={3600} step={1} value={value.spec.interval_secs} onChange={event => change({ interval_secs: Number(event.target.value) })} /></Field><Field label="线路备注"><input maxLength={64} value={value.spec.carrier} onChange={event => change({ carrier: event.target.value })} placeholder="例如：电信 / 联通 / 移动" /></Field></div>
    <p className="helper">{value.spec.kind === 'icmp' ? 'ICMP 测量延迟与丢包；节点需具备检测工具及权限。' : 'TCP 测量连接延迟与连接失败率，每轮尝试四次。'}{editing && '检测方式、目标和端口固定，修改名称与间隔保留已有历史。'}</p>
    <ServerPicker servers={servers} selected={value.server_ids} onChange={server_ids => setValue(current => ({ ...current, server_ids }))} />
    <label className="server-setup-toggle"><span><strong>启用任务</strong><small>暂停后保留配置与已有历史，所有已分配服务器同步暂停。</small></span><input role="switch" type="checkbox" checked={value.spec.enabled} onChange={event => change({ enabled: event.target.checked })} /><span className="server-setup-switch" aria-hidden="true" /></label>
    <label className="server-setup-toggle"><span><strong>默认分配给新服务器</strong><small>仅影响之后新增的服务器；已有服务器按上方选择分配。</small></span><input role="switch" type="checkbox" checked={value.default_enabled} onChange={event => setValue(current => ({ ...current, default_enabled: event.target.checked }))} /><span className="server-setup-switch" aria-hidden="true" /></label>
    <p className="helper">每台服务器的独立拨测与统一任务合计最多 32 个。取消分配后再次分配会开始新的采样历史。</p>
  </fieldset></div><footer><button type="button" className="button button-secondary" disabled={action.busy} onClick={onClose}>取消</button><button className="button button-primary" disabled={action.busy}>{action.busy ? '正在保存…' : '保存任务'}</button></footer></form></Modal>
}

export default function LatencyTasks() {
  const tasks = useResource<Task[]>('/api/latency-tasks', 0), servers = useResource<Server[]>('/api/servers', 0)
  const [editing, setEditing] = useState<Task | null>(null), [removing, setRemoving] = useState<Task | null>(null)
  const action = useAction()
  const refresh = () => { tasks.reload(); servers.reload() }
  return <><PageHeader eyebrow="持续网络监控" title="延迟检测" description="统一配置 TCP / ICMP 目标，批量分配服务器；结果显示在服务器看板和详情中。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={!servers.data || Boolean(servers.error) || (tasks.data?.length ?? 32) >= 32} onClick={() => setEditing(empty)}><Icon name="plus" size={16} />添加任务</button></PageHeader><ErrorNotice message={tasks.error || servers.error || action.error} retry={refresh} />
    <section className="panel"><div className="panel-heading"><h2>延迟任务</h2><span className="subtle">{tasks.data?.length ?? 0} / 32 个任务</span></div>{!tasks.data && tasks.loading ? <Loading /> : !tasks.data?.length ? <Empty icon="activity" title="尚未配置统一延迟任务" description="添加检测目标，再选择需要持续观测线路的服务器。已有单机拨测继续运行。" /> : <div className="table-wrap"><table><thead><tr><th>任务 / 线路</th><th>检测目标</th><th>间隔</th><th>服务器</th><th>状态</th><th>操作</th></tr></thead><tbody>{tasks.data.map(task => <tr key={task.id}><td>{task.spec.name}<small>{task.spec.carrier || '未备注线路'}</small></td><td>{task.spec.kind.toUpperCase()}<small className="mono">{task.spec.target}{task.spec.port && ` · ${task.spec.port}`}</small></td><td>{task.spec.interval_secs} 秒</td><td>{task.server_ids.length} 台<small>{task.default_enabled ? '默认分配新服务器' : '手动分配'}</small></td><td><Badge tone={task.spec.enabled ? 'good' : 'neutral'}>{task.spec.enabled ? '已启用' : '已暂停'}</Badge></td><td><div className="monitoring-actions"><button className="text-button" disabled={!servers.data || Boolean(servers.error)} onClick={() => setEditing(task)}>编辑</button><button className="text-button" disabled={action.busy} onClick={() => void action.run(() => api(`/api/latency-tasks/${task.id}`, 'PATCH', { spec: { ...task.spec, enabled: !task.spec.enabled }, default_enabled: task.default_enabled, server_ids: task.server_ids, revision: task.revision }), tasks.reload)}>{task.spec.enabled ? '暂停' : '启用'}</button><button className="text-button danger-text" onClick={() => setRemoving(task)}>删除</button></div></td></tr>)}</tbody></table></div>}</section>
    {editing && servers.data && <Editor key={editing.id || 'new'} task={editing} servers={servers.data} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); refresh() }} />}
    {removing && <Confirm title="删除延迟任务" busy={action.busy} error={action.error} onClose={() => setRemoving(null)} onConfirm={() => void action.run(() => api(`/api/latency-tasks/${removing.id}`, 'DELETE'), () => { setRemoving(null); tasks.reload() })}>确认删除“{removing.spec.name}”？所有已分配服务器将在同步后停止检测，该任务将从看板移除。</Confirm>}
  </>
}
