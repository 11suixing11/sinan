import { useEffect, useState } from 'react'
import { api } from '../../api'
import { Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, PageHeader, Refresh, Stat } from '../../components'
import { bytes, totalBytes } from '../../format'
import { useAction, useResource } from '../../hooks'
import type { Node, PluginServer, Usage } from '../../types'
import ProtocolFields, { protocolNames, protocolRequest } from './ProtocolFields'
import Chains from './Chains'
import type { Chain } from './groupTypes'

export default function Nodes({ serverId, initialKind = 'direct' }: { serverId?: number; initialKind?: 'direct' | 'chains' } = {}) {
  const direct = initialKind === 'direct'
  const nodes = useResource<Node[]>(direct ? '/api/plugins/sing-box/nodes' : null)
  const servers = useResource<PluginServer[]>(direct ? '/api/plugins/sing-box/servers' : null)
  const usage = useResource<Usage>(direct ? '/api/plugins/sing-box/usage' : null)
  const chains = useResource<Chain[]>(direct ? '/api/plugins/sing-box/chains' : null)
  const action = useAction()
  const [editor, setEditor] = useState<Node | 'new' | null>(null)
  const [deleting, setDeleting] = useState<Node | null>(null)
  const [filter, setFilter] = useState(serverId?.toString() ?? '')
  useEffect(() => { setFilter(serverId?.toString() ?? '') }, [serverId])
  const selectedServer = direct ? filter : serverId?.toString()
  const enabledServers = servers.data?.filter(server => server.enabled) ?? []
  const all = nodes.data ?? []
  const visible = all.filter(node => !filter || node.server_id === Number(filter))
  const refresh = () => { nodes.reload(); servers.reload(); usage.reload(); chains.reload() }
  const edit = (node: Node | 'new') => { action.clearError(); setEditor(node) }
  const submit = (form: FormData) => {
    const fields = { name: String(form.get('name')).trim(), public_host: String(form.get('public_host')).trim(), sni: String(form.get('sni') ?? '').trim(), protocol_config: protocolRequest(form) }
    const port = String(form.get('port') ?? '').trim()
    const selectedPort = port ? { port: Number(port) } : {}
    const request = editor === 'new' ? { ...fields, ...selectedPort, server_id: Number(form.get('server_id')) } : { ...fields, ...selectedPort }
    void action.run(() => api(editor === 'new' ? '/api/plugins/sing-box/nodes' : `/api/plugins/sing-box/nodes/${editor?.id}`, editor === 'new' ? 'POST' : 'PATCH', request), () => { setEditor(null); nodes.reload() })
  }
  return <>
    <PageHeader eyebrow="sing-box 插件" title="代理节点" description="统一管理服务器上的节点监听与两跳链路，授权后等待设备应用配置。">{direct && <><Refresh onClick={refresh} /><button className="button button-primary" disabled={!enabledServers.length} onClick={() => edit('new')}><Icon name="plus" size={18} />创建节点</button></>}</PageHeader>
    <nav className="group-tabs" aria-label="节点资源类型"><a className={`button ${direct ? 'button-primary' : 'button-secondary'}`} aria-current={direct ? 'page' : undefined} href={`#/plugins/sing-box/nodes${selectedServer ? `?server=${selectedServer}` : ''}`}>节点监听</a><a className={`button ${direct ? 'button-secondary' : 'button-primary'}`} aria-current={!direct ? 'page' : undefined} href={`#/plugins/sing-box/nodes?kind=chains${selectedServer ? `&server=${selectedServer}` : ''}`}>两跳链路</a></nav>
    {direct ? <>
    <div className="stats-grid"><Stat icon="nodes" label="节点监听数" value={nodes.data ? all.length : '—'} note="包含普通节点与链路入口、出口" /><Stat icon="server" label="所在服务器" value={nodes.data ? new Set(all.map(node => node.server_id)).size : '—'} note="每台服务器运行一份完整配置" /><Stat icon="activity" label="累计代理流量" value={usage.data ? bytes(usage.data.total) : '—'} note="含已删除节点的历史用量" /></div>
    <ErrorNotice message={nodes.error || servers.error || usage.error || chains.error} retry={refresh} />
    <section className="panel"><div className="panel-heading"><h2>全部节点 <span className="count">{all.length}</span></h2><select className="filter-select" aria-label="按服务器筛选" value={filter} onChange={event => setFilter(event.target.value)}><option value="">全部服务器</option>{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}</option>)}</select></div>
      {nodes.loading && !nodes.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={filter ? '此服务器还没有节点' : '创建你的第一个节点'} description={enabledServers.length ? '填写端口或使用自动分配。为代理用户授权后，等待设备成功应用配置，再连接节点。' : '先在系统的插件设置中为服务器启用 sing-box，再创建代理节点。'}>{enabledServers.length ? <button className="button button-primary" onClick={() => edit('new')}><Icon name="plus" size={17} />创建节点</button> : <a className="button button-primary" href="#/system/plugins">插件设置</a>}</Empty> : <div className="table-wrap"><table><thead><tr><th>节点</th><th>服务器</th><th>公开地址</th><th>协议域名</th><th>累计流量</th><th className="align-right">操作</th></tr></thead><tbody>{visible.map(node => {
        const record = usage.data?.by_node.find(record => record.node_id === node.id)
        const role = !chains.data || chains.error ? '链路身份待确认' : chains.data.some(chain => chain.entry_node_id === node.id) ? '链路专用入口' : chains.data.some(chain => chain.exit_node_id === node.id) ? '链路出口' : '普通节点监听'
        return <tr key={node.id}><td><div className="entity"><span className="entity-icon"><Icon name="nodes" size={18} /></span><div><strong>{node.name}</strong><small>{protocolNames[node.protocol] ?? node.protocol}</small><small>{role}</small></div></div></td><td><a className="text-button" href={`#/servers/${node.server_id}`}>{servers.data?.find(server => server.id === node.server_id)?.name ?? `服务器 #${node.server_id}`}</a></td><td><code>{node.public_host.includes(':') ? `[${node.public_host.replace(/^\[|\]$/g, '')}]` : node.public_host}:{node.port}</code></td><td><span className="mono">{node.sni || '无需证书'}</span></td><td>{record ? bytes(totalBytes(record.uplink, record.downlink)) : usage.data ? '0 B' : '暂无数据'}</td><td><div className="row-actions"><button className="text-button" onClick={() => edit(node)}>编辑</button><button className="text-button danger-text" onClick={() => { action.clearError(); setDeleting(node) }}>删除</button></div></td></tr>
      })}</tbody></table></div>}
    </section>
    <div className="notice quiet-notice"><Icon name="check" size={18} /><div><strong>创建后还需授权与部署</strong><p>创建节点只保存配置。普通节点需为代理用户授权并等待设备成功应用配置；未授权的普通节点在对应配置应用后不会监听端口。链路专用入口通过链路授权，出口可使用内部连接凭据监听，无需为出口单独授权用户。</p><a className="text-button" href="#/plugins/sing-box/users">创建或授权代理用户</a></div></div>
    {editor && <FormDialog title={editor === 'new' ? '创建节点' : '编辑节点'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} error={action.error} submitLabel={editor === 'new' ? '创建并自动发布' : '保存并自动发布'}>
      <Field label="节点名称"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="例如：香港 · 直连" autoComplete="off" /></Field>
      {editor === 'new' && <Field label="所属服务器"><select name="server_id" required defaultValue={filter || enabledServers[0]?.id}>{enabledServers.map(server => <option key={server.id} value={server.id}>{server.name}{server.online ? ' · 在线' : ' · 离线'}</option>)}</select></Field>}
      <Field label="监听端口" hint={editor === 'new' ? '可填写 443 等端口；留空时从 20000–29999 自动分配。18085 为保留端口。' : '修改后客户端需更新订阅；服务器上已有其他服务占用的端口不可使用。'}><input name="port" type="number" min={1} max={65535} step={1} required={editor !== 'new'} defaultValue={editor === 'new' ? '' : editor.port} placeholder="自动分配" /></Field>
      <Field label="公开地址" hint="填写客户端连接使用的域名或 IP，不含协议、端口和路径。"><input name="public_host" required defaultValue={editor === 'new' ? '' : editor.public_host} placeholder="node.example.com" autoComplete="off" spellCheck={false} /></Field>
      <ProtocolFields key={editor === 'new' ? 'new' : editor.id} node={editor} />
    </FormDialog>}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} error={action.error} onClose={() => setDeleting(null)} onConfirm={() => void action.run(() => api(`/api/plugins/sing-box/nodes/${deleting.id}`, 'DELETE'), () => { setDeleting(null); refresh() })}>此节点及其授权将从订阅中移除，历史流量会保留。设备应用新配置后，代理入口停止监听。</Confirm>}
    </> : <Chains serverId={serverId} />}
  </>
}
