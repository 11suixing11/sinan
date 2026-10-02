import type { ProxyResourceFilter, ProxyResourceServerRole } from './groupTypes'

export function nodeRoute(path: string): { serverId?: number; serverRole?: ProxyResourceServerRole; chains: boolean; kind: ProxyResourceFilter; resource?: { kind: 'direct' | 'chain'; id: number } } | null {
  const detail = path.match(/^\/plugins\/sing-box\/nodes\/(direct|chain)\/([1-9]\d*)$/)
  if (detail) return Number.isSafeInteger(Number(detail[2])) ? { kind: 'all', chains: false, resource: { kind: detail[1] as 'direct' | 'chain', id: Number(detail[2]) } } : null
  const [pathname, query = '', ...extra] = path.split('?')
  if (pathname !== '/plugins/sing-box/nodes' || extra.length) return null
  const params = new URLSearchParams(query)
  if ([...params.keys()].some(key => !['server', 'kind', 'role'].includes(key)) || ['server', 'kind', 'role'].some(key => params.getAll(key).length > 1)) return null
  const server = params.get('server'), kind = params.get('kind')
  if (kind !== null && !['all', 'direct', 'chains'].includes(kind) || server !== null && (!/^[1-9]\d*$/.test(server) || !Number.isSafeInteger(Number(server)))) return null
  const role = params.get('role')
  if (role !== null && !['any', 'entry', 'middle', 'exit'].includes(role)) return null
  return { ...(server === null ? {} : { serverId: Number(server) }), ...(role === null ? {} : { serverRole: role as ProxyResourceServerRole }), chains: kind === 'chains', kind: (kind ?? 'all') as ProxyResourceFilter }
}
