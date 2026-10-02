import { useEffect, useState } from 'react'
import { api } from '../../api'
import { CopyField, ErrorNotice, Field, FormDialog, Loading, Modal, Refresh } from '../../components'
import { time } from '../../format'
import { useAction, useResource } from '../../hooks'
import type { PasskeyInfo } from '../../passkeys'

type Access = { configuration: PasskeyInfo; keys: number; url: string | null; activation_expires_at: number | null }
type Invitation = { url: string; expires_at: number }

export default function UserPasskeyAccess({ id, disabled, refreshRevision }: { id: number; disabled: boolean; refreshRevision: number }) {
  const resource = useResource<Access>(`/api/plugins/sing-box/users/${id}/portal`, 0)
  const action = useAction()
  const [editing, setEditing] = useState<{ reset: boolean }>()
  const [invitation, setInvitation] = useState<Invitation>()
  const reset = (resource.data?.keys ?? 0) > 0
  useEffect(() => { if (refreshRevision) resource.reload() }, [refreshRevision, resource.reload])
  const submit = (form: FormData) => {
    if (!editing || disabled || !resource.isCurrent()) return
    void action.run(() => api<Invitation>(`/api/plugins/sing-box/users/${id}/portal/invitation`, 'POST', {
      password: String(form.get('password') ?? ''), totp_code: String(form.get('totp_code') ?? ''), reset: editing.reset,
    }), value => { setEditing(undefined); setInvitation(value); resource.reload() })
  }
  return <section className="panel">
    <div className="panel-heading"><h2>用户 Passkey 入口</h2><Refresh onClick={resource.reload} /></div>
    <div className="panel-body">
      <p>用户通过专属入口登录，查看自己的订阅与流量，并管理通行密钥。</p>
      <ErrorNotice message={resource.error} retry={resource.reload} />
      {resource.loading ? <Loading /> : resource.data && <>
        <p>{reset ? `已绑定 ${resource.data.keys} 把 Passkey。` : resource.data.activation_expires_at ? `等待开通，当前链接有效至 ${time(resource.data.activation_expires_at)}。` : '尚未开通用户入口。'}</p>
        {resource.data.url && <div className="field"><span>用户登录地址</span><CopyField text={resource.data.url} label="复制用户登录地址" /></div>}
        {resource.data.configuration.reason && <p className="helper">{resource.data.configuration.reason}</p>}
        <button className={`button ${reset ? 'button-danger' : 'button-primary'} button-small`} disabled={disabled || action.busy || !resource.ready || !resource.data.configuration.enabled} onClick={() => { action.clearError(); setEditing({ reset }) }}>{reset ? '重置用户 Passkey' : '生成开通链接'}</button>
      </>}
    </div>
    {editing && <FormDialog title={editing.reset ? '重置用户 Passkey？' : '生成用户开通链接'} busy={action.busy} submitDisabled={disabled || !resource.ready} error={action.error} onClose={() => setEditing(undefined)} onSubmit={submit} submitLabel={editing.reset ? '确认重置并生成' : '验证并生成'}>
      <p>{editing.reset ? '原有 Passkey 和用户登录会话将立即失效，用户需要通过新链接重新开通。订阅、授权和流量保持不变。' : '开通链接十五分钟内有效，只能使用一次；重新生成会使之前的链接失效。请仅发送给该代理用户。'}</p>
      <Field label="管理员密码"><input name="password" type="password" required maxLength={1024} autoComplete="current-password" /></Field>
      <Field label="二步验证码" hint="已启用时必填，请使用尚未用于登录的验证码。"><input name="totp_code" inputMode="numeric" pattern="[0-9]{6}" maxLength={6} autoComplete="one-time-code" placeholder="未启用时留空" /></Field>
    </FormDialog>}
    {invitation && <Modal title="用户开通链接" onClose={() => setInvitation(undefined)}><div className="modal-body"><p>有效至 {time(invitation.expires_at)}，只能使用一次。关闭后不会再次显示完整链接。</p><CopyField text={invitation.url} label="复制开通链接" /><p className="helper">请将此链接私下交给该用户。它用于绑定 Passkey，不是代理订阅地址。</p></div></Modal>}
  </section>
}
