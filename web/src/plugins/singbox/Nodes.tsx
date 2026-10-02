import { useRef, useState } from 'react'
import { api } from '../../api'
import { Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, PageHeader, Refresh, Stat } from '../../components'
import { bytes } from '../../format'
import { resourceWriteError, useAction, useResource } from '../../hooks'
import type { Node, PluginServer, Usage } from '../../types'
import ProtocolFields, { protocolRequest } from './ProtocolFields'
import { ConnectionFields, nodeSettingsRequest } from './NodeSettingsFields'
import NodeDeployment from './NodeDeployment'
import SubscriptionSources from './SubscriptionSources'
import ChainEditor from './ChainEditor'
import ProxyResourceDetail from './ProxyResourceDetail'
import ProxyResourceTable from './ProxyResourceTable'
import type { ProxyResource, ResourceKey } from './resourceTypes'
import { resourceLink } from './resourceTypes'
import './nodes.css'

export default function Nodes({ serverId, chainsOnly = false, selected }: { serverId?: number; chainsOnly?: boolean; selected?: ResourceKey }) {
  const nodes = useResource<Node[]>('/api/plugins/sing-box/nodes')
  const resources = useResource<ProxyResource[]>('/api/plugins/sing-box/proxy-resources')
  const servers = useResource<PluginServer[]>('/api/plugins/sing-box/servers')
  const usage = useResource<Usage>('/api/plugins/sing-box/usage')
  const action = useAction()
  const [editor, setEditor] = useState<Node | 'new' | null>(null)
  const [draftServer, setDraftServer] = useState('')
  const [deleting, setDeleting] = useState<ProxyResource | null>(null)
  const [filter, setFilter] = useState(serverId === undefined ? '' : String(serverId))
  const currentFilter = useRef(filter)
  const [kind, setKind] = useState(chainsOnly ? 'chain' : ''), [role, setRole] = useState(''), [search, setSearch] = useState('')
  const [creatingChain, setCreatingChain] = useState(false)
  const [createdChains, setCreatedChains] = useState<number[]>([])
  const [deployment, setDeployment] = useState<number | null>(null)
  const [saved, setSaved] = useState<number | null>(null)
  const enabledServers = servers.data?.filter(server => server.enabled) ?? []
  const canCreate = !resourceWriteError(nodes, resources, servers) && enabledServers.length > 0 && (!filter || enabledServers.some(server => server.id === Number(filter)))
  const all = resources.data ?? []
  const visible = all.filter(resource => (!filter || resource.server_id === Number(filter)) && (!kind || resource.kind === kind) && (!role || resource.role === role) && (!search || `${resource.name} ${resource.public_host}`.toLocaleLowerCase().includes(search.toLocaleLowerCase())))
  const canCreateChain = canCreate && Boolean(nodes.data && resources.data && !nodes.error && !resources.error)
  const refresh = () => { nodes.reload(); resources.reload(); servers.reload(); usage.reload() }
  const creationError = () => resourceWriteError(nodes, resources, servers) || (!servers.getCurrent()?.some(server => server.enabled && (!currentFilter.current || server.id === Number(currentFilter.current))) ? '当前筛选中没有可用的入口服务器，请明确选择后再创建。' : '')
  const writeError = (value?: { kind: 'direct' | 'chain'; id: number }) => {
    const stale = resourceWriteError(nodes, resources, servers)
    if (stale) return stale
    return value && !resources.getCurrent()?.some(item => item.kind === value.kind && item.id === value.id) ? '此代理资源已不存在，请重新选择；当前草稿已保留。' : ''
  }
  const editorError = editor ? writeError(editor === 'new' ? undefined : { kind: 'direct', id: editor.id }) || (editor === 'new' && !servers.getCurrent()?.some(server => String(server.id) === draftServer && server.enabled) ? '已选服务器已不存在或未启用，请明确重新选择；当前草稿已保留。' : '') : ''
  const currentEditor = editor && editor !== 'new' ? nodes.data?.find(node => node.id === editor.id) : null
  const connectionLocked = Boolean(currentEditor?.configuration_locked || (editor && editor !== 'new' && editor.configuration_locked))
  const deletingError = deleting ? writeError(deleting) : ''
  const edit = (node: Node | 'new') => { if (writeError(node === 'new' ? undefined : { kind: 'direct', id: node.id }) || (node === 'new' && creationError())) return; action.clearError(); setDraftServer(currentFilter.current || String(servers.getCurrent()?.find(server => server.enabled)?.id ?? '')); setEditor(node) }
  const submit = (form: FormData) => {
    if (!editor || writeError(editor === 'new' ? undefined : { kind: 'direct', id: editor.id }) || (editor === 'new' && !servers.getCurrent()?.some(server => String(server.id) === draftServer && server.enabled))) return
    const fields = { name: String(form.get('name')).trim(), public_host: String(form.get('public_host')).trim(), sni: String(form.get('sni') ?? '').trim(), protocol_config: protocolRequest(form), enabled: form.get('enabled') === 'on', settings: nodeSettingsRequest(form) }
    const port = String(form.get('port') ?? '').trim()
    const selectedPort = port ? { port: Number(port) } : {}
    const locked = connectionLocked || (editor !== 'new' && nodes.getCurrent()?.find(node => node.id === editor.id)?.configuration_locked)
    const request = editor === 'new' ? { ...fields, ...selectedPort, server_id: Number(draftServer) } : locked ? { name: fields.name, enabled: fields.enabled } : { ...fields, ...selectedPort }
    const serverId = editor === 'new' ? Number(draftServer) : editor!.server_id
    void action.run(() => api(editor === 'new' ? '/api/plugins/sing-box/nodes' : `/api/plugins/sing-box/nodes/${editor?.id}`, editor === 'new' ? 'POST' : 'PATCH', request), () => { setEditor(null); setSaved(serverId); refresh() })
  }
  return <div className="nodes-page" onInvalidCapture={event => { if (event.target instanceof HTMLElement) { const details = event.target.closest('details'); if (details) details.open = true } }}>
    <PageHeader eyebrow="sing-box 插件" title="代理节点" description="统一管理直连节点与有序链路；从受管节点或订阅来源选择中间段和最终出口。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={!canCreate} onClick={() => edit('new')}><Icon name="plus" size={18} />创建节点</button><button className="button button-secondary" disabled={!canCreateChain || creatingChain} onClick={() => { if (creationError()) return; action.clearError(); setCreatingChain(true); setCreatedChains([]) }}>创建链路</button></PageHeader>
    <div className="stats-grid"><Stat icon="nodes" label="代理资源" value={resources.data ? all.length : '—'} note="直连节点与独立链路入口" /><Stat icon="server" label="所在服务器" value={resources.data ? new Set(all.map(node => node.server_id)).size : '—'} note="每台服务器运行一份完整配置" /><Stat icon="activity" label="累计代理流量" value={usage.data ? bytes(usage.data.total) : '—'} note="含已删除节点的历史用量" /></div>
    <ErrorNotice message={resources.error || nodes.error || servers.error || usage.error} retry={refresh} />
    {saved !== null && <div className="notice" role="status"><span>资源已保存，正在等待自动发布与设备应用。</span><button className="text-button" onClick={() => setDeployment(saved)}>查看部署进度</button></div>}
    {!!createdChains.length && <div className="notice" role="status"><span>已保存 {createdChains.length} 条链路，正在等待依赖与路径验证。</span><a href={resourceLink({ kind: 'chain', id: createdChains[0] })}>查看链路详情</a></div>}
    {creatingChain && <ChainEditor writeError={() => writeError()} getCurrent={() => ({ nodes: nodes.getCurrent() ?? [], resources: resources.getCurrent() ?? [], servers: servers.getCurrent()?.filter(server => server.enabled) ?? [], entryServerIds: servers.getCurrent()?.filter(server => server.enabled && (!currentFilter.current || server.id === Number(currentFilter.current))).map(server => server.id) ?? [] })} nodes={nodes.data ?? []} resources={all} servers={enabledServers.filter(server => !filter || server.id === Number(filter))} availableServers={enabledServers} onClose={() => setCreatingChain(false)} onSaved={receipt => { setCreatingChain(false); setCreatedChains(receipt.chain_ids); refresh() }} />}
    <section className="panel"><div className="panel-heading resource-list-heading"><h2>全部代理资源 <span className="count">{all.length}</span></h2><div className="resource-filters"><div className="search-box resource-search"><input aria-label="搜索代理资源" placeholder="搜索名称或公开地址" value={search} onChange={event => setSearch(event.target.value)} /></div><select className="filter-select" aria-label="按服务器筛选" value={filter} onChange={event => { currentFilter.current = event.target.value; setFilter(event.target.value) }}><option value="">全部服务器</option>{filter && !enabledServers.some(server => server.id === Number(filter)) && <option value={filter}>指定服务器尚未启用或不存在</option>}{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}</option>)}</select><select className="filter-select" aria-label="按类型筛选" value={kind} onChange={event => setKind(event.target.value)}><option value="">全部类型</option><option value="direct">直连节点</option><option value="chain">链路</option></select><select className="filter-select" aria-label="按角色筛选" value={role} onChange={event => setRole(event.target.value)}><option value="">全部角色</option><option value="direct">仅作直连</option><option value="managed_hop">受管内部段</option><option value="chain_entry">独立链路入口</option></select></div></div>
      {resources.loading && !resources.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={filter || kind || role || search ? '没有符合条件的代理资源' : '创建你的第一个节点'} description={enabledServers.length ? '创建直连节点，或使用独立入口与有序代理段创建链路。' : '先在系统的插件设置中为服务器启用 sing-box，再创建代理节点。'}>{enabledServers.length ? <button className="button button-primary" disabled={!canCreate} onClick={() => edit('new')}><Icon name="plus" size={17} />创建节点</button> : <a className="button button-primary" href="#/system/plugins">插件设置</a>}</Empty> : <ProxyResourceTable resources={visible} nodes={nodes.data ?? []} usage={usage.data ?? undefined} onEdit={edit} onDelete={resource => { if (writeError(resource)) return; action.clearError(); setDeleting(resource) }} onDeployment={setDeployment} />}
    </section>
    <div className="notice quiet-notice"><Icon name="check" size={18} /><div><strong>配置自动发布</strong><p>变更后等待 5 秒合并发布。未授权给任何用户的节点不会监听端口；订阅只包含设备已成功应用的配置。</p></div></div>
    {!creatingChain && <SubscriptionSources onChange={resources.reload} />}
    {selected && <ProxyResourceDetail key={`${selected.kind}-${selected.id}`} selected={selected} onClose={() => { window.location.hash = '/plugins/sing-box/nodes' }} onChanged={refresh} onEdit={node => { window.location.hash = '/plugins/sing-box/nodes'; edit(node) }} onDelete={resource => { if (writeError(resource)) return; window.location.hash = '/plugins/sing-box/nodes'; action.clearError(); setDeleting(resource) }} onDeployment={id => { window.location.hash = '/plugins/sing-box/nodes'; setDeployment(id) }} />}
    {editor && <FormDialog wide className="node-editor" title={editor === 'new' ? '创建节点' : '编辑节点'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} submitDisabled={Boolean(editorError)} error={editorError || action.error} submitLabel={editor === 'new' ? '创建并自动发布' : '保存并自动发布'}>
      <h3>基本信息</h3><div className="node-fields-grid">
      <Field label="节点名称"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="例如：香港 · 直连" autoComplete="off" /></Field>
      {editor === 'new' && <Field label="所属服务器"><select name="server_id" required value={draftServer} onChange={event => setDraftServer(event.target.value)}>{draftServer && !enabledServers.some(server => String(server.id) === draftServer) && <option value={draftServer}>已选服务器已不存在或未启用（原选择保留）</option>}{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}{server.online ? ' · 在线' : ' · 离线'}</option>)}</select></Field>}
      <label className="node-switch"><input name="enabled" type="checkbox" defaultChecked={editor === 'new' || editor.enabled !== false} /><span>启用节点<small>停用保留授权与历史流量。</small></span></label>
      </div>
      {connectionLocked && <div className="node-lock-notice" role="status">此节点已被链路引用，当前可修改名称和启用状态。调整连接参数请先替换链路中的节点。<br />{(currentEditor?.referenced_chains ?? (editor === 'new' ? [] : editor.referenced_chains) ?? []).map(chain => <a key={chain.id} href={resourceLink({ kind: 'chain', id: chain.id })} onClick={() => setEditor(null)}>{chain.name}</a>)}</div>}
      <fieldset className="node-connection-fields" disabled={connectionLocked}><h3>连接地址</h3><div className="node-fields-grid">
      <Field label="监听端口" hint={editor === 'new' ? '可填写 443 等端口；留空时从 20000–29999 自动分配。18085 为保留端口。' : '修改后客户端需更新订阅；服务器上已有其他服务占用的端口不可使用。'}><input name="port" type="number" min={1} max={65535} step={1} required={editor !== 'new'} defaultValue={editor === 'new' ? '' : editor.port} placeholder="自动分配" /></Field>
      <Field label="公开地址" hint="填写客户端连接使用的域名或 IP，不含协议、端口和路径。"><input name="public_host" required defaultValue={editor === 'new' ? '' : editor.public_host} placeholder="node.example.com" autoComplete="off" spellCheck={false} /></Field>
      <ConnectionFields node={editor} />
      </div><h3>协议与安全</h3>
      <ProtocolFields key={editor === 'new' ? 'new' : editor.id} node={editor} />
      </fieldset>
    </FormDialog>}
    {deployment !== null && <NodeDeployment serverId={deployment} server={servers.data?.find(server => server.id === deployment)} onClose={() => setDeployment(null)} />}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} confirmDisabled={Boolean(deletingError)} error={deletingError || action.error} onClose={() => setDeleting(null)} onConfirm={() => { if (writeError(deleting)) return; void action.run(() => api(deleting.kind === 'chain' ? `/api/plugins/sing-box/proxy-resources/chain/${deleting.id}` : `/api/plugins/sing-box/nodes/${deleting.id}`, 'DELETE'), () => { setDeleting(null); refresh() }) }}>{deleting.kind === 'chain' ? '请先从策略组移除此链路。删除会同时停用专用入口和内部连接，保留历史流量与版本证据；设备应用新配置后停止监听。' : '此节点及其授权将从订阅中移除，历史流量会保留。被链路引用的节点不能直接删除。设备应用新配置后，代理入口停止监听。'}</Confirm>}
  </div>
}
