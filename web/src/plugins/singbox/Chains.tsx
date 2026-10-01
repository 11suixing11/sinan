import { useState } from 'react'
import { api } from '../../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, FormDialog, Loading, Refresh } from '../../components'
import { useAction, useResource } from '../../hooks'
import type { Node } from '../../types'
import type { Chain } from './groupTypes'

const root = '/api/plugins/sing-box'

export default function Chains() {
  const chains = useResource<Chain[]>(`${root}/chains`)
  const nodes = useResource<Node[]>(`${root}/nodes`)
  const action = useAction()
  const [creating, setCreating] = useState(false)
  const [deleting, setDeleting] = useState<Chain | null>(null)
  const refresh = () => { chains.reload(); nodes.reload() }
  const nodeName = (id: number) => nodes.data?.find(node => node.id === id)?.name ?? `节点 #${id}（已不可用）`
  const submit = (form: FormData) => void action.run(() => api(`${root}/chains`, 'POST', {
    name: String(form.get('name') ?? '').trim(),
    entry_node_id: Number(form.get('entry_node_id')),
    exit_node_id: Number(form.get('exit_node_id')),
  }), () => { setCreating(false); refresh() })

  return <>
    <ErrorNotice message={chains.error || nodes.error || action.error} retry={refresh} />
    <section className="panel">
      <div className="panel-heading"><h2>两跳链路</h2><div className="row-actions"><Refresh onClick={refresh} /><button className="button button-primary button-small" disabled={!nodes.data || !chains.data || Boolean(nodes.error || chains.error)} onClick={() => { action.clearError(); setCreating(true) }}>创建两跳链路</button></div></div>
      {chains.loading && !chains.data ? <Loading /> : !chains.data?.length ? <Empty icon="nodes" title="通过入口节点连接另一台服务器的出口" description="先创建两个节点，再选择一个尚未授权的独立入口。创建链路后，还需通过策略组授权给代理用户。" /> : <div className="table-wrap"><table><thead><tr><th>链路</th><th>入口 → 出口</th><th>资源状态</th><th>操作</th></tr></thead><tbody>{chains.data.map(chain => <tr key={chain.id}><td><strong>{chain.name}</strong></td><td>{nodeName(chain.entry_node_id)} → {nodeName(chain.exit_node_id)}</td><td><Badge tone={chain.available ? 'good' : 'bad'}>{chain.available ? '资源存在' : '资源已不可用'}</Badge></td><td><button className="text-button danger-text" onClick={() => { action.clearError(); setDeleting(chain) }}>删除</button></td></tr>)}</tbody></table></div>}
      <div className="panel-body"><p className="helper">目前支持不同服务器上的 VLESS + Reality 两跳链路。创建成功不会自动授权给用户；加入策略组并分配后，还需等两端配置都应用成功，才会进入订阅。</p><a className="text-button" href="#/plugins/sing-box/groups">管理策略组与套餐</a></div>
    </section>
    {creating && <FormDialog title="创建两跳链路" onClose={() => setCreating(false)} onSubmit={submit} busy={action.busy} error={action.error} submitLabel="创建未授权链路">
      <Field label="名称"><input name="name" required maxLength={128} autoComplete="off" /></Field>
      <Field label="入口节点" hint="使用尚未授权的独立节点，不能同时作为直连节点授权。"><select name="entry_node_id" required defaultValue=""><option value="" disabled>选择入口</option>{nodes.data?.filter(node => node.protocol === 'vless-reality').map(node => <option key={node.id} value={node.id}>{node.name}（服务器 #{node.server_id}）</option>)}</select></Field>
      <Field label="出口节点" hint="必须与入口分属不同服务器，出口可被多条链路共用。"><select name="exit_node_id" required defaultValue=""><option value="" disabled>选择出口</option>{nodes.data?.filter(node => node.protocol === 'vless-reality').map(node => <option key={node.id} value={node.id}>{node.name}（服务器 #{node.server_id}）</option>)}</select></Field>
      <p className="helper">创建后将链路加入策略组，再分配给代理用户。用户流量按入口统计。</p>
    </FormDialog>}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} error={action.error} onClose={() => setDeleting(null)} onConfirm={() => void action.run(() => api(`${root}/chains/${deleting.id}`, 'DELETE'), () => { setDeleting(null); refresh() })}>被策略组使用的链路不能直接删除。删除后，两端设备需要应用新配置以撤销内部连接凭据。</Confirm>}
  </>
}
