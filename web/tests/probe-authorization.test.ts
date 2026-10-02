import { expect, test } from 'bun:test'
import { authorizationDraft, authorizationError, authorizationPayload, deleteLatencyTask, deleteServerProbe, initialProbePayloads, overviewProbe, probeAuthorizationState, probeState, saveLatencyTask, saveServerProbe } from '../src/probes'
import type { LatencyTask, Probe, ProbeAuthorization, ProbeWriteSnapshot, TaskWriteSnapshot } from '../src/probes'

const authorization: ProbeAuthorization = { region: '测试地区', source: '自有管理清单', scope: 'owned', evidence: 'TEST_ONLY 管理确认', expires_at: null }
const probe: Probe = { id: 'probe-1', name: '回环目标', kind: 'tcp', target: '127.0.0.1', port: 443, interval_secs: 30, carrier: '', enabled: true, revision: 1, authorization }
const task: LatencyTask = { id: 'task-1', spec: probe, authorization, revision: 1, server_ids: [1], default_enabled: false }
const resource = <T,>(data: T[]) => ({ data, fresh: true, error: '' })
const single = (): ProbeWriteSnapshot => ({ serverId: 1, probes: resource([{ ...probe }]) })
const unified = (): TaskWriteSnapshot => ({ tasks: resource([{ ...task }]), servers: resource([{ id: 1 }, { id: 2 }]) })

test('authorization is explicit, byte bounded, and expires at the exact second', () => {
  expect(authorizationError(null, true, 100)).toContain('待确认')
  expect(authorizationError(null, false, 100)).toBe('')
  expect(() => authorizationPayload(authorizationDraft(), true, 100)).toThrow()
  for (const value of [{ ...authorization, scope: '' }, { ...authorization, source: '' }, { ...authorization, evidence: '' }, { ...authorization, source: 'x\n' }, { ...authorization, evidence: 'x\u0085y' }, { ...authorization, region: '区'.repeat(22) }, { ...authorization, source: '源'.repeat(86) }, { ...authorization, evidence: '证'.repeat(171) }, { ...authorization, expires_at: NaN }, { ...authorization, expires_at: 0 }, { ...authorization, expires_at: 100.5 }]) expect(authorizationError(value as ProbeAuthorization, true, 100)).not.toBe('')
  expect(authorizationError({ ...authorization, region: '', expires_at: 101 }, true, 100)).toBe('')
  expect(authorizationError({ ...authorization, expires_at: 100 }, true, 100)).toContain('过期')
  expect(authorizationError({ ...authorization, expires_at: 100 }, false, 100)).toBe('')
  const draft = authorizationDraft({ ...authorization, expires_at: 1_900_000_001 })
  expect(authorizationPayload(draft, true, 100)?.expires_at).toBe(1_900_000_001)
  expect(() => authorizationPayload(authorizationDraft({ ...authorization, expires_at: Number.MAX_SAFE_INTEGER }), true, 100)).toThrow('截止时间无效')
  expect(authorizationPayload(authorizationDraft(), false, 100)).toBeNull()
})

test('public safe status contains no evidence and legacy or expired authorization never shows current success', () => {
  const point = { id: 'result', probe_id: probe.id, sampled_at: 100_000, latency_ms: 0, loss_percent: 0, error: null }
  expect(probeState({ ...probe, authorization: null }, point, 101_000)).toBe('目标授权待确认')
  expect(probeState({ ...probe, authorization: { ...authorization, expires_at: 101 }, authorization_state: 'allowed' }, point, 101_000)).toBe('目标授权已过期')
  const publicProbe = { ...probe, authorization_state: 'allowed' as const }
  delete publicProbe.authorization
  expect(probeState(publicProbe, point, 101_000)).toBe('最近采样')
  expect(probeState(overviewProbe({ server_id: 1, probe: { ...publicProbe, authorization_state: undefined }, results: [point], authorization_state: 'allowed' }), point, 101_000)).toBe('最近采样')
  expect(probeState(overviewProbe({ server_id: 1, probe: publicProbe, results: [point], authorization_state: 'expired' }), point, 101_000)).toBe('目标授权已过期')
  expect(probeState(overviewProbe({ server_id: 1, probe: { ...probe, authorization: null }, results: [point], authorization_state: 'allowed' }), point, 101_000)).toBe('目标授权待确认')
  expect(probeAuthorizationState({})).toBe('missing')
  expect(probeAuthorizationState({ authorization_state: 'expired' })).toBe('expired')
  expect(probeState(publicProbe, point, 101_000, true)).toBe('状态未知')
  expect(probeState({ ...probe, enabled: false, authorization: null }, point, 101_000)).toBe('已暂停')
})

test('direct single-server callbacks make zero writes on stale, failed or missing source reads', async () => {
  for (const kind of ['pending', 'failure', 'missing'] as const) {
    const snapshot = single(), writes: unknown[] = []
    snapshot.probes.fresh = kind === 'missing'
    snapshot.probes.error = kind === 'failure' ? 'TEST_ONLY 读取失败' : ''
    if (kind === 'missing') snapshot.probes.data = undefined
    await expect(saveServerProbe(snapshot, 1, probe, { ...probe, name: '保留草稿' }, authorization, async body => { writes.push(body) })).rejects.toThrow()
    await expect(deleteServerProbe(snapshot, 1, probe, async (...body) => { writes.push(body) })).rejects.toThrow()
    expect(writes).toEqual([])
    expect(probe.name).toBe('回环目标')
  }
})

test('fresh single-server writes recheck server, ID, immutable target and revision', async () => {
  for (const kind of ['server', 'deleted', 'revision', 'unknown-version', 'target', 'task'] as const) {
    const snapshot = single(), writes: unknown[] = []
    if (kind === 'server') snapshot.serverId = 2
    if (kind === 'deleted') snapshot.probes.data = []
    if (kind === 'revision') snapshot.probes.data![0].revision = 2
    if (kind === 'unknown-version') delete snapshot.probes.data![0].revision
    if (kind === 'target') snapshot.probes.data![0].target = 'other.example.com'
    if (kind === 'task') snapshot.probes.data![0].task_id = 'task-1'
    await expect(saveServerProbe(snapshot, 1, probe, probe, authorization, async body => { writes.push(body) })).rejects.toThrow()
    await expect(deleteServerProbe(snapshot, 1, probe, async (...body) => { writes.push(body) })).rejects.toThrow()
    expect(writes).toEqual([])
  }
})

test('valid single-server writes preserve the eight spec fields and include authorization and CAS separately', async () => {
  const writes: unknown[] = []
  await saveServerProbe(single(), 1, probe, { ...probe, name: ' 新名称 ' }, authorization, async body => { writes.push(body) })
  expect(Object.keys(writes[0] as object).sort()).toEqual(['authorization', 'carrier', 'enabled', 'id', 'interval_secs', 'kind', 'name', 'port', 'revision', 'target'].sort())
  expect((writes[0] as Probe).name).toBe('新名称')
  await deleteServerProbe(single(), 1, probe, async (id, body) => { writes.push({ id, ...body }) })
  expect(writes[1]).toEqual({ id: probe.id, revision: 1 })
  await saveServerProbe(single(), 1, probe, { ...probe, enabled: false }, null, async body => { writes.push(body) })
  expect((writes[2] as Probe).authorization).toBeNull()
  const bad: unknown[] = []
  await expect(saveServerProbe(single(), 1, probe, { ...probe, enabled: true }, null, async body => { bad.push(body) })).rejects.toThrow('待确认')
  expect(bad).toEqual([])
})

test('all unified dependencies gate real save/delete callbacks without discarding drafts', async () => {
  for (const dependency of ['tasks', 'servers'] as const) {
    for (const kind of ['pending', 'failure', 'missing'] as const) {
      const snapshot = unified(), writes: unknown[] = []
      snapshot[dependency].fresh = kind === 'missing'
      snapshot[dependency].error = kind === 'failure' ? 'TEST_ONLY 读取失败' : ''
      if (kind === 'missing') snapshot[dependency].data = undefined
      await expect(saveLatencyTask(snapshot, task, task, authorization, async body => { writes.push(body) })).rejects.toThrow()
      await expect(deleteLatencyTask(snapshot, task, async (...body) => { writes.push(body) })).rejects.toThrow()
      expect(writes).toEqual([])
      expect(task.server_ids).toEqual([1])
    }
  }
})

test('unified writes reject removed selections and changed task revisions while allowing offline allocation', async () => {
  for (const kind of ['server', 'revision', 'deleted', 'target'] as const) {
    const snapshot = unified(), writes: unknown[] = []
    if (kind === 'server') snapshot.servers.data = [{ id: 2 }]
    if (kind === 'revision') snapshot.tasks.data![0].revision = 2
    if (kind === 'deleted') snapshot.tasks.data = []
    if (kind === 'target') snapshot.tasks.data![0].spec = { ...probe, target: 'other.example.com' }
    await expect(saveLatencyTask(snapshot, task, task, authorization, async body => { writes.push(body) })).rejects.toThrow()
    expect(writes).toEqual([])
  }
  const writes: unknown[] = []
  await saveLatencyTask(unified(), task, { ...task, server_ids: [1, 2] }, authorization, async body => { writes.push(body) })
  const body = writes[0] as LatencyTask
  expect(body.server_ids).toEqual([1, 2])
  expect(body.authorization).toEqual(authorization)
  expect(Object.keys(body.spec).sort()).toEqual(['id', 'name', 'kind', 'target', 'port', 'interval_secs', 'carrier', 'enabled'].sort())
  expect(Object.hasOwn(body.spec, 'authorization')).toBe(false)
  await deleteLatencyTask(unified(), task, async (id, body) => { writes.push({ id, ...body }) })
  expect(writes[1]).toEqual({ id: task.id, revision: 1 })
})

test('server enrollment builds all initial authorized targets atomically and preserves failed drafts', () => {
  const draft = { spec: probe, authorization: authorizationDraft(authorization) }
  const incomplete = { spec: { ...probe, name: '第二目标' }, authorization: authorizationDraft() }
  expect(() => initialProbePayloads([draft, incomplete], 100)).toThrow()
  expect(incomplete.spec.name).toBe('第二目标')
  expect(incomplete.authorization.scope).toBe('')
  const result = initialProbePayloads([draft], 100)
  expect(result[0].authorization).toEqual(authorization)
  expect(Object.hasOwn(result[0], 'revision')).toBe(false)
  expect(() => initialProbePayloads(Array.from({ length: 33 }, () => draft), 100)).toThrow('32')
})
