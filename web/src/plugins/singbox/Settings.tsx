import { api } from '../../api'
import { Badge, Empty, ErrorNotice, Loading, Refresh } from '../../components'
import { useAction, useResource } from '../../hooks'
import type { PluginServer } from '../../types'

export function sourceLabel(source: PluginServer['source'] | undefined) {
  return source === 'administrator' ? '管理员明确启用' : source === 'agent_capability' ? '设备声明 sing-box 能力' : source === 'legacy_nodes' ? '兼容已有代理节点' : source === 'legacy_deployments' ? '兼容已有代理部署' : '尚未启用'
}
export default function Settings() {
  const servers = useResource<PluginServer[]>('/api/plugins/sing-box/servers')
  const action = useAction()
  return <section className="panel"><div className="panel-heading"><h2>sing-box</h2><Refresh onClick={servers.reload} /></div><div className="panel-body"><p className="helper">代理节点、代理用户、授权、订阅和代理流量由此插件管理。服务器网卡流量保留在服务器页面。</p><ErrorNotice message={servers.error || action.error} retry={servers.reload} /></div>
    {servers.loading && !servers.data ? <Loading /> : !servers.data?.length ? <Empty icon="server" title="还没有服务器" description="先添加服务器，再选择需要启用的插件。" /> : <div className="table-wrap"><table><thead><tr><th>服务器</th><th>启用来源</th><th>设备能力</th><th>操作</th></tr></thead><tbody>{servers.data.map(server => <tr key={server.id}><td><a className="text-button" href={`#/servers/${server.id}`}>{server.name}</a></td><td><Badge tone={server.enabled ? 'good' : 'neutral'}>{sourceLabel(server.source)}</Badge>{server.read_only && <small>由设备声明或既有配置识别，来源只读</small>}</td><td>{server.agent_supported ? '已声明 sing-box' : '设备尚未声明能力'}</td><td>{server.enabled ? <span className="subtle">已启用</span> : <button className="button button-primary button-small" disabled={action.busy} onClick={() => void action.run(() => api(`/api/plugins/sing-box/servers/${server.id}/enable`, 'POST', {}), servers.reload)}>启用 sing-box</button>}</td></tr>)}</tbody></table></div>}
  </section>
}
