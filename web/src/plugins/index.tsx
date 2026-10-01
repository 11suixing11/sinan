import type { Server } from '../types'
import ServerBusiness from './singbox/ServerBusiness'
import SingboxSettings from './singbox/Settings'

export function ServerPlugins({ server }: { server: Server }) { return <ServerBusiness server={server} /> }
export function PluginSettings({ serverId }: { serverId?: number }) { return <><SingboxSettings serverId={serverId} /><section className="panel"><div className="panel-heading"><h2>DDNS 动态域名解析</h2><a className="button button-secondary" href={serverId ? `#/servers/${serverId}/ddns` : '#/plugins/ddns'}>管理 DDNS 插件</a></div><div className="panel-body"><p className="helper">按服务器启用 Cloudflare 动态解析，复用 Agent 的 IP 上报，由面板插件同步 DNS。</p></div></section></> }
