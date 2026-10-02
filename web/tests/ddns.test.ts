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
    for (const code of ['TEST_ONLY_TOKEN_VALUE', '__proto__', 'constructor', 'toString']) expect(ddnsMessage(code)).toBe('状态未知，请稍后刷新')
    expect(ddnsMessage('no_public_ip')).toContain('公网地址')
    expect(ddnsMessage('server_offline')).toContain('保留')
    expect(ddnsMessage('rate_limited')).toContain('重试')
  })
})

test('cloud provider credentials are sent only as a pair and stale Cloudflare tokens are omitted', () => {
  const config: DdnsConfig = { provider: 'tencent', line: '0', name: '测试', server_id: 1, zone_id: 'example.com', record_name: 'node.example.com', record_type: 'A', ttl: 600, proxied: false, interval_secs: 300, enabled: false, adopt_existing: false }
  const write = ddnsWrite(config, 'TEST_ONLY_OLD_CF_TOKEN', 2, ' TEST_ONLY_ID ', ' TEST_ONLY_SECRET ')
  expect(write).toEqual({ config, revision: 2, access_key_id: 'TEST_ONLY_ID', access_key_secret: 'TEST_ONLY_SECRET' })
  expect(ddnsWrite(config, '', 3)).toEqual({ config, revision: 3 })
  expect(ddnsMessage('submitted')).toContain('等待')
})
