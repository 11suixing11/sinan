import { expect, test } from 'bun:test'
import { lossLabel, probeSlots, probeState, probeTone, probeValue } from '../src/probes'
import type { Probe, ProbeResult } from '../src/probes'

const probe: Probe = { id: 'probe', name: '回环', kind: 'icmp', target: '127.0.0.1', port: null, interval_secs: 10, carrier: '', enabled: true }
const point: ProbeResult = { id: 'point', probe_id: probe.id, sampled_at: 100_000, latency_ms: 0, loss_percent: 0, error: null }

test('probes distinguish zero, total loss and unavailable legacy measurements', () => {
  expect(probeValue(point, 'latency_ms')).toBe(0)
  expect(probeValue({ ...point, latency_ms: null, loss_percent: 100 }, 'loss_percent')).toBe(100)
  expect(probeValue({ ...point, loss_percent: 100, error: 'permission denied' }, 'loss_percent')).toBeNull()
  expect(probeValue({ ...point, error: '' }, 'latency_ms')).toBeNull()
  for (const invalid of [-1, NaN, Infinity, 101]) expect(probeValue({ ...point, loss_percent: invalid }, 'loss_percent')).toBeNull()
  expect(lossLabel(probe)).toBe('丢包率')
  expect(lossLabel({ kind: 'tcp' })).toBe('连接失败率')
  expect(probeTone(100, 'loss_percent')).toBe('danger')
  expect(probeTone(0, 'latency_ms')).toBe('good')
  expect(probeTone(null, 'latency_ms')).toBe('empty')
})

test('paused, offline, stale and failed reads never expose live quality', () => {
  expect(probeState(probe, point, 101_000)).toBe('最近采样')
  expect(probeState(probe, point, 161_000)).toBe('采样已过期')
  expect(probeState(probe, point, 101_000, true)).toBe('状态未知')
  expect(probeState({ ...probe, enabled: false }, point, 101_000)).toBe('已暂停')
  expect(probeState(probe, { ...point, error: 'failed' }, 101_000)).toBe('检测不可用')
  expect(probeState(probe, undefined, 101_000)).toBe('等待采样')
})

test('quality bars keep time gaps and unavailable samples instead of carrying forward success', () => {
  const samples = [point, { ...point, sampled_at: 105_000, loss_percent: 100, latency_ms: null }, { ...point, sampled_at: 80_000, error: 'failed' }, { ...point, sampled_at: 140_000 }, { ...point, probe_id: 'other', sampled_at: 120_000 }]
  const slots = probeSlots(samples, probe, 125_000, 5)
  expect(slots.map(value => value?.sampled_at)).toEqual([80_000, undefined, 105_000, undefined, undefined])
  expect(probeValue(slots[0], 'loss_percent')).toBeNull()
  expect(probeValue(slots[2], 'loss_percent')).toBe(100)
})
