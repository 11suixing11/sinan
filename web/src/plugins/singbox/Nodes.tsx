import { useState } from 'react'
import { api } from '../../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, PageHeader, Refresh, Stat } from '../../components'
import { bytes, totalBytes } from '../../format'
import { useAction, useResource } from '../../hooks'
import type { Node, PluginServer, Usage } from '../../types'
import ProtocolFields, { protocolNames, protocolRequest } from './ProtocolFields'
import { ConnectionFields, nodeSettingsRequest } from './NodeSettingsFields'
import NodeDeployment from './NodeDeployment'
import './nodes.css'

export default function Nodes() {
  const nodes = useResource<Node[]>('/api/plugins/sing-box/nodes')
  const servers = useResource<PluginServer[]>('/api/plugins/sing-box/servers')
  const usage = useResource<Usage>('/api/plugins/sing-box/usage')
  const action = useAction()
  const [editor, setEditor] = useState<Node | 'new' | null>(null)
  const [deleting, setDeleting] = useState<Node | null>(null)
  const [filter, setFilter] = useState('')
  const [deployment, setDeployment] = useState<number | null>(null)
  const [saved, setSaved] = useState<number | null>(null)
  const enabledServers = servers.data?.filter(server => server.enabled) ?? []
  const all = nodes.data ?? []
  const visible = all.filter(node => !filter || node.server_id === Number(filter))
  const refresh = () => { nodes.reload(); servers.reload(); usage.reload() }
  const edit = (node: Node | 'new') => { action.clearError(); setEditor(node) }
  const submit = (form: FormData) => {
    const fields = { name: String(form.get('name')).trim(), public_host: String(form.get('public_host')).trim(), sni: String(form.get('sni') ?? '').trim(), protocol_config: protocolRequest(form), enabled: form.get('enabled') === 'on', settings: nodeSettingsRequest(form) }
    const port = String(form.get('port') ?? '').trim()
    const selectedPort = port ? { port: Number(port) } : {}
    const request = editor === 'new' ? { ...fields, ...selectedPort, server_id: Number(form.get('server_id')) } : { ...fields, ...selectedPort }
    const serverId = editor === 'new' ? Number(form.get('server_id')) : editor!.server_id
    void action.run(() => api(editor === 'new' ? '/api/plugins/sing-box/nodes' : `/api/plugins/sing-box/nodes/${editor?.id}`, editor === 'new' ? 'POST' : 'PATCH', request), () => { setEditor(null); setSaved(serverId); nodes.reload() })
  }
  return <div className="nodes-page">
    <PageHeader eyebrow="sing-box 插件" title="代理节点" description="管理代理协议、TLS 证书与授权，自动发布完整服务器配置。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={!enabledServers.length} onClick={() => edit('new')}><Icon name="plus" size={18} />创建节点</button></PageHeader>
    <div className="stats-grid"><Stat icon="nodes" label="节点总数" value={nodes.data ? all.length : '—'} note="端口可指定，密钥自动生成" /><Stat icon="server" label="所在服务器" value={nodes.data ? new Set(all.map(node => node.server_id)).size : '—'} note="每台服务器运行一份完整配置" /><Stat icon="activity" label="累计代理流量" value={usage.data ? bytes(usage.data.total) : '—'} note="含已删除节点的历史用量" /></div>
    <ErrorNotice message={nodes.error || servers.error || usage.error} retry={refresh} />
    {saved !== null && <div className="notice" role="status"><span>节点已保存，正在等待自动发布与设备应用。</span><button className="text-button" onClick={() => setDeployment(saved)}>查看部署进度</button></div>}
    <section className="panel"><div className="panel-heading"><h2>全部节点 <span className="count">{all.length}</span></h2><select className="filter-select" aria-label="按服务器筛选" value={filter} onChange={event => setFilter(event.target.value)}><option value="">全部服务器</option>{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}</option>)}</select></div>
      {nodes.loading && !nodes.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={filter ? '此服务器还没有节点' : '创建你的第一个节点'} description={enabledServers.length ? '填写端口或使用自动分配。为代理用户授权后，节点会自动启用。' : '先在系统的插件设置中为服务器启用 sing-box，再创建代理节点。'}>{enabledServers.length ? <button className="button button-primary" onClick={() => edit('new')}><Icon name="plus" size={17} />创建节点</button> : <a className="button button-primary" href="#/system/plugins">插件设置</a>}</Empty> : <div className="table-wrap"><table><thead><tr><th>节点</th><th>服务器</th><th>公开地址</th><th>协议域名</th><th>累计流量</th><th className="align-right">操作</th></tr></thead><tbody>{visible.map(node => {
        const record = usage.data?.by_node.find(record => record.node_id === node.id)
        return <tr key={node.id}><td><div className="entity"><span className="entity-icon"><Icon name="nodes" size={18} /></span><div><strong>{node.name}</strong><small>{protocolNames[node.protocol] ?? node.protocol}</small>{node.enabled === false && <Badge tone="warm">已设为停用</Badge>}</div></div></td><td><a className="text-button" href={`#/servers/${node.server_id}`}>{servers.data?.find(server => server.id === node.server_id)?.name ?? `服务器 #${node.server_id}`}</a></td><td><code>{node.public_host.includes(':') ? `[${node.public_host.replace(/^\[|\]$/g, '')}]` : node.public_host}:{node.settings?.public_port ?? node.port}</code><small className="node-listen">监听 {node.settings?.listen ?? '::'} / {node.port}</small></td><td><span className="mono">{node.sni || '无需证书'}</span></td><td>{record ? bytes(totalBytes(record.uplink, record.downlink)) : usage.data ? '0 B' : '暂无数据'}</td><td><div className="row-actions"><button className="text-button" onClick={() => setDeployment(node.server_id)}>部署</button><button className="text-button" onClick={() => edit(node)}>编辑</button><button className="text-button danger-text" onClick={() => { action.clearError(); setDeleting(node) }}>删除</button></div></td></tr>
      })}</tbody></table></div>}
    </section>
    <div className="notice quiet-notice"><Icon name="check" size={18} /><div><strong>配置自动发布</strong><p>变更后等待 5 秒合并发布。未授权给任何用户的节点不会监听端口；订阅只包含设备已成功应用的配置。</p></div></div>
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
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} error={action.error} onClose={() => setDeleting(null)} onConfirm={() => void action.run(() => api(`/api/plugins/sing-box/nodes/${deleting.id}`, 'DELETE'), () => { setDeleting(null); refresh() })}>此节点及其授权将从订阅中移除，历史流量会保留。设备应用新配置后，代理入口停止监听。</Confirm>}
  </div>
}
