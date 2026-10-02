import { Badge, Field } from '../../components'
import type { ChainDraftHop, ProxyWriteSnapshot } from './Chains'
import { validSourceNodePage, validSubscriptionSources } from './sourceTypes'

export function moveChainHop(hops: ChainDraftHop[], from: number, to: number): ChainDraftHop[] {
  if (!Number.isInteger(from) || !Number.isInteger(to) || from < 0 || to < 0 || from >= hops.length || to >= hops.length) return hops
  const next = [...hops], [hop] = next.splice(from, 1)
  next.splice(to, 0, hop)
  return next
}

export default function ChainPathEditor({ label, hops, onChange, snapshot }: { label: string; hops: ChainDraftHop[]; onChange: (hops: ChainDraftHop[]) => void; snapshot: ProxyWriteSnapshot }) {
  const managed = snapshot.nodes.data?.filter(node => node.protocol === 'vless-reality' && node.enabled !== false
    && snapshot.resources.data?.some(resource => resource.kind === 'direct' && resource.id === node.id && resource.available)
    && snapshot.servers.data?.some(server => server.id === node.server_id && server.enabled)) ?? []
  const sources = validSubscriptionSources(snapshot.sources?.data) ? snapshot.sources!.data! : []
  const update = (index: number, hop: ChainDraftHop) => onChange(hops.map((current, i) => i === index ? hop : current))
  return <fieldset className="chain-path-editor" aria-label={label}><legend>{label} · {hops.length} / 8 跳</legend><p className="helper">从入口到出口依次排列；最后一跳是最终出口。移动会改变真实连接顺序，提交后拓扑身份与顺序不可原地修改。</p>
    {hops.map((hop, index) => {
      const page = hop.kind === 'subscription' ? snapshot.sourceNodes?.[hop.source_id] : undefined
      const source = hop.kind === 'subscription' ? sources.find(source => source.id === hop.source_id) : undefined
      const nodes = page && validSourceNodePage(page.data) && page.data.source_id === source?.id ? page.data.nodes : []
      const fresh = Boolean(snapshot.sources?.fresh && !snapshot.sources?.error && source && !source.archived && page?.fresh && !page.error && page.data?.current_identity_epoch === source.identity_epoch)
      return <section key={index} className="chain-hop-draft" data-hop-position={index + 1}><header><strong>第 {index + 1} 跳 · {index === hops.length - 1 ? '最终出口' : '中间段'}</strong><div className="row-actions"><button type="button" className="text-button" aria-label={`${label}第 ${index + 1} 跳上移`} disabled={index === 0} onClick={() => onChange(moveChainHop(hops, index, index - 1))}>上移</button><button type="button" className="text-button" aria-label={`${label}第 ${index + 1} 跳下移`} disabled={index === hops.length - 1} onClick={() => onChange(moveChainHop(hops, index, index + 1))}>下移</button><button type="button" className="text-button danger-text" disabled={hops.length === 1} onClick={() => onChange(hops.filter((_, i) => i !== index))}>移除此跳</button></div></header>
        <div className="node-fields-grid"><Field label={`第 ${index + 1} 跳类型`}><select value={hop.kind} onChange={event => update(index, event.target.value === 'managed' ? { kind: 'managed', node_id: '' } : { kind: 'subscription', source_id: 0, node: null, update_mode: 'follow_node' })}><option value="managed">自有受管节点</option><option value="subscription">订阅中的具体节点</option></select></Field>
          {hop.kind === 'managed' ? <Field label={`第 ${index + 1} 跳受管节点`}><select required value={hop.node_id} onChange={event => update(index, { ...hop, node_id: event.target.value })}><option value="" disabled>选择受管节点</option>{hop.node_id && !managed.some(node => String(node.id) === hop.node_id) && <option value={hop.node_id}>节点 #{hop.node_id}（等待资格确认）</option>}{managed.map(node => <option key={node.id} value={node.id}>{node.name} · 服务器 #{node.server_id}</option>)}</select></Field> : <>
            <Field label={`第 ${index + 1} 跳订阅来源`}><select required disabled={!snapshot.sources?.fresh || Boolean(snapshot.sources?.error)} value={hop.source_id || ''} onChange={event => update(index, { ...hop, source_id: Number(event.target.value), node: null })}><option value="" disabled>选择来源</option>{hop.source_id > 0 && !sources.some(source => source.id === hop.source_id) && <option value={hop.source_id}>来源 #{hop.source_id}（已不可用）</option>}{sources.map(source => <option key={source.id} value={source.id} disabled={source.archived || !source.latest_success}>{source.name} · 代次 {source.identity_epoch}{source.archived ? ' · 已归档' : source.latest_success ? '' : ' · 等待解析'}</option>)}</select></Field>
            <Field label={`第 ${index + 1} 跳订阅节点`} hint="只接受当前来源的明确、可选版本；名称不作为身份，历史记录仅供查看。"><select required disabled={!fresh} value={hop.node?.version_id ?? ''} onChange={event => { const node = nodes.find(node => node.version_id === event.target.value && node.selectable); if (node && fresh) update(index, { ...hop, node }) }}><option value="" disabled>选择具体节点与版本</option>{hop.node && !nodes.some(node => node.version_id === hop.node!.version_id) && <option value={hop.node.version_id}>{hop.node.name}（所选版本已变化，请重新选择）</option>}{nodes.map(node => <option key={node.version_id} value={node.version_id} disabled={!node.selectable}>{node.name} · {node.protocol ?? '未知协议'} · {node.server ?? '未知端点'}:{node.server_port ?? '未知端口'} · {node.version_id.slice(0, 8)}{node.selectable ? '' : ` · ${node.reasons.join('；') || '不可新增引用'}`}</option>)}</select></Field>
            <Field label={`第 ${index + 1} 跳更新方式`}><select value={hop.update_mode} onChange={event => update(index, { ...hop, update_mode: event.target.value as 'follow_node' | 'pinned' })}><option value="follow_node">跟随所选节点更新</option><option value="pinned">固定当前版本</option></select></Field>
          </>}
        </div>
        {hop.kind === 'subscription' && <><p className="helper"><Badge tone="neutral">订阅节点，无 Agent 状态</Badge> {hop.update_mode === 'follow_node' ? '仅同一明确节点的兼容新版本进入候选；失败、缺失或身份歧义不自动换点。' : '刷新不改变此快照；同节点新版本需在资源详情中明确应用。'}</p>{hop.node && <p className="helper">已选身份 <code>{hop.node.id}</code> · 固定输入版本 <code>{hop.node.version_id}</code> · 代次 {hop.node.identity_epoch}。TCP {hop.node.capabilities.tcp ? '支持' : '不支持'} / UDP {hop.node.capabilities.udp ? '支持' : '不支持'} 是单协议语义能力，不代表整条路径可承载或在线。</p>}{!fresh && <p className="helper" role="status">{page?.error || snapshot.sources?.error || '来源或节点版本等待最新确认。'}</p>}</>}
      </section>
    })}
    <button type="button" className="button button-secondary button-small" disabled={hops.length >= 8} onClick={() => onChange([...hops, { kind: 'managed', node_id: '' }])}>添加下一跳</button>
  </fieldset>
}
