import type { Server } from '../types'
import ServerBusiness from './singbox/ServerBusiness'
import SingboxSettings from './singbox/Settings'

export function ServerPlugins({ server }: { server: Server }) { return <ServerBusiness server={server} /> }
export function PluginSettings() { return <SingboxSettings /> }
