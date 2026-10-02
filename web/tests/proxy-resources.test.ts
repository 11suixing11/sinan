import { expect, test } from 'bun:test'
import { deleteProxyResource, prepareChainBatch, proxyWriteError, submitChainBatch, validNodeList, validServerList } from '../src/plugins/singbox/Chains'
import type { ChainBatchDraft, ProxyWriteSnapshot } from '../src/plugins/singbox/Chains'
import { filterProxyResources, proxyResourceCounts, proxyResourceKey, validatedSnapshot, validProxyResources } from '../src/plugins/singbox/groupTypes'
import type { ProxyResource, ResourceEndpoint } from '../src/plugins/singbox/groupTypes'
import { nodeRoute } from '../src/plugins/singbox/nodeRoute'
import { publicResourceFields } from './proxy-resource-fixtures.mjs'
const requestId = '00000000-0000-4000-8000-000000000001'
const nextId = '00000000-0000-4000-8000-000000000002'
function endpoint(id: number, server = id): ResourceEndpoint { return { id, name: `节点 ${id}`, server_id: server, server_name: `服务器 ${server}`, protocol: 'vless-reality', port: 443, public_port: 8443, public_host: 'proxy.example.com', sni: 'www.example.com', enabled: true, node_deleted: false, server_deleted: false, plugin_enabled: true, online: false, desired_revision: null, applied_revision: null, applied_observed_at: null } }
function resource(kind: 'direct' | 'chain', id: number, entry: ResourceEndpoint, exit: ResourceEndpoint | null = null): ProxyResource { return { kind, id, name: `资源 ${kind} ${id}`, entry, exit, available: true, unavailable_reasons: [], policy_group_ids: [], user_count: 0, chain_refs: [], ...publicResourceFields(kind, exit) } as ProxyResource }
function snapshot(): ProxyWriteSnapshot {
  const direct1 = resource('direct', 1, endpoint(1)), direct2 = resource('direct', 2, endpoint(2)), chain1 = resource('chain', 1, endpoint(3), endpoint(2))
  direct2.chain_refs = [{ id: 1, name: chain1.name, role: 'exit', generation: 1, hop_position: 1, state: 'applied' }]
  return { resources: { data: [direct1, direct2, chain1], fresh: true, error: '' },
    nodes: { data: [1, 2, 3].map(id => ({ ...endpoint(id), public_key: 'TEST_ONLY', short_id: 'abcd' })), fresh: true, error: '' },
    servers: { data: [1, 2, 3].map(id => ({ id, name: `服务器 ${id}`, enabled: true, online: false, read_only: false, agent_supported: true, source: 'administrator' })), fresh: true, error: '' } }
}
function draft(mode: 'new' | 'existing' = 'new'): ChainBatchDraft { return { mode, server_id: '1', public_host: 'entry.example.com', sni: 'www.example.com', entry_node_id: '1', exit_node_id: '2', rows: [{ name: ' 链路甲 ', port: '' }, ...(mode === 'new' ? [{ name: '链路乙', port: '24443' }] : [])] } }

test('unified resources distinguish colliding IDs, retain shared exits and filter either endpoint', () => {
  const resources = snapshot().resources.data!
  expect(validProxyResources(resources)).toBe(true)
  expect(resources.map(proxyResourceKey)).toEqual(['direct:1', 'direct:2', 'chain:1'])
  expect(proxyResourceCounts(resources)).toEqual({ total: 3, direct: 2, chains: 1, endpoints: 3 })
  expect(filterProxyResources(resources, 'chains', 2).map(proxyResourceKey)).toEqual(['chain:1'])
  expect(filterProxyResources(resources, 'direct', 3)).toEqual([])
  expect(filterProxyResources(resources, 'all', 2).map(proxyResourceKey)).toEqual(['direct:2', 'chain:1'])
})
test('new filter URLs and legacy chain bookmarks preserve strict query parsing', () => {
  expect(nodeRoute('/plugins/sing-box/nodes')).toEqual({ kind: 'all', chains: false })
  expect(nodeRoute('/plugins/sing-box/nodes?kind=chains&server=2')).toEqual({ kind: 'chains', chains: true, serverId: 2 })
  expect(nodeRoute('/plugins/sing-box/nodes?kind=direct')).toEqual({ kind: 'direct', chains: false })
  for (const query of ['kind=chain', 'server=01', 'server=1e0', 'kind=all&kind=direct', 'other=1']) expect(nodeRoute(`/plugins/sing-box/nodes?${query}`)).toBeNull()
})
test('invalid current metadata preserves a readable prior snapshot without authorizing writes', () => {
  const previous = snapshot().resources.data!
  for (const invalid of [[{ id: 1, name: '旧节点字段' }], [...previous, previous[0]], [{ ...previous[0], entry: { ...previous[0].entry, public_port: 0 } }]]) {
    const view = validatedSnapshot({ data: invalid, fresh: true, error: '' }, validProxyResources, previous)
    expect(view.data).toBe(previous); expect(view.fresh).toBe(false); expect(view.error).toContain('格式不完整')
    const current = snapshot(); current.resources = view
    expect(proxyWriteError(current)).toContain('读取失败')
  }
})
test('malformed auxiliary node fields and legacy server metadata never establish a write snapshot', () => {
  const current = snapshot()
  expect(validNodeList(current.nodes.data)).toBe(true)
  expect(validServerList(current.servers.data)).toBe(true)
  const node = current.nodes.data![0]
  for (const settings of [{listen:{}},{public_port:0},{tls_alpn:'h3'},{reality:{fingerprint:{}}},{hysteria2:{obfs_enabled:1}}]) expect(validNodeList([{...node,settings}])).toBe(false)
  expect(validNodeList([{...node,protocol_config:{type:'vless-reality',tls:{mode:{}}}}])).toBe(false)
  const {read_only: omitted,...legacy} = current.servers.data![0]
  expect(omitted).toBe(false)
  expect(validServerList([legacy])).toBe(false)
  expect(validServerList([{...current.servers.data![0],installation:{state:'ready',reason:'未知版本',target_rev:'1',applied_rev:1}}])).toBe(false)
})
test('one batch contains independent new entry ports and a shared managed exit without grants', async () => {
  const pending = prepareChainBatch(draft(), snapshot(), undefined, () => requestId), writes: unknown[] = []
  await submitChainBatch(pending, snapshot(), async body => { writes.push(body) })
  expect(writes).toEqual([{ request_id: requestId, items: [
    { name: '链路甲', entry: { mode: 'new', server_id: 1, public_host: 'entry.example.com', sni: 'www.example.com', port: null }, hops: [{ kind: 'managed', node_id: 2 }] },
    { name: '链路乙', entry: { mode: 'new', server_id: 1, public_host: 'entry.example.com', sni: 'www.example.com', port: 24443 }, hops: [{ kind: 'managed', node_id: 2 }] },
  ] }]); expect(pending.attempted).toBe(true)
})
test('batch limits, duplicate ports and unsafe names or hosts reject before writer invocation', () => {
  for (const change of ['empty', 'large', 'duplicate-port', 'bad-port', 'name', 'host', 'same-server', 'existing-many'] as const) {
    const value = draft()
    if (change === 'empty') value.rows = []
    if (change === 'large') value.rows = Array.from({ length: 33 }, (_, i) => ({ name: `链路 ${i}`, port: '' }))
    if (change === 'duplicate-port') value.rows.forEach(row => { row.port = '24443' })
    if (change === 'bad-port') value.rows[0].port = '18085'
    if (change === 'name') value.rows[0].name = '名'.repeat(129)
    if (change === 'host') value.public_host = 'https://entry.example.com/path'
    if (change === 'same-server') value.server_id = '2'
    if (change === 'existing-many') value.mode = 'existing'
    expect(() => prepareChainBatch(value, snapshot(), undefined, () => requestId)).toThrow()
  }
  const maximum = draft(); maximum.rows = Array.from({ length: 32 }, (_, i) => ({ name: `链路 ${i}`, port: '' }))
  expect(JSON.parse(prepareChainBatch(maximum, snapshot(), undefined, () => requestId).serialized).items).toHaveLength(32)
})
test('all creation dependencies fail closed for failed, pending and inconsistent metadata', async () => {
  for (const key of ['resources', 'nodes', 'servers'] as const) for (const state of ['failed', 'pending', 'missing'] as const) {
    const current = snapshot(), writes: unknown[] = [], pending = prepareChainBatch(draft(), current, undefined, () => requestId)
    if (state === 'missing') current[key].data = undefined
    else { current[key].fresh = false; current[key].error = state === 'failed' ? '读取失败' : '' }
    expect(() => prepareChainBatch(draft(), current, undefined, () => requestId)).toThrow()
    await expect(submitChainBatch(pending, current, async body => { writes.push(body) })).rejects.toThrow()
    expect(writes).toEqual([]); expect(pending.attempted).toBe(false)
  }
})
test('unchanged failed requests retain exact serialized body and changed drafts create a new UUID', async () => {
  const value = draft(), pending = prepareChainBatch(value, snapshot(), undefined, () => requestId), sent: string[] = []
  await expect(submitChainBatch(pending, snapshot(), async body => { sent.push(JSON.stringify(body)); throw new Error('lost response') })).rejects.toThrow('lost response')
  const retry = prepareChainBatch(value, snapshot(), pending, () => { throw new Error('must not allocate a new UUID') })
  expect(retry).toBe(pending)
  await submitChainBatch(retry, snapshot(), async body => { sent.push(JSON.stringify(body)) })
  expect(sent).toEqual([pending.serialized, pending.serialized])
  const changed = structuredClone(value); changed.rows[0].name = '新名称'
  expect(prepareChainBatch(changed, snapshot(), pending, () => nextId).request_id).toBe(nextId)
  expect(pending.serialized).toBe(sent[0])
})
test('lost-response existing-entry replay survives refreshed role, deletion or retirement changes', async () => {
  const value = draft('existing'), current = snapshot(), pending = prepareChainBatch(value, current, undefined, () => requestId), sent: string[] = []
  await expect(submitChainBatch(pending, current, async body => { sent.push(JSON.stringify(body)); throw new Error('response disappeared') })).rejects.toThrow()
  current.resources.data = current.resources.data!.filter(resource => proxyResourceKey(resource) !== 'direct:1')
  current.resources.data.push(resource('chain', 10, endpoint(1), endpoint(2)))
  current.nodes.data = current.nodes.data!.filter(node => node.id !== 1)
  current.servers.data![0].enabled = false
  const retry = prepareChainBatch(value, current, pending, () => { throw new Error('duplicate allocation') })
  await submitChainBatch(retry, current, async body => { sent.push(JSON.stringify(body)); return { request_id: requestId, chain_ids: [10], entry_node_ids: [1] } })
  expect(sent).toEqual([pending.serialized, pending.serialized])
  expect(() => prepareChainBatch({ ...value, rows: [{ name: '修改后', port: '' }] }, current, pending, () => nextId)).toThrow()
  current.resources.fresh = false
  await expect(submitChainBatch(retry, current, async () => { throw new Error('writer must not run') })).rejects.toThrow('正在刷新')
})
test('new existing-entry selection rejects users, policies and shared-exit roles while exits may be shared', () => {
  for (const change of ['users', 'policies', 'refs'] as const) {
    const current = snapshot(), entry = current.resources.data![0]
    if (change === 'users') entry.user_count = 1
    if (change === 'policies') entry.policy_group_ids = [9]
    if (change === 'refs') entry.chain_refs = [{ id: 9, name: '另一条链路', role: 'exit', generation: 1, hop_position: 1, state: 'applied' }]
    expect(() => prepareChainBatch(draft('existing'), current, undefined, () => requestId)).toThrow('已有授权或链路引用')
  }
  expect(JSON.parse(prepareChainBatch(draft('existing'), snapshot(), undefined, () => requestId).serialized).items[0].entry).toEqual({ mode: 'existing', node_id: 1 })
})
test('delete uses typed current resource identity and can clean broken configuration without old node reads', async () => {
  const current = snapshot(), writes: string[] = [], chain = current.resources.data![2]
  chain.available = false; chain.unavailable_reasons = ['入口公开端口参数无法确认，暂显示监听端口']
  current.nodes.fresh = false; current.nodes.error = '旧节点设置无法解析'; current.servers.fresh = false
  await deleteProxyResource(chain, current, async path => { writes.push(path) })
  expect(writes).toEqual(['/api/plugins/sing-box/proxy-resources/chain/1'])
  current.resources.fresh = false
  await expect(deleteProxyResource(chain, current, async path => { writes.push(path) })).rejects.toThrow('正在刷新')
  current.resources.fresh = true; current.resources.data = current.resources.data!.filter(resource => resource.kind !== 'chain')
  await expect(deleteProxyResource(chain, current, async path => { writes.push(path) })).rejects.toThrow('此资源已不可用')
  expect(writes).toHaveLength(1)
})
