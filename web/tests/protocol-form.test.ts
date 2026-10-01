import { expect, test } from 'bun:test'
import { protocolRequest } from '../src/plugins/singbox/ProtocolFields'
import { nodeSettingsRequest } from '../src/plugins/singbox/NodeSettingsFields'

function form(values: Record<string, string>) {
  const form = new FormData()
  for (const [key, value] of Object.entries(values)) form.set(key, value)
  return form
}

test('editing manual TLS without a replacement preserves stored secrets', () => {
  expect(protocolRequest(form({ protocol: 'anytls', tls_mode: 'manual', certificate: '', key: '' })))
    .toEqual({ type: 'anytls', tls: { mode: 'manual' } })
})

test('automatic certificates send only contact and challenge settings', () => {
  const request = protocolRequest(form({ protocol: 'tuic', tls_mode: 'acme', email: ' admin@example.com ', challenge: 'tls-alpn-01', key: 'TEST_ONLY stale form value' }))
  expect(request).toEqual({ type: 'tuic', tls: { mode: 'acme', email: 'admin@example.com', challenge: 'tls-alpn-01' } })
})

test('switching to a protocol without TLS does not submit certificate material', () => {
  expect(protocolRequest(form({ protocol: 'snell-v6', certificate: 'TEST_ONLY stale certificate', key: 'TEST_ONLY stale key' })))
    .toEqual({ type: 'snell-v6' })
})

test('incomplete manual replacement is sent for validation rather than silently discarded', () => {
  expect(protocolRequest(form({ protocol: 'hysteria2', tls_mode: 'manual', key: 'TEST_ONLY incomplete key' })))
    .toEqual({ type: 'hysteria2', tls: { mode: 'manual', certificate: '', key: 'TEST_ONLY incomplete key' } })
})

test('node settings preserve obfuscation secrets and clear a public port explicitly', () => {
  const request = nodeSettingsRequest(form({ protocol:'hysteria2', listen:'::', public_port:'', obfs_enabled:'on', obfs_password:'', up_mbps:'80', down_mbps:'40' }))
  expect(request.public_port).toBeNull()
  expect(request).toHaveProperty('hysteria2', { up_mbps:80, down_mbps:40, ignore_client_bandwidth:false, obfs_enabled:true })
  expect(request).not.toHaveProperty('hysteria2.obfs_password')
})

test('protocol changes cannot submit another protocol group', () => {
  const request = nodeSettingsRequest(form({ protocol:'tuic', listen:'0.0.0.0', public_port:'443', congestion_control:'bbr', heartbeat_seconds:'10', obfs_enabled:'on', obfs_password:'TEST_ONLY stale field', handshake_port:'8443' }))
  expect(request.public_port).toBe(443)
  expect(request).not.toHaveProperty('hysteria2')
  expect(request).not.toHaveProperty('reality')
  expect(request).toHaveProperty('tuic.heartbeat_seconds', 10)
})
