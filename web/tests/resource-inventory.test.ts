import { expect, test } from 'bun:test'
import { mergeResourceInventory, validFlatInventory } from '../src/plugins/singbox/resourceInventory'
import { flatResourceInScope } from '../src/plugins/singbox/resourceTypes'
import type { ProxyResource as FlatResource } from '../src/plugins/singbox/resourceTypes'
import type { ProxyResource as OrderedResource } from '../src/plugins/singbox/groupTypes'
import { flatResourceFixtures, orderedResourceFixture, pathFixtureUuid, proxyResourceFixtures } from './proxy-resource-fixtures.mjs'

const nodes = [1, 2, 3].map(id => ({ id, name: `节点 ${id}`, server_id: id, protocol: 'vless-reality', enabled: true, port: 20000 + id, public_host: `node${id}.example.com`, sni: 'www.example.com' }))
const servers = [1, 2, 3].map(id => ({ id, name: `服务器 ${id}`, enabled: true, online: false, read_only: false }))

test('combined selectors deduplicate legacy resources and preserve numeric mixed and UUID ordered contracts', () => {
  const legacy = [{ id: 1, name: '旧两跳', entry_node_id: 1, exit_node_id: 2 }]
  const flat = flatResourceFixtures(nodes, servers, legacy) as FlatResource[]
  flat.push({ ...flat.find(resource => resource.kind === 'chain')!, id: 7, name: '数字版本混合链路', legacy: false, active_generation: 2 })
  const rich = proxyResourceFixtures(nodes, servers, legacy) as OrderedResource[]
  const exit = rich.find(resource => resource.kind === 'direct' && resource.id === 3)!.entry
  rich.push(orderedResourceFixture({ id: 9, name: 'UUID 有序链路', entry: rich[0].entry, hops: [{ kind: 'managed', position: 1, node_id: 3, endpoint_version_id: pathFixtureUuid(3), endpoint: exit }] }) as OrderedResource)
  const inventory = mergeResourceInventory(flat, rich)
  expect(inventory.filter(resource => resource.kind === 'chain').map(resource => [resource.id, resource.source])).toEqual([[1, 'flat'], [7, 'flat'], [9, 'ordered']])
  expect(validFlatInventory(flat)).toBe(true)
  expect(validFlatInventory([...flat, flat[0]])).toBe(false)
  expect(mergeResourceInventory([{ ...flat[0], role: 'chain_entry' }], []).every(resource => !resource.available)).toBe(true)
})

test('flat mixed scope uses frozen managed role arrays and unknown exits never borrow the entry server', () => {
  const resource = { ...flatResourceFixtures(nodes, servers)[0], kind: 'chain', role: 'chain_entry', managed_server_ids: [1, 2, 3], managed_middle_server_ids: [2], managed_exit_server_ids: [3] } as FlatResource
  expect(flatResourceInScope(resource, 2)).toBe(true)
  expect(flatResourceInScope(resource, 2, 'middle')).toBe(true)
  expect(flatResourceInScope(resource, 2, 'entry')).toBe(false)
  expect(flatResourceInScope(resource, 3, 'exit')).toBe(true)
  expect(flatResourceInScope({ ...resource, managed_exit_server_ids: [] }, 1, 'exit')).toBe(false)
  expect(flatResourceInScope({ ...resource, managed_exit_server_ids: undefined }, 1, 'exit')).toBe(false)
  expect(flatResourceInScope({ ...resource, managed_middle_server_ids: [0] }, 2, 'middle')).toBe(false)
})
