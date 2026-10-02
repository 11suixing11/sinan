import type { ProxyResource } from './resourceTypes'

export type CatalogNode = Omit<ProxyResource, 'kind' | 'role' | 'server_id' | 'server_name'> & {
  kind: 'direct' | 'chain' | 'external'; role: ProxyResource['role'] | 'external'
  server_id: number | null; server_name: string | null
  original_name: string; tags: string[]; note: string; sort_order: number; revision: string
  source_id?: number; source_name?: string; version_id?: number; identity_epoch?: number
}
export const catalogKey = (node: Pick<CatalogNode, 'kind' | 'id'>) => `${node.kind}:${node.id}`
export const catalogRef = (node: CatalogNode) => ({ kind: node.kind, id: node.id, revision: node.revision })
export type CatalogFilter = { search: string; kind: string; protocol: string; tag: string; status: string; source: string; server: string; role: string }
export function filterCatalog(nodes: CatalogNode[], filter: CatalogFilter): CatalogNode[] {
  const query = filter.search.trim().toLocaleLowerCase()
  return nodes.filter(node => (!filter.kind || node.kind === filter.kind)
    && (!filter.protocol || node.protocol === filter.protocol) && (!filter.tag || node.tags.includes(filter.tag))
    && (!filter.source || node.source_id === Number(filter.source)) && (!filter.server || node.server_id === Number(filter.server))
    && (!filter.role || node.role === filter.role)
    && (!filter.status || (filter.status === 'disabled' ? !node.enabled : filter.status === 'available' ? node.enabled && node.available : node.enabled && !node.available))
    && (!query || [node.name, node.original_name, node.public_host, node.note, ...node.tags].join(' ').toLocaleLowerCase().includes(query)))
}
export function renamed(name: string, mode: string, value: string, replacement = ''): string {
  return (mode === 'prefix' ? value + name : mode === 'suffix' ? name + value : value ? name.split(value).join(replacement) : name).trim()
}
export function parsedTags(value: string): string[] { return [...new Set(value.split(/[,，\n]/).map(tag => tag.trim()).filter(Boolean))] }
export function tagsError(tags: string[]): string { return tags.length > 16 || tags.some(tag => new TextEncoder().encode(tag).length > 64) ? '最多 16 个标签，每个标签不超过 64 字节。' : '' }
