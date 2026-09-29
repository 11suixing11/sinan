import { Badge, CopyButton, Empty, ErrorNotice, Icon, Loading, PageHeader, Refresh } from '../components'
import { bytes } from '../format'
import { useResource } from '../hooks'
import type { Artifact } from '../types'

export default function Artifacts() {
  const resource = useResource<Artifact[]>('/api/artifacts', 15000)
  return <>
    <PageHeader eyebrow="分发管理" title="制品" description="设备通过面板获取经过校验的 Agent 与代理运行时。"><Refresh onClick={resource.reload} /></PageHeader>
    <div className="notice quiet-notice"><Icon name="lock" size={19} /><div><strong>校验通过后才可下载</strong><p>本页列出校验文件与 SHA-256 摘要一致的制品。上传后点击刷新，检查版本与设备架构。</p></div></div>
    <ErrorNotice message={resource.error} retry={resource.reload} />
    <section className="panel"><div className="panel-heading"><h2>可用制品 <span className="count">{resource.data?.length ?? 0}</span></h2><Badge tone="good">SHA-256 校验</Badge></div>{resource.loading && !resource.data ? <Loading /> : !resource.data?.length ? <Empty icon="box" title="还没有可用制品" description="把 Agent 二进制和代理运行时压缩包放入面板的数据目录，并添加 SHA256SUMS 校验文件。" /> : <div className="table-wrap"><table><thead><tr><th>制品</th><th>版本</th><th>架构</th><th>大小</th><th>摘要</th></tr></thead><tbody>{resource.data.map(item => <tr key={`${item.name}/${item.version}/${item.arch}`}><td><div className="entity"><span className="entity-icon"><Icon name={item.name === 'agent' ? 'server' : 'box'} size={18} /></span><div><strong>{item.name === 'agent' ? '设备 Agent' : '代理运行时'}</strong><small>{item.name}</small></div></div></td><td><code>{item.version}</code></td><td><Badge>{item.arch}</Badge></td><td>{bytes(item.bytes)}</td><td><div className="checksum"><code title={item.sha256}>{item.sha256.slice(0, 12)}…</code><CopyButton text={item.sha256} label="复制摘要" /></div></td></tr>)}</tbody></table></div>}</section>
    <section className="panel"><div className="panel-heading"><h2>如何添加制品</h2></div><div className="panel-body artifact-guide"><p>在面板的数据目录中，按照制品名称、版本、架构保存文件：</p><pre>{'artifacts/\n  agent/\n    <版本>/\n      amd64\n      arm64\n      SHA256SUMS\n  sing-box/\n    1.14.2/\n      amd64\n      arm64\n      SHA256SUMS'}</pre><p className="helper">Agent 是可执行二进制；代理运行时为包含可执行文件的 tar.gz 压缩包，需启用官方 v2ray API 构建选项。运行时固定为 1.14.2。amd64 和 arm64 文件只需提供实际使用的架构。</p></div></section>
  </>
}
