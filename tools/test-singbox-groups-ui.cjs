// Exercise the built plugin UI in Chromium against isolated, stateful API fixtures.
const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const path = require('node:path')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright')

async function main() {
  const origin = new URL(process.argv[2] || 'http://127.0.0.1:4176')
  assert(['127.0.0.1', 'localhost', '[::1]'].includes(origin.hostname))
  const browser = await chromium.launch({ headless: true, executablePath: process.env.CHROMIUM_PATH || undefined })
  try {
    const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } })
    const errors = [], writes = [], assignments = []
    const root = '/api/plugins/sing-box'
    const nodes = [{ id: 1, name: '标准节点', server_id: 1 }, { id: 2, name: '入口节点', server_id: 1 }, { id: 3, name: '出口节点', server_id: 2 }, { id: 4, name: '备用入口', server_id: 1 }].map(n => ({ ...n, port: 20000 + n.id, protocol: 'vless-reality', public_host: 'proxy.example.com', sni: 'www.example.com' }))
    const chains = [{ id: 1, name: '两跳示例', entry_node_id: 2, exit_node_id: 3, available: true }]
    const policies = [{ id: 1, name: '常用节点', node_ids: [1], chain_ids: [1], member_count: 1 }]
    const plans = [{ id: 1, name: '月度套餐', monthly_bytes: '536870912000', reset_day: 31, reset_hour: 12, reset_minute: 30, timezone: 'Asia/Taipei', duration_days: 365 }]
    let groupIds = [1], assigned = false, exhausted = false
    page.on('pageerror', error => errors.push(error.message))
    await page.route('**/api/**', async route => {
      const request = route.request(), pathname = new URL(request.url()).pathname, method = request.method()
      const payload = method === 'GET' ? null : request.postDataJSON()
      if (method !== 'GET') writes.push({ pathname, method, payload })
      let data = []
      if (pathname === '/api/me') data = {}
      else if (pathname === `${root}/nodes`) data = nodes
      else if (pathname === `${root}/chains`) {
        if (method === 'POST') { data = { ...payload, id: chains.length + 1, available: true }; chains.push(data) }
        else data = chains
      } else if (pathname === `${root}/policy-groups`) {
        if (method === 'POST') { data = { ...payload, id: policies.length + 1, member_count: 0 }; policies.push(data) }
        else data = policies
      } else if (pathname === `${root}/package-groups`) {
        if (method === 'POST') { data = { ...payload, id: plans.length + 1 }; plans.push(data) }
        else data = plans
      } else if (pathname === `${root}/users`) data = [{ id: 1, name: '测试用户', subscription_token: 'TEST_ONLY', subscription_url: 'https://panel.example.com/s/TEST_ONLY' }]
      else if (pathname === `${root}/usage`) data = { uplink: '40', downlink: '60', total: '100', by_user: [{ user_id: 1, name: '测试用户', deleted: false, uplink: '40', downlink: '60' }], by_node: [] }
      else if (pathname === `${root}/users/1/accesses`) data = groupIds.includes(1) ? [{ user_id: 1, node_id: 1, uuid: 'TEST_ONLY', stat_name: 'u1_n1', direct_grant: false }, { user_id: 1, node_id: 2, uuid: 'TEST_ONLY_CHAIN', stat_name: 'u1_n2', direct_grant: false }] : []
      else if (pathname === `${root}/users/1/policy-groups`) {
        if (method === 'PUT') groupIds = payload.group_ids
        data = { group_ids: groupIds }
      } else if (pathname === `${root}/users/1/entitlement`) data = {
        user_id: 1, package_group_id: assigned ? 1 : null, package_name: assigned ? '月度套餐' : null,
        monthly_bytes: assigned ? '536870912000' : null, reset_day: 31, reset_hour: 12, reset_minute: 30, timezone: 'Asia/Taipei',
        starts_at: 1790812800, expires_at: 1822348800, cycle_start: 1790742600, next_reset: 1793421000,
        used_bytes: exhausted ? '536870912000' : '100', status: assigned ? exhausted ? 'exhausted' : 'active' : 'unmetered', allowed: !exhausted,
      }
      else if (pathname === `${root}/users/1/package`) {
        assignments.push(payload)
        if (assignments.length === 1) { await route.fulfill({ status: 503, json: { error: '测试：响应丢失，请重试' } }); return }
        assigned = true; data = { id: 1, user_id: 1, package_group_id: 1, starts_at: 1790812800, expires_at: 1822348800 }
      }
      await route.fulfill({ json: data })
    })
    await page.goto(`${origin}/#/plugins/sing-box/groups`)
    await page.getByRole('button', { name: '创建策略组', exact: true }).click()
    await page.getByRole('textbox', { name: '名称', exact: true }).fill('测试策略')
    await page.getByRole('checkbox', { name: /标准节点/ }).check()
    await page.getByRole('checkbox', { name: /两跳示例/ }).check()
    assert.equal(await page.locator('input[name="node_ids"][value="2"]').count(), 0)
    await page.getByRole('button', { name: '保存', exact: true }).click()
    await page.getByText('测试策略', { exact: true }).waitFor()
    assert.deepEqual(writes.at(-1).payload, { name: '测试策略', node_ids: [1], chain_ids: [1] })
    await page.getByRole('button', { name: '套餐组', exact: true }).click()
    await page.getByRole('button', { name: '创建套餐组', exact: true }).click()
    await page.getByRole('textbox', { name: '名称', exact: true }).fill('精确额度')
    await page.locator('input[name="amount"]').fill('512')
    await page.locator('input[name="reset_day"]').fill('31')
    await page.locator('input[name="reset_time"]').fill('02:30')
    await page.locator('input[name="timezone"]').fill('Asia/Taipei')
    await page.locator('input[name="duration_days"]').fill('90')
    await page.getByRole('button', { name: '保存', exact: true }).click()
    await page.getByText('精确额度', { exact: true }).waitFor()
    assert.deepEqual(writes.at(-1).payload, { name: '精确额度', monthly_bytes: '549755813888', reset_day: 31, reset_hour: 2, reset_minute: 30, timezone: 'Asia/Taipei', duration_days: 90 })
    await page.getByRole('button', { name: '两跳链路', exact: true }).click()
    await page.getByRole('button', { name: '创建两跳链路', exact: true }).click()
    await page.getByRole('textbox', { name: '名称', exact: true }).fill('测试链路')
    await page.locator('select[name="entry_node_id"]').selectOption('4')
    await page.locator('select[name="exit_node_id"]').selectOption('3')
    await page.getByRole('button', { name: '保存', exact: true }).click()
    await page.getByText('测试链路', { exact: true }).waitFor()
    assert.deepEqual(writes.at(-1).payload, { name: '测试链路', entry_node_id: 4, exit_node_id: 3 })
    await page.goto(`${origin}/#/plugins/sing-box/users`)
    await page.getByRole('heading', { name: '可用范围与套餐', exact: true }).waitFor()
    await page.getByText('来自策略组', { exact: true }).waitFor()
    assert.equal(await page.getByRole('checkbox', { name: '授权 入口节点', exact: true }).count(), 0)
    assert.equal(await page.getByRole('checkbox', { name: '授权 标准节点', exact: true }).isChecked(), false)
    await page.getByRole('checkbox', { name: /常用节点/ }).uncheck()
    await page.getByRole('button', { name: '保存策略组分配', exact: true }).click()
    await page.getByText('策略组分配已保存。单独授权仍保留，设备应用配置后更新可用节点。', { exact: true }).waitFor()
    assert.deepEqual(writes.at(-1).payload, { group_ids: [] })
    await page.getByRole('button', { name: '分配或更换套餐', exact: true }).click()
    await page.locator('select[name="package_group_id"]').selectOption('1')
    await page.getByRole('button', { name: '确认分配', exact: true }).click()
    await page.getByText('测试：响应丢失，请重试', { exact: true }).waitFor()
    await page.getByRole('button', { name: '确认分配', exact: true }).click()
    await page.getByText('套餐已分配，按分配时刻计算有效期。本期历史用量没有清空。', { exact: true }).waitFor()
    assert.equal(assignments.length, 2)
    assert.deepEqual(assignments[0], assignments[1])
    assert.match(assignments[0].request_id, /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/)
    await page.getByText('本期已用 / 每月额度', { exact: true }).waitFor()
    exhausted = true
    await page.reload()
    await page.getByText('本期流量已用完', { exact: true }).waitFor()
    if (process.env.SINAN_GROUPS_SCREENSHOTS) {
      await fs.mkdir(process.env.SINAN_GROUPS_SCREENSHOTS, { recursive: true })
      await page.screenshot({ path: path.join(process.env.SINAN_GROUPS_SCREENSHOTS, 'user-desktop.png'), fullPage: true })
    }
    await page.setViewportSize({ width: 390, height: 844 })
    assert(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth))
    if (process.env.SINAN_GROUPS_SCREENSHOTS) await page.screenshot({ path: path.join(process.env.SINAN_GROUPS_SCREENSHOTS, 'user-mobile.png'), fullPage: true })
    assert.deepEqual(errors, [])
    console.log('PASS: policy/package/chain forms, precise quota, scoped grants, stable assignment retry, expiry display, desktop/mobile Chromium')
  } finally { await browser.close() }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
