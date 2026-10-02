import { useEffect, useState } from 'react'
import { api } from '../../api'
import { Badge, ErrorNotice, Field, FormDialog, Loading } from '../../components'
import { resourceWriteError, useAction, useResource } from '../../hooks'

type Reference = { external_node_id: number; source_id: number; identity_epoch: number; node_version_id: number; update_mode: 'follow_node' | 'pinned'; metadata_revision: number }
type Entry = Reference & { name: string; source_name: string; protocol: string; server: string; port: number; available: boolean; reason: string | null; current_version_id: number | null; resolved_version_id: number | null; source_last_error: string | null }
type AccessView = { revision: number; accesses: Entry[]; available_nodes: Entry[] }
type Draft = { userId: number; revision: number; accesses: Reference[] }
const reference = (entry: Reference): Reference => ({ external_node_id: entry.external_node_id, source_id: entry.source_id, identity_epoch: entry.identity_epoch, node_version_id: entry.node_version_id, update_mode: entry.update_mode, metadata_revision: entry.metadata_revision })
const reasons: Record<string, string> = { source_deleted: '来源已删除', source_archived: '来源已归档', source_replaced: '来源身份已更换', node_deleted: '节点已删除', node_not_adopted: '节点已取消采用', node_disabled: '节点已停用', ambiguous_node_identity: '节点身份不明确', node_missing: '来源中已缺失', version_missing: '版本不可用', version_identity_mismatch: '版本身份不匹配', invalid_version: '版本校验失败', parser_update_required: '请先更新订阅来源' }

export default function ExternalUserAccess({ userId, onChanged }: { userId: number; onChanged?: () => void }) {
  const resource = useResource<AccessView>(`/api/plugins/sing-box/users/${userId}/external-accesses`)
  const action = useAction()
  const [draft, setDraft] = useState<Draft | null>(null)
  const [search, setSearch] = useState('')
  const [saved, setSaved] = useState<number | null>(null)
  const [readDraft, setReadDraft] = useState<number | null>(null)
  const currentDraft = draft?.userId === userId ? draft : null
  const error = () => resourceWriteError(resource) || (currentDraft && resource.getCurrent()?.revision !== currentDraft.revision ? '授权已被修改，请重新读取后再保存；当前草稿已保留。' : '')
  const open = () => {
    const current = resource.getCurrent()
    if (!current) return
    action.clearError(); setSearch(''); setDraft({ userId, revision: current.revision, accesses: current.accesses.map(reference) })
  }
  useEffect(() => {
    if (readDraft === null) return
    if (readDraft !== userId || (resource.error && !resource.loading)) { setReadDraft(null); return }
    const current = resource.getCurrent()
    if (!current) return
    action.clearError(); setDraft({ userId, revision: current.revision, accesses: current.accesses.map(reference) }); setReadDraft(null)
  }, [readDraft, userId, resource.ready, resource.data, resource.error, resource.loading])
  const update = (accesses: Reference[]) => setDraft(previous => previous?.userId === userId ? { ...previous, accesses } : previous)
  const submit = () => {
    if (!currentDraft || error()) return
    const selectedUser = userId
    void action.run(() => api(`/api/plugins/sing-box/users/${selectedUser}/external-accesses`, 'PUT', { revision: currentDraft.revision, accesses: currentDraft.accesses }), () => { setDraft(null); setSaved(selectedUser); resource.reload(); onChanged?.() })
  }
  const entries = resource.data?.accesses ?? []
  const available = resource.data?.available_nodes ?? []
  const chosen = new Set(currentDraft?.accesses.map(value => value.external_node_id))
  const needle = search.trim().toLocaleLowerCase()
  const candidates = available.filter(entry => !chosen.has(entry.external_node_id) && (!needle || `${entry.name} ${entry.source_name} ${entry.protocol} ${entry.server}`.toLocaleLowerCase().includes(needle)))
  return <section className="panel external-user-access" aria-label="外部节点授权" style={{ overflowWrap: 'anywhere' }}>
    <div className="panel-heading"><h2>外部节点 <span className="count">{entries.length}</span></h2><button className="button button-secondary button-small" disabled={Boolean(resourceWriteError(resource))} onClick={open}>管理分配</button></div>
    <div className="panel-body">
      <ErrorNotice message={resource.error} retry={resource.reload} />
      {resource.loading && !resource.data ? <Loading /> : entries.length ? <div className="group-choices">{entries.map(entry => <div className="group-choice" style={{ flexWrap: 'wrap' }} key={entry.external_node_id}><span><strong>{entry.name}</strong><small>{entry.source_name} · {entry.protocol} · {entry.update_mode === 'pinned' ? '固定版本' : '跟随更新'}</small></span><Badge tone={entry.available ? 'good' : 'warm'}>{entry.available ? entry.source_last_error ? '保留上次成功版本' : '可生成订阅' : reasons[entry.reason ?? ''] ?? '当前不可用'}</Badge></div>)}</div> : <p className="helper">尚未分配外部节点。</p>}
      {saved === userId && <p role="status" className="helper">外部节点分配已保存，后续订阅按新分配生成。</p>}
      <p className="helper">外部运行和流量由提供方控制，本面板不计量外部用量；取消分配不会让已下载的外部凭据失效。</p>
    </div>
    {currentDraft && <FormDialog title="分配外部节点" wide onClose={() => setDraft(null)} onSubmit={submit} busy={action.busy} submitDisabled={Boolean(error())} error={error() || action.error} submitLabel="保存分配">
      <div className="row-actions"><strong>已选 {currentDraft.accesses.length} 个</strong><button className="text-button" type="button" disabled={readDraft !== null} onClick={() => { setReadDraft(userId); resource.reload() }}>重新读取授权</button></div>
      <fieldset className="group-choices"><legend>已分配节点</legend>{currentDraft.accesses.map(value => {
        const candidate = available.find(entry => entry.external_node_id === value.external_node_id)
        const entry = entries.find(entry => entry.external_node_id === value.external_node_id) ?? candidate
        return <div className="group-form-grid" key={value.external_node_id}><label className="group-choice"><input type="checkbox" checked onChange={() => update(currentDraft.accesses.filter(item => item.external_node_id !== value.external_node_id))} /><span>{entry?.name ?? `外部节点 #${value.external_node_id}`}<small>{entry?.source_name}{entry && !entry.available ? ` · ${reasons[entry.reason ?? ''] ?? '当前不可用'}` : ''}</small></span></label><Field label="更新方式"><select aria-label={`${entry?.name ?? value.external_node_id} 更新方式`} value={value.update_mode} disabled={!candidate?.available} onChange={event => { if (candidate) update(currentDraft.accesses.map(item => item.external_node_id === value.external_node_id ? { ...reference(candidate), update_mode: event.target.value as Reference['update_mode'] } : item)) }}><option value="follow_node">跟随同一节点更新</option><option value="pinned">固定所选版本</option></select></Field></div>
      })}{!currentDraft.accesses.length && <p className="helper">从下方选择已采用的外部节点。</p>}</fieldset>
      <Field label="搜索外部节点"><input value={search} onChange={event => setSearch(event.target.value)} placeholder="名称、来源、协议或地址" autoComplete="off" /></Field>
      <fieldset className="group-choices"><legend>可选节点</legend>{candidates.map(entry => <label className="group-choice" key={entry.external_node_id}><input type="checkbox" checked={false} disabled={!entry.available} onChange={() => update([...currentDraft.accesses, reference(entry)])} /><span>{entry.name}<small>{entry.source_name} · {entry.protocol} · {entry.server}:{entry.port}{!entry.available ? ` · ${reasons[entry.reason ?? ''] ?? '当前不可用'}` : entry.source_last_error ? ' · 保留上次成功版本' : ''}</small></span></label>)}{!candidates.length && <p className="helper">没有符合条件的未选节点。请先在节点管理中采用并启用外部节点。</p>}</fieldset>
      <p className="helper">生成订阅仍检查用户套餐状态；外部节点使用提供方现有凭据，用量未知。</p>
    </FormDialog>}
  </section>
}
