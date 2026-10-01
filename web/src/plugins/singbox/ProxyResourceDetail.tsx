import { useState } from 'react'
import { api } from '../../api'
import { Badge, ErrorNotice, Field, Loading, Modal } from '../../components'
import { time } from '../../format'
import { resourceWriteError, useAction, useResource } from '../../hooks'
import type { Node } from '../../types'
import type { ProxyResource, ResourceDetail, ResourceKey } from './resourceTypes'
import { endpoint, resourceLink, roleNames, stageName } from './resourceTypes'
import { protocolNames } from './ProtocolFields'

export default function ProxyResourceDetail({ selected, onClose, onChanged, onEdit, onDelete, onDeployment }: { selected: ResourceKey; onClose: () => void; onChanged: () => void; onEdit: (node: Node) => void; onDelete: (resource: ProxyResource) => void; onDeployment: (serverId: number) => void }) {
  const base = `/api/plugins/sing-box/proxy-resources/${selected.kind}/${selected.id}`
  const detail = useResource<ResourceDetail>(base, 3000)
  const action = useAction()
  const [renaming, setRenaming] = useState(false)
  const [chosen, setChosen] = useState<Record<number, number>>({})
  const [notice, setNotice] = useState('')
  const data = detail.data, resource = data?.resource
  const changed = () => { detail.reload(); onChanged() }
  const writeError = () => resourceWriteError(detail)
  const idle = Boolean(resource && (resource.pending_generation === null || resource.stage === 'failed'))
  const apply = (retry = false) => {
    if (!resource || writeError()) return
    const current = detail.getCurrent()
    if (!current || current.resource.kind !== selected.kind || current.resource.id !== selected.id || current.resource.pending_generation !== resource.pending_generation || current.resource.active_generation !== resource.active_generation) return
    if (!retry && Object.entries(chosen).some(([position, version]) => !current.hops.some(hop => hop.position === Number(position) && hop.latest_version_id === version && hop.present))) return
    const versions = retry ? [] : data!.hops.filter(hop => chosen[hop.position] === hop.latest_version_id && hop.present && hop.latest_version_id !== hop.version_id).map(hop => ({ position: hop.position, node_version_id: hop.latest_version_id }))
    void action.run(() => api(`${base}/apply-node-versions`, 'POST', { expected_generation: resource.pending_generation ?? resource.active_generation ?? 0, versions }), () => { setChosen({}); setNotice('新的应用任务已保存，等待受管段就绪与路径验证。'); changed() })
  }
  return <Modal title={resource ? `${resource.kind === 'chain' ? '链路' : '节点'}：${resource.name}` : '代理资源详情'} wide className="proxy-resource-detail" busy={action.busy} onClose={onClose}>
    <div className="modal-body"><ErrorNotice message={detail.error || action.error} retry={detail.reload} />
      {detail.loading && !data ? <Loading /> : data && resource ? <>
        {notice && <p className="notice" role="status">{notice}</p>}
        <div className="resource-summary"><Badge tone={!resource.available ? 'warm' : resource.stage === 'failed' ? 'bad' : resource.stage === 'active' ? 'good' : 'neutral'}>{stageName(resource.stage)}</Badge><span>{roleNames[resource.role]}{resource.legacy ? ' · 原两跳链路' : ''}</span><span>{resource.tcp ? '支持 TCP' : '不支持 TCP'} · {resource.udp ? '支持 UDP' : '仅 TCP'}</span></div>
        <dl className="resource-facts"><div><dt>公开入口</dt><dd><code>{endpoint(resource.public_host, data.node.settings?.public_port ?? resource.port)}</code></dd></div><div><dt>入口服务器</dt><dd><a href={`#/servers/${resource.server_id}`}>{resource.server_name}</a></dd></div><div><dt>入口协议</dt><dd>{protocolNames[resource.protocol] ?? resource.protocol}</dd></div><div><dt>资源状态</dt><dd>{resource.available ? '资源完整' : '部分资源不可用'} · 被引用 {resource.reference_count} 次</dd></div></dl>
        {resource.last_error && <div className="notice" role="status">{resource.last_error}</div>}
        {resource.kind === 'chain' ? <>
          <h3>有序路径</h3><ol className="chain-hop-list"><li className="chain-entry"><strong>公开入口：{resource.name}</strong><small>入口负责用户授权与计量</small></li>{data.hops.map(hop => <li key={hop.position}><div><strong>第 {hop.position + 1} 段{hop.position === data.hops.length - 1 ? ' · 最终出口' : ''}：{hop.name}</strong><small>{hop.kind === 'managed' ? '受管节点' : '订阅节点'} · {protocolNames[hop.protocol] ?? hop.protocol} · {endpoint(hop.server, hop.port)}</small>{hop.kind === 'subscription' && <small>所选版本 #{hop.version_id} · {hop.update_mode === 'pinned' ? '固定版本' : '跟随同一节点更新'}</small>}{!hop.present && <p className="helper">该节点当前缺失或不可用。已应用链路保留原版本，不会自动改用其他节点。</p>}</div><div className="row-actions">{hop.kind === 'managed' ? <a className="text-button" href={resourceLink({ kind: 'direct', id: hop.node_id })}>查看受管节点</a> : hop.present && hop.latest_version_id !== null && hop.latest_version_id !== hop.version_id && <label className="group-choice"><input type="checkbox" aria-label={`更新第 ${hop.position + 1} 段 ${hop.name}`} disabled={action.busy || Boolean(writeError()) || !idle} checked={chosen[hop.position] === hop.latest_version_id} onChange={event => setChosen(previous => { const next = { ...previous }; if (event.target.checked) next[hop.position] = hop.latest_version_id!; else delete next[hop.position]; return next })} /><span>应用新版本 #{hop.latest_version_id}</span></label>}</div></li>)}</ol>
          <div className="row-actions"><button className="button button-primary button-small" disabled={action.busy || Boolean(writeError()) || !idle || !data.hops.some(hop => hop.present && hop.latest_version_id !== hop.version_id && chosen[hop.position] === hop.latest_version_id)} onClick={() => apply()}>应用所选节点更新</button>{resource.stage === 'failed' && <button className="button button-secondary button-small" disabled={action.busy || Boolean(writeError())} onClick={() => apply(true)}>重试当前候选</button>}</div>
          <p className="helper">更新按当前所选节点的明确版本生成候选。受管段、入口和路径验证依次确认后切换；更新失败或来源缺失会保留可恢复版本。路径验证只证明当次目标可达。</p>
          <h3>应用版本</h3><p className="helper">已应用：{resource.active_generation ?? '尚无'} · 候选：{resource.pending_generation ?? '尚无'} · 最低可恢复版本：{resource.minimum_generation}</p>
          <div className="table-wrap"><table><thead><tr><th>版本</th><th>阶段</th><th>创建时间</th></tr></thead><tbody>{data.versions.map(version => <tr key={version.generation}><td>第 {version.generation} 代</td><td>{stageName(version.stage)}{version.last_error && <small>{version.last_error}</small>}</td><td>{time(version.created_at)}</td></tr>)}</tbody></table></div>
          {renaming && <form className="resource-rename" onSubmit={event => { event.preventDefault(); if (writeError()) return; const form = new FormData(event.currentTarget); const name = String(form.get('name')).trim(); void action.run(() => api(base, 'PATCH', { name, ...(form.get('sync_subscription') === 'on' ? { subscription_name: name } : {}) }), () => { setRenaming(false); changed() }) }}><Field label="链路名称"><input name="name" required maxLength={128} defaultValue={resource.name} disabled={action.busy} /></Field><label className="group-choice"><input name="sync_subscription" type="checkbox" disabled={action.busy} /><span>同步修改订阅显示名称<small>当前订阅名称：{data.node.name}。默认只更改管理名称。</small></span></label><button className="button button-primary button-small" disabled={action.busy || Boolean(writeError())}>保存名称</button><button type="button" className="text-button" disabled={action.busy} onClick={() => setRenaming(false)}>取消修改</button></form>}
        </> : <p className="helper">{resource.role === 'managed_hop' ? '此节点也被链路用作受管代理段。更改连接参数前，需处理链路对原端点的依赖。' : '此节点可以直接授权给代理用户，也可以作为新链路的受管段。'}监听地址 {data.node.settings?.listen ?? '::'}，监听端口 {data.node.port}。</p>}
      </> : !detail.error && <p>找不到此资源。</p>}
    </div>
    <footer><button className="button button-secondary" onClick={onClose} disabled={action.busy}>关闭</button>{resource && <><button className="button button-secondary" disabled={action.busy} onClick={() => onDeployment(resource.server_id)}>部署进度</button><button className="button button-secondary" disabled={action.busy || Boolean(writeError())} onClick={() => { if (writeError()) return; resource.kind === 'chain' ? setRenaming(true) : onEdit(data!.node) }}>{resource.kind === 'chain' ? '修改名称' : '编辑节点'}</button><button className="button button-danger" disabled={action.busy || Boolean(writeError())} onClick={() => { if (!writeError()) onDelete(resource) }}>删除{resource.kind === 'chain' ? '链路' : '节点'}</button></>}</footer>
  </Modal>
}
