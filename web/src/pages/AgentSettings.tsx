import { useEffect, useState } from 'react'
import { api } from '../api'
import { ErrorNotice, Loading } from '../components'
import { useAction, useResource } from '../hooks'
import type { AgentSettings as Settings } from '../types'

export function AgentSettings({ serverId }: { serverId: number }) {
  const resource = useResource<Settings>(`/api/servers/${serverId}/agent-settings`)
  const action = useAction()
  const [form, setForm] = useState<Settings | null>(null)
  const [saved, setSaved] = useState(false)
  useEffect(() => { setForm(null); setSaved(false) }, [serverId])
  useEffect(() => { if (!form && resource.data) setForm(resource.data) }, [form, resource.data])
  return <section className="panel"><div className="panel-heading"><h2>Agent 设置</h2><span className="subtle">设备在线后自动同步</span></div><div className="panel-body">
    <ErrorNotice message={resource.error || action.error} />
    {!form ? <Loading /> : <form onSubmit={event => { event.preventDefault(); void action.run(() => api<Settings>(`/api/servers/${serverId}/agent-settings`, 'PATCH', form), value => { setForm(value); setSaved(true); resource.reload() }) }}>
      <div className="form-grid"><label>采样间隔（秒）<input type="number" min="1" max="60" required value={form.sample_interval_secs} onChange={event => { setSaved(false); setForm({ ...form, sample_interval_secs: Number(event.target.value) }) }} /></label>
      <label>批量上传间隔（秒）<input type="number" min={form.sample_interval_secs} max="60" required value={form.upload_interval_secs} onChange={event => { setSaved(false); setForm({ ...form, upload_interval_secs: Number(event.target.value) }) }} /></label></div>
      <label><input type="checkbox" checked={form.discover_public_ips} onChange={event => { setSaved(false); setForm({ ...form, discover_public_ips: event.target.checked }) }} /> 自动识别公网 IPv4 / IPv6</label>
      <p className="helper">公网识别使用固定的 icanhazip 接口；设备本地关闭该功能时，面板设置不会覆盖本地限制。</p>
      <label><input type="checkbox" checked={form.auto_update} onChange={event => { setSaved(false); setForm({ ...form, auto_update: event.target.checked }) }} /> 自动更新 Agent</label>
      <p className="helper">从 GitHub 下载面板选定的兼容签名 Agent，保留身份与账本。可在编辑服务器中设置下载加速。</p>
      <button className="button button-primary" disabled={action.busy} type="submit">{action.busy ? '保存中…' : '保存设置'}</button>{saved && <span className="subtle"> 已保存，设备会在一分钟内同步。</span>}
    </form>}
  </div></section>
}
