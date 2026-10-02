import { describe, expect, test } from 'bun:test'
import { knownSourceAmount, sourceReportedTotal, sourcePreviewError, validSourcePreview, sourceAdoptionError } from '../src/plugins/singbox/sourceTypes'
import type { SourcePreview, ExternalNodePreview } from '../src/plugins/singbox/sourceTypes'
const preview: SourcePreview = { id: '00000000-0000-4000-8000-000000000001', expires_at: 600, format: 'uri', supported_count: 1, unsupported_count: 1, nodes: [{ key: 'node-0', index: 0, name: '节点', protocol: 'tuic', server: 'provider.example.com', port: 443, transport: 'quic', tcp: true, udp: true, supported: true, reason: null }, { key: 'rejected-0', index: 1, name: '不支持', protocol: null, server: null, port: null, transport: null, tcp: false, udp: false, supported: false, reason: 'unsupported_proxy_protocol' }] }
describe('source preview and provider unknown values', () => {
  test('commit requires a live explicit preview and supported unique selected keys', () => {
    expect(validSourcePreview(preview)).toBe(true)
    expect(sourcePreviewError(preview, ['node-0'], 599999)).toBe('')
    expect(sourcePreviewError(preview, ['node-0'], 600000)).not.toBe('')
    for (const selection of [[], ['missing'], ['rejected-0'], ['node-0', 'node-0']]) expect(sourcePreviewError(preview, selection, 0)).not.toBe('')
  })
  test('unknown preview identity/count/expiry is not permission to commit', () => {
    for (const change of [{ id: '' }, { id: 'preview-1' }, { supported_count: 2 }, { expires_at: Number.MAX_SAFE_INTEGER + 1 }, { expires_at: Number.MAX_SAFE_INTEGER }, { expires_at: 0 }]) expect(sourcePreviewError({ ...preview, ...change }, ['node-0'], 0)).not.toBe('')
    expect(validSourcePreview({ ...preview, nodes: [preview.nodes[0], preview.nodes[0]] })).toBe(false)
  })
  test('provider zero is preserved; missing/negative/unsafe values remain unknown', () => {
    expect(knownSourceAmount(0)).toBe(0)
    expect(sourceReportedTotal(0, 0)).toBe(0)
    for (const value of [undefined, null, -1, NaN, Number.MAX_SAFE_INTEGER + 1]) expect(knownSourceAmount(value)).toBeUndefined()
    expect(sourceReportedTotal(Number.MAX_SAFE_INTEGER, 1)).toBeUndefined()
    expect(sourceReportedTotal(undefined, 0)).toBeUndefined()
  })
})

describe('external adoption metadata CAS', () => {
  const node: ExternalNodePreview = { id: 1, source_id: 2, node_version_id: 3, source_revision_id: 4, identity_epoch: 1, metadata_revision: 0, name: '节点', protocol: 'tuic', server: 'provider.example.com', port: 443, transport: 'quic', tcp: true, udp: true, selectable: true, present: true, identity_unique: true, adopted: false, reason: null }
  test('explicit revision zero supports first adoption while unknown revision does not', () => {
    expect(sourceAdoptionError(node, node)).toBe('')
    for (const revision of [undefined, -1, Number.MAX_SAFE_INTEGER + 1]) expect(sourceAdoptionError({ ...node, metadata_revision: revision }, { ...node, metadata_revision: revision })).not.toBe('')
  })
  test('same-version deletion, replaced source and changed adoption cannot replay old intent', () => {
    for (const change of [{ metadata_revision: 1 }, { source_id: undefined }, { identity_epoch: 2 }, { source_id: 3 }, { node_version_id: 5 }, { adopted: true }, { selectable: false }]) expect(sourceAdoptionError(node, { ...node, ...change } as ExternalNodePreview)).not.toBe('')
    expect(sourceAdoptionError(node, undefined)).not.toBe('')
    expect(sourceAdoptionError({ ...node, metadata_revision: 1 }, { ...node, metadata_revision: 1 })).toBe('')
  })
})
