import { useEffect, useState } from 'react'
import UserEntitlements from './UserEntitlements'
import SubscriptionDialog from './SubscriptionDialog'
import type { SubscriptionFormat } from './SubscriptionDialog'
import type { Chain } from './groupTypes'
import { api } from '../../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, Modal, PageHeader, Refresh, Stat } from '../../components'
import { bytes, totalBytes } from '../../format'
import { resourceWriteError, useAction, useResource } from '../../hooks'
import type { Access, Node, Usage, ProxyUser } from '../../types'

export default function ProxyUsers() {
  const users = useResource<ProxyUser[]>('/api/plugins/sing-box/users')
  const nodes = useResource<Node[]>('/api/plugins/sing-box/nodes')
  const chains = useResource<Chain[]>('/api/plugins/sing-box/chains')
  const usage = useResource<Usage>('/api/plugins/sing-box/usage')
  const action = useAction()
  const [selected, setSelected] = useState<number | null>(null)
  const [editor, setEditor] = useState<ProxyUser | 'new' | null>(null)
  const [deleting, setDeleting] = useState<ProxyUser | null>(null)
  const [subscription, setSubscription] = useState<ProxyUser | null>(null)
  const [resetting, setResetting] = useState<ProxyUser | null>(null)
  const [format, setFormat] = useState<SubscriptionFormat>('singbox')
  const [notice, setNotice] = useState('')
  const [search, setSearch] = useState('')
  useEffect(() => {
    if (users.data) setSelected(previous => users.data?.some(user => user.id === previous) ? previous : users.data?.[0]?.id ?? null)
  }, [users.data])
  const accesses = useResource<Access[]>(selected ? `/api/plugins/sing-box/users/${selected}/accesses` : null)
  const selectedUsage = useResource<Usage>(selected ? `/api/plugins/sing-box/usage?user_id=${selected}` : null)
  const userWriteError = resourceWriteError(users)
  const userError = (id?: number) => userWriteError || (id !== undefined && !users.data?.some(user => user.id === id) ? '此代理用户已不可用，暂不能提交。草稿已保留，可关闭窗口后重新选择。' : '')
  const editorWriteError = userError(editor && editor !== 'new' ? editor.id : undefined)
  const deleteWriteError = userError(deleting?.id)
  const resetWriteError = userError(resetting?.id)
  const grantWriteError = resourceWriteError(users, nodes, chains, accesses)
  const user = users.data?.find(user => user.id === selected)
  const record = usage.data?.by_user.find(record => record.user_id === selected)
  const refresh = () => { users.reload(); nodes.reload(); chains.reload(); usage.reload(); accesses.reload(); selectedUsage.reload() }
  const edit = (value: ProxyUser | 'new') => { if (userWriteError) return; action.clearError(); setEditor(value) }
  const submit = (form: FormData) => {
    if (editorWriteError || !editor) return
    void action.run(() => api<ProxyUser>(editor === 'new' ? '/api/plugins/sing-box/users' : `/api/plugins/sing-box/users/${editor?.id}`, editor === 'new' ? 'POST' : 'PATCH', { name: String(form.get('name')).trim() }), value => { setEditor(null); setSelected(value.id); users.reload() })
  }
  const grant = (node: Node, checked: boolean) => {
    if (!selected || grantWriteError) return
    setNotice('')
    void action.run(() => checked ? api(`/api/plugins/sing-box/users/${selected}/accesses`, 'POST', { node_id: node.id }) : api(`/api/plugins/sing-box/users/${selected}/accesses/${node.id}`, 'DELETE'), () => { accesses.reload(); setNotice(checked ? '授权已保存，设备应用新配置后会出现在订阅中。' : '授权已撤销，已从订阅移除；运行时将在新配置应用后更新。') })
  }
  const visible = users.data?.filter(user => user.name.toLocaleLowerCase().includes(search.toLocaleLowerCase())) ?? []
  return <>
    <PageHeader eyebrow="访问管理" title="代理用户" description="分别为代理用户分配策略组和套餐，管理订阅，并查看实际代理流量。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={action.busy || Boolean(userWriteError)} onClick={() => edit('new')}><Icon name="plus" size={18} />创建代理用户</button></PageHeader>
    <div className="stats-grid"><Stat icon="users" label="代理用户总数" value={users.data?.length ?? '—'} note="每个代理用户拥有独立订阅" /><Stat icon="up" label="累计上传" value={usage.data ? bytes(usage.data.uplink) : '—'} note="认证端实际统计的上传流量" /><Stat icon="down" label="累计下载" value={usage.data ? bytes(usage.data.downlink) : '—'} note="含已删除代理用户的历史用量" /></div>
    <ErrorNotice message={users.error || nodes.error || chains.error || usage.error} retry={refresh} />
    {users.loading && !users.data ? <section className="panel"><Loading /></section> : !users.data?.length ? <section className="panel"><Empty icon="users" title="创建代理用户，开始分配节点" description="每个代理用户在每个节点上的凭据相互独立，流量按代理用户和节点统计。"><button className="button button-primary" disabled={action.busy || Boolean(userWriteError)} onClick={() => edit('new')}><Icon name="plus" size={17} />创建代理用户</button></Empty></section> : <div className="users-layout"><section className="panel users-list"><div className="panel-heading"><h2>代理用户列表 <span className="count">{users.data.length}</span></h2></div><div className="search-box"><input aria-label="搜索代理用户" placeholder="搜索代理用户名称…" value={search} onChange={event => setSearch(event.target.value)} /></div><div className="user-roster">{visible.map(entry => { const total = usage.data?.by_user.find(record => record.user_id === entry.id); return <button key={entry.id} className={`user-row ${selected === entry.id ? 'selected' : ''}`} onClick={() => { setSelected(entry.id); setNotice(''); action.clearError() }} disabled={action.busy} aria-pressed={selected === entry.id}><span className="avatar">{entry.name.slice(0, 1)}</span><span><strong>{entry.name}</strong><small>{total ? bytes(totalBytes(total.uplink, total.downlink)) : usage.data ? '尚无流量记录' : '流量加载中…'}</small></span><Icon name="arrow" size={15} /></button> })}{!visible.length && <p className="inline-empty">没有匹配的代理用户。</p>}</div></section><div className="user-content">{user ? <><section className="panel"><div className="user-heading"><div className="entity"><span className="avatar avatar-large">{user.name.slice(0, 1)}</span><div><h2>{user.name}</h2><span className="subtle">代理用户 #{user.id}</span></div></div><div className="row-actions"><button className="text-button" disabled={action.busy || Boolean(userWriteError)} onClick={() => edit(user)}>编辑</button><button className="text-button danger-text" disabled={action.busy || Boolean(userWriteError)} onClick={() => { if (userWriteError) return; action.clearError(); setDeleting(user) }}>删除</button></div></div><div className="user-usage"><div><span>上传</span><strong>{record ? bytes(record.uplink) : usage.data ? '0 B' : '暂无数据'}</strong></div><div><span>下载</span><strong>{record ? bytes(record.downlink) : usage.data ? '0 B' : '暂无数据'}</strong></div><div><span>已授权节点</span><strong>{accesses.data?.length ?? '—'}</strong></div></div><div className="subscription-row"><div><strong>代理用户专属订阅</strong><p>仅包含已应用且健康的授权节点。</p></div><button className="button button-primary button-small" onClick={() => { setFormat('singbox'); setSubscription(user) }}><Icon name="copy" size={15} />订阅链接</button></div></section><UserEntitlements key={user.id} id={user.id} onChange={refresh} userWriteError={userWriteError} /><section className="panel"><div className="panel-heading"><h2>单独节点授权</h2><span className="subtle">更改后自动发布</span></div><div className="panel-body"><ErrorNotice message={grantWriteError || (!editor && !deleting && !resetting ? action.error : '')} retry={grantWriteError ? refresh : undefined} />{notice && <div className="notice notice-success" role="status">{notice}</div>}{accesses.loading && !accesses.data ? <Loading /> : !nodes.data?.length ? <Empty icon="nodes" title="还没有可授权的节点" description="先创建节点，再为代理用户开启访问权限。"><a className="button button-secondary" href="#/plugins/sing-box/nodes">前往节点</a></Empty> : <div className="grant-list">{nodes.data.filter(node => !chains.data?.some(chain => chain.entry_node_id === node.id)).map(node => { const access = accesses.data?.find(access => access.node_id === node.id); return <label className="grant-row" key={node.id}><span className="entity-icon"><Icon name="nodes" size={18} /></span><span className="grant-info"><strong>{node.name}</strong><small>{node.public_host}:{node.port}</small></span>{access && <Badge tone="good">{access.direct_grant ? '单独授权' : '来自策略组'}</Badge>}<input className="switch-input" type="checkbox" checked={Boolean(access?.direct_grant)} disabled={action.busy || Boolean(grantWriteError)} onChange={event => grant(node, event.target.checked)} aria-label={`授权 ${node.name}`} /><span className="switch" aria-hidden="true" /></label> })}</div>}<p className="helper">此开关只管理单独授权，不取消策略组授予的权限。只有全部授权来源移除后才撤销凭据；再次授权需要更新订阅。链路只能通过策略组分配。设备离线时等待重连应用。</p></div></section><section className="panel"><div className="panel-heading"><h2>该代理用户的节点流量</h2><span className="subtle">包含历史记录</span></div><ErrorNotice message={selectedUsage.error} retry={selectedUsage.reload} />{selectedUsage.loading && !selectedUsage.data ? <Loading /> : selectedUsage.data?.by_node.length ? <div className="table-wrap"><table><thead><tr><th>节点</th><th>上传</th><th>下载</th><th>合计</th></tr></thead><tbody>{selectedUsage.data.by_node.map(record => <tr key={record.node_id}><td>{record.name}{record.deleted && <span className="inline-tag">已删除</span>}</td><td>{bytes(record.uplink)}</td><td>{bytes(record.downlink)}</td><td>{bytes(totalBytes(record.uplink, record.downlink))}</td></tr>)}</tbody></table></div> : <div className="inline-empty">此代理用户尚无节点流量记录。使用代理后，通常在 1 至 2 分钟内更新。</div>}</section></> : <section className="panel"><Loading /></section>}</div></div>}
    {usage.data?.by_user.some(record => record.deleted) && <section className="panel"><div className="panel-heading"><h2>历史代理用户流量</h2><span className="subtle">已删除代理用户的数据仍保留</span></div><div className="table-wrap"><table><thead><tr><th>代理用户</th><th>上传</th><th>下载</th></tr></thead><tbody>{usage.data.by_user.filter(record => record.deleted).map(record => <tr key={record.user_id}><td>{record.name} <span className="inline-tag">已删除</span></td><td>{bytes(record.uplink)}</td><td>{bytes(record.downlink)}</td></tr>)}</tbody></table></div></section>}
    {editor && <FormDialog title={editor === 'new' ? '创建代理用户' : '编辑代理用户'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} disabled={Boolean(editorWriteError)} error={editorWriteError || action.error} retry={editorWriteError ? users.reload : undefined} submitLabel={editor === 'new' ? '创建代理用户' : '保存修改'}><Field label="代理用户名称"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="为使用者设置一个名称" autoComplete="off" /></Field></FormDialog>}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} disabled={Boolean(deleteWriteError)} error={deleteWriteError || action.error} retry={deleteWriteError ? users.reload : undefined} onClose={() => setDeleting(null)} onConfirm={() => { if (deleteWriteError) return; void action.run(() => api(`/api/plugins/sing-box/users/${deleting.id}`, 'DELETE'), () => { setDeleting(null); setSelected(null); refresh() }) }}>此代理用户的订阅链接将失效，全部节点授权会被撤销。历史用量保留，设备应用新配置后停止接受旧凭据。</Confirm>}
    {resetting && <Modal title={`重置「${resetting.name}」的订阅链接？`} busy={action.busy} onClose={() => setResetting(null)}><div className="modal-body"><ErrorNotice message={resetWriteError || action.error} retry={resetWriteError ? users.reload : undefined} /><p className="confirm-copy">旧链接将立即失效，代理用户需要在客户端换成新链接。现有节点连接凭据和授权保持不变，已下载的配置仍可使用。</p></div><footer><button className="button button-secondary" disabled={action.busy} onClick={() => setResetting(null)}>取消</button><button className="button button-danger" disabled={action.busy || Boolean(resetWriteError)} onClick={() => { if (resetWriteError) return; void action.run(() => api<ProxyUser>(`/api/plugins/sing-box/users/${resetting.id}/subscription/reset`, 'POST'), value => { setResetting(null); setSubscription(value); users.reload(); setNotice('订阅链接已重置，请将新链接提供给代理用户。') }) }}>{action.busy ? '正在重置…' : '确认重置'}</button></footer></Modal>}
    {subscription && <SubscriptionDialog user={subscription} format={format} onFormatChange={setFormat} onClose={() => setSubscription(null)} onReset={() => { action.clearError(); setResetting(subscription); setSubscription(null) }} />}
  </>
}
