import { useState } from 'react'
import { api } from '../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, PageHeader, Refresh, Stat } from '../components'
import { bytes, totalBytes } from '../format'
import { useAction, useResource } from '../hooks'
import type { Node, Server, Usage } from '../types'

export default function Nodes() {
  const nodes = useResource<Node[]>('/api/nodes')
  const servers = useResource<Server[]>('/api/servers')
  const usage = useResource<Usage>('/api/usage')
  const action = useAction()
  const [editor, setEditor] = useState<Node | 'new' | null>(null)
  const [deleting, setDeleting] = useState<Node | null>(null)
  const [filter, setFilter] = useState('')
  const all = nodes.data ?? []
  const visible = all.filter(node => !filter || node.server_id === Number(filter))
  const refresh = () => { nodes.reload(); servers.reload(); usage.reload() }
  const edit = (node: Node | 'new') => { action.clearError(); setEditor(node) }
  const submit = (form: FormData) => {
    const fields = { name: String(form.get('name')).trim(), public_host: String(form.get('public_host')).trim(), sni: String(form.get('sni')).trim() }
    const request = editor === 'new' ? { ...fields, server_id: Number(form.get('server_id')) } : fields
    void action.run(() => api(editor === 'new' ? '/api/nodes' : `/api/nodes/${editor?.id}`, editor === 'new' ? 'POST' : 'PATCH', request), () => { setEditor(null); nodes.reload() })
  }
  return <>
    <PageHeader eyebrow="代理网络" title="节点" description="使用 VLESS + Reality，让授权用户连接你的服务器。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={!servers.data?.length} onClick={() => edit('new')}><Icon name="plus" size={18} />创建节点</button></PageHeader>
    <div className="stats-grid"><Stat icon="nodes" label="节点总数" value={nodes.data ? all.length : '—'} note="端口与密钥由面板自动分配" /><Stat icon="server" label="所在服务器" value={nodes.data ? new Set(all.map(node => node.server_id)).size : '—'} note="每台服务器运行一份完整配置" /><Stat icon="activity" label="累计代理流量" value={usage.data ? bytes(usage.data.total) : '—'} note="含已删除节点的历史用量" /></div>
    <ErrorNotice message={nodes.error || servers.error || usage.error} retry={refresh} />
    <section className="panel"><div className="panel-heading"><h2>全部节点 <span className="count">{all.length}</span></h2><select className="filter-select" aria-label="按服务器筛选" value={filter} onChange={event => setFilter(event.target.value)}><option value="">全部服务器</option>{servers.data?.map(server => <option key={server.id} value={server.id}>{server.name}</option>)}</select></div>
      {nodes.loading && !nodes.data ? <Loading /> : !visible.length ? <Empty icon="nodes" title={filter ? '此服务器还没有节点' : '创建你的第一个节点'} description={servers.data?.length ? '面板将生成密钥并分配端口。为用户授权后，节点会自动启用。' : '先添加并接入服务器，再创建代理节点。'}>{servers.data?.length ? <button className="button button-primary" onClick={() => edit('new')}><Icon name="plus" size={17} />创建节点</button> : <a className="button button-primary" href="#/servers">前往服务器</a>}</Empty> : <div className="table-wrap"><table><thead><tr><th>节点</th><th>服务器</th><th>公开地址</th><th>伪装域名</th><th>累计流量</th><th className="align-right">操作</th></tr></thead><tbody>{visible.map(node => {
        const record = usage.data?.by_node.find(record => record.node_id === node.id)
        return <tr key={node.id}><td><div className="entity"><span className="entity-icon"><Icon name="nodes" size={18} /></span><div><strong>{node.name}</strong><small>VLESS + Reality</small></div></div></td><td><a className="text-button" href={`#/servers/${node.server_id}`}>{servers.data?.find(server => server.id === node.server_id)?.name ?? `服务器 #${node.server_id}`}</a></td><td><code>{node.public_host.includes(':') ? `[${node.public_host.replace(/^\[|\]$/g, '')}]` : node.public_host}:{node.port}</code></td><td><span className="mono">{node.sni}</span></td><td>{record ? bytes(totalBytes(record.uplink, record.downlink)) : usage.data ? '0 B' : '暂无数据'}</td><td><div className="row-actions"><button className="text-button" onClick={() => edit(node)}>编辑</button><button className="text-button danger-text" onClick={() => { action.clearError(); setDeleting(node) }}>删除</button></div></td></tr>
      })}</tbody></table></div>}
    </section>
    <div className="notice quiet-notice"><Icon name="check" size={18} /><div><strong>配置自动发布</strong><p>变更后等待 5 秒合并发布。未授权给任何用户的节点不会监听端口；订阅只包含设备已成功应用的配置。</p></div></div>
    {editor && <FormDialog title={editor === 'new' ? '创建节点' : '编辑节点'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} error={action.error} submitLabel={editor === 'new' ? '创建并自动发布' : '保存并自动发布'}>
      <Field label="节点名称"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="例如：香港 · 直连" autoComplete="off" /></Field>
      {editor === 'new' ? <Field label="所属服务器"><select name="server_id" required defaultValue={filter || servers.data?.[0]?.id}>{servers.data?.map(server => <option key={server.id} value={server.id}>{server.name}{server.online ? ' · 在线' : ' · 离线'}</option>)}</select></Field> : <div className="field-static"><span>协议与端口</span><div><Badge>VLESS + Reality</Badge><code>:{editor.port}</code></div></div>}
      <Field label="公开地址" hint="填写客户端连接使用的域名或 IP，不含协议、端口和路径。"><input name="public_host" required defaultValue={editor === 'new' ? '' : editor.public_host} placeholder="node.example.com" autoComplete="off" spellCheck={false} /></Field>
      <Field label="伪装域名（SNI）" hint="填写可从服务器访问、支持 TLS 的目标域名。"><input name="sni" required defaultValue={editor === 'new' ? '' : editor.sni} placeholder="www.example.com" autoComplete="off" spellCheck={false} /></Field>
      {editor === 'new' && <p className="helper">端口、Reality 密钥和 short ID 会自动生成。</p>}
    </FormDialog>}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} error={action.error} onClose={() => setDeleting(null)} onConfirm={() => void action.run(() => api(`/api/nodes/${deleting.id}`, 'DELETE'), () => { setDeleting(null); refresh() })}>此节点及其授权将从订阅中移除，历史流量会保留。设备应用新配置后，代理入口停止监听。</Confirm>}
  </>
}
