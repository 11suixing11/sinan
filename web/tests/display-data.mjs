// Isolated browser fixtures for live visibility, aggregate history and real FX data.
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { extname, resolve, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.txt': 'text/plain' }
const host = createServer(async (request, response) => {
  const path = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(root, path === '/' ? 'index.html' : `.${path}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) return response.writeHead(400).end()
  try { response.writeHead(200, { 'Content-Type': mime[extname(file)] ?? 'application/octet-stream' }).end(await readFile(file)) }
  catch { response.writeHead(404).end() }
})
await new Promise((done, reject) => { host.once('error', reject); host.listen(0, '127.0.0.1', done) })
const origin = `http://127.0.0.1:${host.address().port}`
const browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
const screenshots = process.env.SINAN_UI_SCREENSHOT_DIR
if (screenshots) await mkdir(screenshots, { recursive: true })
const settle = () => new Promise(done => setTimeout(done, 100))
const results = []
let activePage

try {
  for (const width of [1440, 390, 320]) {
    const context = await browser.newContext({ viewport: { width, height: 1000 }, colorScheme: 'light' })
    const page = await context.newPage(), errors = [], calls = [], writes = []
    activePage = page
    page.on('pageerror', error => errors.push(error.message))
    await page.clock.install()
    let now = Date.now(), signedIn = true, publicDashboard = true, refreshFailure = true, historyDenied = false
    let holdWindow = '', heldHistory, holdLive = false, heldLive, holdDetail = false, heldDetail
    let probeAuthorized = true, probeError = false, probesDenied = false, probeHistoryDenied = false
    const visible = new Set([1, 2, 3]), GiB = 1024 ** 3
    const rates = { base: 'CNY', rates: { CNY: 1, USD: .125, EUR: .1 }, rate_dates: { USD: '2026-09-30', EUR: '2026-09-29' }, rate_date: '2026-09-29', source: 'frankfurter', source_url: 'https://frankfurter.dev/', fetched_at: Math.floor(now / 1000) - 86400, attempted_at: Math.floor(now / 1000), next_refresh_at: Math.floor(now / 1000) + 3600, stale: true, status: 'stale', error_code: 'fetch_failed' }
    const metrics = id => ({ cpu_percent: id === 1 ? 0 : 72, memory_used: GiB, disk_used: 8 * GiB, swap_used: 0, swap_total: 0, load_1: .5, load_5: .3, load_15: .2, uptime_secs: 86400, network_interfaces: { eth0: { transmitted_bytes: 123456, received_bytes: 654321, transmit_bytes_per_sec: id === 1 ? 0 : 1024, receive_bytes_per_sec: 2048 } }, disks: [{ name: 'vda', mount_point: '/', read_bytes_per_sec: 4096, write_bytes_per_sec: 2048 }] })
    const entry = id => ({ id, name: ['东京 · 测试入口', '法兰克福 · 测试存储', '伦敦 · 未报价币种'][id - 1], public_view: !signedIn, registered: true, device_public_key: signedIn ? 'TEST_ONLY_DEVICE' : undefined,
      served_at: now, online: true, metrics_stale: false, metrics_sampled_at: now - 1000, metrics_received_at: now - 500, metrics_persisted_at: now - 60_000, last_seen: Math.floor(now / 1000), last_heartbeat_at: Math.floor(now / 1000), manifest_rev: 0,
      static_info: { system: 'Debian 12', arch: 'aarch64', hostname: signedIn ? 'TEST_ONLY_PRIVATE_HOST' : undefined, cpu_cores: 4, memory_total: 4 * GiB, disk_total: 64 * GiB }, latest_metrics: metrics(id),
      agent_settings: { sample_interval_secs: 1, upload_interval_secs: 3 }, telemetry_settings: { persist_interval_secs: 60 },
      asset_settings: { region: ['JP', 'DE', 'GB'][id - 1], group_name: '测试分组', tags: ['回环夹具'], hidden: false, price: signedIn ? ['10', '20', '5'][id - 1] : undefined, currency: signedIn ? ['USD', 'EUR', 'GBP'][id - 1] : undefined, billing_cycle: signedIn ? 30 : undefined, expires_at: signedIn ? Math.floor(now / 1000) + 15 * 86400 : undefined, auto_renewal: false, traffic_limit: String(100 * GiB), traffic_limit_type: 'sum', reset_day: 1, network_interface: signedIn ? 'eth0' : undefined },
      traffic: { cycle_start: Math.floor(now / 1000) - 86400, cycle_end: Math.floor(now / 1000) + 29 * 86400, uploaded: String(10 * GiB), downloaded: String(20 * GiB), used: String(30 * GiB), limit: String(100 * GiB), remaining: String(70 * GiB), percent: 30, exceeded: false, observed_from: now - 86400000, last_sample_at: now, incomplete: true, corrected: false },
    })
    const live = () => ({ served_at: now, public_view: !signedIn, servers: [...visible].map(id => {
      const row = entry(id)
      return Object.fromEntries(['id', 'online', 'last_seen', 'last_heartbeat_at', 'metrics_stale', 'metrics_sampled_at', 'metrics_received_at', 'metrics_persisted_at', 'latest_metrics'].map(key => [key, row[key]]))
    }) })
    const probe = () => ({ id: 'fixture-probe', name: '授权拨测夹具', kind: 'tcp', target: signedIn ? 'private-probe.example.invalid' : '', port: signedIn ? 443 : null, interval_secs: 15, carrier: 'telecom', enabled: true,
      execution_authorized: probeAuthorized, monitor: { region: '测试地区', address_family: 'ipv4', authorization: signedIn ? { kind: 'owned', source: 'TEST_ONLY_PRIVATE_AUTH_SOURCE', scope: 'TEST_ONLY_PRIVATE_AUTH_SCOPE', enabled: probeAuthorized, expires_at: null,
        identity: { kind: 'tcp', target: 'private-probe.example.invalid', port: 443, address_family: 'ipv4' } } : null } })
    const probeResults = () => [{ id: 'fixture-old', probe_id: 'fixture-probe', sampled_at: now - 11_000, latency_ms: 12, loss_percent: 25, error: null, address_family: 'ipv4' },
      { id: 'fixture-latest', probe_id: 'fixture-probe', sampled_at: now - 1000, latency_ms: probeError ? null : 0, loss_percent: probeError ? 100 : 0, error: probeError ? 'TEST_ONLY_UNAVAILABLE' : null, address_family: 'ipv4' }]
    const history = (id, window) => {
      const durations = { '15m': 900000, '1h': 3600000, '2h': 7200000, '24h': 86400000, '7d': 604800000, '30d': 2592000000 }
      const bucketMs = { '15m': 2000, '1h': 5000, '2h': 10000, '24h': 120000, '7d': 900000, '30d': 3600000 }[window]
      const start = Math.floor((now - durations[window] + bucketMs * 2) / bucketMs) * bucketMs
      return { window, from: now - durations[window], to: now, bucket_ms: bucketMs, retention_days: 30, points: [0, 1, 2, 4, 6].map((step, index) => ({ bucket_at: start + step * bucketMs, sample_count: 20, first_sampled_at: start + step * bucketMs + 100, last_sampled_at: start + (step + 1) * bucketMs - 100, partial: index === 0,
        metrics: index === 2 ? {} : { cpu_percent: { count: 17, avg: id === 2 ? 77 : 25, min: 0, max: 99 }, memory_used: { count: 20, avg: GiB, min: GiB / 2, max: 2 * GiB }, disk_used: { count: 20, avg: 8 * GiB, min: 8 * GiB, max: 8 * GiB }, network_transmit_bytes_per_sec: { count: 20, avg: 1024, min: 0, max: 2048 } },
        network_counters: { '网卡 1': { sampled_at: start + (step + 1) * bucketMs - 100, received_bytes: '18446744073709551615', transmitted_bytes: null } },
      })) }
    }
    await page.route('**/api/**', async route => {
      const request = route.request(), url = new URL(request.url()), path = url.pathname
      calls.push(path + url.search)
      if (request.method() !== 'GET') writes.push(path)
      const respond = (json, status = 200) => route.fulfill({ status, json }).catch(() => {})
      if (path === '/api/dashboard/access') return respond({ authenticated: signedIn, public_dashboard: publicDashboard })
      if (path === '/api/me') return respond(signedIn ? {} : { error: '请先登录' }, signedIn ? 200 : 401)
      if (path.startsWith('/api/dashboard/') && !signedIn && !publicDashboard) return respond({ error: '公开看板已关闭' }, 401)
      if (path === '/api/dashboard/exchange-rates') return respond(rates)
      if (path === '/api/exchange-rates/refresh' && request.method() === 'POST') {
        if (refreshFailure) return respond({ error: '刷新过于频繁，请稍后重试。' }, 429)
        rates.stale = false; rates.status = 'fresh'; rates.error_code = null
        return respond(rates)
      }
      if (path === '/api/dashboard/servers') return respond([...visible].map(entry))
      if (path === '/api/dashboard/live') {
        const snapshot = live()
        if (holdLive) { holdLive = false; await new Promise(done => { heldLive = done }) }
        return respond(snapshot)
      }
      if (path === '/api/dashboard/probes/overview') return respond([])
      const detail = path.match(/^\/api\/dashboard\/servers\/(\d+)(.*)$/)
      if (detail) {
        const id = Number(detail[1]), endpoint = detail[2]
        if (!visible.has(id)) return respond({ error: '服务器不存在或已隐藏' }, 404)
        if (!endpoint) {
          const value = entry(id)
          if (holdDetail) { holdDetail = false; await new Promise(done => { heldDetail = done }) }
          return respond(value)
        }
        if (endpoint === '/probes') return probesDenied ? respond({ error: '拨测配置权限已撤销' }, 403) : respond(id !== 3 ? [probe()] : [])
        if (endpoint === '/probe-results') return probeHistoryDenied ? respond({ error: '拨测历史权限已撤销' }, 403) : respond(id !== 3 ? probeResults() : [])
        if (endpoint === '/history') {
          if (historyDenied) return respond({ error: '历史读取权限已撤销' }, 403)
          const window = url.searchParams.get('window'), value = history(id, window)
          if (holdWindow === window) { holdWindow = ''; await new Promise(done => { heldHistory = done }) }
          return respond(value)
        }
      }
      throw new Error(`Unexpected API ${request.method()} ${path}`)
    })
    const advance = async milliseconds => {
      for (let left = milliseconds; left > 0;) { const step = Math.min(left, 3000); now += step; await page.clock.runFor(step); await settle(); left -= step }
    }
    const count = path => calls.filter(call => call === path).length
    await page.goto(`${origin}/#/dashboard`)
    await page.locator('.d-card').first().waitFor()
    await page.getByText('实时状态 · 每 3 秒读取', { exact: true }).waitFor()
    await page.getByText('使用上次汇率 · 2026-09-29', { exact: true }).waitFor()
    const costs = page.getByRole('region', { name: '服务器成本总览' })
    assert.match(await costs.innerText(), /CNY\s*280\.00/)
    assert.match(await costs.innerText(), /1 台缺少所需汇率/)
    assert.match(await page.locator('.d-card').first().innerText(), /本期流量额度/)
    assert.match(await page.locator('.d-card').first().innerText(), /本周期剩余约 CNY\s*40\.00/)
    await page.getByLabel('显示币种', { exact: true }).selectOption('USD')
    assert.match(await costs.innerText(), /USD\s*35\.00/)
    await page.getByLabel('显示币种', { exact: true }).selectOption('GBP')
    assert.match(await costs.innerText(), /GBP\s*5\.00/)
    assert.match(await costs.innerText(), /2 台缺少所需汇率/)
    await page.getByLabel('显示币种', { exact: true }).selectOption('CNY')
    await page.getByRole('button', { name: '更新汇率', exact: true }).click()
    await page.getByText('刷新过于频繁，请稍后重试。', { exact: false }).waitFor()
    assert.match(await costs.innerText(), /CNY\s*280\.00/)
    refreshFailure = false
    await page.getByRole('button', { name: '更新汇率', exact: true }).click()
    await page.getByText('每日参考汇率 · 2026-09-29', { exact: true }).waitFor()
    const fullReads = count('/api/dashboard/servers'), liveReads = count('/api/dashboard/live')
    await advance(10_000)
    assert.equal(count('/api/dashboard/servers'), fullReads, 'Live ticks do not re-fetch asset metadata')
    assert(count('/api/dashboard/live') >= liveReads + 3)
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    if (screenshots) await page.screenshot({ path: resolve(screenshots, `display-data-overview-${width}.png`), fullPage: true, animations: 'disabled' })
    await page.locator('.d-card').first().click()
    await page.getByRole('heading', { name: /^东京 · 测试入口/ }).waitFor()
    await page.getByText('每 2 秒 聚合', { exact: false }).waitFor()
    const probes = page.locator('.d-probes'), probeSummary = probes.locator('.d-probe-summary')
    await probeSummary.getByText(/最近采样/).waitFor()
    assert.match(await probeSummary.innerText(), /测试地区 · IPv4 · TCP 连接 · private-probe\.example\.invalid:443/)
    assert.match(await probeSummary.locator('strong').innerText(), /0\.0 ms · 连接失败率 0\.0%/, 'A real zero remains a successful measurement')
    assert.equal(await probes.locator('svg[role="img"]').count(), 2)
    probeAuthorized = false
    await advance(30_000)
    await probeSummary.getByText(/授权已撤销/).waitFor()
    assert.equal(await probeSummary.locator('strong').innerText(), '— · 连接失败率 —')
    assert.equal(await probes.locator('svg[role="img"]').count(), 2, 'Revocation preserves previously authorized history without presenting it as current')
    probeAuthorized = true
    await advance(30_000)
    await probeSummary.getByText(/最近采样/).waitFor()
    probeError = true
    await advance(15_000)
    await probeSummary.getByText(/检测不可用/).waitFor()
    assert.equal(await probeSummary.locator('strong').innerText(), '— · 连接失败率 —', 'An error loss placeholder cannot masquerade as a measured 100%')
    probeHistoryDenied = true
    await advance(15_000)
    await probes.getByText('拨测结果读取权限不可用，已清除历史数据。', { exact: true }).waitFor()
    assert.equal(await probes.locator('svg[role="img"]').count(), 0, 'Permission denial clears the historical curves')
    assert.match(await probeSummary.innerText(), /状态未知/)
    probeHistoryDenied = false; probeError = false
    await probes.getByRole('button', { name: '重试', exact: true }).click()
    await probeSummary.getByText(/最近采样/).waitFor()
    probesDenied = true
    await advance(30_000)
    await probes.getByText('暂时无法读取拨测配置', { exact: true }).waitFor()
    assert.equal(await probes.locator('svg[role="img"]').count(), 0)
    assert.equal(await probes.getByText(/private-probe\.example\.invalid/).count(), 0, 'A denied definition read cannot retain a private target')
    probesDenied = false
    await probes.getByRole('button', { name: '重试', exact: true }).click()
    await probeSummary.getByText(/最近采样/).waitFor()
    assert.match(await page.locator('.d-info-groups').last().innerText(), /采集 \/ 状态上报[\s\S]*1 秒 \/ 3 秒/)
    assert.match(await page.locator('.d-info-groups').last().innerText(), /最近已存采样/)
    const chart = page.getByRole('region', { name: '处理器图表', exact: true })
    assert.equal(await page.locator('.d-resource-charts').count(), 1)
    await chart.locator('svg[role="img"]').focus(); await page.keyboard.press('Home')
    assert.match(await chart.locator('.d-chart-details').innerText(), /最小 0\.0% · 最大 99\.0% · 17 \/ 20 个指标采样/)
    assert.match(await chart.locator('.d-chart-details').innerText(), /采样范围.*聚合窗口.*部分历史/)
    assert.equal(await chart.locator('[data-range="true"]').count(), 1, 'Both missing metrics and wholly absent buckets break the aggregate band')
    await chart.locator('svg[role="img"]').focus(); await page.keyboard.press('End')
    assert.match(await chart.locator('.d-chart-details').innerText(), /实时采样/)
    assert.match(await chart.locator('.d-chart-legend').innerText(), /0\.0%/)
    const windows = page.getByRole('group', { name: '资源时间范围' })
    assert.equal(await windows.getByRole('button').count(), 6)
    await windows.getByRole('button', { name: '2 小时', exact: true }).click()
    await page.getByText('每 10 秒 聚合', { exact: false }).waitFor()
    await windows.getByRole('button', { name: '24 小时', exact: true }).click()
    await page.getByText('每 2 分钟 聚合', { exact: false }).waitFor()
    holdWindow = '7d'
    await windows.getByRole('button', { name: '7 天', exact: true }).click()
    await page.getByText('正在读取历史采样…', { exact: true }).waitFor()
    await settle(); assert(heldHistory)
    await windows.getByRole('button', { name: '30 天', exact: true }).click()
    await page.getByText('每 1 小时 聚合', { exact: false }).waitFor()
    heldHistory(); heldHistory = undefined; await settle()
    assert.match(await page.locator('.d-history-resolution').innerText(), /每 1 小时 聚合/)
    assert.equal(await windows.getByRole('button', { name: '30 天', exact: true }).getAttribute('aria-pressed'), 'true')
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), true)
    if (screenshots) await page.screenshot({ path: resolve(screenshots, `display-data-history-${width}.png`), fullPage: true, animations: 'disabled' })
    historyDenied = true
    await windows.getByRole('button', { name: '1 小时', exact: true }).click()
    await page.getByText('历史读取权限已撤销', { exact: false }).waitFor()
    assert.equal(await page.locator('.d-resource-charts svg[role="img"]').count(), 0)
    historyDenied = false
    await page.locator('.d-resource-charts').getByRole('button', { name: '重试', exact: true }).click()
    await page.getByText('每 5 秒 聚合', { exact: false }).waitFor()
    holdWindow = '15m'
    await windows.getByRole('button', { name: '15 分钟', exact: true }).click()
    await settle(); assert(heldHistory)
    await page.getByRole('link', { name: '返回服务器看板', exact: true }).click()
    await page.getByRole('link', { name: /^法兰克福 · 测试存储，/ }).click()
    await page.getByText('每 2 秒 聚合', { exact: false }).waitFor()
    heldHistory(); heldHistory = undefined; await settle()
    await page.getByRole('region', { name: '处理器图表', exact: true }).locator('svg[role="img"]').focus(); await page.keyboard.press('Home')
    assert.match(await page.getByRole('region', { name: '处理器图表', exact: true }).locator('.d-chart-details').innerText(), /平均 77\.0%/)
    assert.equal(await page.getByRole('heading', { name: /^东京/ }).count(), 0)
    await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' }); document.dispatchEvent(new Event('visibilitychange')) })
    await advance(60_000)
    holdLive = true; holdDetail = true
    await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' }); document.dispatchEvent(new Event('visibilitychange')) })
    await page.getByText('超过 15 秒未收到新的服务器快照，状态待确认。', { exact: false }).waitFor()
    assert.equal(await page.locator('.d-detail-hero .d-status').innerText(), '状态未知')
    assert.equal(await page.locator('.d-live-strip > div').nth(2).locator('strong').innerText(), '—')
    await settle(); assert(heldLive && heldDetail)
    heldLive(); heldDetail(); heldLive = undefined; heldDetail = undefined
    await page.locator('.d-detail-hero .d-status').filter({ hasText: '在线' }).waitFor()
    await page.getByRole('link', { name: '返回服务器看板', exact: true }).click()
    await page.locator('.d-card').first().waitFor()
    holdLive = true
    for (let attempt = 0; attempt < 4 && !heldLive; attempt++) await advance(1500)
    assert(heldLive, 'A live request must be in flight before testing visibility cancellation')
    await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' }); document.dispatchEvent(new Event('visibilitychange')) })
    const hiddenCalls = calls.length
    await advance(60_000)
    assert.equal(calls.length, hiddenCalls, 'Hidden tabs abort and pause every display feed')
    visible.delete(1)
    await page.evaluate(() => { Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'visible' }); document.dispatchEvent(new Event('visibilitychange')) })
    await page.getByRole('link', { name: /^东京 · 测试入口，/ }).waitFor({ state: 'hidden' })
    heldLive(); heldLive = undefined; await settle()
    assert.equal(await page.locator('.d-card').count(), 2, 'A late pre-hide live response cannot restore a removed server')
    await page.getByRole('link', { name: /^法兰克福 · 测试存储，/ }).click()
    await page.getByText('TEST_ONLY_PRIVATE_HOST', { exact: true }).waitFor()
    signedIn = false
    await advance(3000)
    await page.getByText('TEST_ONLY_PRIVATE_HOST', { exact: true }).waitFor({ state: 'hidden' })
    await page.getByRole('heading', { name: /^法兰克福 · 测试存储/ }).waitFor()
    assert.equal(await page.getByLabel('显示币种', { exact: true }).count(), 0)
    assert.equal(await page.getByRole('button', { name: '更新汇率', exact: true }).count(), 0)
    assert(!/EUR|CNY|本周期剩余/.test(await page.locator('.d-detail').innerText()), 'Public fallback clears private asset metadata')
    assert(!/private-probe\.example\.invalid|TEST_ONLY_PRIVATE_AUTH_SOURCE|TEST_ONLY_PRIVATE_AUTH_SCOPE/.test(await page.locator('.d-detail').innerText()), 'Public fallback cannot retain target or authorization provenance')
    assert.equal(writes.length, 2, 'Only the two explicitly requested admin FX refreshes write')
    publicDashboard = false
    await advance(3000)
    await page.getByRole('heading', { name: '欢迎回来', exact: true }).waitFor()
    assert.equal(await page.locator('.server-display').count(), 0)
    assert.deepEqual(errors, [])
    results.push({ width, live_reads: count('/api/dashboard/live'), aggregate_reads: calls.filter(path => path.includes('/history?')).length, scope_cleanup: true, hidden_abort: true, probe_zero_unknown_revocation: true, overflow: false })
    await context.close()
  }
  console.log(JSON.stringify({ ok: true, results }, null, 2))
} catch (error) {
  if (screenshots && activePage) await activePage.screenshot({ path: resolve(screenshots, 'display-data-failure.png'), fullPage: true }).catch(() => {})
  throw error
} finally { await browser.close(); await new Promise(done => host.close(done)) }
