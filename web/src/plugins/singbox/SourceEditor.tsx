import { useEffect, useRef, useState } from 'react'
import { errorMessage } from '../../api'
import { ErrorNotice, Field, Modal } from '../../components'
import { useAction } from '../../hooks'
import { emptySourceDraft, prepareSourceMutation, sourceCommand, SourceFileReader, sourceCommandError, sourceMetadataError, sourceMutationReplay, sourceWriteError, submitSourceMutation } from './sourceRequests'
import type { PendingSourceMutation, SourceDraft, SourceEditorMode, SourceSnapshot } from './sourceRequests'
import type { SourceReceipt, SubscriptionSource } from './orderedSourceTypes'

export type SourceEditorSession = { mode: SourceEditorMode; source?: SubscriptionSource; generation: number }
const titles: Record<SourceEditorMode, string> = { create: '添加订阅来源', metadata: '修改来源设置', update: '更新同一来源内容', replace: '更换订阅来源', archive: '归档订阅来源', unarchive: '恢复订阅来源' }
export default function SourceEditor({ session, open, snapshot, refresh, onClose, onSaved, onReset }: { session: SourceEditorSession; open: boolean; snapshot: SourceSnapshot; refresh: () => void; onClose: () => void; onSaved: (receipt: SourceReceipt) => void; onReset: () => void }) {
  const [draft, setDraft] = useState<SourceDraft>(emptySourceDraft)
  const [inputError, setInputError] = useState('')
  const [fileName, setFileName] = useState('')
  const [fileBusy, setFileBusy] = useState(false)
  const [fileKey, setFileKey] = useState(0)
  const pending = useRef<PendingSourceMutation | undefined>(undefined)
  const reader = useRef(new SourceFileReader())
  const action = useAction()
  const { mode, source } = session
  useEffect(() => {
    reader.current.invalidate(); pending.current = undefined; setInputError(''); action.clearError(); setFileName(''); setFileBusy(false); setFileKey(value => value + 1)
    setDraft({ ...emptySourceDraft(), name: source?.name ?? '', kind: source?.kind ?? 'url', interval: String(source?.kind === 'url' ? source.refresh_interval_secs : 86400) })
  // A session is changed only by an explicit editor operation, never by list polling.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session.generation])
  useEffect(() => () => reader.current.invalidate(), [])
  let command: ReturnType<typeof sourceCommand> | undefined
  try { command = sourceCommand(mode, draft, source) } catch { /* Input validation is shown on submit. */ }
  const replay = sourceMutationReplay(command, pending.current)
  const writeError = command ? sourceCommandError(command, snapshot, replay) : sourceWriteError(snapshot, source?.id, source?.settings_revision, mode === 'archive' || mode === 'unarchive')
  const set = (key: keyof SourceDraft, value: string) => {
    if (action.busy) return
    reader.current.invalidate(); setFileBusy(false); setInputError(''); action.clearError()
    setDraft(current => ({ ...current, [key]: value }))
  }
  const switchKind = (kind: 'url' | 'inline') => {
    reader.current.invalidate(); setFileBusy(false); setFileName(''); setFileKey(value => value + 1); setInputError(''); action.clearError()
    setDraft(current => ({ ...current, kind, url: '', content: '', authorization: '', cookie: '', apiKey: '', authAction: 'preserve' }))
  }
  const selectFile = (file?: File) => {
    if (!file || action.busy) return
    setFileBusy(true); setInputError('')
    void reader.current.read(file).then(content => {
      if (content === null) return
      setDraft(current => ({ ...current, content })); setFileName(file.name); setFileBusy(false)
    }).catch(error => { setInputError(errorMessage(error)); setFileBusy(false) })
  }
  const submit = () => {
    if (action.busy || fileBusy) return
    void action.run(async () => {
      const current = sourceCommand(mode, draft, source)
      const request = prepareSourceMutation(current, snapshot, pending.current)
      pending.current = request
      return submitSourceMutation(request, snapshot)
    }, receipt => {
      // Clear write-only inputs as soon as the save is acknowledged, even while parsing is queued.
      reader.current.invalidate(); pending.current = undefined; setDraft(emptySourceDraft()); setFileName(''); setFileBusy(false); setFileKey(value => value + 1); setInputError('')
      onSaved(receipt)
    })
  }
  if (!open) return null
  const input = mode === 'create' || mode === 'replace' || mode === 'update'
  const confirm = mode === 'archive' || mode === 'unarchive'
  const submitLabel = replay ? '重试原请求' : mode === 'archive' ? '确认归档' : mode === 'unarchive' ? '确认恢复' : '保存来源'
  return <Modal wide className="source-editor" title={titles[mode]} onClose={onClose} busy={action.busy}><form onSubmit={event => { event.preventDefault(); if (!action.busy && !writeError && !fileBusy) submit() }}><div className="modal-body"><ErrorNotice message={writeError || action.error} retry={refresh} /><fieldset disabled={action.busy || Boolean(writeError)}>
    {confirm ? <p>{mode === 'archive' ? `归档「${source?.name}」后停止抓取与新引用；已保存版本和历史依赖仍保留。` : `恢复「${source?.name}」后可以再次更新；请查看抓取与解析结果。`}</p> : <>
      <Field label="来源名称"><input name="source_name" required maxLength={128} value={draft.name} onChange={event => set('name', event.target.value)} autoComplete="off" /></Field>
      {input && <>
        {mode !== 'update' && <Field label="输入方式"><select aria-label="订阅输入方式" value={draft.kind} onChange={event => switchKind(event.target.value as 'url' | 'inline')}><option value="url">HTTPS 订阅地址</option><option value="inline">粘贴内容或上传文件</option></select></Field>}
        {mode === 'replace' && <p className="notice">更换地址、认证信息或来源类型会建立新的身份代次。旧节点版本保留供查看，不会自动绑定到新来源。</p>}
        {mode === 'update' && <p className="helper">这次内容属于同一来源；唯一且一致的节点身份可以延续。名称相同不会自动视为同一节点。</p>}
        {draft.kind === 'url' ? <>
          <Field label="HTTPS 订阅地址" hint={mode === 'create' || source?.kind === 'inline' ? '由面板抓取；完整地址不会通过读取接口返回。' : '原地址不会回填。留空可保留地址，只替换或清除认证信息。'}><input type="url" name="source_url" required={mode === 'create' || source?.kind === 'inline'} value={draft.url} onChange={event => set('url', event.target.value)} autoComplete="off" spellCheck={false} /></Field>
          {mode === 'replace' && <Field label="认证信息处理"><select aria-label="认证信息处理" value={draft.authAction} onChange={event => set('authAction', event.target.value)}><option value="preserve">保留已存认证信息</option><option value="replace">替换认证信息</option><option value="clear">清除认证信息</option></select></Field>}
          {(mode === 'create' || draft.authAction === 'replace') && <div className="source-auth"><p className="helper">可选认证头，仅支持以下三项，合计不超过 8 KiB。保存后立即清空，不会回填。</p>{(['authorization', 'cookie', 'apiKey'] as const).map((key, index) => <Field key={key} label={['Authorization', 'Cookie', 'X-API-Key'][index]}><input type="password" name={`source_${key}`} value={draft[key]} onChange={event => set(key, event.target.value)} autoComplete="new-password" spellCheck={false} /></Field>)}</div>}
        </> : <>
          <Field label="订阅内容" hint="支持 URI、Base64 URI、sing-box JSON 和 Clash YAML；最多 2 MiB。"><textarea name="source_content" rows={9} value={draft.content} onChange={event => { set('content', event.target.value); setFileName('') }} autoComplete="off" spellCheck={false} /></Field>
          <Field label="订阅文件" hint="读取 UTF-8 文件，仅在当前浏览器内存中保存内容。"><input key={fileKey} type="file" name="source_file" onChange={event => selectFile(event.target.files?.[0])} /></Field>
          {fileBusy && <p role="status">正在读取文件…</p>}{fileName && <p className="helper">已读取：{fileName}</p>}
        </>}
      </>}
      {draft.kind === 'url' && <Field label="自动刷新周期（秒）" hint="最少 1 小时，最多 7 天；默认每天一次。"><input type="number" name="source_interval" required min={3600} max={604800} step={1} value={draft.interval} onChange={event => set('interval', event.target.value)} /></Field>}
      {draft.kind === 'inline' && <p className="helper">粘贴或文件来源不自动刷新，需手动更新内容。</p>}
      <ErrorNotice message={inputError} />
      <p className="helper">输入和未确认的原请求仅保存在当前页面内存。保存后会清空敏感输入；抓取或解析失败不抹掉上次成功版本。离开此页面会丢失未确认请求。</p>
      {pending.current?.attempted && !replay && <p className="helper">草稿已修改，下次保存将使用新的请求标识；请先确认上次保存结果。</p>}
    </>}
  </fieldset>{(pending.current?.attempted || writeError) && <button type="button" className="text-button" disabled={action.busy || Boolean(sourceMetadataError(snapshot))} onClick={() => { if (!action.busy && !sourceMetadataError(snapshot)) onReset() }}>丢弃原请求和草稿，重新读取设置</button>}</div><footer><button type="button" className="button button-secondary" onClick={onClose} disabled={action.busy}>取消</button><button className="button button-primary" disabled={action.busy || Boolean(writeError) || fileBusy}>{action.busy ? '正在保存…' : submitLabel}</button></footer></form></Modal>
}
