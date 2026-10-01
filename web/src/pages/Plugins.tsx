import { PageHeader } from '../components'
import { PluginSettings } from '../plugins'
import ServerNavigation from './ServerNavigation'

export default function Plugins({ serverId }: { serverId?: number }) {
  return <><PageHeader eyebrow={serverId ? `服务器 #${serverId}` : '服务器扩展'} title="服务器插件" description="按服务器管理插件的启用状态，不在面板本机安装插件。设备能力和已有配置会作为兼容证据保留。"><a className="button button-secondary" href="#/plugins/catalog">浏览插件目录</a></PageHeader>{serverId && <ServerNavigation id={serverId} active="plugins" />}<PluginSettings key={serverId ?? 'all'} serverId={serverId} /></>
}
