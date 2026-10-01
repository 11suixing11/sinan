import { useState } from 'react'
import { api } from '../../api'
import { Badge, ErrorNotice, Loading, Modal, Refresh } from '../../components'
import { useAction, useResource } from '../../hooks'
import type { Deployment, PluginServer } from '../../types'
import RuntimeOperations from './RuntimeOperations'

type Progress = Deployment & { pending: boolean; enabled_nodes: number; authorized_nodes: number }
type Readiness = { ready: boolean; checks: { name: string; passed: boolean; detail: string }[] }

export default function NodeDeployment({ serverId, server, onClose }: { serverId: number; server?: PluginServer; onClose: () => void }) {
  const path = `/api/plugins/sing-box/servers/${serverId}/deployments`
  const resource = useResource<Progress>(path)
  const action = useAction()
  const [check, setCheck] = useState<Readiness | null>(null)
  const status = resource.data?.status
  const failed = status && status.last_result_rev === status.target_rev && !!status.last_error
  const applied = status && status.applied_rev === status.target_rev && status.healthy && !failed
  const title = resource.data?.pending ? '等待合并发布' : failed ? '最新配置应用失败' : applied ? '目标配置已应用' : status ? '等待设备应用' : '尚未发布配置'
  return <Modal title={`${server?.name ?? `服务器 #${serverId}`} · 节点部署`} onClose={onClose} wide busy={action.busy}>
    <div className="modal-body node-deployment">
      <ErrorNotice message={resource.error || action.error} retry={resource.reload} />
      {!resource.data && resource.loading ? <Loading /> : resource.data && <>
        <div className="node-deployment-heading"><Badge tone={failed ? 'bad' : applied && !resource.data.pending ? 'good' : 'warm'}>{title}</Badge><Refresh onClick={resource.reload} /></div>
        <dl className="node-deployment-facts"><div><dt>目标版本</dt><dd>{status?.target_rev ?? '—'}</dd></div><div><dt>已应用版本</dt><dd>{status?.applied_rev || '—'}</dd></div><div><dt>启用 / 有效授权节点</dt><dd>{resource.data.enabled_nodes} / {resource.data.authorized_nodes}</dd></div></dl>
        {!!status?.last_error && <div className="notice notice-error"><span>版本 {status.last_result_rev}：{status.last_error}</span></div>}
        <p>每台服务器统一发布完整配置。没有有效授权或已停用的节点不会监听；设备离线时需等待重连。新配置失败时可能仍运行上一次健康配置。</p>
        {resource.data.authorized_nodes === 0 && <p>当前没有有效授权节点。请先到<a href="#/plugins/sing-box/users" onClick={onClose}>代理用户</a>分配权限与套餐，再检查部署。</p>}
      </>}
      <div className="node-deployment-heading"><h3>部署条件检查</h3><button className="button button-secondary" disabled={action.busy} onClick={() => { setCheck(null); void action.run(() => api<Readiness>(`${path}/check`, 'POST'), setCheck) }}>{action.busy ? '正在检查…' : '检查部署条件'}</button></div>
      <p>检查设备接入、在线状态、插件能力和匹配的签名运行时。检查通过后仍以 Agent 的应用与健康回报为准。</p>
      {check && <ul className="node-checks">{check.checks.map(item => <li key={item.name}><Badge tone={item.passed ? 'good' : 'warm'}>{item.name}</Badge><span>{item.detail}</span></li>)}</ul>}
      <RuntimeOperations serverId={serverId} status={status} />
      <div className="node-deployment-links"><a href={`#/servers/${serverId}`} onClick={onClose}>服务器接入与状态</a><a href="#/plugins/catalog" onClick={onClose}>运行时制品</a></div>
    </div><footer><button className="button button-secondary" onClick={onClose} disabled={action.busy}>关闭</button></footer>
  </Modal>
}
