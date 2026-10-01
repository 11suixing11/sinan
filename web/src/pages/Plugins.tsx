import { PageHeader } from '../components'
import { PluginSettings } from '../plugins'

export default function Plugins() {
  return <><PageHeader eyebrow="系统" title="插件设置" description="为服务器明确启用业务插件。设备能力和已有配置会作为兼容证据保留。" /><PluginSettings /></>
}
