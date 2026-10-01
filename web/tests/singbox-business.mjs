import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { resolve, extname, sep } from 'node:path'

// Shipped dist with controlled API responses; PostgreSQL verifies migration/data.
const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml' }
const server = createServer(async (request, response) => {
  const path = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(root, path === '/' ? 'index.html' : `.${path}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) { response.writeHead(400).end(); return }
  try { response.writeHead(200, { 'Content-Type': mime[extname(file)] ?? 'application/octet-stream' }); response.end(await readFile(file)) } catch { response.writeHead(404).end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
try {
  for (const width of [1280, 390]) {
    const page = await browser.newPage({ viewport: { width, height: 900 } })
    const errors = [], requests = [], now = Math.floor(Date.now() / 1000)
    page.on('pageerror', error => errors.push(error.message))
    const metadata = { id: 1, name: '纯监控验收服务器', enabled: false, source: null, read_only: false, online: true, agent_supported: false }
    const entry = { id: 1, name: metadata.name, online: true, device_public_key: 'test-only-key', static_info: { runtime_version: 'test-only-runtime' }, latest_metrics: { network_interfaces: { eth0: { received_bytes: 1024, transmitted_bytes: 2048 } } }, last_seen: now, manifest_rev: 0, capabilities: [] }
    const node = { id: 2, name: '插件代理节点', server_id: 1, protocol: 'vless-reality', port: 443, public_host: 'proxy.example.com', sni: 'www.example.com', public_key: 'public-test', short_id: '0123abcd' }
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname
      requests.push(path)
      let value
      if (path === '/api/me') value = { authenticated: true }
      else if (path === '/api/servers/1') value = entry
      else if (path === '/api/plugins/sing-box/servers/1') value = metadata
      else if (path === '/api/plugins/sing-box/servers') value = [metadata]
      else if (path === '/api/plugins/sing-box/servers/1/enable') {
        assert.equal(route.request().method(), 'POST')
        assert.deepEqual(route.request().postDataJSON(), {})
        Object.assign(metadata, { enabled: true, source: 'administrator' }); value = metadata
      } else if (path === '/api/plugins/sing-box/servers/1/deployments') {
        assert.equal(metadata.enabled, true); value = { status: null, history: [] }
      } else if (path === '/api/plugins/sing-box/nodes') {
        assert.equal(metadata.enabled, true); value = [node]
      } else if (path === '/api/plugins/sing-box/users') value = []
      else if (path === '/api/plugins/sing-box/chains') value = []
      else if (path === '/api/plugins/sing-box/usage') value = { uplink: '0', downlink: '0', total: '0', by_user: [], by_node: [] }
      else if (path === '/api/servers/1/agent-settings') value = { sample_interval_secs: 1, upload_interval_secs: 3, discover_public_ips: false, auto_update: false }
      else if (path === '/api/servers/1/node-quality') value = { ip_addresses: [], quality: [], plugin_ready: false, plugin_reason: '夹具未启用诊断', reports: [] }
      else if (['/api/servers/1/probes', '/api/servers/1/probe-results', '/api/servers/1/commands'].includes(path)) value = []
      else if (path === '/api/security/totp') value = { enabled: false }
      else { errors.push(`Unexpected API: ${path}`); await route.fulfill({ status: 404, json: {} }); return }
      await route.fulfill({ json: value })
    })
    const origin = `http://127.0.0.1:${server.address().port}`
    await page.goto(`${origin}/#/servers/1`)
    await page.getByRole('heading', { name: metadata.name, exact: true }).waitFor()
    await page.waitForFunction(() => document.querySelector('table')?.textContent.includes('eth0'))
    await page.waitForTimeout(150)
    assert.equal(await page.locator('[data-plugin="sing-box"]').count(), 0)
    assert.equal(await page.getByText('test-only-runtime', { exact: true }).count(), 0)
    assert.equal(requests.some(path => path.endsWith('/deployments') || path.endsWith('/nodes')), false)
    await page.getByRole('link', { name: '插件设置', exact: true }).click()
    await page.getByRole('button', { name: '启用 sing-box', exact: true }).click()
    await page.getByText('管理员明确启用', { exact: true }).waitFor()
    assert.equal(metadata.enabled, true)
    await page.goto(`${origin}/#/servers/1`)
    await page.getByRole('heading', { name: '配置部署', exact: true }).waitFor()
    await page.getByText('插件代理节点', { exact: true }).waitFor()
    assert.equal(await page.locator('[data-plugin="sing-box"]').count(), 1)
    Object.assign(metadata, { source: 'agent_capability', read_only: true, agent_supported: true })
    await page.getByRole('link', { name: '插件设置', exact: true }).click()
    await page.getByText('由设备声明或既有配置识别，来源只读', { exact: true }).waitFor()
    assert.equal(await page.getByRole('button', { name: '启用 sing-box', exact: true }).count(), 0)
    await page.getByRole('link', { name: '代理用户', exact: true }).click()
    await page.getByRole('heading', { name: '代理用户', exact: true }).waitFor()
    assert.equal(await page.getByRole('button', { name: '创建代理用户', exact: true }).count(), 2)
    await page.getByRole('link', { name: '系统管理员', exact: true }).click()
    await page.getByRole('heading', { name: '系统管理员', exact: true }).waitFor()
    assert.equal(await page.getByRole('button', { name: '创建代理用户', exact: true }).count(), 0)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    assert.deepEqual(errors, [])
    assert.equal(requests.some(path => ['/api/nodes', '/api/users', '/api/usage'].includes(path)), false)
    await page.close()
  }
  console.log('PASS: dist desktop/mobile, monitor hides business and skips business requests, explicit enable, read-only sources, administrator/proxy-user navigation, canonical APIs')
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)) }
