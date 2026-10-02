import { Badge, Icon } from '../../components'
import { bytes, totalBytes } from '../../format'
import type { Node, Usage } from '../../types'
import type { ProxyResource } from './resourceTypes'
import { endpoint, resourceLink, roleNames, stageName } from './resourceTypes'
import { protocolNames } from './ProtocolFields'

export default function ProxyResourceTable({ resources, nodes, usage, onEdit, onDelete, onDeployment }: { resources: ProxyResource[]; nodes: Node[]; usage?: Usage; onEdit: (node: Node) => void; onDelete: (resource: ProxyResource) => void; onDeployment: (id: number) => void }) {
  return <div className="table-wrap"><table><thead><tr><th>节点 / 链路</th><th>服务器</th><th>公开地址</th><th>角色与状态</th><th>累计流量</th><th className="align-right">操作</th></tr></thead><tbody>{resources.map(resource => {
    const nodeId = resource.entry_node_id ?? resource.id
    const node = nodes.find(node => node.id === nodeId)
    const record = usage?.by_node.find(record => record.node_id === nodeId)
    return <tr key={`${resource.kind}-${resource.id}`}><td><div className="entity"><span className="entity-icon"><Icon name="nodes" size={18} /></span><div><a className="text-button" href={resourceLink(resource)}><strong>{resource.name}</strong></a><small>{resource.kind === 'chain' ? '链路' : '直连'} · {protocolNames[resource.protocol] ?? resource.protocol}</small>{!resource.enabled && <Badge tone="warm">已设为停用</Badge>}</div></div></td><td><a className="text-button" href={`#/servers/${resource.server_id}`}>{resource.server_name}</a></td><td><code>{endpoint(resource.public_host, node?.settings?.public_port ?? resource.port)}</code><small className="node-listen">监听 {node?.settings?.listen ?? '::'} / {resource.port}</small><small>{resource.udp ? 'TCP / UDP' : '仅 TCP'}</small></td><td><span>{roleNames[resource.role]}</span><small>{stageName(resource.stage)}{!resource.available && ' · 资源不可用'}</small>{resource.last_error && <small className="danger-text">{resource.last_error}</small>}</td><td>{record ? bytes(totalBytes(record.uplink, record.downlink)) : usage ? '0 B' : '暂无数据'}</td><td><div className="row-actions"><a className="text-button" href={resourceLink(resource)}>详情</a><button className="text-button" onClick={() => onDeployment(resource.server_id)}>部署</button>{resource.kind === 'direct' && <button className="text-button" disabled={!node} onClick={() => node && onEdit(node)}>编辑</button>}<button className="text-button danger-text" onClick={() => onDelete(resource)}>删除</button></div></td></tr>
  })}</tbody></table></div>
}
