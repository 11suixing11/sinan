import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { readFile, mkdir } from 'node:fs/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { resolve, extname, sep } from 'node:path'

// Shipped dist with controlled API responses; PostgreSQL verifies migration/data.
const catalogView = resources => resources.map(resource => ({ ...resource, original_name: resource.name, tags: [], note: '', sort_order: resource.id, revision: '1'.repeat(64), metadata_revision: 0 }))
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
  for (const width of [1440, 390, 340]) {
    const page = await browser.newPage({viewport:{width,height:1000}}), errors=[], writes=[]
    page.on('pageerror',error=>errors.push(error.message))
    let catalogFail=false, rejectBatch=false
    const managed={id:1,kind:'direct',name:'受管节点',original_name:'受管节点',server_id:1,server_name:'服务器',public_host:'managed.example.com',port:443,protocol:'vless-reality',enabled:true,available:true,role:'direct',entry_node_id:null,tcp:true,udp:true,stage:'direct',reference_count:0,tags:[],note:'',sort_order:0,revision:'managed-1'}
    const catalog=[managed,...Array.from({length:22},(_,i)=>({id:i+1,kind:'external',name:`外部 ${String(i+1).padStart(2,'0')}`,original_name:`来源名称 ${i+1}`,server_id:null,server_name:null,public_host:'provider.example.com',port:443,protocol:i%2?'tuic':'shadowsocks',enabled:true,available:true,role:'external',tcp:true,udp:true,stage:'external',reference_count:0,source_id:1,source_name:'测试来源',version_id:i+1,identity_epoch:1,tags:i===0?['常用']:[],note:'',sort_order:i+1,revision:`external-${i+1}-1`}))]
    let sources=[], previewCalls=0
    const responseSource={id:1,name:'导入测试',kind:'inline',source_host:null,url_configured:false,authorization_configured:false,content_configured:true,settings_revision:1,identity_epoch:1,refresh_interval_seconds:1800,auto_refresh:false,archived:false,current_revision_id:1,last_attempt_at:1,last_success_at:1,last_error:null,supported_count:2,unsupported_count:1,active_job_id:null,dependency_ids:[],traffic:{},changes:{added:2,updated:0,missing:0,unsupported:1}}
    await page.route('**/api/**',async route=>{
      const path=new URL(route.request().url()).pathname,method=route.request().method(),body=method==='GET'?undefined:route.request().postDataJSON()
      let value
      if(path==='/api/me')value={authenticated:true}
      else if(path==='/api/dashboard/access')value={authenticated:true,public_dashboard:false}
      else if(path==='/api/plugins/sing-box/nodes')value=[{...managed,protocol_config:{type:'vless-reality'},sni:'managed.example.com',settings:{public_port:8443}}]
      else if(path==='/api/plugins/sing-box/proxy-resources')value=[managed]
      else if(path==='/api/plugins/sing-box/servers')value=[{id:1,name:'服务器',enabled:true,online:true}]
      else if(path==='/api/plugins/sing-box/usage')value={total:'0',uplink:'0',downlink:'0',by_node:[],by_user:[]}
      else if(path==='/api/plugins/sing-box/node-catalog') {if(catalogFail)return route.fulfill({status:503,json:{error:'测试读取失败'}});value=catalog}
      else if(path==='/api/plugins/sing-box/node-catalog/batch'){
        writes.push({method,body})
        if(rejectBatch)return route.fulfill({status:409,json:{error:'节点已更新，请重新确认'}})
        for(const item of body.items){const node=catalog.find(node=>node.kind===item.kind&&node.id===item.id);assert.equal(item.revision,node.revision);Object.assign(node,item,{revision:`${node.revision}-next`})}
        value=body.items.map(item=>catalog.find(node=>node.kind===item.kind&&node.id===item.id))
      }
      else if(path==='/api/plugins/sing-box/subscription-sources')value=sources
      else if(path==='/api/plugins/sing-box/subscription-source-previews'&&method==='POST'){
        previewCalls++;assert.equal(sources.length,0);assert.equal(body.kind,'inline');assert.ok(body.content.includes('TEST_ONLY'))
        value={id:'preview-1',expires_at:Math.floor(Date.now()/1000)+600,format:'uri',supported_count:2,unsupported_count:1,nodes:[{key:'node-0',name:'导入甲',protocol:'shadowsocks',server:'import.example.com',port:443,supported:true},{key:'node-1',name:'导入乙',protocol:'tuic',server:'import.example.com',port:8443,supported:true},{key:'rejected-0',name:'不支持节点',supported:false,reason:'unsupported_proxy_protocol'}]}
      }
      else if(path.endsWith('/subscription-source-previews/preview-1/commit')){
        assert.equal(previewCalls,1);assert.deepEqual(body.selected,['node-0']);assert.equal(body.name,'导入测试');assert.equal(JSON.stringify(body).includes('TEST_ONLY'),false);writes.push({method,body});sources=[responseSource];value=responseSource
      }
      else if(path==='/api/plugins/sing-box/subscription-sources/1'&&method==='PATCH'){assert.equal('refresh_interval_seconds' in body,false);assert.equal('content' in body,false);Object.assign(responseSource,body);sources=[responseSource];value=responseSource;writes.push({method,body})}
      else if(path==='/api/plugins/sing-box/subscription-sources/1/nodes')value=[{id:23,source_id:1,node_version_id:23,source_revision_id:1,identity_epoch:1,name:'导入甲',protocol:'shadowsocks',server:'import.example.com',port:443,transport:'tcp',tcp:true,udp:true,selectable:true,present:true,identity_unique:true,adopted:true,reason:null}]
      else {errors.push(`Unexpected ${method} ${path}`);return route.fulfill({status:404,json:{}})}
      return route.fulfill({json:structuredClone(value)})
    })
    await page.goto(`http://127.0.0.1:${server.address().port}/#/plugins/sing-box/nodes`)
    const library=page.getByRole('region',{name:'节点库'}),dialog=page.getByRole('dialog')
    const row=name=>width<768?library.locator('.catalog-card').filter({has:page.getByText(name,{exact:true})}):library.getByRole('row').filter({has:page.getByText(name,{exact:true})})
    await row('外部 01').waitFor()
    await row('受管节点').getByText('managed.example.com:8443',{exact:true}).waitFor()
    await library.getByRole('button',{name:'下一页',exact:true}).click();await row('外部 22').waitFor()
    await library.getByRole('button',{name:'上一页',exact:true}).click()
    await library.getByLabel('按类型筛选').selectOption('external');await library.getByLabel('按协议筛选').selectOption('shadowsocks')
    await row('外部 03').getByRole('button',{name:'上移 外部 03',exact:true}).click();await page.waitForTimeout(150)
    assert.equal(catalog.find(node=>node.kind==='external'&&node.id===3).sort_order,1);assert.equal(catalog.find(node=>node.kind==='external'&&node.id===2).sort_order,2);assert.equal(catalog.find(node=>node.kind==='external'&&node.id===1).sort_order,3)
    await library.getByLabel('按协议筛选').selectOption('');await library.getByLabel('按标签筛选').selectOption('常用')
    assert.equal(await library.locator(width<768?'.catalog-card':'.catalog-table tbody tr').count(),1)
    await row('外部 01').getByRole('checkbox').check()
    await library.getByRole('button',{name:'批量改名',exact:true}).click();await dialog.getByLabel('添加文本').fill('香港 ')
    assert.ok((await dialog.innerText()).includes('香港 外部 01'));rejectBatch=true
    await dialog.getByRole('button',{name:'确认保存',exact:true}).click();await dialog.getByText('节点已更新，请重新确认').waitFor()
    assert.equal(await dialog.getByLabel('添加文本').inputValue(),'香港 ')
    rejectBatch=false;await dialog.getByRole('button',{name:'确认保存',exact:true}).click();await dialog.waitFor({state:'hidden'});await row('香港 外部 01').waitFor()
    assert.equal(writes.filter(write=>write.body.items?.[0]?.name).at(-1).body.items[0].kind,'external');assert.equal(managed.name,'受管节点')
    await row('香港 外部 01').getByRole('button',{name:'整理',exact:true}).click()
    await dialog.getByLabel('备注').fill('整理后的备注');await dialog.getByLabel(/^标签/).fill('常用, 高速')
    catalogFail=true;await page.getByRole('button',{name:'刷新',exact:true}).first().evaluate(button=>button.click());await page.getByText('测试读取失败').waitFor()
    const before=writes.length;await dialog.locator('form').evaluate(form=>form.dispatchEvent(new Event('submit',{bubbles:true,cancelable:true})))
    await page.waitForTimeout(100);assert.equal(writes.length,before);assert.equal(await dialog.getByLabel('备注').inputValue(),'整理后的备注')
    catalogFail=false;await page.getByRole('button',{name:'刷新',exact:true}).first().evaluate(button=>button.click());await page.waitForTimeout(150)
    await dialog.getByRole('button',{name:'确认保存',exact:true}).click();await dialog.waitFor({state:'hidden'});await row('香港 外部 01').getByText('整理后的备注').waitFor()
    assert.equal('name' in writes.at(-1).body.items[0],false)
    await page.getByRole('button',{name:'添加来源',exact:true}).first().click();await dialog.getByLabel('来源名称').fill('导入测试');await dialog.getByLabel('来源类型').selectOption('inline');await dialog.getByLabel('配置内容').fill('TEST_ONLY provider data')
    await dialog.getByRole('button',{name:'解析并预览',exact:true}).click();await dialog.getByRole('checkbox',{name:'导入 导入乙'}).uncheck();assert.equal(sources.length,0)
    assert.equal(await dialog.getByRole('checkbox',{name:'导入 不支持节点'}).isDisabled(),true)
    assert.equal(await dialog.locator('textarea').count(),0);await dialog.getByRole('button',{name:'加入节点库（1）',exact:true}).click();await dialog.waitFor({state:'hidden'})
    await page.getByRole('button',{name:'移出节点库',exact:true}).waitFor()
    await page.getByRole('button',{name:'设置与更新',exact:true}).click();await dialog.getByLabel('来源名称').fill('导入测试改名');await dialog.getByRole('button',{name:'保存并解析更新',exact:true}).click();await dialog.waitFor({state:'hidden'});assert.equal(responseSource.refresh_interval_seconds,1800)
    assert.equal(sources.length,1);assert.equal(await page.evaluate(()=>Object.values(localStorage).some(value=>value.includes('TEST_ONLY'))),false)
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth),false,`overflow at ${width}`)
    assert.deepEqual(errors,[])
    const dir=process.env.SINAN_UI_SCREENSHOT_DIR??'/tmp/sinan-node-catalog';await mkdir(dir,{recursive:true});await page.screenshot({path:resolve(dir,`node-catalog-${width}.png`),fullPage:true})
    console.log(`node catalog ${width}: pagination, filters, batch preview/conflict, write guards and selected import passed`)
    await page.close()
  }
} finally {await browser.close();await new Promise(resolve=>server.close(resolve))}
