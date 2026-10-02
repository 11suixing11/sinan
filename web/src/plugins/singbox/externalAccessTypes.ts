export type ExternalReference = { external_node_id: number; source_id: number; identity_epoch: number; node_version_id: number; update_mode: 'follow_node' | 'pinned'; metadata_revision: number }
export type ExternalEntry = ExternalReference & { name: string; source_name: string; protocol: string; server: string; port: number; available: boolean; reason: string | null; current_version_id: number | null; resolved_version_id: number | null; source_last_error: string | null }
export type ExternalAccessView = { revision: number; accesses: ExternalEntry[]; available_nodes: ExternalEntry[] }
export type ExternalDraft = { userId: number; revision: number; accesses: ExternalReference[] }
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value)
const integer = (value: unknown, minimum = 0): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= minimum
const nullableId = (value: unknown) => value === null || integer(value, 1)
const nullableText = (value: unknown) => value === null || typeof value === 'string'
export function validExternalReference(value: unknown): value is ExternalReference {
  return object(value) && ['external_node_id', 'source_id', 'identity_epoch', 'node_version_id'].every(key => integer(value[key], 1)) && integer(value.metadata_revision) && ['follow_node', 'pinned'].includes(String(value.update_mode))
}
function validEntry(value: unknown): value is ExternalEntry {
  return validExternalReference(value) && ['name', 'source_name', 'protocol', 'server'].every(key => typeof (value as unknown as Record<string, unknown>)[key] === 'string') && integer((value as ExternalEntry).port) && (value as ExternalEntry).port <= 65535 && typeof (value as ExternalEntry).available === 'boolean' && nullableText((value as ExternalEntry).reason) && nullableText((value as ExternalEntry).source_last_error) && nullableId((value as ExternalEntry).current_version_id) && nullableId((value as ExternalEntry).resolved_version_id) && (!(value as ExternalEntry).available || (value as ExternalEntry).protocol.length > 0 && (value as ExternalEntry).server.length > 0 && (value as ExternalEntry).port > 0 && (value as ExternalEntry).reason === null && (value as ExternalEntry).resolved_version_id !== null)
}
export function validExternalAccessView(value: unknown): value is ExternalAccessView {
  return object(value) && integer(value.revision) && ['accesses', 'available_nodes'].every(key => Array.isArray(value[key]) && (value[key] as unknown[]).every(validEntry) && new Set((value[key] as ExternalEntry[]).map(entry => entry.external_node_id)).size === (value[key] as unknown[]).length)
}
export const externalReference = (entry: ExternalReference): ExternalReference => ({ external_node_id: entry.external_node_id, source_id: entry.source_id, identity_epoch: entry.identity_epoch, node_version_id: entry.node_version_id, update_mode: entry.update_mode, metadata_revision: entry.metadata_revision })
export function sameExternalReference(left: ExternalReference, right: ExternalReference) {
  return left.external_node_id === right.external_node_id && left.source_id === right.source_id && left.identity_epoch === right.identity_epoch && left.node_version_id === right.node_version_id && left.update_mode === right.update_mode && left.metadata_revision === right.metadata_revision
}
export function externalDraftError(draft: ExternalDraft, current: ExternalAccessView | undefined, userId: number): string {
  if (!current || !validExternalAccessView(current) || !integer(userId, 1) || draft.userId !== userId || !integer(draft.revision) || draft.revision !== current.revision) return '授权或代理用户已改变，请重新读取后再保存；当前草稿已保留。'
  if (draft.accesses.length > 5000 || new Set(draft.accesses.map(entry => entry.external_node_id)).size !== draft.accesses.length || !draft.accesses.every(validExternalReference)) return '所选外部节点身份不完整；当前草稿已保留。'
  for (const value of draft.accesses) {
    // Unchanged historical bindings remain removable, even while unavailable.
    if (current.accesses.some(entry => sameExternalReference(entry, value))) continue
    const entry = current.available_nodes.find(entry => entry.external_node_id === value.external_node_id)
    if (!entry || !entry.available || entry.reason !== null || entry.source_id !== value.source_id || entry.identity_epoch !== value.identity_epoch || entry.node_version_id !== value.node_version_id || entry.current_version_id !== value.node_version_id || entry.resolved_version_id !== value.node_version_id || entry.metadata_revision !== value.metadata_revision) return '所选外部节点已缺失、停用或来源与版本已变化；请明确重新选择，当前草稿已保留。'
  }
  return ''
}

export function eligibleExternalSelection(entry: ExternalEntry) {
  return entry.available && entry.reason === null && entry.current_version_id === entry.node_version_id && entry.resolved_version_id === entry.node_version_id
}
