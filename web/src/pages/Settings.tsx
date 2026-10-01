import { useState } from 'react'
import { api } from '../api'
import { ErrorNotice, Field, Loading, PageHeader } from '../components'
import { useAction, useResource } from '../hooks'
import './server-setup.css'

type Preferences = { public_dashboard: boolean; offline_alerts: boolean; offline_minutes: number; telegram_enabled: boolean; telegram_chat_id: string; telegram_token_configured: boolean }
function Form({ initial }: { initial: Preferences }) {
  const [value, setValue] = useState(initial), [token, setToken] = useState(''), [clearToken, setClearToken] = useState(false), [saved, setSaved] = useState(false)
  const action = useAction()
  const change = (next: Partial<Preferences>) => { setValue(current => ({ ...current, ...next })); setSaved(false) }
  return <form className="panel" onSubmit={event => { event.preventDefault(); const { telegram_token_configured: _, ...settings } = value; void action.run(() => api<Preferences>('/api/settings', 'PATCH', { ...settings, ...(clearToken ? { telegram_token: '' } : token ? { telegram_token: token } : {}) }), result => { setValue(result); setToken(''); setClearToken(false); setSaved(true) }) }}><div className="panel-body"><fieldset disabled={action.busy}>
    <label className="server-setup-toggle"><span><strong>公开服务器看板</strong><small>开启后，无需登录即可查看未隐藏服务器的运行状态和拨测数据。管理操作仍需登录。</small></span><input role="switch" type="checkbox" checked={value.public_dashboard} onChange={event => change({ public_dashboard: event.target.checked })} /><span className="server-setup-switch" aria-hidden="true" /></label>
    <label className="server-setup-toggle"><span><strong>启用离线告警</strong><small>已上报过的服务器持续离线达到阈值时告警，重新上线后记录恢复。</small></span><input role="switch" type="checkbox" checked={value.offline_alerts} onChange={event => change({ offline_alerts: event.target.checked })} /><span className="server-setup-switch" aria-hidden="true" /></label>
    <Field label="离线告警阈值（分钟）" hint="2–1440 分钟。面板重启后有同等时长的宽限期。"><input required type="number" min={2} max={1440} value={value.offline_minutes} onChange={event => change({ offline_minutes: Number(event.target.value) })} /></Field>
    <label className="server-setup-toggle"><span><strong>Telegram 通知</strong><small>将离线及恢复事件发送到指定会话；发送失败时自动重试。</small></span><input role="switch" type="checkbox" checked={value.telegram_enabled} onChange={event => change({ telegram_enabled: event.target.checked })} /><span className="server-setup-switch" aria-hidden="true" /></label>
    <div className="server-setup-grid"><Field label="机器人令牌" hint={value.telegram_token_configured ? '已配置；留空保留原令牌。' : '通过 BotFather 创建机器人并获取令牌。'}><input type="password" autoComplete="new-password" maxLength={256} value={token} onChange={event => { setToken(event.target.value); setClearToken(false); setSaved(false) }} placeholder={value.telegram_token_configured ? '已配置，留空保持' : '输入机器人令牌'} /></Field><Field label="会话 ID" hint="支持数字会话 ID 或公开频道 @名称，需先向机器人发送消息或将其加入群组。"><input maxLength={128} value={value.telegram_chat_id} onChange={event => change({ telegram_chat_id: event.target.value })} /></Field></div>
    {value.telegram_token_configured && <label><input type="checkbox" checked={clearToken} onChange={event => { setClearToken(event.target.checked); if (event.target.checked) change({ telegram_enabled: false }) }} />清除已保存的机器人令牌</label>}
  </fieldset><ErrorNotice message={action.error} />{saved && <p role="status">设置已保存。</p>}<button className="button button-primary" disabled={action.busy}>{action.busy ? '正在保存…' : '保存设置'}</button></div></form>
}

export default function Settings() {
  const resource = useResource<Preferences>('/api/settings', 0)
  return <><PageHeader eyebrow="系统设置" title="看板与通知" description="设置服务器看板访问权限和离线通知。" /><ErrorNotice message={resource.error} retry={resource.reload} />{resource.data ? <Form initial={resource.data} /> : resource.loading ? <Loading /> : null}</>
}
