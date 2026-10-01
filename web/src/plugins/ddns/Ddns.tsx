import { useState } from 'react'
import { api } from '../../api'
import { Badge, Confirm, Empty, ErrorNotice, Field, Icon, Loading, Modal, PageHeader, Refresh } from '../../components'
import { ddnsMessage, ddnsWrite } from './types'
import type { DdnsConfig, DdnsRule } from './types'
import { time } from '../../format'
import { useAction, useResource } from '../../hooks'
type Server = { id: number; name: string; online: boolean; enabled: boolean }
import './ddns.css'

const empty: DdnsConfig = { name: '', server_id: 0, zone_id: '', record_name: '', record_type: 'A', ttl: 1, proxied: false, interval_secs: 300, enabled: true, adopt_existing: false }

function Editor({ rule, servers, close, saved }: { rule?: DdnsRule; servers: Server[]; close: () => void; saved: () => void }) {
  const [config, setConfig] = useState<DdnsConfig>(rule?.config ?? { ...empty, server_id: servers[0]?.id ?? 0 })
  const [token, setToken] = useState('')
  const action = useAction()
  const change = (part: Partial<DdnsConfig>) => setConfig(value => ({ ...value, ...part }))
  return <Modal title={rule ? `编辑动态解析 · ${rule.config.name}` : '添加动态解析'} busy={action.busy} onClose={close} wide className="ddns-editor"><form onSubmit={event => {
    event.preventDefault()
    void action.run(() => api(rule ? `/api/plugins/ddns/rules/${rule.id}` : '/api/plugins/ddns/rules', rule ? 'PATCH' : 'POST', ddnsWrite(config, token, rule?.revision)), () => { setToken(''); saved() })
  }}><div className="modal-body"><ErrorNotice message={action.error} /><fieldset disabled={action.busy}>
    <div className="ddns-provider"><strong>Cloudflare</strong><span>从 Agent 上报的公网地址更新 DNS</span></div>
    <div className="form-grid"><Field label="规则名称"><input value={config.name} required maxLength={128} onChange={event => change({ name: event.target.value })} placeholder="例如：家庭服务器 IPv4" /></Field>
      <Field label="绑定服务器"><select value={config.server_id} required onChange={event => change({ server_id: Number(event.target.value) })}>{!servers.some(server => server.id === config.server_id) && <option value={config.server_id}>{rule?.server_name ?? '请选择服务器'}（不可用）</option>}{servers.map(server => <option key={server.id} value={server.id}>{server.name}{!server.online ? '（离线）' : ''}</option>)}</select></Field>
      <Field label="完整域名" hint="支持泛域名与中文域名，例如 node.example.com 或 *.example.com。"><input value={config.record_name} disabled={Boolean(rule)} required maxLength={253} autoCapitalize="none" spellCheck={false} onChange={event => change({ record_name: event.target.value })} placeholder="node.example.com" /></Field>
      <Field label="记录类型" hint="同时使用 IPv4 与 IPv6 时，各添加一条规则。"><select value={config.record_type} disabled={Boolean(rule)} onChange={event => change({ record_type: event.target.value as DdnsConfig['record_type'] })}><option value="A">A · IPv4</option><option value="AAAA">AAAA · IPv6</option></select></Field>
      <Field label="Zone ID" hint="在 Cloudflare 对应站点的概述页面复制区域标识。"><input value={config.zone_id} disabled={Boolean(rule)} required pattern="[a-fA-F0-9]{32}" maxLength={32} spellCheck={false} autoCapitalize="none" onChange={event => change({ zone_id: event.target.value.trim() })} placeholder="32 位区域标识" /></Field>
      <Field label="API Token" hint={rule ? '已保存的凭据不回显；留空保留，填写则替换。' : '创建仅限此 Zone、包含 DNS 编辑权限的 API 令牌。'}><input type="password" value={token} autoComplete="new-password" required={!rule} minLength={16} maxLength={256} onChange={event => setToken(event.target.value)} placeholder={rule?.token_configured ? '已配置，留空保留' : '输入 Cloudflare API Token'} /></Field>
      <Field label="TTL（秒）" hint="1 表示自动；手动范围为 60–86400 秒。"><input type="number" required min={1} max={86400} step={1} value={config.proxied ? 1 : config.ttl} disabled={config.proxied} onChange={event => change({ ttl: Number(event.target.value) })} /></Field>
      <Field label="检查间隔（秒）" hint="60–86400 秒；仅在地址或设置变化时更新 DNS。"><input type="number" required min={60} max={86400} step={1} value={config.interval_secs} onChange={event => change({ interval_secs: Number(event.target.value) })} /></Field>
    </div>
    <label className="ddns-toggle"><input type="checkbox" checked={config.proxied} onChange={event => change({ proxied: event.target.checked })} /><span><strong>启用 Cloudflare 代理</strong><small>代理仅适用于 Cloudflare 支持的流量与端口，会自动使用自动 TTL；普通代理协议通常应保持关闭。</small></span></label>
    <label className="ddns-toggle"><input type="checkbox" checked={config.adopt_existing} onChange={event => change({ adopt_existing: event.target.checked })} /><span><strong>接管已有的同名同类型记录</strong><small>勾选后允许覆盖唯一已有记录的 IP、TTL 和代理设置。重复记录仍需先在 Cloudflare 整理。</small></span></label>
    <label className="ddns-toggle"><input type="checkbox" checked={config.enabled} onChange={event => change({ enabled: event.target.checked })} /><span><strong>启用自动同步</strong><small>保存后按间隔同步；离线、无有效公网地址时保留现有解析。暂停或删除规则不会删除 DNS 记录。</small></span></label>
    <p className="helper">多地址优先沿用仍在 Agent 报告中的上次成功地址，否则按地址排序选取首个。若服务器处于 NAT 后，请在采样设置中启用公网 IP 发现。{rule && 'Zone、域名和记录类型固定，变更时请另建规则。'}</p>
  </fieldset></div><footer><button type="button" className="button button-secondary" disabled={action.busy} onClick={close}>取消</button><button className="button button-primary" disabled={action.busy || !config.server_id}>{action.busy ? '正在保存…' : '保存规则'}</button></footer></form></Modal>
}

export default function Ddns({ serverId }: { serverId?: number }) {
  const rules = useResource<DdnsRule[]>('/api/plugins/ddns/rules', 5000), servers = useResource<Server[]>('/api/plugins/ddns/servers', 5000)
  const [editing, setEditing] = useState<DdnsRule | 'new' | null>(null), [removing, setRemoving] = useState<DdnsRule | null>(null)
  const action = useAction()
  const scope = servers.data?.filter(server => !serverId || server.id === serverId) ?? []
  const enabledServers = scope.filter(server => server.enabled)
  const visibleRules = rules.data?.filter(rule => !serverId || rule.config.server_id === serverId)
  const refresh = () => { rules.reload(); servers.reload() }
  const open = (rule: DdnsRule | 'new') => { action.clearError(); setEditing(rule) }
  return <div className="ddns-page"><PageHeader eyebrow="服务器网络" title="动态域名解析" description="让域名跟随服务器公网 IP 更新，当前支持 Cloudflare。"><Refresh onClick={refresh} /><button className="button button-primary" disabled={!enabledServers.length || Boolean(servers.error) || (rules.data?.length ?? 32) >= 32} onClick={() => open('new')}><Icon name="plus" size={16} />添加规则</button></PageHeader>
    <ErrorNotice message={rules.error || servers.error || action.error} retry={refresh} />
    <section className="panel"><div className="panel-heading"><h2>DDNS 插件启用状态</h2><Badge>面板插件</Badge></div><div className="panel-body"><p className="helper">按服务器启用；使用已有 Agent 上报的 IP，Cloudflare 凭据与 DNS 更新由面板插件管理。无需在设备上下载或安装额外程序。</p>{scope.map(server => <div className="ddns-rule-footer ddns-activation" key={server.id}><span>{server.name} · {server.enabled ? '插件已启用' : '插件未启用'}</span><button className="button button-secondary button-small" disabled={action.busy || Boolean(servers.error)} onClick={() => void action.run(() => api(`/api/plugins/ddns/servers/${server.id}/${server.enabled ? 'disable' : 'enable'}`, 'POST'), refresh)}>{server.enabled ? '停用 DDNS 插件' : '启用 DDNS 插件'}</button></div>)}{!scope.length && <p className="helper">请先添加服务器，再启用 DDNS 插件。</p>}</div></section>
    <section className="panel"><div className="panel-heading"><h2>Cloudflare 解析规则</h2><span className="subtle">{rules.data?.length ?? 0} / 32 条</span></div>
      {!rules.data && rules.loading ? <Loading /> : !visibleRules?.length ? <Empty icon="nodes" title="尚未配置动态解析" description="添加域名、Zone ID 和 API Token，选择已经接入 Agent 的服务器。" /> : <div className="ddns-list">{visibleRules.map(rule => <article className="ddns-rule" key={rule.id}>
        <div className="ddns-rule-heading"><div><h3>{rule.config.name}</h3><code>{rule.config.record_name}</code><span className="subtle"> · {rule.config.record_type}</span></div><Badge tone={!rule.config.enabled || !rule.plugin_enabled ? 'neutral' : rule.error_code ? 'warm' : rule.last_success_at ? 'good' : 'neutral'}>{!rule.plugin_enabled ? '插件未启用' : !rule.config.enabled ? '已暂停' : rule.busy ? '正在同步' : ddnsMessage(rule.status === 'running' ? 'pending' : rule.status)}</Badge></div>
        <dl className="ddns-details"><div><dt>绑定服务器</dt><dd>{rule.server_name}</dd></div><div><dt>本次候选地址</dt><dd className="mono">{rule.candidate_ip ?? '等待有效地址'}</dd></div><div><dt>上次成功地址</dt><dd className="mono">{rule.last_ip ?? '尚未同步'}</dd></div><div><dt>检查间隔 / TTL</dt><dd>{rule.config.interval_secs} 秒 / {rule.config.ttl === 1 ? '自动' : `${rule.config.ttl} 秒`}</dd></div><div><dt>最近成功核对</dt><dd>{rule.last_success_at ? time(rule.last_success_at) : '尚未成功'}</dd></div><div><dt>下次检查</dt><dd>{!rule.config.enabled || !rule.plugin_enabled ? '暂停期间不检查' : rule.busy ? '正在检查' : rule.next_run_at ? time(rule.next_run_at) : '即将检查'}</dd></div></dl>
        <div className="ddns-rule-footer"><span className="subtle">{rule.config.proxied ? '已开启 Cloudflare 代理' : '仅 DNS'} · 凭据{rule.token_configured ? '已配置' : '未配置'}{rule.ip_received_at && ` · IP 报告 ${time(rule.ip_received_at)}`}</span><div className="ddns-actions"><button className="text-button" disabled={action.busy || rule.busy || !rule.config.enabled || !rule.plugin_enabled} onClick={() => void action.run(() => api(`/api/plugins/ddns/rules/${rule.id}/sync`, 'POST'), rules.reload)}>立即同步</button><button className="text-button" disabled={action.busy || rule.busy || !servers.data || Boolean(servers.error)} onClick={() => open(rule)}>编辑</button><button className="text-button" disabled={action.busy || rule.busy} onClick={() => void action.run(() => api(`/api/plugins/ddns/rules/${rule.id}`, 'PATCH', ddnsWrite({ ...rule.config, enabled: !rule.config.enabled }, '', rule.revision)), rules.reload)}>{rule.config.enabled ? '暂停' : '启用'}</button><button className="text-button danger-text" disabled={action.busy || rule.busy} onClick={() => { action.clearError(); setRemoving(rule) }}>删除</button></div></div>
        {(rule.error_code || rule.ip_status !== 'ready') && <p className="ddns-status" role="status">{ddnsMessage(rule.error_code) || ddnsMessage(rule.ip_status)}</p>}
      </article>)}</div>}
    </section><p className="helper">同步成功表示 Cloudflare 已接收配置或核对一致；各地 DNS 缓存生效仍受 TTL 影响。手动同步有 60 秒冷却，限流或失败时等待页面显示的重试时间。</p>
    {editing && servers.data && <Editor key={editing === 'new' ? 'new' : editing.id} rule={editing === 'new' ? undefined : editing} servers={enabledServers} close={() => setEditing(null)} saved={() => { setEditing(null); refresh() }} />}
    {removing && <Confirm title="删除动态解析规则" busy={action.busy} error={action.error} onClose={() => setRemoving(null)} onConfirm={() => void action.run(() => api(`/api/plugins/ddns/rules/${removing.id}`, 'DELETE'), () => { setRemoving(null); rules.reload() })}>确认删除“{removing.config.name}”？Cloudflare 中的 DNS 记录会保留，之后不再自动更新。</Confirm>}
  </div>
}
