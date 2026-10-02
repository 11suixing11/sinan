import { Field } from '../components'
import { probeLeaseNotice } from '../probes'
import type { ProbeAuthorizationDraft } from '../probes'

export default function ProbeAuthorizationFields({ value, onChange }: { value: ProbeAuthorizationDraft; onChange: (value: ProbeAuthorizationDraft) => void }) {
  const change = (part: Partial<ProbeAuthorizationDraft>) => onChange({ ...value, ...part })
  return <section aria-label="目标使用授权">
    <h3>目标使用授权</h3>
    <p className="helper">只检测自有目标或已取得同意的第三方目标。请明确登记，主机名、地区和历史测量不会自动证明授权。记录仅供管理员查看。</p>
    <div className="form-grid">
      <Field label="目标地区（可选）" hint="最多 64 个 UTF-8 字节。"><input maxLength={64} value={value.region} onChange={event => change({ region: event.target.value })} placeholder="例如：日本 / 华东" /></Field>
      <Field label="目标来源" hint="例如自有服务清单或对方公布的测点说明，最多 256 个 UTF-8 字节。"><input maxLength={256} value={value.source} onChange={event => change({ source: event.target.value })} placeholder="请填写真实来源" /></Field>
      <Field label="使用依据"><select value={value.scope} onChange={event => change({ scope: event.target.value as ProbeAuthorizationDraft['scope'] })}><option value="">请选择使用依据</option><option value="owned">自有目标</option><option value="third_party">第三方已获同意</option></select></Field>
      <Field label="同意或管理记录" hint="记录负责人、批准时间或可核对的管理依据，最多 512 个 UTF-8 字节。"><input maxLength={512} value={value.evidence} onChange={event => change({ evidence: event.target.value })} placeholder="请填写可核对的记录" /></Field>
      <Field label="授权截止时间（可选）" hint="使用本地时间；留空表示没有设置授权截止，但设备仍需刷新短期许可。"><input type="datetime-local" step={1} value={value.expires} onChange={event => change({ expires: event.target.value })} /></Field>
    </div>
    <p className="helper">{probeLeaseNotice}</p>
  </section>
}
