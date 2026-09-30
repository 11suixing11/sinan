import { useState } from 'react'
import { api, errorMessage } from '../api'
import { Badge, Confirm, CopyField, Empty, ErrorNotice, Field, FormDialog, Icon, Loading, Meter, Modal, PageHeader, Refresh, Stat } from '../components'
import { bytes, navigate, percent, time } from '../format'
import { useAction, useResource } from '../hooks'
import type { Enrollment, Server } from '../types'

export default function Servers() {
  const resource = useResource<Server[]>('/api/servers')
  const action = useAction()
  const [editor, setEditor] = useState<Server | 'new' | null>(null)
  const [deleting, setDeleting] = useState<Server | null>(null)
  const [installation, setInstallation] = useState<{ server: Server; enrollment?: Enrollment; warning?: string } | null>(null)
  const [agentVersion, setAgentVersion] = useState('')
  const servers = resource.data ?? []
  const edit = (value: Server | 'new') => { action.clearError(); setEditor(value) }
  const enroll = (server: Server) => { action.clearError(); setInstallation({ server }); const query = agentVersion.trim() ? `?agent_version=${encodeURIComponent(agentVersion.trim())}` : ''; void action.run(() => api<Enrollment>(`/api/servers/${server.id}/enrollment${query}`, 'POST'), enrollment => setInstallation({ server, enrollment })) }
  const submit = (form: FormData) => {
    const name = String(form.get('name') ?? '').trim()
    if (editor === 'new') {
      setAgentVersion('')
      void action.run(async () => {
        const server = await api<Server>('/api/servers', 'POST', { name })
        try { return { server, enrollment: await api<Enrollment>(`/api/servers/${server.id}/enrollment`, 'POST') } }
        catch (error) { return { server, warning: `服务器已创建，接入命令生成失败：${errorMessage(error)}` } }
      }, result => { setEditor(null); setInstallation(result); resource.reload() })
    } else if (editor) {
      void action.run(() => api(`/api/servers/${editor.id}`, 'PATCH', { name }), () => { setEditor(null); resource.reload() })
    }
  }
  return <>
    <PageHeader eyebrow="基础设施" title="服务器" description="连接你的服务器，集中查看运行状态与配置部署。"><Refresh onClick={resource.reload} /><button className="button button-primary" onClick={() => edit('new')}><Icon name="plus" size={18} />添加服务器</button></PageHeader>
    <div className="stats-grid"><Stat icon="server" label="服务器总数" value={resource.data ? servers.length : '—'} note="已添加到面板的服务器" /><Stat icon="activity" label="当前在线" value={resource.data ? servers.filter(server => server.online).length : '—'} note="最近 60 秒内收到设备消息" /><Stat icon="check" label="已发布配置" value={resource.data ? servers.filter(server => server.manifest_rev > 0).length : '—'} note="部署结果可在服务器详情查看" /></div>
    <ErrorNotice message={resource.error} retry={resource.reload} />
    <section className="panel"><div className="panel-heading"><h2>全部服务器 <span className="count">{servers.length}</span></h2><span className="subtle live-label"><span />每 5 秒刷新</span></div>
      {resource.loading && !resource.data ? <Loading /> : !servers.length ? <Empty icon="server" title="从第一台服务器开始" description="添加服务器后，在服务器上执行安装命令，即可自动接入。"><button className="button button-primary" onClick={() => edit('new')}><Icon name="plus" size={17} />添加服务器</button></Empty> : <div className="table-wrap"><table><thead><tr><th>服务器</th><th>状态</th><th>处理器</th><th>内存</th><th>设备版本</th><th className="align-right">操作</th></tr></thead><tbody>{servers.map(server => {
        const cpu = server.latest_metrics.cpu_percent
        const used = server.latest_metrics.memory_used, total = server.static_info.memory_total
        return <tr key={server.id}><td><button className="entity-link" onClick={() => navigate(`/servers/${server.id}`)}><span className="entity-icon"><Icon name="server" size={18} /></span><span><strong>{server.name}</strong><small>{server.static_info.hostname ?? `服务器 #${server.id}`}</small></span></button></td><td><Badge tone={server.online ? 'good' : 'neutral'}>{server.online ? '在线' : server.device_public_key ? '离线' : '待接入'}</Badge></td><td><div className="metric-cell"><span>{percent(cpu)}</span>{cpu !== undefined && <Meter value={cpu} />}</div></td><td><div className="metric-cell"><span>{bytes(used)}{used !== undefined && total !== undefined ? <small> / {bytes(total)}</small> : null}</span>{used !== undefined && total ? <Meter value={used / total * 100} /> : null}</div></td><td><span className="mono">{server.static_info.agent_version ?? '尚未上报'}</span></td><td><div className="row-actions"><button className="text-button" onClick={() => navigate(`/servers/${server.id}`)}>详情</button><button className="text-button" onClick={() => edit(server)}>编辑</button><button className="text-button danger-text" onClick={() => { action.clearError(); setDeleting(server) }}>删除</button></div></td></tr>
      })}</tbody></table></div>}
    </section>
    <div className="page-footnote"><Icon name="lock" size={14} />设备主动连接面板；面板离线时，已应用的节点配置继续运行。</div>
    {editor && <FormDialog title={editor === 'new' ? '添加服务器' : '编辑服务器'} onClose={() => setEditor(null)} onSubmit={submit} busy={action.busy} error={action.error} submitLabel={editor === 'new' ? '创建并获取安装命令' : '保存修改'}><Field label="服务器名称" hint="用位置或用途命名，方便以后辨认。"><input name="name" required maxLength={128} defaultValue={editor === 'new' ? '' : editor.name} placeholder="例如：香港 · 主节点" autoComplete="off" /></Field></FormDialog>}
    {deleting && <Confirm title={`删除「${deleting.name}」？`} busy={action.busy} error={action.error} onClose={() => setDeleting(null)} onConfirm={() => void action.run(() => api(`/api/servers/${deleting.id}`, 'DELETE'), () => { setDeleting(null); resource.reload() })}>此服务器将从面板和订阅中移除，历史流量仍会保留。设备在线时会先停止代理与诊断服务、清除本机凭证，确认后再删除；过程可能需要片刻。设备离线时仅移除面板记录，本地服务仍需手动停用。</Confirm>}
    {installation && <Modal title="接入服务器" onClose={() => setInstallation(null)} busy={action.busy} wide><div className="modal-body"><div className="step-label"><span>1</span><div><strong>{installation.server.name} 已添加</strong><p>在这台 Linux 服务器上，以 root 身份执行以下命令。</p></div></div><Field label="Agent 版本" hint="留空选择最新兼容版本，或指定已导入的签名版本。"><input value={agentVersion} onChange={event => setAgentVersion(event.target.value)} maxLength={96} placeholder="例如：0.3.0" disabled={action.busy} /></Field><button className="button button-secondary" disabled={action.busy} onClick={() => enroll(installation.server)}>生成所选版本的命令</button><ErrorNotice message={installation.warning || action.error} />{installation.enrollment ? <>{installation.enrollment.install_command ? <><p className="helper">目标 Agent 版本：{installation.enrollment.installation?.version}</p><CopyField text={installation.enrollment.install_command} label="复制安装命令" /></> : <ErrorNotice message={installation.enrollment.warning ?? "请先导入已签名的 Agent 制品"} />}<p className="helper">先按部署文档独立核对发布公钥并准备可信 sinan-bootstrap，再执行接入命令。</p><p className="helper">一次性令牌有效至 {time(installation.enrollment.expires_at)}。请勿公开分享安装命令。</p><div className="step-label"><span>2</span><div><strong>等待设备上线</strong><p>执行完成后，通常在 30 秒内可看到系统信息和实时指标。</p></div></div></> : action.busy ? <Loading /> : <button className="button button-secondary" onClick={() => enroll(installation.server)}>重新生成安装命令</button>}</div><footer><button className="button button-primary" disabled={action.busy} onClick={() => navigate(`/servers/${installation.server.id}`)}>查看服务器<Icon name="arrow" size={16} /></button></footer></Modal>}
  </>
}
