import { useRef, useState } from 'react'
import { api } from '../../api'
import { Field, FormDialog } from '../../components'
import { resourceWriteError, useAction } from '../../hooks'
import type { Node, PluginServer } from '../../types'
import { assignmentRequestId, proxyResourceKey, validProxyResources } from './groupTypes'
import type { Chain, ProxyResource, ResourceSnapshot } from './groupTypes'

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

export type ProxyWriteSnapshot = { resources: ResourceSnapshot<ProxyResource[]>; nodes: ResourceSnapshot<Node[]>; servers: ResourceSnapshot<PluginServer[]> }
export type ChainBatchDraft = { mode: 'new' | 'existing'; server_id: string; public_host: string; sni: string; entry_node_id: string; exit_node_id: string; rows: { name: string; port: string }[] }
export type ChainBatchRequest = { request_id: string; items: { name: string; entry: { mode: 'new'; server_id: number; public_host: string; sni: string; port: number | null } | { mode: 'existing'; node_id: number }; hops: { kind: 'managed'; node_id: number }[] }[] }
export type PendingChainBatch = { draft: string; source_draft: string; serialized: string; request_id: string; attempted: boolean }
export type ChainBatchResult = { request_id: string; chain_ids: number[]; entry_node_ids: number[] }

const record = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value)
function validSettings(value: unknown) {
  if (value === undefined) return true
  if (!record(value)) return false
  for (const [key, field] of Object.entries(value)) {
    if (['listen'].includes(key) && typeof field !== 'string') return false
    if (key === 'public_port' && field !== null && (!Number.isSafeInteger(field) || Number(field) < 1 || Number(field) > 65535)) return false
    if (key === 'tcp_fast_open' && typeof field !== 'boolean') return false
    if (key === 'tls_alpn' && (!Array.isArray(field) || !field.every(item => typeof item === 'string'))) return false
    if (['reality', 'hysteria2', 'tuic', 'anytls'].includes(key)) {
      if (!record(field)) return false
      for (const [name, setting] of Object.entries(field)) {
        if (['handshake_server', 'fingerprint', 'congestion_control'].includes(name) && typeof setting !== 'string' && !(name === 'handshake_server' && setting === null)) return false
        if (['ignore_client_bandwidth', 'obfs_enabled', 'zero_rtt_handshake'].includes(name) && typeof setting !== 'boolean') return false
        if (['handshake_port', 'up_mbps', 'down_mbps', 'auth_timeout_seconds', 'heartbeat_seconds', 'idle_session_check_seconds', 'idle_session_timeout_seconds', 'min_idle_session'].includes(name)
          && setting !== null && (!Number.isSafeInteger(setting) || Number(setting) < 0)) return false
      }
    }
  }
  return true
}
export function validNodeList(value: unknown): value is Node[] {
  return Array.isArray(value) && value.every(node => node && Number.isSafeInteger(node.id) && node.id > 0 && Number.isSafeInteger(node.server_id) && node.server_id > 0
    && typeof node.name === 'string' && typeof node.protocol === 'string' && typeof node.public_host === 'string' && typeof node.sni === 'string'
    && Number.isSafeInteger(node.port) && node.port >= 1 && node.port <= 65535 && (node.enabled === undefined || typeof node.enabled === 'boolean') && validSettings(node.settings)
    && (node.protocol_config === undefined || record(node.protocol_config) && typeof node.protocol_config.type === 'string'
      && (node.protocol_config.method === undefined || typeof node.protocol_config.method === 'string')
      && (node.protocol_config.tls === undefined || record(node.protocol_config.tls) && (node.protocol_config.tls.mode === 'acme' || node.protocol_config.tls.mode === 'manual')
        && (node.protocol_config.tls.email === undefined || typeof node.protocol_config.tls.email === 'string')
        && (node.protocol_config.tls.configured === undefined || typeof node.protocol_config.tls.configured === 'boolean')
        && (node.protocol_config.tls.challenge === undefined || node.protocol_config.tls.challenge === 'http-01' || node.protocol_config.tls.challenge === 'tls-alpn-01'))))
    && new Set(value.map(node => node.id)).size === value.length
}
export function validServerList(value: unknown): value is PluginServer[] {
  return Array.isArray(value) && value.every(server => server && Number.isSafeInteger(server.id) && server.id > 0 && typeof server.name === 'string'
    && typeof server.enabled === 'boolean' && typeof server.online === 'boolean' && typeof server.read_only === 'boolean'
    && (server.installation === undefined || record(server.installation) && typeof server.installation.state === 'string' && typeof server.installation.reason === 'string'
      && Number.isSafeInteger(server.installation.target_rev) && Number(server.installation.target_rev) >= 0 && Number.isSafeInteger(server.installation.applied_rev) && Number(server.installation.applied_rev) >= 0))
    && new Set(value.map(server => server.id)).size === value.length
}
export function proxyWriteError(snapshot: ProxyWriteSnapshot, resource?: Pick<ProxyResource, 'kind' | 'id'>) {
  const error = resourceWriteError(snapshot.resources, snapshot.nodes, snapshot.servers)
  if (error) return error
  if (!validProxyResources(snapshot.resources.data) || !validNodeList(snapshot.nodes.data) || !validServerList(snapshot.servers.data)) return '最新资源信息格式不完整，暂不能修改；草稿已保留。'
  return resource && !snapshot.resources.data.some(item => proxyResourceKey(item) === proxyResourceKey(resource)) ? '此资源已不可用，暂不能提交。可关闭窗口后重新选择。' : ''
}
function managedSelectionError(snapshot: ProxyWriteSnapshot, entry: number | null, exit: number | null, entryServer?: number) {
  for (const id of [entry, exit].filter((id): id is number => id !== null)) {
    const node = snapshot.nodes.data?.find(node => node.id === id)
    const server = node && snapshot.servers.data?.find(server => server.id === node.server_id)
    if (!node || node.protocol !== 'vless-reality' || node.enabled === false || !server?.enabled) return '已选节点已不可用或所属服务器尚未启用，请重新选择；名称和其他草稿已保留。'
    if (snapshot.resources.data?.some(resource => resource.kind === 'chain' && resource.entry.id === id)) return '已选节点的链路身份已变更，不支持重复入口、嵌套或循环链路；请重新选择，草稿已保留。'
    const direct = snapshot.resources.data?.find(resource => resource.kind === 'direct' && resource.id === id)
    if (!direct) return '已选节点的链路身份已变更，请重新选择，草稿已保留。'
    if (!direct.available) return '已选节点已不可用，公开配置需要修复；草稿已保留。'
  }
  const entryNode = snapshot.nodes.data?.find(node => node.id === entry)
  const exitNode = snapshot.nodes.data?.find(node => node.id === exit)
  if (exitNode && (entryServer ?? entryNode?.server_id) === exitNode.server_id) return '入口和出口必须属于两台不同服务器；草稿已保留。'
  const entryResource = snapshot.resources.data?.find(resource => resource.kind === 'direct' && resource.id === entry)
  if (entryResource && (entryResource.chain_refs.length || entryResource.policy_group_ids.length || entryResource.user_count)) return '现有入口已有授权或链路引用，不能转为专用入口；请选择未授权且未被引用的节点。'
  return ''
}
export function chainBatchSelectionError(draft: ChainBatchDraft, snapshot: ProxyWriteSnapshot) {
  const error = proxyWriteError(snapshot)
  if (error) return error
  const entry = draft.mode === 'existing' ? nodeId(draft.entry_node_id) : null
  const exit = nodeId(draft.exit_node_id)
  const serverId = draft.mode === 'new' ? nodeId(draft.server_id) : undefined
  if (draft.mode === 'new' && draft.server_id && !snapshot.servers.data?.some(server => server.id === serverId && server.enabled)) return '已选入口服务器已不可用或尚未启用，请重新选择；草稿已保留。'
  if (draft.mode === 'existing' && draft.entry_node_id && entry === null || draft.exit_node_id && exit === null) return '请选择有效的入口和出口；草稿已保留。'
  return managedSelectionError(snapshot, entry, exit, serverId ?? undefined)
}
function publicHost(value: string) { return value.length <= 253 && !/[\s/?#@\\]/.test(value) && !value.includes('://') && (value.includes(':') ? /^\[?[0-9a-fA-F:]+\]?$/.test(value) : /^[a-zA-Z0-9.-]+$/.test(value)) }
export function prepareChainBatch(draft: ChainBatchDraft, snapshot: ProxyWriteSnapshot, previous?: PendingChainBatch, requestId: () => string = assignmentRequestId): PendingChainBatch {
  const writeError = proxyWriteError(snapshot)
  if (writeError) throw new Error(writeError)
  if (previous?.attempted && previous.source_draft === JSON.stringify(draft)) {
    savedBatchRequest(previous)
    return previous
  }
  const error = chainBatchSelectionError(draft, snapshot)
  if (error) throw new Error(error)
  if (!['new', 'existing'].includes(draft.mode) || draft.rows.length < 1 || draft.rows.length > 32 || draft.mode === 'existing' && draft.rows.length !== 1) throw new Error('新入口批次支持 1–32 条链路；现有入口只能创建一条。')
  const exit = nodeId(draft.exit_node_id), entry = nodeId(draft.entry_node_id), serverId = nodeId(draft.server_id)
  if (exit === null || draft.mode === 'existing' && entry === null || draft.mode === 'new' && serverId === null) throw new Error('请选择入口服务器或节点，以及出口节点；草稿已保留。')
  const host = draft.public_host.trim(), sni = draft.sni.trim()
  if (draft.mode === 'new' && (!publicHost(host) || !sni || sni.length > 253 || !/^[a-zA-Z0-9.-]+$/.test(sni))) throw new Error('请填写有效的公开域名或 IP，以及 Reality 协议域名；不包含协议、端口或路径。')
  const ports = new Set<number>()
  const items: ChainBatchRequest['items'] = draft.rows.map(row => {
    const name = row.name.trim(), port = row.port.trim() ? Number(row.port.trim()) : null
    if (!name || Array.from(name).length > 128) throw new Error('每条链路需填写不超过 128 个字符的名称；草稿已保留。')
    if (draft.mode === 'new' && port !== null) {
      if (!/^[1-9]\d*$/.test(row.port.trim()) || !Number.isSafeInteger(port) || port > 65535 || port === 18085) throw new Error('入口端口需为 1–65535 的整数，18085 为保留端口；留空可自动分配。')
      if (ports.has(port)) throw new Error('同一入口服务器上的批次端口不能重复；自动端口会分别分配。')
      ports.add(port)
    }
    return { name, entry: draft.mode === 'new' ? { mode: 'new', server_id: serverId!, public_host: host, sni, port } : { mode: 'existing', node_id: entry! }, hops: [{ kind: 'managed', node_id: exit }] }
  })
  const fingerprint = JSON.stringify(items)
  if (previous?.draft === fingerprint && previous.source_draft === JSON.stringify(draft)) return previous
  const request_id = requestId()
  return { draft: fingerprint, source_draft: JSON.stringify(draft), request_id, serialized: JSON.stringify({ request_id, items }), attempted: false }
}
function savedBatchRequest(pending: PendingChainBatch): ChainBatchRequest {
  const request = JSON.parse(pending.serialized) as ChainBatchRequest
  if (!request || request.request_id !== pending.request_id || !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(request.request_id)
    || !Array.isArray(request.items) || JSON.stringify(request.items) !== pending.draft || request.items.length < 1 || request.items.length > 32
    || JSON.stringify(request) !== pending.serialized) throw new Error('原批次信息不完整，请重新填写草稿。')
  return request
}
async function requestChainBatch(body: ChainBatchRequest) {
  const controller = new AbortController()
  const timer = window.setTimeout(() => controller.abort(), 30000)
  try { return await api(`${root}/chains/batch`, 'POST', body, controller.signal) }
  catch (error) { if (controller.signal.aborted) throw new Error('提交超时，批次可能已完成；请刷新确认或重试原批次，原请求已保留。'); throw error }
  finally { window.clearTimeout(timer) }
}
export async function submitChainBatch(pending: PendingChainBatch, snapshot: ProxyWriteSnapshot, write: (body: ChainBatchRequest) => Promise<unknown> = requestChainBatch) {
  const error = proxyWriteError(snapshot)
  if (error) throw new Error(error)
  const request = savedBatchRequest(pending)
  if (!pending.attempted) for (const item of request.items) {
    const entry = item.entry
    const selection = managedSelectionError(snapshot, entry.mode === 'existing' ? entry.node_id : null, item.hops[0]?.node_id ?? null, entry.mode === 'new' ? entry.server_id : undefined)
    if (selection) throw new Error(selection)
    if (entry.mode === 'new' && !snapshot.servers.data?.some(server => server.id === entry.server_id && server.enabled)) throw new Error('已选入口服务器已不可用或尚未启用；草稿已保留。')
  }
  pending.attempted = true
  return write(request)
}
export function proxyDeleteError(resources: ResourceSnapshot<ProxyResource[]>, resource: Pick<ProxyResource, 'kind' | 'id'>) {
  const error = resourceWriteError(resources)
  if (error) return error
  if (!validProxyResources(resources.data)) return '最新资源信息格式不完整，暂不能删除。'
  return !resources.data.some(item => proxyResourceKey(item) === proxyResourceKey(resource)) ? '此资源已不可用，暂不能提交。可关闭窗口后重新选择。' : ''
}
export async function deleteProxyResource(resource: Pick<ProxyResource, 'kind' | 'id'>, snapshot: ProxyWriteSnapshot, write: (path: string) => Promise<unknown> = path => api(path, 'DELETE')) {
  const error = proxyDeleteError(snapshot.resources, resource)
  if (error) throw new Error(error)
  return write(`${root}/proxy-resources/${resource.kind}/${resource.id}`)
}

export default function Chains({ snapshot, serverId, refresh, onCreated }: { snapshot: ProxyWriteSnapshot; serverId?: number; refresh: () => void; onCreated: (result: ChainBatchResult) => void }) {
  const action = useAction()
  const [creating, setCreating] = useState(false)
  const [draft, setDraft] = useState<ChainBatchDraft>({ mode: 'new', server_id: '', public_host: '', sni: '', entry_node_id: '', exit_node_id: '', rows: [{ name: '', port: '' }] })
  const pending = useRef<PendingChainBatch | undefined>(undefined)
  const [submittedDraft, setSubmittedDraft] = useState('')
  const [lastRequestId, setLastRequestId] = useState('')
  const enabledServers = snapshot.servers.data?.filter(server => server.enabled) ?? []
  const choices = snapshot.nodes.data?.filter(node => node.protocol === 'vless-reality' && node.enabled !== false && enabledServers.some(server => server.id === node.server_id)
    && snapshot.resources.data?.some(resource => resource.kind === 'direct' && resource.id === node.id && resource.available)) ?? []
  const writeError = proxyWriteError(snapshot)
  const unchanged = submittedDraft === JSON.stringify(draft)
  const replay = unchanged && pending.current?.attempted === true
  const selectionError = writeError || replay ? '' : chainBatchSelectionError(draft, snapshot)
  const update = (value: Partial<ChainBatchDraft>) => { action.clearError(); setDraft(current => ({ ...current, ...value })) }
  const rowUpdate = (index: number, value: Partial<ChainBatchDraft['rows'][number]>) => update({ rows: draft.rows.map((row, i) => i === index ? { ...row, ...value } : row) })
  const open = () => {
    if (action.busy || proxyWriteError(snapshot)) return
    action.clearError()
    if (!draft.server_id) setDraft(current => ({ ...current, server_id: enabledServers.find(server => server.id === serverId)?.id.toString() ?? enabledServers[0]?.id.toString() ?? '' }))
    setCreating(true)
  }
  const submit = () => {
    if (!creating || action.busy || proxyWriteError(snapshot) || !replay && chainBatchSelectionError(draft, snapshot)) return
    void action.run(async () => {
      const submission = prepareChainBatch(draft, snapshot, pending.current)
      pending.current = submission
      setSubmittedDraft(JSON.stringify(draft)); setLastRequestId(submission.request_id)
      const result = await submitChainBatch(submission, snapshot) as ChainBatchResult
      const ids = (value: unknown): value is number[] => Array.isArray(value) && value.length === draft.rows.length && value.every(id => Number.isSafeInteger(id) && id > 0) && new Set(value).size === value.length
      if (!result || result.request_id !== submission.request_id || !ids(result.chain_ids) || !ids(result.entry_node_ids)) throw new Error('面板返回的批次结果不完整，请刷新确认或重试原批次；原请求已保留。')
      return result
    }, result => { pending.current = undefined; setSubmittedDraft(''); setLastRequestId(''); setCreating(false); setDraft({ mode: 'new', server_id: '', public_host: '', sni: '', entry_node_id: '', exit_node_id: '', rows: [{ name: '', port: '' }] }); onCreated(result); refresh() })
  }
  return <>
    <button className="button button-primary" disabled={action.busy || Boolean(writeError)} onClick={open}>创建两跳链路</button>
    {creating && <FormDialog wide className="chain-editor" title="创建两跳链路" onClose={() => setCreating(false)} onSubmit={submit} busy={action.busy} disabled={Boolean(writeError)} submitDisabled={Boolean(selectionError)} error={writeError || selectionError || action.error} retry={writeError || selectionError ? refresh : undefined} submitLabel={lastRequestId && unchanged ? '重试原批次' : '创建未授权链路'}>
      <p className="helper">当前支持两台受管服务器的 VLESS + Reality 两跳链路。一次批量提交全部创建或全部拒绝；不支持外部订阅和多段混合链路。</p>
      <Field label="入口方式"><select name="entry_mode" value={draft.mode} onChange={event => update({ mode: event.target.value as 'new' | 'existing', rows: event.target.value === 'existing' ? draft.rows.slice(0, 1) : draft.rows })}><option value="new">新建 Reality 专用入口</option><option value="existing">使用现有未授权入口（单条）</option></select></Field>
      <div className="node-fields-grid">
        {draft.mode === 'new' ? <><Field label="入口服务器"><select name="server_id" required value={draft.server_id} onChange={event => update({ server_id: event.target.value })}><option value="" disabled>选择入口服务器</option>{draft.server_id && !enabledServers.some(server => String(server.id) === draft.server_id) && <option value={draft.server_id}>服务器 #{draft.server_id}（已不可用）</option>}{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}{server.online ? ' · 在线' : ' · 离线，等待应用'}</option>)}</select></Field><Field label="入口公开地址" hint="同批次共享客户端连接域名或 IP，不含协议、端口和路径。"><input name="public_host" required value={draft.public_host} onChange={event => update({ public_host: event.target.value })} placeholder="entry.example.com" autoComplete="off" /></Field><Field label="Reality 协议域名" hint="同批次共享 SNI。"><input name="sni" required value={draft.sni} onChange={event => update({ sni: event.target.value })} placeholder="www.example.com" autoComplete="off" /></Field></> : <Field label="入口节点" hint="只能选择未授权、无链路引用的独立 Reality 节点。"><select name="entry_node_id" required value={draft.entry_node_id} onChange={event => update({ entry_node_id: event.target.value })}><option value="" disabled>选择入口</option>{draft.entry_node_id && !choices.some(node => String(node.id) === draft.entry_node_id) && <option value={draft.entry_node_id}>节点 #{draft.entry_node_id}（已不可用，请重新选择）</option>}{choices.map(node => <option key={node.id} value={node.id}>{node.name}（服务器 #{node.server_id}）</option>)}</select></Field>}
        <Field label="出口节点" hint="必须与入口分属不同服务器，同一现有出口可被多条链路共享。"><select name="exit_node_id" required value={draft.exit_node_id} onChange={event => update({ exit_node_id: event.target.value })}><option value="" disabled>选择出口</option>{draft.exit_node_id && !choices.some(node => String(node.id) === draft.exit_node_id) && <option value={draft.exit_node_id}>节点 #{draft.exit_node_id}（已不可用，请重新选择）</option>}{choices.map(node => <option key={node.id} value={node.id}>{node.name}（服务器 #{node.server_id}）</option>)}</select></Field>
      </div>
      <h3>链路与入口端口 <span className="count">{draft.rows.length} / 32</span></h3>
      {draft.rows.map((row, index) => <div className="chain-draft-row" key={index}><Field label={`链路 ${index + 1} 名称`}><input name={index === 0 ? 'name' : `name_${index}`} required maxLength={128} value={row.name} onChange={event => rowUpdate(index, { name: event.target.value })} autoComplete="off" /></Field>{draft.mode === 'new' && <Field label={`入口端口 ${index + 1}`} hint="留空时分别自动分配；18085 为保留端口。"><input name={`entry_port_${index}`} type="number" min={1} max={65535} step={1} value={row.port} onChange={event => rowUpdate(index, { port: event.target.value })} placeholder="自动分配" /></Field>}{draft.rows.length > 1 && <button className="text-button danger-text" type="button" onClick={() => update({ rows: draft.rows.filter((_, i) => i !== index) })}>移除第 {index + 1} 条</button>}</div>)}
      {draft.mode === 'new' && <button type="button" className="button button-secondary button-small" disabled={draft.rows.length >= 32} onClick={() => update({ rows: [...draft.rows, { name: '', port: '' }] })}>添加一条链路</button>}
      <div className="chain-preview" aria-label="批次预览"><h3>批次预览</h3><p>{draft.rows.length} 条链路，共享入口服务器和出口；{draft.mode === 'new' ? '每条新建独立入口监听，自动端口由面板分别分配。' : '使用所选现有入口监听。'}</p><ol>{draft.rows.map((row, index) => <li key={index}><strong>{row.name.trim() || `链路 ${index + 1}`}</strong><span>{draft.mode === 'new' ? `${draft.public_host || '待填写地址'}:${row.port || '自动端口'}` : `入口节点 #${draft.entry_node_id || '待选择'}`} → 出口节点 #{draft.exit_node_id || '待选择'}</span></li>)}</ol></div>
      {lastRequestId && <p className="helper" role="status">批次编号：<code>{lastRequestId}</code>。{unchanged ? '请求内容已保留；失败或超时后重试会复用同一编号和内容。' : '草稿已修改，下次提交会使用新批次编号；可先刷新列表确认原批次结果。'}关闭窗口后仍可重新打开继续，离开此页面前请先确认提交结果。</p>}
      <p className="helper">创建成功不会自动授权给用户。加入策略组并分配后，还需等两端配置应用成功；设备在线和应用状态不代表已验证公网连通。</p>
    </FormDialog>}
  </>
}
