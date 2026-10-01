import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { extname, resolve, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml' }
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(root, pathname === '/' ? 'index.html' : `.${pathname}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) { response.writeHead(400).end(); return }
  try { response.writeHead(200, { 'Content-Type': mime[extname(file)] ?? 'application/octet-stream' }); response.end(await readFile(file)) }
  catch { response.end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const origin = `http://127.0.0.1:${server.address().port}`
const browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
const screenshots = process.env.SINAN_UI_SCREENSHOT_DIR
if (screenshots) await mkdir(screenshots, { recursive: true })
const results = []
const installCommand = version => `sh -c 'set -eu; d=$(mktemp -d); curl --fail --silent --show-error --proto "=https" --tlsv1.2 -H "Accept: application/vnd.github.raw+json" "$1" -o "$d/bootstrap.sh"; printf "%s  %s\\n" "$2" "$d/bootstrap.sh" | sha256sum -c -; /bin/sh "$d/bootstrap.sh" --tag "$3" --panel "$4" --token "$5"' sinan-bootstrap 'https://api.github.com/repos/theLucius7/sinan/git/blobs/${'1'.repeat(40)}' '${'a'.repeat(64)}' 'agent-v${version}' 'https://panel.example.com' 'TEST_ONLY'`

try {
  for (const width of [1440, 390]) {
    const context = await browser.newContext({ viewport: { width, height: width > 800 ? 1000 : 844 } })
    await context.grantPermissions(['clipboard-read', 'clipboard-write'], { origin })
    const page = await context.newPage(), errors = [], unexpected = [], creates = [], enrollments = []
    page.on('pageerror', error => errors.push(error.message))
    let entry, settings, probes = [], createFailure = true, enrollmentMode = 'failure', statusFailure = false
    await page.route('**/api/**', async route => {
      const request = route.request(), url = new URL(request.url()), path = url.pathname.replace('/api/dashboard/', '/api/')
      const fulfill = (json, status = 200) => route.fulfill({ status, json })
      if (path === '/api/access') return route.fulfill({ json: { authenticated: true, public_dashboard: false } })
      if (path === '/api/me') return fulfill({ id: 1 })
      if (path === '/api/servers' && request.method() === 'GET') return fulfill(entry ? [entry] : [])
      if (path === '/api/servers' && request.method() === 'POST') {
        const body = request.postDataJSON(); creates.push(body)
        if (createFailure) return fulfill({ error: '测试：创建失败，请重试' }, 400)
        await new Promise(resolve => setTimeout(resolve, 100))
        settings = body.agent_settings; probes = body.probes
        entry = { id: 1, name: body.name, device_public_key: null, online: false, static_info: {}, latest_metrics: {}, manifest_rev: 0, capabilities: [], metrics_stale: false, metrics_sampled_at: null }
        return fulfill(entry, 201)
      }
      if (path === '/api/servers/1') return fulfill(statusFailure ? { error: '测试：状态暂不可用' } : entry, statusFailure ? 503 : 200)
      if (path === '/api/servers/1/enrollment' && request.method() === 'POST') {
        enrollments.push(url.searchParams.get('agent_version'))
        if (enrollmentMode === 'failure') return fulfill({ error: '测试：命令接口暂不可用' }, 503)
        const version = url.searchParams.get('agent_version') ?? '0.3.0'
        return fulfill({ token: 'TEST_ONLY', expires_at: Math.floor(Date.now() / 1000) + (enrollmentMode === 'expired' ? -1 : 86400), installation: enrollmentMode === 'missing' ? null : { version, tag: `agent-v${version}` }, install_command: enrollmentMode === 'missing' ? null : installCommand(version), warning: enrollmentMode === 'missing' ? '测试：请先导入兼容的签名制品' : null })
      }
      if (path === '/api/servers/1/agent-settings') return fulfill(settings)
      if (path === '/api/servers/1/probes') return fulfill(probes)
      if (path === '/api/servers/1/probe-results' || path === '/api/servers/1/commands') return fulfill([])
      if (path === '/api/plugins/sing-box/servers/1') return fulfill({ id: 1, name: entry.name, enabled: false, online: entry.online, agent_supported: false, read_only: false, source: null })
      unexpected.push(`${request.method()} ${path}`)
      return fulfill({ error: 'Unexpected fixture request' }, 500)
    })
    await page.goto(`${origin}/#/servers`)
    await page.getByRole('button', { name: '添加服务器', exact: true }).first().click()
    const dialog = page.getByRole('dialog')
    await dialog.getByText('连接一台新的服务器', { exact: true }).waitFor()
    if (width > 800) assert.ok((await dialog.boundingBox()).width >= 800, 'Desktop setup uses the wider layout')
    assert.equal(await dialog.getByLabel('采样间隔（秒）', { exact: false }).inputValue(), '1')
    assert.equal(await dialog.getByRole('switch', { name: /自动更新 Agent/ }).isChecked(), false)
    assert.equal(await dialog.getByRole('switch', { name: /自动识别公网地址/ }).isChecked(), true)
    if (screenshots) await page.screenshot({ animations: 'disabled', path: resolve(screenshots, `setup-empty-${width}.png`) })
    await dialog.getByLabel('服务器名称', { exact: false }).fill('   ')
    await dialog.getByRole('button', { name: '创建并继续' }).click()
    assert.equal(creates.length, 0)
    await dialog.getByLabel('服务器名称', { exact: false }).fill(' 东京 · 主节点 ')
    await dialog.getByRole('button', { name: /均衡/ }).click()
    await dialog.getByLabel('采样间隔（秒）', { exact: false }).fill('11')
    await dialog.getByRole('button', { name: '创建并继续' }).click()
    assert.equal(creates.length, 0, 'Upload interval must not be smaller than sampling')
    await dialog.getByRole('button', { name: /均衡/ }).click()
    await dialog.getByRole('switch', { name: /自动更新 Agent/ }).check()
    await dialog.getByRole('switch', { name: /自动识别公网地址/ }).uncheck()
    await dialog.getByRole('button', { name: '添加目标' }).click()
    const tcp = dialog.getByRole('group', { name: '拨测目标 1', exact: true })
    await tcp.getByLabel('拨测名称').fill('主站连通性')
    await tcp.getByLabel('目标地址').fill('probe.example.com')
    await tcp.getByLabel('目标端口').fill('8443')
    await tcp.getByLabel('拨测间隔（秒）').fill('45')
    await tcp.getByLabel('线路备注').fill('测试线路')
    await dialog.getByRole('button', { name: '添加目标' }).click()
    const icmp = dialog.getByRole('group', { name: '拨测目标 2', exact: true })
    await icmp.getByLabel('检测方式').selectOption('icmp')
    assert.equal(await icmp.getByLabel('目标端口').count(), 0)
    await icmp.getByLabel('拨测名称').fill('本地回显')
    await icmp.getByLabel('目标地址').fill('::1')
    await dialog.getByRole('button', { name: '添加目标' }).click()
    await dialog.getByRole('button', { name: '移除目标 3' }).click()
    assert.equal(await dialog.getByRole('group', { name: /拨测目标/ }).count(), 2)
    assert.equal(await tcp.getByLabel('目标地址').inputValue(), 'probe.example.com')
    await dialog.getByRole('button', { name: '创建并继续' }).click()
    await dialog.getByRole('alert').filter({ hasText: '测试：创建失败' }).waitFor()
    assert.equal(await tcp.getByLabel('目标端口').inputValue(), '8443', 'Failed creation preserves the form')
    assert.equal(await dialog.getByRole('switch', { name: /自动更新 Agent/ }).isChecked(), true)
    const overflow = await dialog.evaluate(element => element.scrollWidth > element.clientWidth + 1)
    assert.equal(overflow, false, `Dialog overflows horizontally at ${width}px`)
    if (screenshots) { await dialog.evaluate(element => { element.scrollTop = 0 }); await page.screenshot({ animations: 'disabled', path: resolve(screenshots, `setup-probes-${width}.png`) }) }
    createFailure = false
    await dialog.locator('form').evaluate(form => { form.requestSubmit(); form.requestSubmit() })
    await dialog.getByRole('alert').filter({ hasText: '测试：命令接口暂不可用' }).waitFor()
    assert.equal(creates.length, 2, 'Repeated submission must not duplicate the successful creation')
    assert.equal(enrollments.length, 1)
    assert.equal(creates[1].name, '东京 · 主节点')
    assert.deepEqual(settings, { sample_interval_secs: 3, upload_interval_secs: 10, auto_update: true, discover_public_ips: false })
    assert.deepEqual(probes.map(({ kind, port, interval_secs }) => ({ kind, port, interval_secs })), [{ kind: 'tcp', port: 8443, interval_secs: 45 }, { kind: 'icmp', port: null, interval_secs: 30 }])
    enrollmentMode = 'ok'
    await dialog.getByRole('button', { name: '重新生成命令' }).click()
    await dialog.getByRole('button', { name: '复制安装命令' }).waitFor()
    await dialog.getByRole('button', { name: '复制安装命令' }).click()
    await dialog.getByRole('button', { name: '已复制' }).waitFor()
    assert.equal(await page.evaluate(() => navigator.clipboard.readText()), installCommand('0.3.0'), 'Copy preserves the complete URL command and its shell quoting')
    assert.equal(await dialog.evaluate(element => element.scrollWidth > element.clientWidth + 1), false, `The URL command overflows the dialog at ${width}px`)
    assert.equal(creates.length, 2, 'Retrying enrollment must not recreate the server')
    await dialog.getByLabel('Agent 版本', { exact: false }).fill('0.2.9')
    assert.equal(await dialog.getByRole('button', { name: '复制安装命令' }).count(), 0, 'Changing the version hides the old command')
    await dialog.getByRole('button', { name: '重新生成命令' }).click()
    await dialog.locator('code').filter({ hasText: 'agent-v0.2.9' }).waitFor()
    assert.equal(enrollments.at(-1), '0.2.9')
    enrollmentMode = 'expired'
    await dialog.getByRole('button', { name: '重新生成命令' }).click()
    await dialog.getByText('接入令牌已过期，请重新生成命令。', { exact: true }).waitFor()
    assert.equal(await dialog.locator('code').count(), 0)
    enrollmentMode = 'missing'
    await dialog.getByRole('button', { name: '重新生成命令' }).click()
    await dialog.getByText(/测试：请先导入兼容的签名制品/).waitFor()
    assert.equal(await dialog.locator('code').count(), 0)
    enrollmentMode = 'ok'
    await dialog.getByRole('button', { name: '重新生成命令' }).click()
    await dialog.getByRole('button', { name: '复制安装命令' }).waitFor()
    entry = { ...entry, device_public_key: 'TEST_ONLY' }
    await dialog.getByRole('heading', { name: '已注册，等待设备连接' }).waitFor({ timeout: 10000 })
    entry = { ...entry, online: true, static_info: { agent_version: '0.3.0' } }
    await dialog.getByRole('heading', { name: '服务器已上线' }).waitFor({ timeout: 10000 })
    if (screenshots) { await dialog.evaluate(element => { element.scrollTop = 0 }); await page.screenshot({ animations: 'disabled', path: resolve(screenshots, `enrollment-online-${width}.png`) }) }
    statusFailure = true
    await dialog.getByRole('button', { name: '立即检查' }).click()
    await dialog.getByRole('heading', { name: '暂时无法确认设备状态' }).waitFor()
    assert.equal(await dialog.getByRole('heading', { name: '服务器已上线' }).count(), 0)
    statusFailure = false
    await dialog.getByRole('button', { name: '立即检查' }).click()
    await dialog.getByRole('heading', { name: '服务器已上线' }).waitFor()
    await dialog.getByRole('button', { name: '设备迟迟未上线？' }).click()
    await dialog.getByText(/确认设备能访问命令中的面板地址/).waitFor()
    await dialog.getByRole('button', { name: '查看服务器' }).focus()
    await page.keyboard.press('Tab')
    assert.equal(await page.evaluate(() => document.activeElement.getAttribute('aria-label')), '关闭对话框', 'Tab stays inside the dialog')
    await dialog.getByRole('button', { name: '查看服务器' }).click()
    await page.waitForURL('**/#/servers/1')
    await page.getByRole('heading', { name: '东京 · 主节点', exact: true }).waitFor()
    await page.getByRole('button', { name: '接入 / 升级', exact: true }).click()
    await dialog.getByRole('button', { name: '复制安装命令' }).waitFor()
    await dialog.getByRole('heading', { name: '设备当前在线' }).waitFor()
    assert.equal(await dialog.getByRole('heading', { name: '服务器已上线' }).count(), 0, 'An already online device does not confirm an upgrade')
    await page.keyboard.press('Escape')
    await dialog.waitFor({ state: 'hidden' })
    assert.equal(creates.length, 2)
    assert.deepEqual(unexpected, [])
    assert.deepEqual(errors, [])
    results.push({ width, creation: 'passed', enrollment: 'passed', recovery: 'passed', liveStatus: 'passed', keyboard: 'passed' })
    await context.close()
  }
  console.log(JSON.stringify(results, null, 2))
} finally {
  await browser.close()
  await new Promise(resolve => server.close(resolve))
}
