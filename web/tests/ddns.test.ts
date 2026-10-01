import { describe, expect, test } from 'bun:test'
import { ddnsMessage, ddnsWrite } from '../src/plugins/ddns/types'
import type { DdnsConfig } from '../src/plugins/ddns/types'

describe('DDNS credentials and status', () => {
  const config: DdnsConfig = { name: '测试', server_id: 1, zone_id: '00000000000000000000000000000001', record_name: 'node.example.com', record_type: 'A', ttl: 300, proxied: false, interval_secs: 300, enabled: true, adopt_existing: false }
  test('ordinary edits omit saved credentials and preserve a revision', () => {
    expect(ddnsWrite(config, ' ', 3)).toEqual({ config, revision: 3 })
    const replacement = ddnsWrite({ ...config, proxied: true }, ' TEST_ONLY_TOKEN_VALUE ', 4)
    expect(replacement.api_token).toBe('TEST_ONLY_TOKEN_VALUE')
    expect(replacement.config.ttl).toBe(1)
    expect(config.ttl).toBe(300)
  })
  test('unknown provider text is never displayed as a raw message', () => {
    expect(ddnsMessage('TEST_ONLY_TOKEN_VALUE')).toBe('状态未知，请稍后刷新')
    expect(ddnsMessage('no_public_ip')).toContain('公网地址')
    expect(ddnsMessage('server_offline')).toContain('保留')
    expect(ddnsMessage('rate_limited')).toContain('重试')
  })
})
