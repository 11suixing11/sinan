import { PageHeader } from '../components'
import { PluginSettings } from '../plugins'
import ServerNavigation from './ServerNavigation'

export default function Plugins({ serverId }: { serverId?: number }) {
  return <><PageHeader eyebrow={serverId ? `服务器 #${serverId}` : '服务器扩展'} title="服务器插件" description="选择服务器安装 sing-box，查看设备确认的安装与运行结果，再创建节点和分配代理用户。"><a className="button button-secondary" href="#/plugins/catalog">浏览插件目录</a></PageHeader>{serverId && <ServerNavigation id={serverId} active="plugins" />}<PluginSettings key={serverId ?? 'all'} serverId={serverId} /></>
}
