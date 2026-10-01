import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { resolve, extname, sep } from 'node:path'

// Serve the actual built dist; all business requests use private API fixtures.
const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const dist = fileURLToPath(new URL('../dist/', import.meta.url))
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml' }
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(dist, pathname === '/' ? 'index.html' : `.${pathname}`)
  if (!file.startsWith(dist.endsWith(sep) ? dist : `${dist}${sep}`)) { response.writeHead(400).end(); return }
  try { response.writeHead(200, { 'Content-Type': mime[extname(file)] ?? 'application/octet-stream' }); response.end(await readFile(file)) }
  catch { response.writeHead(404).end() }
})

let browser
try {
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
  browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
  const origin = `http://127.0.0.1:${server.address().port}`
  for (const width of [1280, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 1000 } })
    const errors = [], writes = [], policyReads = [], usageReads = []
    const prefix = '/api/plugins/sing-box'
    let groupIds = [1], packageId = 1
    const policies = [
      { id: 1, name: '原策略', node_ids: [1], chain_ids: [], member_count: 1 },
      { id: 2, name: '新策略', node_ids: [2], chain_ids: [], member_count: 0 },
    ]
    const packages = [
      { id: 1, name: '原套餐', monthly_bytes: '1073741824', reset_day: 1, reset_hour: 0, reset_minute: 0, timezone: 'UTC', duration_days: 30 },
      { id: 2, name: '新套餐', monthly_bytes: '2147483648', reset_day: 15, reset_hour: 8, reset_minute: 30, timezone: 'UTC', duration_days: 60 },
    ]
    const nodes = [1, 2].map(id => ({ id, name: `节点 ${id}`, server_id: id, protocol: 'vless-reality', public_host: 'proxy.example.com', port: 20000 + id, sni: 'www.example.com', public_key: 'TEST_ONLY', short_id: '0123abcd' }))
    const user = { id: 1, name: '测试代理用户', subscription_token: 'TEST_ONLY', subscription_url: 'https://panel.example.com/s/TEST_ONLY' }
    const usage = { uplink: '10', downlink: '20', total: '30', by_user: [{ user_id: 1, name: user.name, deleted: false, uplink: '10', downlink: '20' }], by_node: [] }
    const entitlement = () => ({
      user_id: 1, package_group_id: packageId, package_name: packages.find(p => p.id === packageId).name,
      ...packages.find(p => p.id === packageId), starts_at: 1790812800, expires_at: 1795996800,
      cycle_start: 1790812800, next_reset: 1793491200, used_bytes: '30', status: 'active', allowed: true,
    })
    page.on('pageerror', error => errors.push(error.message))
    await page.route('**/api/**', async route => {
      const request = route.request(), pathname = new URL(request.url()).pathname, method = request.method()
      let value
      if (pathname === '/api/dashboard/access' && method === 'GET') value = { authenticated: true, public_dashboard: false }
      else if (pathname === '/api/me' && method === 'GET') value = { authenticated: true }
      else if (pathname === `${prefix}/users` && method === 'GET') value = [user]
      else if (pathname === `${prefix}/nodes` && method === 'GET') value = nodes
      else if (pathname === `${prefix}/chains` && method === 'GET') value = []
      else if (pathname === `${prefix}/policy-groups` && method === 'GET') value = policies
      else if (pathname === `${prefix}/package-groups` && method === 'GET') value = packages
      else if (pathname === `${prefix}/usage` && method === 'GET') { usageReads.push(Date.now()); value = usage }
      else if (pathname === `${prefix}/users/1/accesses` && method === 'GET') value = groupIds.map(id => ({ user_id: 1, node_id: id, uuid: 'TEST_ONLY', stat_name: `fixture_${id}`, direct_grant: false }))
      else if (pathname === `${prefix}/users/1/policy-groups` && method === 'GET') { policyReads.push([...groupIds]); value = { group_ids: [...groupIds] } }
      else if (pathname === `${prefix}/users/1/policy-groups` && method === 'PUT') {
        const payload = request.postDataJSON()
        assert.deepEqual(payload, { group_ids: [2] })
        writes.push({ pathname, method, payload })
        groupIds = [...payload.group_ids]; value = { group_ids: [...groupIds] }
      } else if (pathname === `${prefix}/users/1/entitlement` && method === 'GET') value = entitlement()
      else if (pathname === `${prefix}/users/1/subscription` && method === 'GET') {
        const format = new URL(request.url()).searchParams.get('format')
        assert(['singbox', 'links'].includes(format))
        value = { format, status: 'ready', message: '当前授权节点已应用。', subscription_url: user.subscription_url,
          available_formats: ['singbox', 'links'], granted_nodes: groupIds.length, eligible_nodes: groupIds.length,
          ready_nodes: nodes.filter(node => groupIds.includes(node.id)), entitlement: entitlement(),
          content: format === 'singbox' ? JSON.stringify({ outbounds: [{ tag: 'TEST_ONLY 当前用户节点' }] }) : 'vless://TEST_ONLY@proxy.example.com:443',
          filename: format === 'singbox' ? 'fixture.json' : 'fixture.txt', content_type: 'text/plain' }
      }
      else if (pathname === `${prefix}/users/1/package` && method === 'POST') {
        const payload = request.postDataJSON()
        assert.equal(payload.package_group_id, 2)
        assert.match(payload.request_id, /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/)
        writes.push({ pathname, method, payload })
        packageId = 2; value = entitlement()
      } else {
        errors.push(`Unexpected API: ${method} ${pathname}`)
        await route.fulfill({ status: 404, json: { error: '测试拒绝未知接口' } }); return
      }
      await route.fulfill({ json: value })
    })

    await page.goto(`${origin}/#/plugins/sing-box/users`)
    await page.getByRole('heading', { name: '可用范围与套餐', exact: true }).waitFor()
    const previous = page.getByRole('checkbox', { name: /原策略/ })
    const next = page.getByRole('checkbox', { name: /新策略/ })
    await previous.waitFor()
    assert.equal(await previous.isChecked(), true)
    assert.equal(await next.isChecked(), false)
    const readCount = policyReads.length
    const usageCount = usageReads.length
    await previous.uncheck()
    await next.check()
    const editedAt = Date.now()
    await page.waitForTimeout(5600)
    assert(Date.now() - editedAt >= 5500)
    assert(usageReads.length > usageCount, 'other resource polling must remain active')
    assert.equal(policyReads.length, readCount, 'unsaved policy assignments must not be refreshed by polling')
    assert.equal(await previous.isChecked(), false)
    assert.equal(await next.isChecked(), true)
    assert.deepEqual(groupIds, [1], 'editing a draft must not mutate the fixture server')
    const readback = page.waitForResponse(response => new URL(response.url()).pathname === `${prefix}/users/1/policy-groups` && response.request().method() === 'GET')
    await page.getByRole('button', { name: '保存策略组分配', exact: true }).click()
    await page.getByText('策略组分配已保存。单独授权仍保留，设备应用配置后更新可用节点。', { exact: true }).waitFor()
    assert.deepEqual(await (await readback).json(), { group_ids: [2] })
    assert.deepEqual(writes.filter(w => w.pathname.endsWith('/policy-groups')), [{ pathname: `${prefix}/users/1/policy-groups`, method: 'PUT', payload: { group_ids: [2] } }])
    assert(policyReads.length > readCount, 'saving must explicitly reload authoritative assignments')
    assert.deepEqual(policyReads.at(-1), [2])
    assert.equal(await previous.isChecked(), false)
    assert.equal(await next.isChecked(), true)

    await page.getByRole('button', { name: '分配或更换套餐', exact: true }).click()
    await page.locator('select[name="package_group_id"]').selectOption('2')
    await page.getByRole('button', { name: '确认分配', exact: true }).click()
    await page.getByText('套餐已分配，按分配时刻计算有效期。本期历史用量没有清空。', { exact: true }).waitFor()
    await page.getByText('新套餐', { exact: true }).waitFor()
    assert.equal(packageId, 2)
    assert.equal(writes.filter(w => w.pathname.endsWith('/package')).length, 1)
    await page.getByRole('button', { name: '订阅链接', exact: true }).click()
    assert.equal(await page.getByRole('combobox', { name: '订阅格式', exact: true }).inputValue(), 'singbox')
    await page.getByRole('dialog').getByText('可以获取', { exact: true }).waitFor()
    assert.equal(await page.locator('.subscription-address code').textContent(), `${user.subscription_url}?format=singbox`)
    await page.getByRole('combobox', { name: '订阅格式', exact: true }).selectOption('links')
    await page.getByRole('dialog').getByText('可以获取', { exact: true }).waitFor()
    assert.equal(await page.locator('.subscription-address code').textContent(), `${user.subscription_url}?format=links`)
    await page.getByRole('button', { name: '完成', exact: true }).click()
    assert.equal(writes.length, 2, 'subscription display must not write credentials or grants')
    assert.deepEqual(errors, [])
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    await page.close()
  }
  console.log('PASS: dist desktop/mobile, draft preserved across real 5-second polling, scoped PUT and readback, package assignment and subscription formats unchanged')
} finally {
  try { await browser?.close() }
  finally { await new Promise(resolve => server.close(resolve)) }
}
