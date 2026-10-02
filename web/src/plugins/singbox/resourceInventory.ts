import type { ProxyResource as FlatResource } from './resourceTypes'
import type { ProxyResource as OrderedResource } from './groupTypes'
import { publicPathText } from './ChainLifecycle'

export type InventoryResource = { kind: 'direct' | 'chain'; id: number; name: string; available: boolean; description: string; source: 'flat' | 'ordered' }
// Only common public selection fields are combined. Numeric mixed-path versions
// remain in the flat model; UUID lifecycle generations remain in the ordered model.
export function mergeResourceInventory(flat: FlatResource[], ordered: OrderedResource[]): InventoryResource[] {
  const values: InventoryResource[] = flat.map(resource => ({ kind: resource.kind, id: resource.id, name: resource.name,
    available: resource.available === true && (resource.kind === 'chain' || resource.role !== 'chain_entry'), source: 'flat',
    description: `${resource.server_name} · ${resource.public_host}:${resource.port}` }))
  for (const resource of ordered) if (resource.kind === 'chain' && resource.path_kind === 'ordered' && !values.some(value => value.kind === resource.kind && value.id === resource.id)) values.push({ kind: resource.kind, id: resource.id, name: resource.name, available: resource.available, source: 'ordered', description: `${resource.entry.name} → ${publicPathText(resource.hops)}` })
  return values
}
export function validFlatInventory(value: unknown): value is FlatResource[] {
  return Array.isArray(value) && value.every(resource => resource && typeof resource === 'object'
    && ['direct', 'chain'].includes(resource.kind) && Number.isSafeInteger(resource.id) && resource.id > 0
    && typeof resource.name === 'string' && typeof resource.available === 'boolean'
    && Number.isSafeInteger(resource.server_id) && resource.server_id > 0 && typeof resource.server_name === 'string'
    && typeof resource.public_host === 'string' && Number.isSafeInteger(resource.port) && resource.port > 0 && resource.port <= 65535
    && ['direct', 'managed_hop', 'chain_entry'].includes(resource.role))
    && new Set(value.map(resource => `${resource.kind}:${resource.id}`)).size === value.length
}
