import { describe, expect, test } from 'bun:test'
import { catalogKey, filterCatalog, parsedTags, renamed, tagsError } from '../src/plugins/singbox/catalog'
import type { CatalogNode, CatalogFilter } from '../src/plugins/singbox/catalog'

const node = (fields: Partial<CatalogNode>): CatalogNode => ({ id: 1, kind: 'direct', name: '香港入口', original_name: '香港入口', public_host: 'node.example.com', protocol: 'vless-reality', enabled: true, available: true, server_id: 1, tags: [], note: '', ...fields }) as CatalogNode
const empty: CatalogFilter = { search: '', kind: '', protocol: '', tag: '', status: '', source: '', server: '', role: '' }
const nodes = [node({}), node({ kind: 'external', name: '东京', original_name: '提供方旧名', source_id: 3, server_id: null, tags: ['常用'], note: '待续费', protocol: 'tuic' }), node({ id: 2, enabled: false, available: false }), node({ id: 3, available: false })]
describe('node catalog', () => {
  test('resource identities never collide across kinds', () => expect(new Set(nodes.map(catalogKey)).size).toBe(4))
  test('search retains origin, notes and tags after local rename', () => {
    for (const search of ['提供方旧名', '待续费', '常用']) expect(filterCatalog(nodes, { ...empty, search }).map(catalogKey)).toEqual(['external:1'])
  })
  test('combines source/protocol/tag and server filters without inventing external servers', () => {
    expect(filterCatalog(nodes, { ...empty, protocol: 'tuic', tag: '常用', source: '3' }).length).toBe(1)
    expect(filterCatalog(nodes, { ...empty, server: '1', kind: 'external' })).toEqual([])
  })
  test('disabled and unavailable remain separate', () => {
    expect(filterCatalog(nodes, { ...empty, status: 'disabled' }).map(catalogKey)).toEqual(['direct:2'])
    expect(filterCatalog(nodes, { ...empty, status: 'unavailable' }).map(catalogKey)).toEqual(['direct:3'])
  })
  test('rename replaces literal strings, not regular expressions or replacement templates', () => {
    expect(renamed('a.*a.*', 'replace', '.*', '$&')).toBe('a$&a$&')
    expect(renamed('node', 'prefix', 'HK ')).toBe('HK node')
    expect(renamed('node', 'suffix', ' 02')).toBe('node 02')
    expect(renamed('node', 'replace', '', 'x')).toBe('node')
  })
  test('tag normalization deduplicates and enforces UTF-8 byte limits', () => {
    expect(parsedTags(' 香港, 香港，常用\n 测试 ')).toEqual(['香港', '常用', '测试'])
    expect(tagsError(['中'.repeat(21)])).toBe('')
    expect(tagsError(['中'.repeat(22)])).not.toBe('')
    expect(tagsError(Array.from({ length: 17 }, (_, index) => String(index)))).not.toBe('')
  })
})
