import { useState } from 'react'
import { api } from '../../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, FormDialog, Loading, Refresh } from '../../components'
import { resourceWriteError, useAction, useResource } from '../../hooks'
import type { Node, PluginServer } from '../../types'
import type { Chain } from './groupTypes'
import { installationView } from './Settings'

const root = '/api/plugins/sing-box'

type Snapshot<T> = { data?: T[]; fresh: boolean; error: string }
export type ChainWriteSnapshot = { chains: Snapshot<Chain>; nodes: Snapshot<Node>; servers: Snapshot<PluginServer> }
type ChainRequest = { name: string; entry_node_id: number; exit_node_id: number }

export function chainWriteError(snapshot: ChainWriteSnapshot, id?: number) {
  const error = resourceWriteError(snapshot.chains, snapshot.nodes, snapshot.servers)
  if (error) return error
  if (!snapshot.chains.data || !snapshot.nodes.data || !snapshot.servers.data) return '正在刷新相关信息，暂不能修改；草稿已保留。'
  return id !== undefined && !snapshot.chains.data.some(chain => chain.id === id)
    ? '此链路已不可用，暂不能提交。可关闭窗口后重新选择。' : ''
}

function nodeId(value: string) {
  const id = Number(value)
  return /^[1-9]\d*$/.test(value) && Number.isSafeInteger(id) ? id : null
}

export function chainSelectionError(snapshot: ChainWriteSnapshot, entry: string, exit: string) {
  for (const value of [entry, exit].filter(Boolean)) {
    const id = nodeId(value)
    const node = snapshot.nodes.data?.find(node => node.id === id)
    const server = node && snapshot.servers.data?.find(server => server.id === node.server_id)
    if (!node || node.protocol !== 'vless-reality' || node.enabled === false || !server?.enabled) {
      return '已选节点已不可用或所属服务器尚未启用，请重新选择；名称和其他草稿已保留。'
    }
  }
  const entryId = nodeId(entry), exitId = nodeId(exit)
  if (entryId !== null && exitId !== null) {
    const entryNode = snapshot.nodes.data?.find(node => node.id === entryId)
    const exitNode = snapshot.nodes.data?.find(node => node.id === exitId)
    if (entryNode?.server_id === exitNode?.server_id) return '入口和出口必须属于两台不同服务器；草稿已保留。'
  }
  if (snapshot.chains.data?.some(chain => chain.entry_node_id === entryId || chain.exit_node_id === entryId || chain.entry_node_id === exitId)) {
    return '已选节点的链路身份已变更，不支持重复入口、嵌套或循环链路；请重新选择，草稿已保留。'
  }
  return ''
}

export async function createChain(form: FormData, snapshot: ChainWriteSnapshot, write: (request: ChainRequest) => Promise<unknown> = request => api(`${root}/chains`, 'POST', request)) {
  const entry = String(form.get('entry_node_id') ?? ''), exit = String(form.get('exit_node_id') ?? '')
  const error = chainWriteError(snapshot) || chainSelectionError(snapshot, entry, exit)
  if (error) throw new Error(error)
  const entryId = nodeId(entry), exitId = nodeId(exit)
  if (entryId === null || exitId === null) throw new Error('请选择入口节点和出口节点；草稿已保留。')
  const name = String(form.get('name') ?? '').trim()
  if (!name || Array.from(name).length > 128) throw new Error('请填写不超过 128 个字符的名称；草稿已保留。')
  return write({ name, entry_node_id: entryId, exit_node_id: exitId })
}

export async function deleteChain(id: number, snapshot: ChainWriteSnapshot, write: (id: number) => Promise<unknown> = id => api(`${root}/chains/${id}`, 'DELETE')) {
  const error = chainWriteError(snapshot, id)
  if (error) throw new Error(error)
  return write(id)
}

function applicationView(server?: PluginServer) {
  if (!server?.installation) return { label: '应用状态待确认', tone: 'neutral' as const, reason: '尚未取得设备应用状态，请查看服务器详情。' }
  const view = installationView(server)
  const { target_rev, applied_rev } = server.installation
  if (server.installation.state === 'ready') {
    if (server.enabled && server.online && Number.isSafeInteger(target_rev) && target_rev > 0 && applied_rev === target_rev) return { ...view, label: '目标配置已应用' }
    return { label: '应用状态待确认', tone: 'warm' as const, reason: '设备状态与目标版本尚未确认一致，请查看服务器详情。' }
  }
  return { ...view, label: server.installation.state === 'queued' ? '等待生成配置' : view.label }
}

export default function Chains({ serverId }: { serverId?: number } = {}) {
  const chains = useResource<Chain[]>(`${root}/chains`)
  const nodes = useResource<Node[]>(`${root}/nodes`)
  const servers = useResource<PluginServer[]>(`${root}/servers`)
  const action = useAction()
  const [creating, setCreating] = useState(false)
  const [deleting, setDeleting] = useState<Chain | null>(null)
  const [entryNodeId, setEntryNodeId] = useState('')
  const [exitNodeId, setExitNodeId] = useState('')
  const snapshot = { chains, nodes, servers }
  const writeError = chainWriteError(snapshot)
  const selectionError = writeError ? '' : chainSelectionError(snapshot, entryNodeId, exitNodeId)
  const deleteError = chainWriteError(snapshot, deleting?.id)
  const refresh = () => { chains.reload(); nodes.reload(); servers.reload() }
  const nodeName = (id: number) => nodes.data?.find(node => node.id === id)?.name ?? `节点 #${id}（${nodes.error || !nodes.data ? '信息待确认' : '已不可用'}）`
  const all = chains.data ?? []
  const visible = serverId ? all.filter(chain => nodes.data?.some(node => node.server_id === serverId && [chain.entry_node_id, chain.exit_node_id].includes(node.id))) : all
  const filterServer = !servers.error && servers.data?.find(server => server.id === serverId)
  const unknownEndpoints = serverId && all.some(chain => [chain.entry_node_id, chain.exit_node_id].some(id => !nodes.data?.some(node => node.id === id)))
  const endpoint = (id: number) => {
    const node = nodes.data?.find(node => node.id === id)
    const server = node && nodes.fresh && servers.fresh ? servers.data?.find(server => server.id === node.server_id) : undefined
    const application = applicationView(server)
    const installation = server?.installation
    return <td><strong>{nodeName(id)}</strong><small>{node ? <a className="text-button" href={`#/servers/${node.server_id}`}>{server?.name ?? `服务器 #${node.server_id}`}</a> : '服务器归属待确认'}</small><small><Badge tone={application.tone}>{application.label}</Badge></small>{installation && Number.isSafeInteger(installation.target_rev) && Number.isSafeInteger(installation.applied_rev) && <small>目标版本 {installation.target_rev} · 已应用版本 {installation.applied_rev}</small>}<small>{application.reason}</small></td>
  }
  const open = () => { if (action.busy || chainWriteError(snapshot)) return; action.clearError(); setEntryNodeId(''); setExitNodeId(''); setCreating(true) }
  const remove = (chain: Chain) => { if (action.busy || chainWriteError(snapshot, chain.id)) return; action.clearError(); setDeleting(chain) }
  const submit = (form: FormData) => {
    if (!creating || action.busy || chainWriteError(snapshot) || chainSelectionError(snapshot, String(form.get('entry_node_id') ?? ''), String(form.get('exit_node_id') ?? ''))) return
    void action.run(() => createChain(form, snapshot), () => { setCreating(false); refresh() })
  }
  const confirmDelete = () => {
    if (!deleting || action.busy || chainWriteError(snapshot, deleting.id)) return
    void action.run(() => deleteChain(deleting.id, snapshot), () => { setDeleting(null); refresh() })
  }
  const choices = nodes.data?.filter(node => node.protocol === 'vless-reality' && node.enabled !== false && servers.data?.some(server => server.id === node.server_id && server.enabled)) ?? []

  return <>
    <ErrorNotice message={chains.error || nodes.error || servers.error || action.error} retry={refresh} />
    <section className="panel">
      {writeError && <div className="panel-body"><p className="helper" role="status">{writeError} 已取得的链路列表保留供查看，资源状态等待确认。</p></div>}
      <div className="panel-heading"><h2>两跳链路</h2><div className="row-actions"><Refresh onClick={refresh} /><button className="button button-primary button-small" disabled={action.busy || Boolean(writeError)} onClick={open}>创建两跳链路</button></div></div>
      <div className="panel-body"><p className="helper">{serverId ? `筛选范围：入口或出口属于${filterServer ? `「${filterServer.name}」` : `服务器 #${serverId}`}的链路。` : '筛选范围：全部服务器的链路。'}{serverId && <a className="text-button" href="#/plugins/sing-box/nodes?kind=chains">查看全部链路</a>}</p>{unknownEndpoints && <p className="helper">部分节点信息不可用，无法确认其服务器归属；只展示已确认关联的链路。</p>}</div>
      {chains.loading && !chains.data || serverId && nodes.loading && !nodes.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={serverId ? '此服务器暂无已确认关联的链路' : '通过入口节点连接另一台服务器的出口'} description={nodes.error && serverId ? '节点信息暂不可用，请刷新确认筛选结果。' : '先创建两个节点，再选择一个尚未授权的独立入口。创建链路后，还需通过策略组授权给代理用户。'} /> : <div className="table-wrap"><table><thead><tr><th>链路</th><th>入口服务器与应用状态</th><th>出口服务器与应用状态</th><th>资源状态</th><th>操作</th></tr></thead><tbody>{visible.map(chain => <tr key={chain.id}><td><strong>{chain.name}</strong></td>{endpoint(chain.entry_node_id)}{endpoint(chain.exit_node_id)}<td><Badge tone={writeError ? 'neutral' : chain.available ? 'good' : 'bad'}>{writeError ? '资源状态待确认' : chain.available ? '资源存在' : '资源已不可用'}</Badge></td><td><button className="text-button danger-text" disabled={action.busy || Boolean(writeError)} onClick={() => remove(chain)}>删除</button></td></tr>)}</tbody></table></div>}
      <div className="panel-body"><p className="helper">目前支持不同服务器上的 VLESS + Reality 两跳链路。创建成功不会自动授权给用户；加入策略组并分配后，还需等两端配置都应用成功，才会进入订阅。两端状态仅表示设备应用与健康信息，尚未验证公网可达或链路连通。</p><a className="text-button" href="#/plugins/sing-box/groups">管理策略组与套餐</a></div>
    </section>
    {creating && <FormDialog title="创建两跳链路" onClose={() => setCreating(false)} onSubmit={submit} busy={action.busy} disabled={Boolean(writeError)} submitDisabled={Boolean(selectionError)} error={writeError || selectionError || action.error} retry={writeError || selectionError ? refresh : undefined} submitLabel="创建未授权链路">
      <Field label="名称"><input name="name" required maxLength={128} autoComplete="off" /></Field>
      <Field label="入口节点" hint="使用尚未授权的独立节点，不能同时作为直连节点授权。"><select name="entry_node_id" required value={entryNodeId} onChange={event => setEntryNodeId(event.target.value)}><option value="" disabled>选择入口</option>{entryNodeId && !choices.some(node => String(node.id) === entryNodeId) && <option value={entryNodeId}>节点 #{entryNodeId}（已不可用，请重新选择）</option>}{choices.map(node => <option key={node.id} value={node.id}>{node.name}（服务器 #{node.server_id}）</option>)}</select></Field>
      <Field label="出口节点" hint="必须与入口分属不同服务器，出口可被多条链路共用。"><select name="exit_node_id" required value={exitNodeId} onChange={event => setExitNodeId(event.target.value)}><option value="" disabled>选择出口</option>{exitNodeId && !choices.some(node => String(node.id) === exitNodeId) && <option value={exitNodeId}>节点 #{exitNodeId}（已不可用，请重新选择）</option>}{choices.map(node => <option key={node.id} value={node.id}>{node.name}（服务器 #{node.server_id}）</option>)}</select></Field>
      <p className="helper">创建后将链路加入策略组，再分配给代理用户。用户流量按入口统计；当前服务器筛选只限定列表，创建时可选择其他服务器的节点。</p>
    </FormDialog>}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} disabled={Boolean(deleteError)} error={deleteError || action.error} retry={deleteError ? refresh : undefined} onClose={() => setDeleting(null)} onConfirm={confirmDelete}>被策略组使用的链路不能直接删除。删除后，两端设备需要应用新配置以撤销内部连接凭据。</Confirm>}
  </>
}
