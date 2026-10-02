import { useEffect, useRef, useState } from 'react'
import { api } from '../../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, Modal, PageHeader, Refresh, Stat } from '../../components'
import { bytes, totalBytes } from '../../format'
import { useAction, useResource } from '../../hooks'
import type { Node, PluginServer, Usage } from '../../types'
import ProtocolFields, { protocolNames, protocolRequest } from './ProtocolFields'
import Chains, { deleteProxyResource, proxyDeleteError, proxyWriteError, validNodeList, validServerList } from './Chains'
import type { ProxyWriteSnapshot } from './Chains'
import { dateText, filterProxyResources, proxyResourceCounts, proxyResourceKey, validatedSnapshot, validProxyResource, validProxyResources } from './groupTypes'
import type { ProxyResource, ProxyResourceFilter, ResourceEndpoint } from './groupTypes'
import { ConnectionFields, nodeSettingsRequest } from './NodeSettingsFields'
import NodeDeployment from './NodeDeployment'
import { installationView } from './Settings'
import Sources from './Sources'
import './nodes.css'

const root = '/api/plugins/sing-box'
function address(endpoint: ResourceEndpoint) {
  const host = endpoint.public_host.includes(':') ? `[${endpoint.public_host.replace(/^\[|\]$/g, '')}]` : endpoint.public_host
  return `${host}:${endpoint.public_port}`
}
function applicationView(endpoint: ResourceEndpoint, server?: PluginServer, fresh = false) {
  if (!fresh || !server?.installation) return { label: '应用状态待确认', tone: 'neutral' as const, reason: '尚未取得最新设备应用状态，请查看服务器详情。' }
  const view = installationView(server)
  if (server.installation.state === 'ready') {
    if (!endpoint.server_deleted && endpoint.plugin_enabled && endpoint.online && endpoint.desired_revision !== null && endpoint.desired_revision > 0
      && endpoint.applied_revision === endpoint.desired_revision && endpoint.applied_observed_at !== null && server.enabled && server.online
      && server.installation.target_rev === endpoint.desired_revision && server.installation.applied_rev === endpoint.applied_revision) return { ...view, label: '目标配置已应用' }
    return { label: '应用状态待确认', tone: 'warm' as const, reason: '设备状态与目标版本尚未确认一致，请查看服务器详情。' }
  }
  return { ...view, label: server.installation.state === 'queued' ? '等待生成配置' : view.label }
}
function EndpointView({ endpoint, server, node, fresh }: { endpoint: ResourceEndpoint; server?: PluginServer; node?: Node; fresh: boolean }) {
  const application = applicationView(endpoint, server, fresh)
  return <div className="resource-endpoint"><strong>{endpoint.name}</strong><a className="text-button" href={`#/servers/${endpoint.server_id}`}>{fresh ? endpoint.server_name : `服务器 #${endpoint.server_id}`}</a><code>{address(endpoint)}</code><small>{protocolNames[endpoint.protocol] ?? endpoint.protocol} · {endpoint.sni || '无需证书'}</small>
    {node && <small className="node-listen">监听 {node.settings?.listen ?? '::'} / {endpoint.port}</small>}
    {!endpoint.enabled && <Badge tone="warm">已设为停用</Badge>}
    <small><Badge tone={fresh && endpoint.online ? 'good' : 'neutral'}>{fresh ? endpoint.online ? '设备在线' : '设备离线' : '设备状态待确认'}</Badge></small>
    <small><Badge tone={application.tone}>{application.label}</Badge></small>
    {fresh && endpoint.desired_revision !== null && endpoint.applied_revision !== null && <small>目标版本 {endpoint.desired_revision} · 已应用版本 {endpoint.applied_revision}</small>}
    <small>{application.reason}</small>
    {endpoint.node_deleted && <small>节点已删除，保留链路记录供清理。</small>}{endpoint.server_deleted && <small>服务器已删除，保留链路记录供清理。</small>}
  </div>
}
function ResourceDetail({ selected, snapshot, onClose }: { selected: ProxyResource; snapshot: ProxyWriteSnapshot; onClose: () => void }) {
  const query = useResource<unknown>(`${root}/proxy-resources/${selected.kind}/${selected.id}`)
  const history = useRef(selected)
  const current = validatedSnapshot(query, validProxyResource, history.current)
  const sameIdentity = current.data && proxyResourceKey(current.data) === proxyResourceKey(selected)
  if (sameIdentity && current.fresh) history.current = current.data!
  const resource = sameIdentity ? current.data! : history.current
  const error = current.error || (!sameIdentity ? '资源详情与所选记录不一致，请刷新确认。' : '')
  const fresh = current.fresh && sameIdentity === true && !proxyDeleteError(snapshot.resources, selected)
  const deviceFresh = fresh && snapshot.servers.fresh && !snapshot.servers.error
  return <Modal wide title={`资源详情：${resource.name}`} onClose={onClose}><div className="modal-body resource-details"><ErrorNotice message={error} retry={query.reload} />
    {!fresh && <p className="helper" role="status">当前显示已取得的历史信息，资源与设备状态等待最新确认。</p>}
    <dl className="resource-facts"><div><dt>资源类型</dt><dd>{resource.kind === 'direct' ? '直连节点' : '受管两跳链路'}</dd></div><div><dt>资源标识</dt><dd><code>{proxyResourceKey(resource)}</code></dd></div><div><dt>授权范围</dt><dd>{resource.policy_group_ids.length} 个策略组 · {resource.user_count} 位代理用户</dd></div></dl>
    <p className="helper">授权人数表示当前授权关系，不代表套餐仍然有效。配置可用、设备在线和目标配置应用分别记录，尚未验证公网可达或链路连通。</p>
    <div className="resource-topology"><section><h3>{resource.kind === 'chain' ? '入口' : '直连监听'}</h3><EndpointView endpoint={resource.entry} server={snapshot.servers.data?.find(server => server.id === resource.entry.server_id)} node={snapshot.nodes.fresh ? snapshot.nodes.data?.find(node => node.id === resource.entry.id) : undefined} fresh={deviceFresh} /></section>{resource.exit && <><span className="topology-arrow" aria-hidden="true">→</span><section><h3>共享出口</h3><EndpointView endpoint={resource.exit} server={snapshot.servers.data?.find(server => server.id === resource.exit!.server_id)} node={snapshot.nodes.fresh ? snapshot.nodes.data?.find(node => node.id === resource.exit!.id) : undefined} fresh={deviceFresh} /></section></>}</div>
    <p><Badge tone={!fresh ? 'neutral' : resource.available ? 'good' : 'bad'}>{!fresh ? '资源状态待确认' : resource.available ? '资源存在' : '资源已不可用'}</Badge></p>
    {!!resource.unavailable_reasons.length && <ul className="resource-reasons">{resource.unavailable_reasons.map((reason, index) => <li key={index}>{reason}</li>)}</ul>}
    <p className="helper">策略组：{resource.policy_group_ids.map(id => `#${id}`).join('、') || '尚未加入'}。{resource.chain_refs.length ? `作为共享出口被 ${resource.chain_refs.map(ref => `「${ref.name}」#${ref.id}`).join('、')} 引用。` : '没有共享出口链路引用。'}</p>
    <p className="helper">入口最后应用观测：{dateText(resource.entry.applied_observed_at)}{resource.exit && `；出口：${dateText(resource.exit.applied_observed_at)}`}。</p>
  </div><footer><Refresh onClick={query.reload} /><button className="button button-secondary" onClick={onClose}>关闭</button></footer></Modal>
}

export default function Nodes({ serverId, initialKind = 'all' }: { serverId?: number; initialKind?: ProxyResourceFilter } = {}) {
  const resourcesQuery = useResource<unknown>(`${root}/proxy-resources`)
  const nodesQuery = useResource<unknown>(`${root}/nodes`)
  const serversQuery = useResource<unknown>(`${root}/servers`)
  const usage = useResource<Usage>(`${root}/usage`)
  const history = useRef<{ resources?: ProxyResource[]; nodes?: Node[]; servers?: PluginServer[] }>({})
  const resources = validatedSnapshot(resourcesQuery, validProxyResources, history.current.resources)
  const nodes = validatedSnapshot(nodesQuery, validNodeList, history.current.nodes)
  const servers = validatedSnapshot(serversQuery, validServerList, history.current.servers)
  if (resources.fresh) history.current.resources = resources.data
  if (nodes.fresh) history.current.nodes = nodes.data
  if (servers.fresh) history.current.servers = servers.data
  const snapshot: ProxyWriteSnapshot = { resources, nodes, servers }
  const action = useAction()
  const [editor, setEditor] = useState<Node | 'new' | null>(null)
  const [deleting, setDeleting] = useState<ProxyResource | null>(null)
  const [detail, setDetail] = useState<ProxyResource | null>(null)
  const [deployment, setDeployment] = useState<number | null>(null)
  const [saved, setSaved] = useState<number | null>(null)
  const [batchSaved, setBatchSaved] = useState(0)
  const [filter, setFilter] = useState(serverId?.toString() ?? '')
  const [kind, setKind] = useState(initialKind)
  useEffect(() => { setFilter(serverId?.toString() ?? '') }, [serverId])
  useEffect(() => { setKind(initialKind) }, [initialKind])
  const all = resources.data ?? []
  const counts = proxyResourceCounts(all)
  const visible = filterProxyResources(all, kind, filter ? Number(filter) : undefined)
  const enabledServers = servers.data?.filter(server => server.enabled) ?? []
  const filterServer = servers.data?.find(server => server.id === Number(filter))
  const writeError = proxyWriteError(snapshot)
  const canCreate = !writeError && enabledServers.length > 0 && (!filter || enabledServers.some(server => server.id === Number(filter)))
  const refresh = () => { resourcesQuery.reload(); nodesQuery.reload(); serversQuery.reload(); usage.reload() }
  const selectedResource = editor && editor !== 'new' ? { kind: 'direct' as const, id: editor.id } : undefined
  const editorError = proxyWriteError(snapshot, selectedResource)
  const deleteError = deleting ? proxyDeleteError(resources, deleting) : ''
  const edit = (node: Node | 'new') => {
    if (action.busy || proxyWriteError(snapshot, node === 'new' ? undefined : { kind: 'direct', id: node.id }) || node === 'new' && !canCreate) return
    action.clearError(); setEditor(node)
  }
  const remove = (resource: ProxyResource) => { if (action.busy || proxyDeleteError(resources, resource)) return; action.clearError(); setDeleting(resource) }
  const submit = (form: FormData) => {
    if (!editor || action.busy || proxyWriteError(snapshot, selectedResource)) return
    const current = editor === 'new' ? undefined : nodes.data?.find(node => node.id === editor.id)
    const selectedServer = editor === 'new' ? Number(form.get('server_id')) : current?.server_id
    void action.run(async () => {
      if (selectedServer === undefined || !servers.data?.some(server => server.id === selectedServer && server.enabled)) throw new Error('所属服务器已不可用或尚未启用；草稿已保留。')
      if (editor !== 'new' && !current) throw new Error('节点已不可用，暂不能保存；草稿已保留。')
      const fields = { name: String(form.get('name') ?? '').trim(), public_host: String(form.get('public_host') ?? '').trim(), sni: String(form.get('sni') ?? '').trim(), protocol_config: protocolRequest(form), enabled: form.get('enabled') === 'on', settings: nodeSettingsRequest(form) }
      const port = String(form.get('port') ?? '').trim()
      const selectedPort = port ? { port: Number(port) } : {}
      const request = editor === 'new' ? { ...fields, ...selectedPort, server_id: selectedServer } : { ...fields, ...selectedPort }
      return api(editor === 'new' ? `${root}/nodes` : `${root}/nodes/${editor.id}`, editor === 'new' ? 'POST' : 'PATCH', request)
    }, () => { setEditor(null); setSaved(selectedServer!); refresh() })
  }
  const route = (category: ProxyResourceFilter) => `#/plugins/sing-box/nodes${category === 'all' ? filter ? `?server=${filter}` : '' : `?kind=${category}${filter ? `&server=${filter}` : ''}`}`
  const title = kind === 'chains' ? '两跳链路' : kind === 'direct' ? '直连节点' : '全部代理资源'
  return <div className="nodes-page">
    <PageHeader eyebrow="sing-box 插件" title="代理节点" description="统一管理直连节点和受管两跳链路；创建后还需授权，并等待设备应用配置。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={!canCreate || action.busy} onClick={() => edit('new')}><Icon name="plus" size={18} />创建节点</button><Chains snapshot={snapshot} serverId={filter ? Number(filter) : undefined} refresh={refresh} onCreated={result => setBatchSaved(result.chain_ids.length)} /></PageHeader>
    <nav className="group-tabs" aria-label="节点资源类型">{([['all', '全部'], ['direct', '直连节点'], ['chains', '两跳链路']] as const).map(([category, label]) => <a key={category} className={`button ${kind === category ? 'button-primary' : 'button-secondary'}`} aria-current={kind === category ? 'page' : undefined} href={route(category)}>{label}</a>)}</nav>
    <div className="stats-grid"><Stat icon="nodes" label="代理资源数" value={resources.data ? counts.total : '—'} note={`${counts.direct} 个直连 · ${counts.chains} 条链路${resources.fresh ? '' : ' · 等待最新确认'}`} /><Stat icon="server" label="物理监听数" value={nodes.fresh ? nodes.data!.length : '—'} note="包含专用入口；共享出口只计一次，尚未确认实际监听" /><Stat icon="activity" label="累计代理流量" value={usage.data ? bytes(usage.data.total) : '—'} note="含已删除节点的历史用量" /></div>
    <ErrorNotice message={resources.error || nodes.error || servers.error || usage.error} retry={refresh} />
    {saved !== null && <div className="notice" role="status"><span>节点已保存，正在等待自动发布与设备应用。</span><button className="text-button" onClick={() => setDeployment(saved)}>查看部署进度</button></div>}
    {batchSaved > 0 && <div className="notice" role="status"><span>{batchSaved} 条链路已原子创建，尚未授权给代理用户；两端配置仍需应用。</span><a className="text-button" href="#/plugins/sing-box/groups">管理策略组与套餐</a></div>}
    <section className="panel"><div className="panel-heading"><h2>{title} <span className="count">{visible.length}</span></h2><select className="filter-select" aria-label="按服务器筛选" value={filter} onChange={event => setFilter(event.target.value)}><option value="">全部服务器</option>{filter && !servers.data?.some(server => server.id === Number(filter)) && <option value={filter}>服务器 #{filter}（信息待确认）</option>}{servers.data?.map(server => <option key={server.id} value={server.id}>{server.name}{server.enabled ? '' : ' · 未启用'}</option>)}</select></div>
      <div className="panel-body"><p className="helper">{kind === 'chains' ? filter ? `筛选范围：入口或出口属于${filterServer ? `「${filterServer.name}」` : `服务器 #${filter}`}的链路。` : '筛选范围：全部服务器的链路。' : filter ? `筛选范围：任一端点属于${filterServer ? `「${filterServer.name}」` : `服务器 #${filter}`}的资源。` : '筛选范围：全部服务器的代理资源。'}{filter && <a className="text-button" href={kind === 'all' ? '#/plugins/sing-box/nodes' : `#/plugins/sing-box/nodes?kind=${kind}`}>{kind === 'chains' ? '查看全部链路' : '查看全部资源'}</a>}</p>{writeError && <p className="helper" role="status">{writeError} 已取得的列表保留供查看，资源状态等待确认。</p>}</div>
      {resourcesQuery.loading && !resources.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={filter ? kind === 'chains' ? '此服务器暂无已确认关联的链路' : '此服务器暂无代理资源' : '创建你的第一个代理资源'} description={enabledServers.length ? '填写端口或使用自动分配。为代理用户授权后，等待设备成功应用配置，再连接节点。' : '先在系统的插件设置中为服务器启用 sing-box，再创建代理节点。'}>{!enabledServers.length && <a className="button button-primary" href="#/system/plugins">插件设置</a>}</Empty> : <div className="table-wrap"><table className="proxy-resource-table"><thead><tr><th>资源</th><th>入口或直连监听</th><th>出口</th><th>资源与授权</th><th>累计流量</th><th>操作</th></tr></thead><tbody>{visible.map(resource => {
        const node = nodes.data?.find(node => node.id === resource.entry.id)
        const record = usage.data?.by_node.find(record => record.node_id === resource.entry.id)
        return <tr key={proxyResourceKey(resource)} data-resource-key={proxyResourceKey(resource)}><td><strong>{resource.name}</strong><small>{resource.kind === 'direct' ? '直连节点' : '受管两跳链路'}</small><small><code>{proxyResourceKey(resource)}</code></small>{resource.chain_refs.length > 0 && <small>共享出口 · {resource.chain_refs.length} 条链路引用</small>}</td>
          <td><EndpointView endpoint={resource.entry} node={nodes.fresh ? node : undefined} server={servers.data?.find(server => server.id === resource.entry.server_id)} fresh={resources.fresh && servers.fresh} /></td><td>{resource.exit ? <EndpointView endpoint={resource.exit} node={nodes.fresh ? nodes.data?.find(node => node.id === resource.exit!.id) : undefined} server={servers.data?.find(server => server.id === resource.exit!.server_id)} fresh={resources.fresh && servers.fresh} /> : '—'}</td>
          <td><Badge tone={!resources.fresh ? 'neutral' : resource.available ? 'good' : 'bad'}>{!resources.fresh ? '资源状态待确认' : resource.available ? '资源存在' : '资源已不可用'}</Badge><small>{resource.policy_group_ids.length} 个策略组 · {resource.user_count} 位代理用户</small>{resource.unavailable_reasons.map((reason, index) => <small key={index}>{reason}</small>)}</td><td>{record ? bytes(totalBytes(record.uplink, record.downlink)) : usage.data ? '0 B' : '暂无数据'}{resource.kind === 'chain' && <small>按入口计量，不重复累计出口</small>}</td>
          <td><div className="row-actions"><button className="text-button" onClick={() => setDetail(resource)}>详情</button><button className="text-button" onClick={() => setDeployment(resource.entry.server_id)}>部署</button>{resource.kind === 'direct' && <button className="text-button" disabled={action.busy || Boolean(writeError) || !node} onClick={() => { if (node) edit(node) }}>编辑</button>}<button className="text-button danger-text" disabled={action.busy || Boolean(proxyDeleteError(resources, resource))} onClick={() => remove(resource)}>删除</button></div></td></tr>
      })}</tbody></table></div>}
    </section>
    <div className="notice quiet-notice"><Icon name="check" size={18} /><div><strong>创建后还需授权与部署</strong><p>普通节点需为代理用户授权并等待设备成功应用配置；未授权的普通节点在对应配置应用后不会监听端口。链路专用入口只显示在链路内，出口可使用内部连接凭据监听，无需为出口单独授权用户。两端状态仅表示设备应用与健康信息，尚未验证公网可达或链路连通。</p><a className="text-button" href="#/plugins/sing-box/users">创建或授权代理用户</a></div></div>
    {editor && <FormDialog wide className="node-editor" title={editor === 'new' ? '创建节点' : '编辑节点'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} disabled={Boolean(editorError)} error={editorError || action.error} retry={editorError ? refresh : undefined} submitLabel={editor === 'new' ? '创建并自动发布' : '保存并自动发布'}>
      <h3>基本信息与连接地址</h3><div className="node-fields-grid"><Field label="节点名称"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="例如：香港 · 直连" autoComplete="off" /></Field>
      {editor === 'new' && <Field label="所属服务器"><select name="server_id" required defaultValue={filter || enabledServers[0]?.id}>{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}{server.online ? ' · 在线' : ' · 离线'}</option>)}</select></Field>}
      <Field label="监听端口" hint={editor === 'new' ? '可填写 443 等端口；留空时从 20000–29999 自动分配。18085 为保留端口。' : '修改后客户端需更新订阅；服务器上已有其他服务占用的端口不可使用。'}><input name="port" type="number" min={1} max={65535} step={1} required={editor !== 'new'} defaultValue={editor === 'new' ? '' : editor.port} placeholder="自动分配" /></Field>
      <Field label="公开地址" hint="填写客户端连接使用的域名或 IP，不含协议、端口和路径。"><input name="public_host" required defaultValue={editor === 'new' ? '' : editor.public_host} placeholder="node.example.com" autoComplete="off" spellCheck={false} /></Field><ConnectionFields node={editor} /></div><h3>协议与安全</h3><ProtocolFields key={editor === 'new' ? 'new' : editor.id} node={editor} />
    </FormDialog>}
    {deployment !== null && <NodeDeployment serverId={deployment} server={servers.fresh ? servers.data?.find(server => server.id === deployment) : undefined} onClose={() => setDeployment(null)} />}
    <Sources />
    {detail && <ResourceDetail selected={detail} snapshot={snapshot} onClose={() => setDetail(null)} />}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} disabled={Boolean(deleteError)} error={deleteError || action.error} retry={deleteError ? refresh : undefined} onClose={() => setDeleting(null)} onConfirm={() => { if (!deleting || action.busy || proxyDeleteError(resources, deleting)) return; void action.run(() => deleteProxyResource(deleting, snapshot), () => { setDeleting(null); refresh() }) }}>
      {deleting.kind === 'chain' ? '删除链路会删除其专用入口，并撤销内部连接凭据；共享出口保留，历史流量保留。' : '删除直连节点会移除监听配置及直接用户授权，历史流量保留。'}存在策略组或链路引用时，面板会拒绝删除并列出引用；请先解除对应关系，再重试。设备应用新配置后才完成监听与内部连接的撤销。
    </Confirm>}
  </div>
}
