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
    const errors = [], requests = [], mutations = [], now = Math.floor(Date.now() / 1000)
    page.on('pageerror', error => errors.push(error.message))
    const metadata = { id: 1, name: '纯监控验收服务器', enabled: false, source: null, read_only: false, online: true, agent_supported: false }
    const entry = { id: 1, name: metadata.name, online: true, device_public_key: 'test-only-key', static_info: { runtime_version: 'test-only-runtime' }, latest_metrics: { network_interfaces: { eth0: { received_bytes: 1024, transmitted_bytes: 2048 } } }, last_seen: now, manifest_rev: 0, capabilities: [] }
    const node = { id: 2, name: '插件代理节点', server_id: 1, protocol: 'vless-reality', port: 443, public_host: 'proxy.example.com', sni: 'www.example.com', public_key: 'public-test', short_id: '0123abcd' }
    const exitNode = { ...node, id: 3, name: '另一台服务器的出口', server_id: 2, public_host: 'exit.example.com' }
    const chains = []
    let chainsFailure = false
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname
      requests.push(path)
      if (route.request().method() !== 'GET') mutations.push({ path, method: route.request().method() })
      let value
      if (path === '/api/dashboard/access') return route.fulfill({ json: { authenticated: true, public_dashboard: false } })
      if (path === '/api/me') value = { authenticated: true }
      else if (path === '/api/servers/1') value = entry
      else if (path === '/api/plugins/sing-box/servers/1') value = metadata
      else if (path === '/api/plugins/sing-box/servers') value = [metadata]
      else if (path === '/api/plugins/sing-box/servers/1/enable') {
        assert.equal(route.request().method(), 'POST')
        assert.deepEqual(route.request().postDataJSON(), {})
        Object.assign(metadata, { enabled: true, source: 'administrator', installation: { state: 'queued', reason: '已安排首次安装，等待设备应用。', target_rev: 1, applied_rev: 0 } }); value = metadata
      } else if (path === '/api/plugins/sing-box/servers/1/deployments') {
        assert.equal(metadata.enabled, true)
        value = { status: metadata.installation?.state === 'ready' ? { module: 'singbox', target_rev: 1, applied_rev: 1, last_result_rev: 1, healthy: true, last_error: null, updated_at: now } : null, history: [] }
      } else if (path === '/api/plugins/sing-box/nodes') {
        assert.equal(metadata.enabled, true); value = [node, exitNode]
      } else if (path === '/api/plugins/sing-box/users') value = []
      else if (path === '/api/plugins/sing-box/chains') {
        if (route.request().method() === 'POST') {
          assert.deepEqual(route.request().postDataJSON(), { name: '未授权验收链路', entry_node_id: 2, exit_node_id: 3 })
          const chain = { id: 9, name: '未授权验收链路', entry_node_id: 2, exit_node_id: 3, available: true }
          chains.push(chain); value = chain
        } else if (chainsFailure) { await route.fulfill({ status: 500, json: { error: '链路夹具读取失败' } }); return }
        else value = chains
      }
      else if (['/api/plugins/sing-box/policy-groups', '/api/plugins/sing-box/package-groups'].includes(path)) value = []
      else if (path === '/api/plugins/sing-box/usage') value = { uplink: '0', downlink: '0', total: '0', by_user: [], by_node: [] }
      else if (path === '/api/servers/1/agent-settings') value = { sample_interval_secs: 1, upload_interval_secs: 3, discover_public_ips: false, auto_update: false }
      else if (path === '/api/servers/1/node-quality') value = { ip_addresses: [], quality: [], plugin_ready: false, plugin_reason: '夹具未启用诊断', reports: [] }
      else if (path === '/api/servers/1/enrollment') value = { token: 'TEST_ONLY_ENROLLMENT', expires_at: now + 3600, install_command: null, warning: '夹具未导入 Agent 制品。' }
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

    // Adapter support alone must not opt this server into proxy business.
    Object.assign(metadata, { agent_supported: true, installation: { state: 'not_enabled', reason: '尚未启用。', target_rev: 0, applied_rev: 0 } })
    entry.capabilities = ['singbox']
    const supportedStart = requests.length
    await page.reload()
    await page.getByRole('heading', { name: metadata.name, exact: true }).waitFor()
    await page.waitForTimeout(150)
    assert.equal(await page.locator('[data-plugin="sing-box"]').count(), 0)
    assert.equal(requests.slice(supportedStart).some(path => path.endsWith('/deployments') || path.endsWith('/nodes')), false)
    await page.getByRole('link', { name: '插件设置', exact: true }).click()
    await page.getByText('设备支持 sing-box', { exact: true }).waitFor()
    await page.getByRole('button', { name: '启用并安装 sing-box', exact: true }).click()
    await page.getByText('管理员明确启用', { exact: true }).waitFor()
    await page.getByText('安装已安排', { exact: true }).waitFor()
    assert.equal(metadata.enabled, true)
    assert.equal(await page.getByText('已安装并运行', { exact: true }).count(), 0)

    metadata.installation = { state: 'failed', reason: '缺少此平台的签名 sing-box 制品。', target_rev: 1, applied_rev: 0 }
    await page.reload()
    await page.getByText('安装或部署失败', { exact: true }).waitFor()
    await page.getByText(metadata.installation.reason, { exact: true }).waitFor()
    assert.equal(await page.getByText('已安装并运行', { exact: true }).count(), 0)

    metadata.installation = { state: 'ready', reason: '设备已确认 sing-box 安装并运行。', target_rev: 1, applied_rev: 1 }
    await page.reload()
    await page.getByText('已安装并运行', { exact: true }).waitFor()
    await page.goto(`${origin}/#/servers/1`)
    await page.getByRole('heading', { name: '配置部署', exact: true }).waitFor()
    await page.getByText('插件代理节点', { exact: true }).waitFor()
    await page.getByText('已安装并运行', { exact: true }).waitFor()
    assert.equal(await page.locator('[data-plugin="sing-box"]').count(), 1)

    // A legacy response and a runtime-version string cannot certify installation.
    delete metadata.installation
    await page.reload()
    await page.getByText('安装状态待确认', { exact: true }).waitFor()
    assert.equal(await page.getByText('已安装并运行', { exact: true }).count(), 0)
    Object.assign(metadata, { source: 'agent_capability', read_only: true, agent_supported: true })
    await page.getByRole('link', { name: '插件设置', exact: true }).click()
    await page.getByText('保留已有启用记录', { exact: true }).waitFor()
    assert.equal(await page.getByRole('button', { name: '启用并安装 sing-box', exact: true }).count(), 0)

    await page.getByRole('link', { name: '代理服务', exact: true }).click()
    await page.getByRole('heading', { name: '代理服务', exact: true }).waitFor()
    await page.getByText('安装状态待确认', { exact: true }).waitFor()
    await page.getByRole('link', { name: '管理此服务器节点', exact: true }).click()
    await page.getByRole('heading', { name: '代理节点', exact: true }).waitFor()
    assert.equal(new URL(page.url()).hash, '#/plugins/sing-box/nodes?server=1')
    assert.equal(await page.getByRole('combobox', { name: '按服务器筛选', exact: true }).inputValue(), '1')
    await page.getByRole('button', { name: '创建节点', exact: true }).click()
    assert.equal(await page.locator('select[name="server_id"]').inputValue(), '1')
    await page.getByRole('dialog').getByRole('button', { name: '取消', exact: true }).click()
    await page.getByRole('navigation', { name: '节点资源类型', exact: true }).getByRole('link', { name: '两跳链路', exact: true }).click()
    await page.getByRole('heading', { name: '代理节点', exact: true, level: 1 }).waitFor()
    await page.getByRole('heading', { name: '两跳链路', exact: true, level: 2 }).waitFor()
    assert.equal(await page.getByRole('navigation', { name: '节点资源类型', exact: true }).getByRole('link', { name: '两跳链路', exact: true }).getAttribute('aria-current'), 'page')
    assert.equal(await page.locator('nav[aria-label="主导航"]').getByRole('link', { name: '两跳链路', exact: true }).count(), 0)
    const chainMutationStart = mutations.length
    await page.getByRole('button', { name: '创建两跳链路', exact: true }).click()
    const chainDialog = page.getByRole('dialog')
    await chainDialog.locator('input[name="name"]').fill('未授权验收链路')
    await chainDialog.locator('select[name="entry_node_id"]').selectOption('2')
    await chainDialog.locator('select[name="exit_node_id"]').selectOption('3')
    await chainDialog.getByRole('button', { name: '创建未授权链路', exact: true }).click()
    await page.getByText('未授权验收链路', { exact: true }).waitFor()
    assert.deepEqual(mutations.slice(chainMutationStart), [{ path: '/api/plugins/sing-box/chains', method: 'POST' }])
    await page.getByRole('navigation', { name: '节点资源类型', exact: true }).getByRole('link', { name: '节点监听', exact: true }).click()
    await page.getByText('链路专用入口', { exact: true }).waitFor()
    await page.goto(`${origin}/#/plugins/sing-box/nodes`)
    await page.getByText('链路出口', { exact: true }).waitFor()
    chainsFailure = true
    await page.reload()
    await page.getByText('链路夹具读取失败', { exact: true }).waitFor()
    await page.getByText('链路身份待确认', { exact: true }).first().waitFor()
    assert.equal(await page.getByText('链路身份待确认', { exact: true }).count(), 2)
    assert.equal(await page.getByText('普通节点监听', { exact: true }).count(), 0)
    chainsFailure = false

    await page.goto(`${origin}/#/servers/1`)
    await page.getByRole('button', { name: '接入 / 升级', exact: true }).click()
    const enrollment = page.getByRole('dialog')
    await enrollment.getByRole('link', { name: '安装服务器插件', exact: true }).waitFor()
    assert.equal(await enrollment.getByRole('link', { name: '安装服务器插件', exact: true }).getAttribute('href'), '#/plugins/sing-box')
    await enrollment.getByRole('link', { name: '安装服务器插件', exact: true }).click()
    await page.getByRole('heading', { name: '代理服务', exact: true }).waitFor()

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
  console.log('PASS: dist desktop/mobile, support does not enable business, queued/failed/ready installation and conservative legacy fallback, overview/server-node selection/unified ungranted chain creation/listener roles/failure/enrollment navigation, administrator/proxy-user separation, canonical APIs')
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)) }
