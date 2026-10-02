import { expect, test } from 'bun:test'
import { resolveRoute } from '../src/app/routes'
import { currentNavigation } from '../src/app/navigation'

test('dashboard aliases keep the public read-only route after shell extraction', () => {
  for (const path of ['/', '/dashboard', '/overview']) expect(resolveRoute(path)).toEqual({ page: 'dashboard' })
  for (const path of ['/dashboard/42', '/overview/42']) expect(resolveRoute(path)).toEqual({ page: 'dashboard', serverId: 42 })
  expect(resolveRoute('/servers')).toEqual({ page: 'servers' })
  expect(currentNavigation(resolveRoute('/'))?.path).toBe('/dashboard')
})

test('old device DDNS, catalog alias and unified node selection retain their destinations', () => {
  expect(resolveRoute('/servers/42/ddns')).toEqual({ page: 'server', serverId: 42, section: 'ddns' })
  expect(currentNavigation(resolveRoute('/servers/42/ddns'))?.path).toBe('/plugins/ddns')
  expect(currentNavigation(resolveRoute('/servers/42/node-quality'))?.path).toBe('/servers')
  expect(resolveRoute('/artifacts')).toEqual({ page: 'catalog' })
  expect(currentNavigation(resolveRoute('/artifacts'))?.path).toBe('/plugins/catalog')
  expect(resolveRoute('/plugins/sing-box/nodes?server=42')).toEqual({ page: 'nodes', serverId: 42, chains: false })
  expect(resolveRoute('/plugins/sing-box/nodes?kind=chains')).toEqual({ page: 'nodes', chains: true })
  expect(resolveRoute('/plugins/sing-box/nodes/chain/42')).toEqual({ page: 'nodes', selected: { kind: 'chain', id: 42 } })
  expect(currentNavigation(resolveRoute('/plugins/alicloud'))?.path).toBe('/plugins/alicloud')
})

test('unsafe resource IDs and ambiguous filtered paths never fall through into management routes', () => {
  for (const path of ['/servers/0', '/servers/9007199254740992/plugins', '/dashboard/9007199254740992',
    '/plugins/sing-box/nodes/direct/9007199254740992', '/plugins/sing-box/nodes?server=42&kind=chains',
    '/plugins/sing-box/nodes?server=42&server=43', 'constructor', '__proto__']) {
    expect(resolveRoute(path)).toEqual({ page: 'not-found' })
    expect(currentNavigation(resolveRoute(path))).toBeUndefined()
  }
})
