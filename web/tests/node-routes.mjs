import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile } from 'node:fs/promises'
import { resolve, extname, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const server = createServer(async (request, response) => {
  const file = resolve(root, new URL(request.url, 'http://127.0.0.1').pathname === '/' ? 'index.html' : `.${new URL(request.url, 'http://127.0.0.1').pathname}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) return response.writeHead(400).end()
  try { const body = await readFile(file); response.writeHead(200, { 'Content-Type': ({ '.html':'text/html', '.js':'text/javascript', '.css':'text/css' })[extname(file)] ?? 'application/octet-stream' }).end(body) }
  catch { response.writeHead(404).end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const origin = `http://127.0.0.1:${server.address().port}`
const browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
try {
  for (const width of [1440, 390]) {
    const page = await browser.newPage({ viewport: { width, height:950 } }), errors = [], writes = []
    page.on('pageerror', error => errors.push(error.message))
    const nodes = [1,2].map(id => ({ id, server_id:id, name:`服务器${id}节点`, protocol:'vless-reality', port:443, public_host:`node${id}.example.com`, sni:'www.example.com' }))
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname
      if (route.request().method() !== 'GET') { writes.push(path); return route.fulfill({ status:405, json:{} }) }
      let data
      if (path === '/api/dashboard/access') data = { authenticated:true, public_dashboard:false }
      else if (path === '/api/plugins/sing-box/servers') data = [1,2].map(id => ({ id, name:`服务器${id}`, enabled:true, online:true, agent_supported:true }))
      else if (path === '/api/plugins/sing-box/nodes') data = nodes
      else if (path === '/api/plugins/sing-box/usage') data = { total:'0', uplink:'0', downlink:'0', by_node:[], by_user:[] }
      else if (['policy-groups','package-groups','chains'].some(key => path === `/api/plugins/sing-box/${key}`)) data = []
      else { errors.push(`Unexpected API ${path}`); return route.fulfill({ status:404, json:{} }) }
      await route.fulfill({ json:data })
    })
    await page.goto(`${origin}/#/plugins/sing-box/nodes?server=2`)
    await page.getByRole('heading', { name:'代理节点', exact:true }).waitFor({ timeout:3000 })
    await page.waitForFunction(() => document.querySelector('.filter-select')?.value === '2')
    assert.equal(await page.locator('tbody tr').count(), 1)
    assert.match(await page.locator('tbody tr').innerText(), /服务器2节点/)
    assert.equal(await page.getByRole('navigation', { name:'主导航' }).getByRole('link', { name:'代理节点', exact:true }).getAttribute('aria-current'), 'page')
    await page.getByRole('button', { name:'创建节点', exact:true }).first().click()
    assert.equal(await page.getByRole('dialog').locator('[name=server_id]').inputValue(), '2')
    await page.getByRole('dialog').getByRole('button', { name:'取消', exact:true }).click()
    await page.evaluate(() => { location.hash = '/plugins/sing-box/nodes?server=1' })
    await page.waitForFunction(() => document.querySelector('.filter-select')?.value === '1')
    assert.match(await page.locator('tbody tr').innerText(), /服务器1节点/)
    await page.evaluate(() => { location.hash = '/plugins/sing-box/nodes?server=3' })
    await page.waitForFunction(() => document.querySelector('.filter-select')?.value === '3')
    assert.equal(await page.getByRole('button', { name:'创建节点', exact:true }).first().isDisabled(), true)
    assert.equal(await page.locator('tbody tr').count(), 0)
    await page.evaluate(() => { location.hash = '/plugins/sing-box/nodes?kind=chains' })
    await page.getByRole('button', { name:'创建两跳链路', exact:true }).waitFor()
    assert.equal(await page.getByRole('navigation', { name:'节点资源类型', exact:true }).getByRole('link', { name:'两跳链路', exact:true }).getAttribute('aria-current'), 'page')
    await page.getByRole('button', { name:'创建两跳链路', exact:true }).click()
    await page.getByRole('dialog').getByRole('heading', { name:'创建两跳链路', exact:true }).waitFor()
    assert.equal(await page.getByRole('dialog').locator('[name=entry_node_id] option').count(), 3)
    await page.getByRole('dialog').getByRole('button', { name:'取消', exact:true }).click()
    await page.evaluate(() => { location.hash = '/plugins/sing-box/nodes?kind=chains&server=2' })
    await page.getByText('筛选范围：入口或出口属于「服务器2」的链路。', { exact:false }).waitFor()
    assert.equal(await page.getByRole('navigation', { name:'主导航' }).getByRole('link', { name:'代理节点', exact:true }).getAttribute('aria-current'), 'page')
    assert.equal(await page.getByRole('navigation', { name:'节点资源类型', exact:true }).getByRole('link', { name:'节点监听', exact:true }).getAttribute('href'), '#/plugins/sing-box/nodes?server=2')
    for (const query of ['server=0','server=-1','server=01','server=1.0','server=1&server=2','server=9007199254740992','server=1e2','kind=unknown','kind=chains&kind=chains','server=2&kind=direct','server=2&kind=chains&other=1','other=1']) {
      await page.evaluate(query => { location.hash = '/plugins/sing-box/nodes?' + query }, query)
      await page.getByRole('heading', { name:'这个页面不存在', exact:true }).waitFor()
      assert.equal(await page.locator('.node-editor').count(), 0)
    }
    assert.deepEqual(writes, [])
    assert.deepEqual(errors, [])
    await page.close()
  }
  console.log('PASS: selected-server node route and create default, live route change, server-scoped chain category, invalid/duplicate routes refuse without writes')
} finally { await browser.close(); await new Promise(resolve => server.close(resolve)) }
