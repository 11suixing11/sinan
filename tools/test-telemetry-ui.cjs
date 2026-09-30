// Run against a local built preview; all API data is an isolated loopback fixture.
const assert = require('node:assert/strict')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright')

async function main() {
  const origin = new URL(process.argv[2] || 'http://127.0.0.1:4175')
  assert(['127.0.0.1', 'localhost', '[::1]'].includes(origin.hostname))
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROMIUM_PATH || undefined })
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } })
    const errors = []
    page.on('pageerror', error => errors.push(error.message))
    let mode = 'stale'
    const sampled = 1700000000123
    const heartbeat = 1700000600
    await page.route('**/api/**', async route => {
      const path = new URL(route.request().url()).pathname
      let data = []
      if (path === '/api/me') data = {}
      else if (path === '/api/servers/1') data = {
        id: 1, name: '遥测隔离验收夹具', device_public_key: 'TEST_ONLY_device',
        static_info: { hostname: 'fixture-host', memory_total: 1024 ** 3 },
        latest_metrics: { cpu_percent: 7.5, memory_used: 64 * 1024 ** 2, uptime_secs: 42 },
        manifest_rev: 0, capabilities: [], online: mode !== 'offline',
        last_seen: heartbeat + 2, last_heartbeat_at: mode === 'unknown' ? null : heartbeat,
        metrics_sampled_at: mode === 'unknown' ? null : sampled,
        metrics_stale: mode === 'stale' || mode === 'offline',
      }
      else if (path.endsWith('/deployments')) data = { status: null, history: [] }
      else if (path.endsWith('/node-quality')) data = { ip_addresses: [], quality: [], plugin_ready: false, plugin_reason: null, reports: [] }
      else if (path.endsWith('/agent-settings')) data = { sample_interval_secs: 1, upload_interval_secs: 3, auto_update: false, discover_public_ips: false }
      await route.fulfill({ json: data })
    })
    await page.goto(`${origin}/#/servers/1`)
    const row = label => page.locator('.info-grid div').filter({ has: page.locator('dt', { hasText: label }) }).locator('dd')
    await page.getByText('指标过期', { exact: true }).waitFor()
    assert.equal(await page.getByText('在线', { exact: true }).count(), 1)
    assert.equal(await page.getByText('7.5%', { exact: true }).count(), 1)
    const expected = await page.evaluate(({ sampled, heartbeat }) => ({
      sample: new Date(sampled).toLocaleString('zh-CN', { hour12: false }),
      heartbeat: new Date(heartbeat * 1000).toLocaleString('zh-CN', { hour12: false }),
    }), { sampled, heartbeat })
    assert.equal(await row('最后指标').innerText(), expected.sample)
    assert.equal(await row('最后心跳').innerText(), expected.heartbeat)
    assert.notEqual(await row('最近设备消息').innerText(), expected.heartbeat)
    assert((await page.getByText('指标过期，以下保留的是最后一次采集的历史数据。', { exact: false }).innerText()).includes('心跳独立更新'))
    mode = 'fresh'
    await page.reload()
    await row('最后指标').waitFor()
    assert.equal(await page.getByText('指标过期', { exact: true }).count(), 0)
    assert.equal(await page.getByText('7.5%', { exact: true }).count(), 1)
    mode = 'offline'
    await page.reload()
    await page.getByText('离线', { exact: true }).waitFor()
    await page.getByText('指标过期', { exact: true }).waitFor()
    assert.equal(await page.getByText('7.5%', { exact: true }).count(), 1)
    mode = 'unknown'
    await page.reload()
    await page.getByText('尚未记录', { exact: true }).waitFor()
    assert.equal(await row('最后指标').innerText(), '时间未知')
    assert.equal(await page.getByText('指标过期', { exact: true }).count(), 0)
    assert.deepEqual(errors, [])
    console.log(JSON.stringify({ passed: ['online-stale-history', 'milliseconds-conversion', 'separate-heartbeat-message', 'fresh', 'offline-history', 'unknown-legacy-times'], browser_errors: 0 }))
  } finally { await browser.close() }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
