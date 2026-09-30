import { useState } from 'react'
import { api } from '../api'
import { Badge, CopyButton, Empty, ErrorNotice, Icon, Loading, PageHeader, Refresh } from '../components'
import { bytes } from '../format'
import { useAction, useResource } from '../hooks'
import type { Artifact } from '../types'

export default function Artifacts() {
  const resource = useResource<Artifact[]>('/api/artifacts', 15000)
  const action = useAction()
  const [tag, setTag] = useState('')
  return <>
    <PageHeader eyebrow="分发管理" title="制品" description="设备通过面板获取经过校验的 Agent、代理运行时与节点诊断插件。"><Refresh onClick={resource.reload} /></PageHeader>
    <div className="notice quiet-notice"><Icon name="lock" size={19} /><div><strong>发布签名通过后才可使用</strong><p>面板与 Agent 分别验证发布签名和制品内容。相同版本的不同内容不能覆盖现有制品。</p></div></div>
    <section className="panel"><div className="panel-heading"><h2>从 Release 导入制品</h2></div><div className="panel-body"><form onSubmit={event => { event.preventDefault(); void action.run(() => api('/api/artifacts/import-release', 'POST', { tag }), () => { setTag(''); resource.reload() }) }}><label className="field"><span>发布标签</span><input value={tag} onChange={event => setTag(event.target.value)} required pattern="agent-v[0-9A-Za-z.-]+" placeholder="agent-v0.3.0" disabled={action.busy} /></label><ErrorNotice message={action.error} /><button className="button button-primary" disabled={action.busy}>{action.busy ? '正在验证并导入…' : '导入制品'}</button></form><p className="helper">选择官方仓库中已发布、已签名的版本。草稿和缺少签名的版本暂不可导入。</p></div></section>
    <ErrorNotice message={resource.error} retry={resource.reload} />
    <section className="panel"><div className="panel-heading"><h2>可用制品 <span className="count">{resource.data?.length ?? 0}</span></h2>{!!resource.data?.length && <Badge tone="good">发布签名已验证</Badge>}</div>{resource.loading && !resource.data ? <Loading /> : !resource.data?.length ? <Empty icon="box" title="还没有可用制品" description="输入已签名的发布标签，导入设备所需的版本与架构。" /> : <div className="table-wrap"><table><thead><tr><th>制品</th><th>版本</th><th>架构</th><th>大小</th><th>摘要</th></tr></thead><tbody>{resource.data.map(item => <tr key={`${item.name}/${item.version}/${item.arch}`}><td><div className="entity"><span className="entity-icon"><Icon name={item.name === 'agent' ? 'server' : 'box'} size={18} /></span><div><strong>{item.name === 'agent' ? '设备 Agent' : item.name === 'nodequality' ? '节点诊断插件' : '代理运行时'}</strong><small>{item.name}</small></div></div></td><td><code>{item.version}</code></td><td><Badge>{item.arch}</Badge></td><td>{bytes(item.bytes)}</td><td><div className="checksum"><code title={item.sha256}>{item.sha256.slice(0, 12)}…</code><CopyButton text={item.sha256} label="复制摘要" /></div></td></tr>)}</tbody></table></div>}</section>
    <div className="page-footnote"><Icon name="lock" size={14} />自建签名版本、离线导入与公钥轮换步骤见<a href="https://github.com/theLucius7/sinan/blob/main/docs/deploy.md" target="_blank" rel="noreferrer">部署文档</a>。Agent 的信任公钥在构建时固定。</div>
  </>
}
