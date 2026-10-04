import { useEffect, useRef, useState } from 'react'
import { Badge, Confirm, ErrorNotice, Icon, Loading } from '../../components'
import { useAction, useResource } from '../../hooks'
import { dateText, validatedSnapshot } from './groupTypes'
import SourceDetailDrawer from './SourceDetailDrawer'
import SourceEditor from './SourceEditor'
import type { SourceEditorSession } from './SourceEditor'
import OrderedSourceImport from './OrderedSourceImport'
import { SourceUsage } from './SourceUsage'
import { deleteSource, refreshSource, sourceMetadataError, sourceWriteError } from './sourceRequests'
import type { SourceEditorMode } from './sourceRequests'
import { sourceInterval, sourceRoot, sourceStatusText, validSubscriptionSources } from './orderedSourceTypes'
import type { SourceReceipt, SubscriptionSource } from './orderedSourceTypes'
import './sources.css'

// After the source migration these are the only subscription sources: they feed
// the node catalog, user grants and ordered chains.
export default function Sources({ createRequest = 0, migrated, onCatalogChange }: { createRequest?: number; migrated?: boolean; onCatalogChange?: () => void } = {}) {
  const query = useResource<unknown>(sourceRoot)
  const previous = useRef<SubscriptionSource[] | undefined>(undefined)
  const snapshot = validatedSnapshot(query, validSubscriptionSources, previous.current)
  if (snapshot.fresh) previous.current = snapshot.data
  const action = useAction()
  const [detail, setDetail] = useState<SubscriptionSource | null>(null)
  const [deleting, setDeleting] = useState<SubscriptionSource | null>(null)
  const [session, setSession] = useState<SourceEditorSession>({ mode: 'create', generation: 0 })
  const [editorOpen, setEditorOpen] = useState(false)
  const [notice, setNotice] = useState('')
  const [includeArchived, setIncludeArchived] = useState(false)
  const [importing, setImporting] = useState(false)
  const error = sourceMetadataError(snapshot)
  const requested = useRef(0)
  useEffect(() => {
    if (!createRequest || createRequest === requested.current || error || action.busy) return
    requested.current = createRequest
    setDetail(null); setEditorOpen(true); action.clearError()
    if (session.mode !== 'create') setSession(current => ({ mode: 'create', generation: current.generation + 1 }))
  }, [createRequest, error, action.busy, session.mode])
  const refresh = () => query.reload()
  const edit = (mode: SourceEditorMode, source?: SubscriptionSource) => {
    if (action.busy || sourceWriteError(snapshot, source?.id, source?.settings_revision, mode === 'archive' || mode === 'unarchive')) return
    if (session.mode !== mode || session.source?.id !== source?.id) setSession({ mode, source, generation: session.generation + 1 })
    else if (!editorOpen && session.source && source && session.source.settings_revision !== source.settings_revision) setNotice('来源设置已变化。编辑窗口保留原草稿；请先确认上次保存。')
    setDetail(null); setEditorOpen(true); action.clearError()
  }
  const openImport = () => { if (action.busy || error) return; setDetail(null); setImporting(true); action.clearError() }
  const imported = (receipt: SourceReceipt) => {
    setImporting(false); setNotice(`来源 #${receipt.source_id} 已保存，所选节点已加入节点库。`); refresh(); onCatalogChange?.()
  }
  const remove = (source: SubscriptionSource) => { if (action.busy || sourceWriteError(snapshot, source.id, source.settings_revision, true)) return; setDetail(null); setDeleting(source); action.clearError() }
  const saved = (receipt: SourceReceipt) => {
    setEditorOpen(false); setSession(current => ({ mode: 'create', generation: current.generation + 1 })); setNotice(receipt.job_id ? '来源已保存，敏感输入已清空；抓取与解析仍在进行，请查看来源详情。' : '来源设置已保存，敏感输入已清空。'); refresh()
  }
  const resetEditor = () => {
    const source = session.source && snapshot.data?.find(item => item.id === session.source!.id)
    if (sourceMetadataError(snapshot) || session.source && !source) return
    setSession({ mode: session.mode, source, generation: session.generation + 1 })
  }
  return <section className="panel sources-panel" aria-label="订阅来源管理"><div className="panel-heading"><div><h2>{migrated ? '订阅来源' : '有序链路订阅来源'} <span className="count">{snapshot.data?.length ?? '—'}</span></h2><p className="helper">{migrated ? '抓取、解析和保存不可变外部节点版本。加入节点库的节点可分配给代理用户；有序链路引用明确节点与版本。来源本身不作为公开授权入口。' : '抓取、解析和保存不可变外部节点版本；链路只引用明确节点与版本，来源本身不作为公开授权入口。'}</p></div><div className="source-section-actions"><button className="button button-secondary" onClick={refresh}><Icon name="refresh" size={16} />刷新来源列表</button><button className="button button-secondary" disabled={Boolean(error) || action.busy} onClick={openImport}><Icon name="plus" size={16} />导入并选择节点</button><button className="button button-secondary" disabled={Boolean(error) || action.busy} onClick={() => edit('create')}><Icon name="plus" size={16} />添加订阅来源</button></div></div>
    <div className="panel-body"><ErrorNotice message={snapshot.error || action.error} retry={refresh} />{notice && <p className="notice" role="status">{notice}</p>}{error && <p className="helper" role="status">来源修改暂不可用，已读取的历史信息仍可查看。</p>}<label className="source-archive-filter"><input type="checkbox" checked={includeArchived} onChange={event => setIncludeArchived(event.target.checked)} />显示已归档来源</label>
      {query.loading && !snapshot.data ? <Loading /> : !snapshot.data ? <p className="helper">来源列表尚未取得，请刷新确认。</p> : !snapshot.data.filter(source => includeArchived || !source.archived).length ? <p className="helper">暂无{includeArchived ? '' : '活跃'}订阅来源。可添加 HTTPS 地址，或粘贴、上传订阅内容。</p> : <ul className="source-list">{snapshot.data.filter(source => includeArchived || !source.archived).map(source => <li key={source.id} data-source-id={source.id}>
        <div className="source-card-heading"><div><strong>{source.name}</strong><small>{source.kind === 'url' ? `HTTPS · ${source.host ?? '主机未知'}` : '粘贴或文件'} · 设置 {source.settings_revision} / 代次 {source.identity_epoch}</small></div><Badge tone={!snapshot.fresh || source.archived ? 'neutral' : source.last_error ? 'warm' : source.latest_success ? 'good' : 'neutral'}>{!snapshot.fresh ? '来源状态待确认' : source.archived ? '已归档' : source.active_job ? sourceStatusText[source.active_job.status] : source.last_error ? '更新失败，保留上次结果' : source.latest_success ? '有成功解析版本' : '等待首次解析'}</Badge></div>
        <p className="helper">支持 {source.counts.supported} · 不支持 {source.counts.unsupported} · 身份不唯一 {source.counts.ambiguous} · 当前缺失 {source.counts.missing}</p><p className="helper">最近尝试 {dateText(source.last_attempt_at)} · 最近成功 {dateText(source.last_success_at)}{source.kind === 'url' && ` · ${source.auto_refresh ? `${sourceInterval(source.refresh_interval_secs)}自动刷新` : '自动刷新已关闭'}`}</p>
        <p className="helper">最近变化：新增 {source.changes.added} · 更新 {source.changes.updated} · 缺失 {source.changes.missing}</p><SourceUsage traffic={source.traffic} />
        {source.stale_reason && <p className="source-stale">{source.stale_reason}</p>}{source.last_error && <p className="source-stale">{source.last_error.kind}：{source.last_error.message}{source.last_error.http_status !== null && `（HTTP ${source.last_error.http_status}）`}</p>}
        <div className="source-card-actions"><button className="text-button" onClick={() => { setDetail(source); action.clearError() }}>查看来源</button><button className="text-button" disabled={action.busy || Boolean(sourceWriteError(snapshot, source.id, source.settings_revision))} onClick={() => {
          if (action.busy || sourceWriteError(snapshot, source.id, source.settings_revision)) return
          if (source.kind === 'inline') { edit('update', source); return }
          void action.run(() => refreshSource(snapshot, source), () => { setNotice('刷新任务已提交，请查看抓取与解析状态。'); refresh() })
        }}>{source.kind === 'url' ? '抓取并解析' : '更新内容'}</button><button className="text-button danger-text" disabled={action.busy || Boolean(sourceWriteError(snapshot, source.id, source.settings_revision, true))} onClick={() => remove(source)}>删除来源</button></div>
      </li>)}</ul>}
      <p className="helper">解析成功表示格式支持，不表示外部节点在线。外部节点作为有序链路中的明确版本引用，来源本身不计入上方代理资源数，也不直接授予用户。</p>
    </div>
    <SourceEditor session={session} open={editorOpen} snapshot={snapshot} refresh={refresh} onClose={() => setEditorOpen(false)} onSaved={saved} onReset={resetEditor} />
    {detail && !editorOpen && !importing && <SourceDetailDrawer key={detail.id} selected={detail} snapshot={snapshot} refresh={refresh} onClose={() => setDetail(null)} edit={edit} remove={remove} onCatalogChange={onCatalogChange} />}
    {importing && <OrderedSourceImport snapshot={snapshot} onClose={() => setImporting(false)} onSaved={imported} />}
    {deleting && <Confirm title="删除订阅来源" busy={action.busy} disabled={Boolean(sourceWriteError(snapshot, deleting.id, deleting.settings_revision, true))} error={sourceWriteError(snapshot, deleting.id, deleting.settings_revision, true) || action.error} retry={refresh} onClose={() => setDeleting(null)} onConfirm={() => { if (action.busy || sourceWriteError(snapshot, deleting.id, deleting.settings_revision, true)) return; void action.run(() => deleteSource(snapshot, deleting), () => { setDeleting(null); setNotice('来源已删除，历史版本和依赖证据按面板规则保留。'); refresh() }) }}>删除「{deleting.name}」会停止后续抓取，保留已有历史证据。若当前、待发布或恢复路径仍引用该来源，面板会拒绝删除并列出引用。</Confirm>}
  </section>
}
