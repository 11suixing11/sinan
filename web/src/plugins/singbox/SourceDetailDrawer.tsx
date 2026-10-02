import { useEffect, useRef, useState } from 'react'
import { Badge, ErrorNotice, Loading, Modal } from '../../components'
import { useAction, useResource } from '../../hooks'
import { dateText, validatedSnapshot } from './groupTypes'
import { cancelSourceJob, sourceMetadataError } from './sourceRequests'
import type { SourceEditorMode, SourceSnapshot } from './sourceRequests'
import { sourceFormatText, sourceJobActive, sourceJobRoot, sourceRoot, sourceStageText, sourceStatusText, validSourceHistory, validSourceJob, validSourceNodePage, validSubscriptionSource } from './sourceTypes'
import type { SourceHistory, SourceJob, SourceNode, SourceNodePage, SubscriptionSource } from './sourceTypes'

function nodeAddress(node: SourceNode) {
  if (!node.server || node.server_port === null) return '端点未知'
  const host = node.server.replace(/^\[|\]$/g, '')
  return `${host.includes(':') ? `[${host}]` : host}:${node.server_port}`
}
function SourceNodeRow({ node, historical, fresh }: { node: SourceNode; historical: boolean; fresh: boolean }) {
  const identity = { unique: '身份唯一', ambiguous: '身份不唯一', unresolved: '身份尚未确认' }[node.identity_state]
  const reasons = [...new Set([...node.reasons, ...node.unsupported_reasons.map(reason => `${reason.code}：${reason.message}`)])]
  return <tr data-source-node-id={node.id}>
    <td><strong>{node.name}</strong><small>节点 <code>{node.id}</code></small><small>版本 <code>{node.version_id}</code></small></td>
    <td><code>{nodeAddress(node)}</code><small>{node.protocol ?? '协议未知'} · {node.transport ?? '传输未知'}</small><small>SNI：{node.sni ?? '未知'}</small></td>
    <td><Badge tone={node.supported ? 'good' : 'warm'}>{node.supported ? '解析支持' : '不支持解析'}</Badge><small>{identity}</small><small>{node.present_in_latest ? historical ? '所选批次存在' : '当前批次存在' : '当前批次缺失'}</small><small>{historical ? '历史版本仅供查看' : !fresh ? '引用资格待确认' : node.selectable ? '可供后续路径引用' : '当前不可引用'}</small></td>
    <td><small>TCP {node.capabilities.tcp ? '支持' : '不支持'} · UDP {node.capabilities.udp ? '支持' : '不支持'}</small>{reasons.map((reason, index) => <small key={index}>{reason}</small>)}</td>
  </tr>
}
export default function SourceDetailDrawer({ selected, snapshot, refresh, onClose, edit, remove }: { selected: SubscriptionSource; snapshot: SourceSnapshot; refresh: () => void; onClose: () => void; edit: (mode: SourceEditorMode, source: SubscriptionSource) => void; remove: (source: SubscriptionSource) => void }) {
  const query = useResource<unknown>(`${sourceRoot}/${selected.id}`)
  const sourceHistory = useRef(selected)
  const sourceResult = validatedSnapshot(query, validSubscriptionSource, sourceHistory.current)
  const sameSource = sourceResult.data?.id === selected.id
  if (sameSource && sourceResult.fresh) sourceHistory.current = sourceResult.data!
  const source = sameSource ? sourceResult.data! : sourceHistory.current
  const listed = snapshot.data?.find(item => item.id === selected.id)
  const fresh = sourceResult.fresh && sameSource && !sourceMetadataError(snapshot) && listed?.settings_revision === source.settings_revision && listed.identity_epoch === source.identity_epoch
  const action = useAction()
  const [revision, setRevision] = useState('')
  const [page, setPage] = useState(0)
  const [lastJob, setLastJob] = useState<SourceJob | undefined>(undefined)
  const nodesQuery = useResource<unknown>(`${sourceRoot}/${selected.id}${revision ? `/revisions/${revision}` : ''}/nodes`, 0)
  const historyQuery = useResource<unknown>(`${sourceRoot}/${selected.id}/revisions`, 0)
  const previousNodes = useRef<{ path: string; data?: SourceNodePage }>({ path: '' })
  const nodePath = `${selected.id}:${revision}`
  const nodes = validatedSnapshot(nodesQuery, validSourceNodePage, previousNodes.current.path === nodePath ? previousNodes.current.data : undefined)
  if (nodes.fresh && nodes.data?.source_id === selected.id) previousNodes.current = { path: nodePath, data: nodes.data }
  const previousHistory = useRef<SourceHistory | undefined>(undefined)
  const history = validatedSnapshot(historyQuery, validSourceHistory, previousHistory.current)
  if (history.fresh && history.data?.source_id === selected.id) previousHistory.current = history.data
  const jobId = source.active_job?.id ?? (sourceJobActive(lastJob) ? lastJob!.id : null)
  const jobQuery = useResource<unknown>(jobId ? `${sourceJobRoot}/${jobId}` : null, jobId ? 1500 : 0)
  const previousJob = useRef<SourceJob | undefined>(undefined)
  const job = validatedSnapshot(jobQuery, validSourceJob, previousJob.current?.id === jobId ? previousJob.current : undefined)
  if (job.fresh && job.data?.id === jobId && job.data.source_id === selected.id) previousJob.current = job.data
  const observedJob = job.data?.id === jobId && job.data.source_id === selected.id ? job.data : source.active_job
  const activeJob = lastJob?.id === jobId && lastJob.status === 'cancelling' && observedJob && ['queued', 'running'].includes(observedJob.status) ? lastJob : observedJob ?? lastJob
  const reload = () => { query.reload(); historyQuery.reload(); nodesQuery.reload(); if (jobId) jobQuery.reload(); refresh() }
  useEffect(() => { setPage(0) }, [revision, nodes.data?.success_revision?.id])
  useEffect(() => { nodesQuery.reload(); historyQuery.reload() }, [source.latest_success?.id, source.settings_revision, source.identity_epoch]) // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { if (job.data?.source_id === selected.id) { setLastJob(job.data); if (!sourceJobActive(job.data)) { query.reload(); historyQuery.reload(); nodesQuery.reload(); refresh() } } }, [job.data?.id, job.data?.status]) // eslint-disable-line react-hooks/exhaustive-deps
  const currentNodes = nodes.data?.source_id === selected.id ? nodes.data : undefined
  const nodesFresh = nodes.fresh && fresh && currentNodes?.current_settings_revision === source.settings_revision && currentNodes.current_identity_epoch === source.identity_epoch
    && (revision ? currentNodes.success_revision?.id === revision : (currentNodes.success_revision?.id ?? null) === (source.latest_success?.id ?? null))
  const nodeRows = currentNodes?.nodes ?? []
  const pages = Math.max(1, Math.ceil(nodeRows.length / 50)), boundedPage = Math.min(page, pages - 1)
  const openEditor = (mode: SourceEditorMode) => { if (!fresh || action.busy) return; edit(mode, source) }
  const cancel = () => {
    if (!fresh || action.busy || !activeJob || !job.fresh) return
    void action.run(() => cancelSourceJob(job, activeJob.id), receipt => { setLastJob(receipt); reload() })
  }
  return <Modal wide className="source-drawer" title={`订阅来源：${source.name}`} onClose={onClose} busy={action.busy}><div className="modal-body">
    <ErrorNotice message={sourceResult.error || (!sameSource ? '来源详情标识不一致，请刷新确认。' : '') || action.error} retry={reload} />
    {!fresh && <p className="helper" role="status">当前显示历史来源信息，等待最新设置确认；修改操作暂不可用。</p>}
    <div className="source-detail-actions"><button className="button button-secondary" disabled={!fresh || action.busy || source.archived} onClick={() => openEditor('metadata')}>修改名称与周期</button>{source.kind === 'inline' && <button className="button button-secondary" disabled={!fresh || action.busy || source.archived} onClick={() => openEditor('update')}>更新同一来源内容</button>}<button className="button button-secondary" disabled={!fresh || action.busy || source.archived} onClick={() => openEditor('replace')}>更换来源</button><button className="button button-secondary" disabled={!fresh || action.busy} onClick={() => openEditor(source.archived ? 'unarchive' : 'archive')}>{source.archived ? '恢复来源' : '归档来源'}</button><button className="button button-secondary danger-text" disabled={!fresh || action.busy} onClick={() => { if (fresh && !action.busy) remove(source) }}>删除来源</button></div>
    <dl className="source-facts"><div><dt>输入来源</dt><dd>{source.kind === 'url' ? `HTTPS · ${source.host ?? '主机未知'}` : '粘贴或文件内容'}</dd></div><div><dt>设置 / 身份代次</dt><dd>{source.settings_revision} / {source.identity_epoch}</dd></div><div><dt>认证信息</dt><dd>{source.auth_configured ? '已配置（不回填）' : '未配置'}</dd></div><div><dt>自动刷新</dt><dd>{source.kind === 'url' ? `每 ${source.refresh_interval_secs} 秒` : '仅手动更新'}</dd></div><div><dt>最后尝试</dt><dd>{dateText(source.last_attempt_at)}</dd></div><div><dt>最后成功</dt><dd>{dateText(source.last_success_at)}</dd></div></dl>
    <p><Badge tone={source.archived ? 'neutral' : source.latest_success ? 'good' : 'warm'}>{source.archived ? '已归档' : source.latest_success ? '有成功解析版本' : '尚无成功解析版本'}</Badge></p>
    {source.stale_reason && <p className="notice">{source.stale_reason}</p>}
    {source.last_error && <p className="notice notice-error" role="alert">{source.last_error.stage} / {source.last_error.kind}：{source.last_error.message}{source.last_error.http_status !== null && `（HTTP ${source.last_error.http_status}）`}。上次成功版本保留。</p>}
    <section className="source-task" aria-label="来源处理任务"><h3>抓取与解析任务</h3>{activeJob ? <><p><Badge tone={activeJob.status === 'failed' ? 'bad' : activeJob.status === 'succeeded' ? 'good' : 'neutral'}>{sourceStatusText[activeJob.status]}</Badge> · {sourceStageText[activeJob.stage]}</p><p className="helper">任务 <code>{activeJob.id}</code> · 设置 {activeJob.settings_revision} / 身份代次 {activeJob.identity_epoch}</p><p className="helper">开始 {dateText(activeJob.started_at)} · 结束 {dateText(activeJob.finished_at)}</p><ErrorNotice message={job.error || activeJob.error?.message} retry={jobQuery.reload} />{sourceJobActive(activeJob) && <button className="button button-secondary" disabled={!fresh || !job.fresh || action.busy || activeJob.status === 'cancelling'} onClick={cancel}>{activeJob.status === 'cancelling' ? '等待取消确认' : '取消来源任务'}</button>}</> : <p className="helper">没有正在处理的任务。</p>}<p className="helper">以上表示面板抓取和解析进度，不表示节点在线、出口可达或链路已经部署。</p></section>
    <section aria-label="来源节点预览"><div className="source-preview-heading"><h3>节点与不可变版本</h3><select aria-label="选择订阅版本" value={revision} onChange={event => { setRevision(event.target.value); setPage(0) }}><option value="">当前成功版本</option>{history.data?.source_id === selected.id && history.data.revisions.map(item => <option key={item.id} value={item.id}>{dateText(item.parsed_at)} · 代次 {item.identity_epoch} · {item.id.slice(0, 8)}</option>)}</select></div>
      <ErrorNotice message={nodes.error || history.error || (nodes.data && nodes.data.source_id !== selected.id || history.data && history.data.source_id !== selected.id ? '节点或历史记录与当前来源不一致，请刷新确认。' : '')} retry={reload} />
      {!nodesFresh && <p className="helper" role="status">节点预览等待最新版本确认，当前信息仅供查看。</p>}
      {currentNodes?.success_revision && <p className="helper">{sourceFormatText[currentNodes.success_revision.format]} · 解析器 {currentNodes.success_revision.parser_version} · 成功版本设置 {currentNodes.success_revision.settings_revision} / 代次 {currentNodes.success_revision.identity_epoch} · 当前设置 {currentNodes.current_settings_revision} / 代次 {currentNodes.current_identity_epoch}</p>}
      {revision && <p className="notice">此历史版本视图仅供查看，不提供新的引用操作。</p>}
      <p className="helper">解析支持、TCP/UDP 语义能力和身份资格分别记录。创建有序链路时明确选择当前可用节点及不可变版本；完整路径通过承载与指定运行验证后才切换入口。</p>
      <p className="helper">单次预览最多 5000 个节点，优先显示当前批次；更多历史节点可通过成功版本查看。</p>
      {nodesQuery.loading && !currentNodes ? <Loading /> : !currentNodes ? <p className="helper">节点预览尚未取得，请刷新确认。</p> : !nodeRows.length ? <p className="helper">当前视图暂无节点，已有历史节点可通过成功版本查看。</p> : <><div className="table-wrap"><table className="source-node-table"><thead><tr><th>节点与版本</th><th>公开端点</th><th>解析与身份</th><th>能力与原因</th></tr></thead><tbody>{nodeRows.slice(boundedPage * 50, (boundedPage + 1) * 50).map(node => <SourceNodeRow key={node.version_id} node={node} historical={Boolean(revision)} fresh={Boolean(nodesFresh)} />)}</tbody></table></div><div className="source-pagination"><button className="button button-secondary" disabled={boundedPage === 0} onClick={() => setPage(Math.max(0, boundedPage - 1))}>上一页</button><span>第 {boundedPage + 1} / {pages} 页 · {nodeRows.length} 个节点 · 每页 50 个</span><button className="button button-secondary" disabled={boundedPage >= pages - 1} onClick={() => setPage(boundedPage + 1)}>下一页</button></div></>}
    </section>
    <section aria-label="来源路径引用"><h3>来源路径引用</h3>{source.dependencies.length ? <ul className="resource-reasons">{source.dependencies.map(ref => <li key={`${ref.chain_id}:${ref.generation}:${ref.state}:${ref.hop_position}`}><a href={`#/plugins/sing-box/nodes/chain/${ref.chain_id}`} onClick={onClose}>「{ref.chain_name}」#{ref.chain_id}</a> · 第 {ref.hop_position} 跳 · {ref.state === 'applied' ? '已应用' : ref.state === 'candidate' ? '候选' : '恢复'}代 {ref.generation} · 来源代次 {ref.identity_epoch}<small>节点 {ref.external_node_id} · 版本 {ref.node_version_id}</small></li>)}</ul> : <p className="helper">暂无路径引用。</p>}<p className="helper">当前、候选和恢复路径引用未解除时不能删除来源；归档停止刷新和新引用，已应用快照保留，不会自动改接节点。</p></section>
  </div><footer><button className="button button-secondary" onClick={reload}>刷新来源详情</button><button className="button button-secondary" onClick={onClose} disabled={action.busy}>关闭来源详情</button></footer></Modal>
}
