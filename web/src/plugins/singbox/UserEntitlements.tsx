import { useEffect, useState } from 'react'
import { api } from '../../api'
import { Badge, ErrorNotice, Field, FormDialog, Loading } from '../../components'
import { bytes } from '../../format'
import { useAction, useResource } from '../../hooks'
import { assignmentRequestId, dateText, scheduleText, statusText } from './groupTypes'
import type { Entitlement, PackageGroup, PolicyGroup, UserPolicies } from './groupTypes'

const root = '/api/plugins/sing-box'
export default function UserEntitlements({ id, onChange }: { id: number; onChange: () => void }) {
  const policies = useResource<PolicyGroup[]>(`${root}/policy-groups`)
  const packages = useResource<PackageGroup[]>(`${root}/package-groups`)
  const assigned = useResource<UserPolicies>(`${root}/users/${id}/policy-groups`)
  const entitlement = useResource<Entitlement>(`${root}/users/${id}/entitlement`)
  const [selected, setSelected] = useState<number[]>([])
  const [assignment, setAssignment] = useState<string | null>(null)
  const [notice, setNotice] = useState('')
  const action = useAction()
  useEffect(() => { if (assigned.data) setSelected(assigned.data.group_ids) }, [assigned.data])
  useEffect(() => { const timer = window.setInterval(entitlement.reload, 15000); return () => window.clearInterval(timer) }, [entitlement.reload])
  const refresh = () => { policies.reload(); packages.reload(); assigned.reload(); entitlement.reload() }
  const e = entitlement.data
  const savePolicies = () => void action.run(() => api(`${root}/users/${id}/policy-groups`, 'PUT', { group_ids: selected }), () => { assigned.reload(); onChange(); setNotice('策略组分配已保存。单独授权仍保留，设备应用配置后更新可用节点。') })
  const assign = (form: FormData) => void action.run(() => api(`${root}/users/${id}/package`, 'POST', { package_group_id: Number(form.get('package_group_id')), request_id: assignment }), () => { setAssignment(null); entitlement.reload(); onChange(); setNotice('套餐已分配，按分配时刻计算有效期。本期历史用量没有清空。') })
  return <section className="panel">
    <div className="panel-heading"><h2>可用范围与套餐</h2><a className="text-button" href="#/plugins/sing-box/groups">管理策略与套餐</a></div>
    <div className="panel-body">
      <ErrorNotice message={policies.error || packages.error || assigned.error || entitlement.error || (!assignment ? action.error : '')} retry={refresh} />
      {notice && <p className="notice notice-success" role="status">{notice}</p>}
      <h3>策略组</h3>
      {!assigned.data ? <Loading /> : policies.data?.length ? <><div className="group-choices">{policies.data.map(p => <label className="group-choice" key={p.id}><input type="checkbox" checked={selected.includes(p.id)} disabled={action.busy} onChange={event => setSelected(previous => event.target.checked ? [...previous, p.id] : previous.filter(value => value !== p.id))} /><span>{p.name}<small>{p.node_ids.length} 个节点，{p.chain_ids.length} 条链路</small></span></label>)}</div><button className="button button-secondary button-small" disabled={action.busy || !policies.data || Boolean(policies.error)} onClick={savePolicies}>{action.busy ? '正在保存…' : '保存策略组分配'}</button></> : <p className="helper">尚未创建策略组，仍可使用下方的单独节点授权。</p>}
      <div className="group-entitlement-heading"><h3>当前套餐</h3><button className="button button-secondary button-small" disabled={action.busy || !packages.data?.length} onClick={() => { action.clearError(); setAssignment(assignmentRequestId()) }}>分配或更换套餐</button></div>
      {!e ? <Loading /> : <>
        <div className="group-entitlement-heading"><strong>{e.package_name ?? '未设置流量与到期限制'}</strong><Badge tone={e.allowed ? 'good' : 'bad'}>{statusText[e.status]}</Badge></div>
        {e.package_group_id !== null && <>
          <dl className="group-details"><div><dt>本期已用 / 每月额度</dt><dd>{bytes(e.used_bytes)} / {e.monthly_bytes === null ? '不限量' : bytes(e.monthly_bytes)}</dd></div><div><dt>本期起点</dt><dd>{dateText(e.cycle_start, e.timezone)}</dd></div><div><dt>下次重置</dt><dd>{dateText(e.next_reset, e.timezone)}</dd></div><div><dt>可使用至</dt><dd>{dateText(e.expires_at, e.timezone)}</dd></div></dl>
          <p className="helper">按 {e.timezone} 显示。重置只恢复月度流量，不延长套餐有效期。</p>
        </>}
        <p className="helper">{e.package_group_id === null ? '未分配套餐时保留兼容模式，不限制流量或有效期。分配套餐后，所有策略组与单独授权共享此用户的套餐额度。' : '到期或用完流量后立即停止提供可用订阅，设备应用新配置后停止服务。设备离线或流量尚未上报时，限制不会瞬间生效。'}</p>
      </>}
    </div>
    {assignment && <FormDialog title="分配套餐" onClose={() => setAssignment(null)} onSubmit={assign} busy={action.busy} error={action.error} submitLabel="确认分配">
      <Field label="套餐组"><select name="package_group_id" required defaultValue=""><option value="" disabled>选择套餐</option>{packages.data?.map(p => <option key={p.id} value={p.id}>{p.name} · {p.monthly_bytes === null ? '不限量' : bytes(p.monthly_bytes)} / 月 · {p.duration_days} 天</option>)}</select></Field>
      {packages.data?.map(p => <p className="helper" key={p.id}>{p.name}：{scheduleText(p)}</p>)}
      <p>此操作替换当前套餐，使用期限从现在重新计算，不是在原到期日上续加。本月已记录的流量不会清空。变更月度重置规则会重新按新规则统计当前周期。</p>
      <p className="helper">套餐与节点权限分别分配；仅分配套餐不会自动授予节点。</p>
    </FormDialog>}
  </section>
}
