import { describe, expect, test } from 'bun:test'
import { externalDraftError, externalReference, validExternalAccessView } from '../src/plugins/singbox/externalAccessTypes'
import type { ExternalEntry, ExternalAccessView, ExternalDraft } from '../src/plugins/singbox/externalAccessTypes'
const node: ExternalEntry = { external_node_id: 1, source_id: 2, identity_epoch: 1, node_version_id: 3, update_mode: 'follow_node', metadata_revision: 0, name: '原节点', source_name: '原来源', protocol: 'tuic', server: 'provider.example.com', port: 443, available: true, reason: null, current_version_id: 3, resolved_version_id: 3, source_last_error: null }
const current: ExternalAccessView = { revision: 0, accesses: [], available_nodes: [node] }
const draft: ExternalDraft = { userId: 1, revision: 0, accesses: [externalReference(node)] }
describe('external authorization current identity', () => {
  test('first authorization uses explicit revision0 and metadata0', () => { expect(validExternalAccessView(current)).toBe(true); expect(externalDraftError(draft, current, 1)).toBe('') })
  test('unknown/negative/unsafe revision cannot match itself into eligibility', () => {
    for (const revision of [undefined, -1, Number.MAX_SAFE_INTEGER + 1]) expect(externalDraftError({ ...draft, revision } as ExternalDraft, { ...current, revision } as ExternalAccessView, 1)).not.toBe('')
  })
  test('changed target/revision and absent current snapshot never redirect the draft', () => {
    expect(externalDraftError(draft, current, 2)).not.toBe('')
    expect(externalDraftError(draft, { ...current, revision: 1 }, 1)).not.toBe('')
    expect(externalDraftError(draft, undefined, 1)).not.toBe('')
  })
  test('new selection requires current source/epoch/version/metadata and availability', () => {
    for (const change of [{ source_id: 4 }, { identity_epoch: 2 }, { node_version_id: 4, current_version_id: 4, resolved_version_id: 4 }, { metadata_revision: 1 }, { available: false, reason: 'node_disabled' }, { available: false, reason: 'source_archived' }]) expect(externalDraftError(draft, { ...current, available_nodes: [{ ...node, ...change }] }, 1)).not.toBe('')
    expect(externalDraftError(draft, { ...current, available_nodes: [] }, 1)).not.toBe('')
  })
  test('an unchanged unavailable historical binding may remain or be explicitly removed', () => {
    const inactive = { ...node, available: false, reason: 'version_missing', port: 0, protocol: '', server: '', current_version_id: null, resolved_version_id: null }
    const history = { revision: 0, accesses: [inactive], available_nodes: [] }
    expect(validExternalAccessView(history)).toBe(true)
    expect(externalDraftError(draft, history, 1)).toBe('')
    expect(externalDraftError({ ...draft, accesses: [] }, history, 1)).toBe('')
    expect(externalDraftError({ ...draft, accesses: [{ ...draft.accesses[0], update_mode: 'pinned' }] }, history, 1)).not.toBe('')
  })
  test('a claimed available row requires a real endpoint while inactive zero remains readable', () => {
    for (const change of [{ port: 0 }, { port: -1 }, { port: 65536 }, { protocol: '' }, { server: '' }]) expect(validExternalAccessView({ ...current, available_nodes: [{ ...node, ...change }] })).toBe(false)
  })
})
