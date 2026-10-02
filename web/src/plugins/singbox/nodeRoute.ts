import type { ProxyResourceFilter } from './groupTypes'

export function nodeRoute(path: string): { serverId?: number; chains: boolean; kind: ProxyResourceFilter } | null {
  const [pathname, query = '', ...extra] = path.split('?')
  if (pathname !== '/plugins/sing-box/nodes' || extra.length) return null
  const params = new URLSearchParams(query)
  if ([...params.keys()].some(key => !['server', 'kind'].includes(key)) || params.getAll('server').length > 1 || params.getAll('kind').length > 1) return null
  const server = params.get('server'), kind = params.get('kind')
  if (kind !== null && !['all', 'direct', 'chains'].includes(kind) || server !== null && (!/^[1-9]\d*$/.test(server) || !Number.isSafeInteger(Number(server)))) return null
  return { ...(server === null ? {} : { serverId: Number(server) }), chains: kind === 'chains', kind: (kind ?? 'all') as ProxyResourceFilter }
}
