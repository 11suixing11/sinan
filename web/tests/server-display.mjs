import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { extname, resolve, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.txt': 'text/plain' }
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(root, pathname === '/' ? 'index.html' : `.${pathname}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) { response.writeHead(400).end(); return }
  try { const content = await readFile(file); response.writeHead(200, { 'Content-Type': mime[extname(file)] ?? 'application/octet-stream' }); response.end(content) }
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
    const context = await browser.newContext({ viewport: { width, height: 1000 }, colorScheme: 'light' })
    const page = await context.newPage(), errors = [], writes = [], requests = []
    page.on('pageerror', error => errors.push(error.message))
    const now = Date.now(), GiB = 1024 ** 3
    const metrics = (cpu, rate) => ({ cpu_percent: cpu, memory_used: GiB * 1.2, disk_used: GiB * 16, swap_used: 0, swap_total: GiB, processes: 84, tcp_connections: 32, udp_connections: 5, load_1: .2, load_5: .3, load_15: .25, uptime_secs: 86400 * 28, network_interfaces: { eth0: { received_bytes: GiB * 48, transmitted_bytes: GiB * 16, receive_bytes_per_sec: rate * 2, transmit_bytes_per_sec: rate } }, disks: [{ name: 'vda', mount_point: '/', used_bytes: GiB * 16, total_bytes: GiB * 64, read_bytes_per_sec: null, write_bytes_per_sec: 0 }], gpus: [] })
    let entries = [
      ['东京 · 主节点', 'Ubuntu 24.04', 'x86_64', true, false, now, 'TEST_ONLY'],
      ['新加坡 · 边缘节点', 'Debian 12', 'aarch64', true, false, now, 'TEST_ONLY'],
      ['法兰克福 · 存储节点', 'FreeBSD 14', 'x86_64', true, true, now - 600_000, 'TEST_ONLY'],
      ['本地 · 开发设备', 'macOS', 'aarch64', true, false, null, 'TEST_ONLY'],
      ['伦敦 · 备用节点', 'Alpine Linux', 'x86_64', false, true, now - 3600_000, 'TEST_ONLY'],
      ['等待接入的服务器', undefined, undefined, false, false, null, null],
    ].map(([name, system, arch, online, metrics_stale, metrics_sampled_at, device_public_key], index) => ({ id: index + 1, name, device_public_key, static_info: { hostname: `fixture-${index + 1}`, system, arch, kernel: 'TEST_ONLY', cpu_model: '测试处理器', cpu_cores: 4, memory_total: GiB * 4, disk_total: GiB * 64, virtualization: 'KVM', agent_version: '0.3.0' }, online, metrics_stale, metrics_sampled_at, last_seen: Math.floor((metrics_sampled_at ?? now) / 1000), last_heartbeat_at: Math.floor(now / 1000), manifest_rev: 0, latest_metrics: index === 5 ? {} : metrics(index === 0 ? 0 : index * 12.5, (index < 2 ? index + 1 : 100) * 1024 ** 2) }))
    const samples = Array.from({ length: 120 }, (_, index) => ({ id: `sample-${index}`, sampled_at: now - (120 - index) * 5000, metrics: metrics(index === 44 ? 96 : 15 + Math.sin(index / 6) * 10, (1 + Math.sin(index / 4) * .5) * 1024 ** 2) })).filter((_, index) => index < 55 || index > 68)
    let failure = 0, signedIn = true, historyFailure = false, probeFailure = false, missing = false, reads = 0
    await page.route('**/api/**', async route => {
      const request = route.request(), url = new URL(request.url()), path = url.pathname
      requests.push(path)
      if (request.method() !== 'GET') writes.push(path)
      if (path === '/api/me') { await route.fulfill({ status: signedIn ? 200 : 401, json: signedIn ? {} : { error: '登录已过期' } }); return }
      if (path === '/api/servers') {
        reads++
        await route.fulfill({ status: failure || 200, json: failure ? { error: '测试读取失败' } : entries }); return
      }
      if (/^\/api\/servers\/\d+$/.test(path)) {
        const entry = entries.find(entry => path.endsWith(`/${entry.id}`))
        await route.fulfill({ status: missing || !entry ? 404 : 200, json: missing || !entry ? { error: '服务器不存在' } : entry }); return
      }
      if (path.endsWith('/metrics')) { await route.fulfill({ status: historyFailure ? 403 : 200, json: historyFailure ? { error: '测试历史读取失败' } : samples.filter(sample => sample.sampled_at >= Number(url.searchParams.get('since'))) }); return }
      if (probeFailure && (path.endsWith('/probes') || path.endsWith('/probe-results'))) { await route.fulfill({ status: 403, json: { error: '测试拨测读取失败' } }); return }
      if (path.endsWith('/probes')) { await route.fulfill({ json: [{ id: 'probe-1', name: '测试目标', kind: 'tcp', interval_secs: 10, enabled: true }] }); return }
      if (path.endsWith('/probe-results')) { await route.fulfill({ json: Array.from({ length: 20 }, (_, index) => ({ id: `probe-result-${index}`, probe_id: 'probe-1', sampled_at: now - index * 10_000, latency_ms: index === 0 ? 0 : 20 + index, loss_percent: 0, error: null })) }); return }
      throw Error(`Unexpected API ${path}`)
    })
    await page.goto(origin)
    await page.getByRole('heading', { name: '服务器总览', exact: true }).waitFor()
    await page.locator('.d-card').first().waitFor()
    assert.equal(await page.locator('.d-card').count(), 6)
    const stats = page.locator('.d-overview-item')
    assert.equal(await stats.nth(2).locator('.d-overview-value').innerText(), '3.0 MiB/秒')
    assert.equal(await stats.nth(3).locator('.d-overview-value').innerText(), '6.0 MiB/秒')
    assert.match(await page.locator('.d-card').nth(0).innerText(), /0\.0%/)
    assert.match(await page.locator('.d-card').nth(2).innerText(), /指标已过期/)
    assert.match(await page.locator('.d-card').nth(3).innerText(), /采样时间未知/)
    assert(!await page.locator('.sidebar').count())
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    if (screenshots) await page.screenshot({ path: resolve(screenshots, `display-light-${width}.png`), fullPage: true, animations: 'disabled' })
    await page.getByLabel('搜索服务器', { exact: true }).fill('  deBIan ')
    assert.equal(await page.locator('.d-card').count(), 1)
    await page.getByLabel('搜索服务器', { exact: true }).fill('no-match')
    await page.getByText('没有匹配的服务器', { exact: true }).waitFor()
    await page.getByRole('button', { name: '清除筛选', exact: true }).click()
    await page.getByRole('button', { name: '待接入', exact: true }).click()
    assert.equal(await page.locator('.d-card').count(), 1)
    await page.getByRole('button', { name: '全部', exact: true }).click()
    await page.getByRole('button', { name: '切换深色主题' }).click()
    if (screenshots) await page.screenshot({ path: resolve(screenshots, `display-dark-${width}.png`), fullPage: true, animations: 'disabled' })
    await page.reload()
    await page.locator('.d-card').first().waitFor()
    assert.equal(await page.locator('.server-display').getAttribute('data-theme'), 'dark')
    await page.getByRole('link', { name: '东京 · 主节点，在线，查看详情', exact: true }).click()
    await page.getByRole('heading', { name: /^东京 · 主节点/ }).waitFor()
    await page.locator('.d-chart svg[role="img"]').first().waitFor()
    await page.getByRole('heading', { name: '测试目标', exact: true }).waitFor()
    const chart = page.locator('.d-chart').first()
    const svg = chart.locator('svg[role="img"]')
    await svg.focus(); await page.keyboard.press('End')
    assert.match(await chart.locator('.d-chart-legend').innerText(), /0\.0%/)
    await chart.getByRole('button', { name: '使用率' }).click()
    await chart.getByText('已隐藏全部曲线', { exact: true }).waitFor()
    await chart.getByRole('button', { name: '使用率' }).click()
    const probe = page.locator('.d-probe')
    await probe.locator('svg[role="img"]').focus(); await page.keyboard.press('End')
    assert.match(await probe.locator('.d-chart-legend').innerText(), /0\.0 ms/)
    assert.match(await probe.locator('.d-probe-summary').innerText(), /连接失败率 0\.0%/)
    probeFailure = true
    await page.reload()
    await page.getByText('暂时无法读取拨测配置', { exact: true }).waitFor()
    assert.equal(await page.getByText('尚无已配置的拨测项目', { exact: true }).count(), 0)
    probeFailure = false
    await page.locator('.d-probes').getByRole('button', { name: '重试', exact: true }).click()
    await page.getByRole('heading', { name: '测试目标', exact: true }).waitFor()
    await page.getByRole('group', { name: '资源时间范围' }).getByRole('button', { name: '2 小时' }).click()
    await page.getByText('正在读取历史采样…', { exact: true }).waitFor({ state: 'hidden' })
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    const scale = await page.locator('.d-chart svg[role="img"]').first().evaluate(element => element.getBoundingClientRect().width / element.viewBox.baseVal.width)
    assert(scale > .95 && scale < 1.05, 'Chart labels must retain their readable font size on mobile')
    if (screenshots) await page.screenshot({ path: resolve(screenshots, `detail-dark-${width}.png`), fullPage: true, animations: 'disabled' })
    await page.getByRole('button', { name: '切换浅色主题' }).click()
    if (screenshots) await page.screenshot({ path: resolve(screenshots, `detail-light-${width}.png`), fullPage: true, animations: 'disabled' })
    await page.getByRole('link', { name: '返回服务器总览', exact: true }).click()
    await page.getByRole('link', { name: '进入后台', exact: true }).click()
    await page.locator('.sidebar').waitFor()
    assert.equal(await page.locator('.server-display').count(), 0)
    assert.equal(await page.evaluate(() => document.body.classList.contains('has-server-display')), false)
    assert.equal(await page.locator('.sidebar').evaluate(element => getComputedStyle(element).backgroundColor), 'rgb(251, 252, 249)')
    await page.getByRole('link', { name: '展示首页', exact: true }).click()
    await page.locator('.d-card').first().waitFor()
    failure = 403
    await page.getByRole('button', { name: '刷新服务器', exact: true }).click()
    await page.getByRole('alert').waitFor()
    assert.equal(await page.locator('.d-card').count(), 6)
    assert.equal(await stats.nth(2).locator('.d-overview-value').innerText(), '—')
    assert.match(await page.locator('.d-card').first().innerText(), /状态未知/)
    failure = 0
    await page.getByRole('button', { name: '重试', exact: true }).click()
    await page.getByRole('alert').waitFor({ state: 'hidden' })
    assert.equal(await stats.nth(2).locator('.d-overview-value').innerText(), '3.0 MiB/秒')
    await page.getByRole('link', { name: '法兰克福 · 存储节点，在线，查看详情', exact: true }).click()
    await page.getByText('指标已过期；在线心跳不代表指标仍在采集。', { exact: false }).waitFor()
    assert.equal(await page.locator('.d-live-strip > div').nth(2).locator('strong').innerText(), '—')
    historyFailure = true
    await page.getByRole('group', { name: '资源时间范围' }).getByRole('button', { name: '1 小时' }).click()
    await page.getByRole('alert').waitFor()
    assert.match(await page.getByRole('alert').innerText(), /历史采样刷新失败/)
    historyFailure = false
    missing = true
    await page.goto(`${origin}/#/overview/999`)
    await page.getByRole('alert').waitFor()
    assert.equal(await page.locator('.d-detail-hero').count(), 0)
    missing = false
    const savedEntries = entries
    entries = []
    await page.goto(`${origin}/#/overview`)
    await page.getByText('还没有服务器', { exact: true }).waitFor()
    entries = savedEntries
    failure = 401
    await page.getByRole('button', { name: '刷新服务器', exact: true }).click()
    await page.getByRole('heading', { name: '欢迎回来', exact: true }).waitFor()
    assert.equal(await page.locator('.server-display').count(), 0)
    signedIn = false
    const before = reads
    await page.reload()
    await page.getByRole('heading', { name: '欢迎回来', exact: true }).waitFor()
    assert.equal(reads, before)
    assert.deepEqual(writes, [])
    assert.deepEqual(errors, [])
    results.push({ width, browserErrors: errors.length, writes: writes.length, checked: ['real-zero', 'missing-data', 'stale-rates-excluded', 'search', 'status-filter', 'theme-persistence', 'history-ranges', 'keyboard-chart', 'zero-latency', 'admin-unchanged', '403-recovery', '404', 'empty', '401-login', 'no-overflow'] })
    await context.close()
  }
  console.log(JSON.stringify(results))
} finally {
  await browser.close()
  await new Promise(resolve => server.close(resolve))
}
