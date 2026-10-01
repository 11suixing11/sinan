import { expect, test } from 'bun:test'
import { protocolRequest } from '../src/plugins/singbox/ProtocolFields'

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
