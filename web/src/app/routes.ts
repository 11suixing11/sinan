import { dashboardRoute } from '../display/dashboard'
import { isCatalogPath } from '../plugins/catalog'
import { nodeRoute } from '../plugins/singbox/nodeRoute'
import { resourceRoute } from '../plugins/singbox/resourceTypes'
import type { ResourceKey } from '../plugins/singbox/resourceTypes'

type ServerSection = 'ip-info' | 'node-quality' | 'tcp-quality' | 'plugins' | 'ddns'
type SimplePage = 'servers' | 'statistics' | 'latency' | 'alicloud' | 'ddns'
  | 'proxy-users' | 'groups' | 'plugins' | 'catalog' | 'settings' | 'notifications' | 'security' | 'not-found'

export type AppRoute =
  | { page: 'dashboard'; serverId?: number }
  | { page: 'proxy-portal'; account: string; activation?: string }
  | { page: 'server'; serverId: number; section?: ServerSection }
  | { page: 'nodes'; serverId?: number; chains?: boolean; selected?: ResourceKey }
  | { page: SimplePage }

const pages: Readonly<Record<string, SimplePage>> = {
  '/servers': 'servers',
  '/statistics': 'statistics',
  '/latency': 'latency',
  '/plugins/alicloud': 'alicloud',
  '/plugins/ddns': 'ddns',
  '/plugins/sing-box/users': 'proxy-users',
  '/plugins/sing-box/groups': 'groups',
  '/system/plugins': 'plugins',
  '/system/settings': 'settings',
  '/system/notifications': 'notifications',
  '/system/administrator': 'security',
}

export function resolveRoute(path: string): AppRoute {
  const portal = path.match(/^\/plugins\/sing-box\/account\/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:\?activate=([A-Za-z0-9_-]{43}))?$/)
  if (portal) return { page: 'proxy-portal', account: portal[1], activation: portal[2] }
  const display = dashboardRoute(path)
  if (display) return { page: 'dashboard', ...display }

  const server = path.match(/^\/servers\/([1-9]\d*)(?:\/(ip-info|node-quality|tcp-quality|plugins|ddns))?$/)
  if (server && Number.isSafeInteger(Number(server[1]))) {
    return { page: 'server', serverId: Number(server[1]), section: server[2] as ServerSection | undefined }
  }

  const node = nodeRoute(path)
  if (node) return { page: 'nodes', ...node }
  const resource = resourceRoute(path)
  if (resource) return { page: 'nodes', selected: resource }
  if (isCatalogPath(path)) return { page: 'catalog' }

  return { page: Object.hasOwn(pages, path) ? pages[path] : 'not-found' }
}
