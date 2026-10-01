import { useState } from 'react'
import { api } from '../../api'
import { Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, PageHeader, Refresh, Stat } from '../../components'
import { bytes } from '../../format'
import { useAction, useResource } from '../../hooks'
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
  const [deleting, setDeleting] = useState<ProxyResource | null>(null)
  const [filter, setFilter] = useState(serverId === undefined ? '' : String(serverId))
  const [kind, setKind] = useState(chainsOnly ? 'chain' : '')
  const [creatingChain, setCreatingChain] = useState(false)
  const [savedChain, setSavedChain] = useState<number | null>(null)
  const [deployment, setDeployment] = useState<number | null>(null)
  const [saved, setSaved] = useState<number | null>(null)
  const enabledServers = servers.data?.filter(server => server.enabled) ?? []
  const canCreate = !servers.error && enabledServers.length > 0 && (!filter || enabledServers.some(server => server.id === Number(filter)))
  const all = resources.data ?? []
  const visible = all.filter(resource => (!filter || resource.server_id === Number(filter)) && (!kind || resource.kind === kind))
  const canCreateChain = canCreate && Boolean(nodes.data && resources.data && !nodes.error && !resources.error)
  const refresh = () => { nodes.reload(); resources.reload(); servers.reload(); usage.reload() }
  const edit = (node: Node | 'new') => { action.clearError(); setEditor(node) }
  const submit = (form: FormData) => {
    const fields = { name: String(form.get('name')).trim(), public_host: String(form.get('public_host')).trim(), sni: String(form.get('sni') ?? '').trim(), protocol_config: protocolRequest(form), enabled: form.get('enabled') === 'on', settings: nodeSettingsRequest(form) }
    const port = String(form.get('port') ?? '').trim()
    const selectedPort = port ? { port: Number(port) } : {}
    const request = editor === 'new' ? { ...fields, ...selectedPort, server_id: Number(form.get('server_id')) } : { ...fields, ...selectedPort }
    const serverId = editor === 'new' ? Number(form.get('server_id')) : editor!.server_id
    void action.run(() => api(editor === 'new' ? '/api/plugins/sing-box/nodes' : `/api/plugins/sing-box/nodes/${editor?.id}`, editor === 'new' ? 'POST' : 'PATCH', request), () => { setEditor(null); setSaved(serverId); refresh() })
  }
  return <div className="nodes-page">
    <PageHeader eyebrow="sing-box 插件" title="代理节点" description="统一管理直连节点与有序链路，自动发布完整服务器配置。"><Refresh onClick={refresh} /><button className="button button-secondary" disabled={!canCreateChain || creatingChain} onClick={() => { action.clearError(); setCreatingChain(true) }}>创建链路</button><button className="button button-primary" disabled={!canCreate} onClick={() => edit('new')}><Icon name="plus" size={18} />创建节点</button></PageHeader>
    <div className="stats-grid"><Stat icon="nodes" label="节点与链路" value={resources.data ? all.length : '—'} note="公开入口与内部角色分开显示" /><Stat icon="server" label="所在服务器" value={resources.data ? new Set(all.map(resource => resource.server_id)).size : '—'} note="每台服务器运行一份完整配置" /><Stat icon="activity" label="累计代理流量" value={usage.data ? bytes(usage.data.total) : '—'} note="含已删除节点的历史用量" /></div>
    <ErrorNotice message={nodes.error || resources.error || servers.error || usage.error} retry={refresh} />
    {saved !== null && <div className="notice" role="status"><span>节点已保存，正在等待自动发布与设备应用。</span><button className="text-button" onClick={() => setDeployment(saved)}>查看部署进度</button></div>}
    {savedChain !== null && <p className="notice" role="status">链路已保存，等待依赖与路径验证。<a className="text-button" href={resourceLink({ kind: 'chain', id: savedChain })}>查看链路详情</a></p>}
    {creatingChain && <ChainEditor nodes={nodes.data ?? []} resources={all} servers={enabledServers.filter(server => !filter || server.id === Number(filter))} onClose={() => setCreatingChain(false)} onSaved={receipt => { setCreatingChain(false); setSavedChain(receipt.chain_ids[0] ?? null); refresh() }} />}
    <section className="panel"><div className="panel-heading"><h2>全部节点与链路 <span className="count">{all.length}</span></h2><select aria-label="按类型筛选" value={kind} onChange={event => setKind(event.target.value)}><option value="">全部类型</option><option value="direct">直连节点</option><option value="chain">链路</option></select><select className="filter-select" aria-label="按服务器筛选" value={filter} onChange={event => setFilter(event.target.value)}><option value="">全部服务器</option>{filter && !enabledServers.some(server => server.id === Number(filter)) && <option value={filter}>指定服务器尚未启用或不存在</option>}{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}</option>)}</select></div>
      {resources.loading && !resources.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={filter ? '此服务器还没有节点' : kind === 'chain' ? '尚无链路' : '创建你的第一个节点'} description={enabledServers.length ? '填写端口或使用自动分配。为代理用户授权后，节点会自动启用。' : '先在系统的插件设置中为服务器启用 sing-box，再创建代理节点。'}>{enabledServers.length ? <button className="button button-primary" disabled={!canCreate} onClick={() => edit('new')}><Icon name="plus" size={17} />创建节点</button> : <a className="button button-primary" href="#/system/plugins">插件设置</a>}</Empty> : <ProxyResourceTable resources={visible} nodes={nodes.data ?? []} usage={usage.data ?? undefined} onEdit={edit} onDelete={resource => { action.clearError(); setDeleting(resource) }} onDeployment={setDeployment} />}
    </section>
    <div className="notice quiet-notice"><Icon name="check" size={18} /><div><strong>配置自动发布</strong><p>变更后等待 5 秒合并发布。未授权给任何用户的节点不会监听端口；订阅只包含设备已成功应用的配置。</p></div></div>
    {!creatingChain && <SubscriptionSources onChange={resources.reload} />}
    {selected && <ProxyResourceDetail key={`${selected.kind}-${selected.id}`} selected={selected} onClose={() => { window.location.hash = '/plugins/sing-box/nodes' }} onChanged={refresh} onEdit={node => { window.location.hash = '/plugins/sing-box/nodes'; edit(node) }} onDelete={resource => { window.location.hash = '/plugins/sing-box/nodes'; action.clearError(); setDeleting(resource) }} onDeployment={id => { window.location.hash = '/plugins/sing-box/nodes'; setDeployment(id) }} />}
    {editor && <FormDialog wide className="node-editor" title={editor === 'new' ? '创建节点' : '编辑节点'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} error={action.error} submitLabel={editor === 'new' ? '创建并自动发布' : '保存并自动发布'}>
      <h3>基本信息与连接地址</h3><div className="node-fields-grid">
      <Field label="节点名称"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="例如：香港 · 直连" autoComplete="off" /></Field>
      {editor === 'new' && <Field label="所属服务器"><select name="server_id" required defaultValue={filter || enabledServers[0]?.id}>{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}{server.online ? ' · 在线' : ' · 离线'}</option>)}</select></Field>}
      <Field label="监听端口" hint={editor === 'new' ? '可填写 443 等端口；留空时从 20000–29999 自动分配。18085 为保留端口。' : '修改后客户端需更新订阅；服务器上已有其他服务占用的端口不可使用。'}><input name="port" type="number" min={1} max={65535} step={1} required={editor !== 'new'} defaultValue={editor === 'new' ? '' : editor.port} placeholder="自动分配" /></Field>
      <Field label="公开地址" hint="填写客户端连接使用的域名或 IP，不含协议、端口和路径。"><input name="public_host" required defaultValue={editor === 'new' ? '' : editor.public_host} placeholder="node.example.com" autoComplete="off" spellCheck={false} /></Field>
      <ConnectionFields node={editor} />
      </div><h3>协议与安全</h3>
      <ProtocolFields key={editor === 'new' ? 'new' : editor.id} node={editor} />
    </FormDialog>}
    {deployment !== null && <NodeDeployment serverId={deployment} server={servers.data?.find(server => server.id === deployment)} onClose={() => setDeployment(null)} />}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} error={action.error} onClose={() => setDeleting(null)} onConfirm={() => void action.run(() => api(deleting.kind === 'chain' ? `/api/plugins/sing-box/proxy-resources/chain/${deleting.id}` : `/api/plugins/sing-box/nodes/${deleting.id}`, 'DELETE'), () => { setDeleting(null); refresh() })}>{deleting.kind === 'chain' ? '请先从策略组移除此链路。删除会同时停用专用入口和内部连接，保留历史流量与版本证据；设备应用新配置后停止监听。' : '此节点及其授权将从订阅中移除，历史流量会保留。被链路引用的节点不能直接删除。设备应用新配置后，代理入口停止监听。'}</Confirm>}
  </div>
}
