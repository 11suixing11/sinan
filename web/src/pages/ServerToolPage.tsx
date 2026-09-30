import { ErrorNotice, Icon, Loading, PageHeader } from '../components'
import { useResource } from '../hooks'
import type { Server } from '../types'
import NodeQuality from './NodeQuality'
import ServerIpInfo from './ServerIpInfo'
import ServerNavigation from './ServerNavigation'

export default function ServerToolPage({ id, section }: { id: number; section: 'ip-info' | 'node-quality' }) {
  const server = useResource<Server>(`/api/servers/${id}`)
  const entry = server.data
  return <>
    <a className="back-link" href="#/servers"><Icon name="back" size={16} />返回服务器</a>
    <ErrorNotice message={server.error} retry={server.reload} />
    {!entry ? server.loading && <Loading /> : <>
      <PageHeader eyebrow={`服务器 #${entry.id}`} title={entry.name} description={section === 'ip-info' ? '查看服务器 IP 查询结果、逐源状态与历史数据。' : '运行节点诊断并查看已保存的验机报告。'} />
      <ServerNavigation id={id} active={section} />
      {section === 'ip-info' ? <ServerIpInfo serverId={id} /> : <NodeQuality serverId={id} />}
    </>}
  </>
}
