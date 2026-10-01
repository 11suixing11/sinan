import { expect, test } from 'bun:test'
import { chainSelectionError, chainWriteError, createChain, deleteChain } from '../src/plugins/singbox/Chains'
import type { ChainWriteSnapshot } from '../src/plugins/singbox/Chains'
import type { Node, PluginServer } from '../src/types'

function snapshot(): ChainWriteSnapshot {
  const nodes: Node[] = [1, 2, 3].map(id => ({ id, server_id: id, name: `节点 ${id}`, enabled: true, protocol: 'vless-reality', public_host: 'proxy.example.com', port: 443, sni: 'www.example.com', public_key: 'TEST_ONLY', short_id: '0123abcd' }))
  const servers: PluginServer[] = [1, 2, 3].map(id => ({ id, name: `服务器 ${id}`, enabled: true, online: false, agent_supported: true, read_only: false, source: 'administrator' }))
  return {
    nodes: { data: nodes, fresh: true, error: '' },
    servers: { data: servers, fresh: true, error: '' },
    chains: { data: [{ id: 10, name: '原链路', entry_node_id: 3, exit_node_id: 2, available: true }], fresh: true, error: '' },
  }
}

function draft(entry = '1', exit = '2') {
  const data = new FormData()
  data.set('name', ' 保留的链路草稿 ')
  data.set('entry_node_id', entry)
  data.set('exit_node_id', exit)
  return data
}

test('direct create and delete callbacks send no writes for any failed or pending dependency', async () => {
  for (const resource of ['chains', 'nodes', 'servers'] as const) {
    for (const state of ['failed', 'pending'] as const) {
      const current = snapshot(), form = draft(), writes: unknown[] = []
      const previous = current[resource].data
      current[resource].fresh = false
      current[resource].error = state === 'failed' ? '夹具读取失败' : ''
      await expect(createChain(form, current, async body => { writes.push(body) })).rejects.toThrow(state === 'failed' ? '读取失败' : '正在刷新')
      await expect(deleteChain(10, current, async id => { writes.push(id) })).rejects.toThrow(state === 'failed' ? '读取失败' : '正在刷新')
      expect(writes).toEqual([])
      expect(current[resource].data).toBe(previous)
      expect(Array.from(form.entries())).toEqual([['name', ' 保留的链路草稿 '], ['entry_node_id', '1'], ['exit_node_id', '2']])
    }
  }
})

test('recovered snapshots revalidate a preserved node selection before a direct create', async () => {
  for (const change of ['missing-node', 'disabled-node', 'protocol', 'missing-server', 'disabled-server', 'same-server'] as const) {
    const current = snapshot(), form = draft(), writes: unknown[] = []
    current.nodes.fresh = false
    expect(chainWriteError(current)).toContain('正在刷新')
    current.nodes.fresh = true
    if (change === 'missing-node') current.nodes.data = current.nodes.data!.filter(node => node.id !== 1)
    if (change === 'disabled-node') current.nodes.data![0].enabled = false
    if (change === 'protocol') current.nodes.data![0].protocol = 'hysteria2'
    if (change === 'missing-server') current.servers.data = current.servers.data!.filter(server => server.id !== 1)
    if (change === 'disabled-server') current.servers.data![0].enabled = false
    if (change === 'same-server') current.nodes.data![0].server_id = 2
    await expect(createChain(form, current, async body => { writes.push(body) })).rejects.toThrow(change === 'same-server' ? '不同服务器' : '已不可用')
    expect(writes).toEqual([])
    expect(form.get('entry_node_id')).toBe('1')
    expect(form.get('name')).toBe(' 保留的链路草稿 ')
  }
})

test('a fresh chain role change blocks repeated entries and nested or cyclic paths', async () => {
  for (const selected of [draft('3', '1'), draft('2', '1'), draft('1', '3')]) {
    const writes: unknown[] = []
    await expect(createChain(selected, snapshot(), async body => { writes.push(body) })).rejects.toThrow('链路身份已变更')
    expect(writes).toEqual([])
  }
})

test('fresh valid creation keeps the original two-hop request and permits a shared exit', async () => {
  const current = snapshot(), writes: unknown[] = []
  // A successful GET establishes availability; an offline device can still receive a later deployment.
  const value = await createChain(draft(), current, async body => { writes.push(body); return { id: 11 } })
  expect(value).toEqual({ id: 11 })
  expect(writes).toEqual([{ name: '保留的链路草稿', entry_node_id: 1, exit_node_id: 2 }])
  expect(current.chains.data).toHaveLength(1)
})

test('restored deletion checks the current chain identity and permits explicit unavailable-resource cleanup', async () => {
  const current = snapshot(), writes: number[] = []
  current.chains.data = []
  await expect(deleteChain(10, current, async id => { writes.push(id) })).rejects.toThrow('此链路已不可用')
  expect(writes).toEqual([])
  current.chains.data = [{ id: 10, name: '原链路', entry_node_id: 3, exit_node_id: 2, available: false }]
  await deleteChain(10, current, async id => { writes.push(id) })
  expect(writes).toEqual([10])
})

test('missing snapshot data fails closed even if a caller supplies an inconsistent fresh flag', async () => {
  for (const resource of ['chains', 'nodes', 'servers'] as const) {
    const current = snapshot(), writes: unknown[] = []
    current[resource].data = undefined
    await expect(createChain(draft(), current, async body => { writes.push(body) })).rejects.toThrow('正在刷新')
    await expect(deleteChain(10, current, async id => { writes.push(id) })).rejects.toThrow('正在刷新')
    expect(writes).toEqual([])
  }
})

test('invalid or incomplete IDs cannot reach a writer through direct form calls', async () => {
  for (const value of ['', '0', '-1', '01', '1e0', '1.0', '9007199254740992']) {
    const writes: unknown[] = []
    await expect(createChain(draft(value), snapshot(), async body => { writes.push(body) })).rejects.toThrow()
    expect(writes).toEqual([])
  }
  expect(chainSelectionError(snapshot(), '', '')).toBe('')
})

test('invalid name rejection preserves the form and does not invoke the business writer', async () => {
  for (const name of ['   ', '名'.repeat(129)]) {
    const form = draft(), writes: unknown[] = []
    form.set('name', name)
    await expect(createChain(form, snapshot(), async body => { writes.push(body) })).rejects.toThrow('名称')
    expect(form.get('name')).toBe(name)
    expect(writes).toEqual([])
  }
})
