import { describe, expect, test } from 'bun:test'
import { hopReference, sourceReason, sourceStatus } from '../src/plugins/singbox/sourceTypes'
import type { ExternalNodePreview, SubscriptionSource } from '../src/plugins/singbox/sourceTypes'

const node: ExternalNodePreview = {
  id: 10, source_id: 2, node_version_id: 20, source_revision_id: 3,
  identity_epoch: 1, name: '示例节点', protocol: 'http', server: 'proxy.example.com',
  port: 443, transport: 'tcp', tcp: true, udp: false, selectable: true,
  present: true, identity_unique: true, reason: null,
}

describe('subscription source selection', () => {
  test('chain drafts freeze a version reference and contain no source secrets', () => {
    const response = { ...node, secret_url: 'https://source.example.com/private-fixture-token', password: 'fixture-secret' }
    const reference = hopReference(response, 'pinned')
    expect(reference).toEqual({ kind: 'subscription', source_id: 2, external_node_id: 10, node_version_id: 20, update_mode: 'pinned' })
    expect(JSON.stringify(reference)).not.toContain('fixture')
    expect(hopReference(response, 'follow_node')?.update_mode).toBe('follow_node')
  })

  test('missing, archived and ambiguous previews cannot become a draft hop', () => {
    for (const reason of ['node_missing', 'source_archived', 'ambiguous_node_identity']) {
      expect(hopReference({ ...node, selectable: false, reason }, 'follow_node')).toBeNull()
      expect(sourceReason(reason)).not.toBe('')
    }
    expect(hopReference({ ...node, id: null }, 'pinned')).toBeNull()
    expect(hopReference({ ...node, node_version_id: null }, 'pinned')).toBeNull()
  })

  test('failed refresh displays the retained version without asserting network health', () => {
    const source = { archived: false, active_job_id: null, last_error: 'download_failed', current_revision_id: 3, supported_count: 1 } as SubscriptionSource
    expect(sourceStatus(source)).toBe('更新失败，使用上次版本')
    expect(sourceStatus({ ...source, current_revision_id: null })).toBe('导入失败')
    expect(sourceStatus({ ...source, last_error: null, supported_count: 0 })).toBe('没有可选节点')
    expect(sourceStatus({ ...source, last_error: 'cancelled' })).toBe('已取消刷新，保留上次版本')
    expect(sourceStatus({ ...source, last_error: 'cancelled', current_revision_id: null })).toBe('已取消导入')
    expect(sourceReason('unknown-fixture-secret')).not.toContain('fixture-secret')
  })
})
