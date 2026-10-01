import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { extname, resolve, sep } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const { chromium } = await import(process.env.SINAN_PLAYWRIGHT_MODULE ? pathToFileURL(process.env.SINAN_PLAYWRIGHT_MODULE).href : 'playwright')
const root = fileURLToPath(new URL('../dist/', import.meta.url))
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url, 'http://127.0.0.1').pathname
  const file = resolve(root, pathname === '/' ? 'index.html' : `.${pathname}`)
  if (!file.startsWith(root.endsWith(sep) ? root : `${root}${sep}`)) { response.writeHead(400).end(); return }
  try { response.writeHead(200, { 'Content-Type': ({ '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.svg': 'image/svg+xml' })[extname(file)] ?? 'application/octet-stream' }); response.end(await readFile(file)) }
  catch { response.writeHead(404).end() }
})
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
const origin = `http://127.0.0.1:${server.address().port}`
const browser = await chromium.launch({ headless: true, ...(process.env.SINAN_CHROME_PATH ? { executablePath: process.env.SINAN_CHROME_PATH } : {}) })
const screenshots = process.env.SINAN_UI_SCREENSHOT_DIR
if (screenshots) await mkdir(screenshots, { recursive: true })
const results = []

try {
  for (const width of [1440, 390]) {
    const context = await browser.newContext({ viewport: { width, height: width > 800 ? 1000 : 844 } })
    const page = await context.newPage(), errors = [], unexpected = [], imports = []
    let inventory = [], failImport = false
    page.on('pageerror', error => errors.push(error.message))
    await page.route('**/api/**', async route => {
      const request = route.request(), path = new URL(request.url()).pathname
      const fulfill = (json, status = 200) => route.fulfill({ status, json })
      if (path === '/api/dashboard/access') return fulfill({ authenticated: true, public_dashboard: false })
      if (path === '/api/me') return fulfill({ id: 1 })
      if (path === '/api/artifacts' && request.method() === 'GET') return fulfill(inventory)
      if (path === '/api/artifacts/targets') return fulfill({ default_targets: ['linux-gnu-arm64', 'linux-musl-arm64'], supported_targets: ['amd64', 'arm64', 'linux-gnu-amd64', 'linux-gnu-arm64', 'linux-musl-amd64', 'linux-musl-arm64', 'macos-arm64', 'freebsd-amd64', 'freebsd-arm64', 'windows-amd64', 'windows-arm64'] })
      if (path === '/api/artifacts/import-release' && request.method() === 'POST') {
        const body = request.postDataJSON(); imports.push(body)
        if (failImport) return fulfill({ error: '测试：制品导入失败，请重试' }, 409)
        inventory = (body.targets ?? ['linux-gnu-arm64', 'linux-musl-arm64']).map(arch => ({ name: 'agent', arch, version: body.tag.slice(7), sha256: 'a'.repeat(64), bytes: 12345678 }))
        return fulfill({ imported: inventory.length })
      }
      unexpected.push(`${request.method()} ${path}`)
      return fulfill({ error: 'Unexpected fixture request' }, 500)
    })
    await page.goto(`${origin}/#/artifacts`)
    const target = page.getByLabel('目标架构', { exact: false })
    await target.waitFor()
    await page.getByText(/当前匹配：Linux ARM64（GNU \/ glibc）、Linux ARM64（musl \/ Alpine）/).waitFor()
    assert.equal(await target.inputValue(), '', 'The default import automatically matches server targets')
    await page.getByLabel('发布标签').fill('agent-v0.3.0')
    await page.getByRole('button', { name: '导入制品', exact: true }).click()
    await page.getByRole('cell', { name: 'linux-gnu-arm64', exact: true }).waitFor()
    assert.deepEqual(imports[0], { tag: 'agent-v0.3.0' }, 'Automatic import delegates architecture selection to the panel')
    assert.equal(await page.getByRole('cell', { name: /amd64/ }).count(), 0, 'An ARM inventory shows no AMD artifacts')
    await target.selectOption('linux-musl-arm64')
    await page.getByText('只导入 Linux ARM64（musl / Alpine） 所需的制品。', { exact: true }).waitFor()
    await page.getByLabel('发布标签').fill('agent-v0.3.1')
    failImport = true
    await page.getByRole('button', { name: '导入制品', exact: true }).click()
    await page.getByRole('alert').filter({ hasText: '测试：制品导入失败' }).waitFor()
    assert.equal(await page.getByLabel('发布标签').inputValue(), 'agent-v0.3.1', 'Failed import preserves the release tag')
    assert.equal(await target.inputValue(), 'linux-musl-arm64', 'Failed import preserves the chosen target')
    failImport = false
    await page.getByRole('button', { name: '导入制品', exact: true }).click()
    await page.getByRole('cell', { name: '0.3.1', exact: true }).waitFor()
    assert.deepEqual(imports.at(-1), { tag: 'agent-v0.3.1', targets: ['linux-musl-arm64'] }, 'Manual selection sends only the selected architecture')
    assert.equal(await page.getByRole('cell', { name: 'linux-gnu-arm64', exact: true }).count(), 0)
    await target.selectOption('windows-amd64')
    await page.getByLabel('发布标签').fill('agent-v0.3.2')
    await page.getByRole('button', { name: '导入制品', exact: true }).click()
    await page.getByRole('cell', { name: 'windows-amd64', exact: true }).waitFor()
    assert.deepEqual(imports.at(-1), { tag: 'agent-v0.3.2', targets: ['windows-amd64'] }, 'A server with a different OS can explicitly request its own target')
    assert.equal(await page.locator('main').evaluate(element => element.scrollWidth > element.clientWidth + 1), false, `Artifacts page overflows horizontally at ${width}px`)
    if (screenshots) await page.screenshot({ animations: 'disabled', path: resolve(screenshots, `artifacts-${width}.png`) })
    assert.deepEqual(unexpected, [])
    assert.deepEqual(errors, [])
    results.push({ width, automaticArm: 'passed', explicitTarget: 'passed', recovery: 'passed' })
    await context.close()
  }
  console.log(JSON.stringify(results, null, 2))
} finally {
  await browser.close()
  await new Promise(resolve => server.close(resolve))
}
