import { expect, test } from 'bun:test'
import { mergeLiveServers } from '../src/display/live'
import type { LiveSnapshot } from '../src/display/live'
import type { Server } from '../src/types'

const node = { id: 1, name: '节点一', public_view: false, metrics_sampled_at: 2000, latest_metrics: { cpu_percent: 20 } } as Server
test('live visibility snapshots prune removed nodes and only allow safe newer metrics', () => {
  const update = { id: 1, online: true, metrics_sampled_at: 1000, latest_metrics: { cpu_percent: 10 }, name: '覆盖名称', asset_settings: { price: '999' } }
  const live = { served_at: 3000, public_view: false, servers: [update] } as unknown as LiveSnapshot
  const result = mergeLiveServers([node, { ...node, id: 2 }], live)
  expect(result).toHaveLength(1)
  expect(result[0].name).toBe('节点一')
  expect(result[0].latest_metrics.cpu_percent).toBe(20)
  expect(result[0].asset_settings).toBeUndefined()
  expect(mergeLiveServers([node], { ...live, public_view: true })).toEqual([])
  expect(mergeLiveServers([node], { ...live, servers: [] })).toEqual([])
})
