import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { extname, resolve, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const server = createServer(async (request, response) => {
  const path = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(root, path === '/' ? 'index.html' : `.${path}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) { response.writeHead(400).end(); return }
  try { const body = await readFile(file); response.writeHead(200, { 'Content-Type': ({ '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml' })[extname(file)] ?? 'application/octet-stream' }).end(body) }
  catch { response.writeHead(404).end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const origin = `http://127.0.0.1:${server.address().port}`
const browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
const screenshots = process.env.SINAN_UI_SCREENSHOT_DIR
if (screenshots) await mkdir(screenshots, { recursive: true })
const results = []
try {
  for (const width of [1440, 390, 320]) {
    const context = await browser.newContext({ viewport: { width, height: 1000 } })
    const page = await context.newPage(), errors = [], unexpected = [], writes = []
    page.on('pageerror', error => errors.push(error.message))
    let rules = [], failSave = true, failSync = true
    const now = Math.floor(Date.now() / 1000)
    const servers = [{ id: 1, name: '测试服务器', online: true, enabled: false }, { id: 2, name: '另一台服务器', online: true, enabled: true }]
    await page.route('**/api/**', async route => {
      const request = route.request(), path = new URL(request.url()).pathname, method = request.method()
      const respond = (json, status = 200) => route.fulfill({ status, json })
      if (path === '/api/dashboard/access') return respond({ authenticated: true, public_dashboard: false })
      if (path === '/api/plugins/ddns/servers') return respond(servers)
      if (path === '/api/plugins/ddns/servers/1/enable') { servers[0].enabled = true; return respond({ enabled: true }) }
      if (path === '/api/plugins/ddns/servers/1/disable') { servers[0].enabled = false; rules.forEach(rule => { rule.plugin_enabled = false }); return respond({ enabled: false }) }
      if (method !== 'GET') writes.push({ path, method, body: request.postData() ? request.postDataJSON() : null })
      if (path === '/api/plugins/ddns/rules') {
        if (method === 'GET') return respond(rules)
        if (failSave) return respond({ error: '测试：域名已存在规则' }, 409)
        const body = request.postDataJSON()
        rules.push({ id: 'test-rule', config: body.config, revision: 1, token_configured: true, busy: false, plugin_enabled: true, server_name: '测试服务器', candidate_ip: '2001:db8::10', ip_status: 'ready', ip_received_at: now, last_ip: null, last_success_at: null, attempted_at: null, next_run_at: 0, status: 'pending', error_code: null, failures: 0 })
        return respond(rules[0], 201)
      }
      if (path === '/api/plugins/ddns/rules/test-rule') {
        if (method === 'DELETE') { rules = []; return route.fulfill({ status: 204 }) }
        const body = request.postDataJSON()
        assert.equal(body.revision, rules[0].revision)
        rules[0] = { ...rules[0], config: body.config, revision: body.revision + 1 }
        return respond(rules[0])
      }
      if (path === '/api/plugins/ddns/rules/test-rule/sync') {
        if (failSync) return respond({ error: '测试：规则正在同步，请稍后重试' }, 409)
        Object.assign(rules[0], { status: 'updated', last_ip: '2001:db8::10', last_success_at: now, next_run_at: now + 300 })
        return respond(rules[0])
      }
      unexpected.push(`${method} ${path}`)
      return respond({ error: 'Unexpected request' }, 500)
    })
    await page.goto(`${origin}/#/plugins/ddns`)
    await page.getByRole('heading', { name: '动态域名解析', exact: true }).waitFor()
    await page.goto(`${origin}/#/servers/1/ddns`)
    await page.getByRole('heading', { name: '动态域名解析', exact: true }).waitFor()
    assert.equal(await page.getByText('另一台服务器', { exact: false }).count(), 0)
    await page.getByRole('button', { name: '启用 DDNS 插件', exact: true }).click()
    await page.getByRole('button', { name: '添加规则' }).click()
    const dialog = page.getByRole('dialog')
    await dialog.getByLabel('规则名称', { exact: true }).fill('家庭 IPv6')
    await dialog.getByLabel('完整域名', { exact: false }).fill('node.example.com')
    await dialog.getByLabel('Zone ID', { exact: false }).fill('00000000000000000000000000000001')
    await dialog.getByLabel('API Token', { exact: false }).fill('TEST_ONLY_CLOUDFLARE_TOKEN')
    await dialog.getByLabel('记录类型', { exact: false }).selectOption('AAAA')
    await dialog.getByLabel('启用 Cloudflare 代理', { exact: false }).check()
    assert.equal(await dialog.getByLabel('TTL（秒）', { exact: false }).inputValue(), '1')
    await dialog.getByRole('button', { name: '保存规则' }).click()
    await dialog.getByRole('alert').filter({ hasText: '测试：域名已存在规则' }).waitFor()
    assert.equal(await dialog.getByLabel('规则名称', { exact: true }).inputValue(), '家庭 IPv6')
    failSave = false
    await dialog.getByRole('button', { name: '保存规则' }).click()
    await dialog.waitFor({ state: 'hidden' })
    await page.getByRole('heading', { name: '家庭 IPv6' }).waitFor()
    assert.equal(writes.at(-1).body.config.ttl, 1)
    assert.equal(writes.at(-1).body.config.server_id, 1)
    assert.equal(writes.at(-1).body.config.record_type, 'AAAA')
    await page.getByRole('button', { name: '编辑', exact: true }).click()
    assert.equal(await dialog.getByLabel('API Token', { exact: false }).inputValue(), '')
    assert.equal(await dialog.getByLabel('完整域名', { exact: false }).isDisabled(), true)
    await dialog.getByLabel('检查间隔（秒）', { exact: false }).fill('600')
    await dialog.getByRole('button', { name: '保存规则' }).click()
    await dialog.waitFor({ state: 'hidden' })
    assert.equal(Object.hasOwn(writes.at(-1).body, 'api_token'), false)
    await page.getByRole('button', { name: '立即同步', exact: true }).click()
    await page.getByRole('alert').filter({ hasText: '测试：规则正在同步' }).waitFor()
    failSync = false
    await page.getByRole('button', { name: '立即同步', exact: true }).click()
    await page.getByText('已更新解析', { exact: true }).waitFor()
    await page.getByRole('button', { name: '暂停', exact: true }).click()
    await page.getByText('已暂停', { exact: true }).waitFor()
    assert.equal(await page.getByRole('button', { name: '立即同步', exact: true }).isDisabled(), true)
    const overflow = await page.evaluate(() => ({ width: window.innerWidth, scroll: document.documentElement.scrollWidth, elements: [...document.querySelectorAll('body *')].filter(element => element.getBoundingClientRect().right > window.innerWidth + 1).slice(0, 12).map(element => ({ tag: element.tagName, class: element.className, right: element.getBoundingClientRect().right })) }))
    if (overflow.scroll > width) console.log('Overflow evidence', JSON.stringify(overflow))
    assert.ok(overflow.scroll <= width)
    assert.equal(await page.locator('body').innerText().then(text => text.includes('TEST_ONLY_CLOUDFLARE_TOKEN')), false)
    if (screenshots) await page.screenshot({ path: `${screenshots}/ddns-${width}.png`, fullPage: true, animations: 'disabled' })
    await page.getByRole('button', { name: '编辑', exact: true }).click()
    if (screenshots) await page.screenshot({ path: `${screenshots}/ddns-editor-${width}.png`, fullPage: false, animations: 'disabled' })
    await dialog.getByRole('button', { name: '取消', exact: true }).click()
    await page.getByRole('button', { name: '停用 DDNS 插件', exact: true }).click()
    await page.getByRole('button', { name: '启用 DDNS 插件', exact: true }).waitFor()
    assert.equal(await page.getByRole('button', { name: '立即同步', exact: true }).isDisabled(), true)
    await page.getByRole('button', { name: '删除', exact: true }).click()
    await dialog.getByText(/Cloudflare 中的 DNS 记录会保留/).waitFor()
    await dialog.getByRole('button', { name: '确认删除' }).click()
    await page.getByRole('heading', { name: '尚未配置动态解析' }).waitFor()
    assert.deepEqual(errors, [])
    assert.deepEqual(unexpected, [])
    results.push({ width, create: 'passed', editWithoutToken: 'passed', manualSync: 'passed', pause: 'passed', removePreservesDns: 'passed' })
    await context.close()
  }
  console.log(JSON.stringify(results))
} finally {
  await browser.close()
  await new Promise(resolve => server.close(resolve))
}
