import { useState } from 'react'
import { api } from '../api'
import { Badge, ErrorNotice, Icon, Loading } from '../components'
import { time } from '../format'
import { useAction, useResource } from '../hooks'
import type { DiagnosticRecord, IpQuality, NodeQuality as NodeQualityData } from '../types'

function safeReportLink(value?: string) {
  if (!value) return undefined
  try {
    const url = new URL(value)
    if (url.protocol === 'https:' && ['nodequality.com', 'www.nodequality.com'].includes(url.hostname) && !url.username && !url.password && !url.port && !url.hash) return url.href
  } catch { /* Reports remain available as plain text when a link is invalid. */ }
  return undefined
}

const statusLabels = { queued: '等待设备领取', running: '设备正在测试', succeeded: '报告已完成', failed: '报告失败' }
const versionLabels: Record<string, string> = { both: 'IPv4 与 IPv6', ipv4: 'IPv4', ipv6: 'IPv6' }

function QualityResult({ result }: { result: IpQuality }) {
  const successes = result.databases.filter(item => item.status === 'succeeded').length
  const expired = result.expires_at * 1000 <= Date.now()
  return <div className="quality-result">
    <div className="quality-summary"><Badge tone={result.status === 'succeeded' ? 'good' : result.status === 'partial' ? 'warm' : 'bad'}>{successes ? `${successes} / ${result.databases.length} 个数据库有结果` : '质量未知'}</Badge><span className="subtle">查询于 {time(result.checked_at)}{expired ? ' · 缓存已过期' : ' · 缓存有效一天'}</span></div>
    <p className="helper">各数据库的类型、标记和评分独立展示，评分保留上游原值。没有返回的字段表示未知。</p>
    <div className="quality-databases">{result.databases.map(database => <details key={database.database} className="quality-database">
      <summary><span>{database.label}</span><Badge tone={database.status === 'succeeded' ? 'good' : 'neutral'}>{database.status === 'succeeded' ? '已查询' : '未知'}</Badge></summary>
      {database.error ? <p className="quality-database-error">{database.error}</p> : <dl className="detail-list">{database.fields.map(field => <div key={field.label}><dt>{field.label}</dt><dd>{typeof field.value === 'boolean' ? field.value ? '是' : '否' : String(field.value)}</dd></div>)}</dl>}
    </details>)}</div>
  </div>
}

function ReportResult({ record }: { record: DiagnosticRecord }) {
  const copy = useAction()
  const reportUrl = safeReportLink(record.report?.report_url)
  return <article className="quality-report">
    <div className="quality-report-heading"><Badge tone={record.status === 'succeeded' ? 'good' : record.status === 'failed' ? 'bad' : 'warm'}>{statusLabels[record.status]}</Badge><span className="subtle">{time(record.created_at)} · {versionLabels[record.job.options.ip_version] ?? '全部 IP'} · {record.job.options.network_mode === 'low' ? '低流量模式' : '标准流量模式'}{record.job.options.upload_report !== undefined && ` · ${record.job.options.upload_report === 'true' ? '允许公开上传' : '仅本地报告'}`}</span></div>
    {record.status === 'queued' && <p className="helper">任务已保存，等待在线 Agent 领取。通常会在数秒内开始。</p>}
    {record.status === 'running' && <p className="helper">正在服务器本机测试处理器、磁盘、IP 质量与网络。完整测试需要数分钟，页面会自动更新。任务截止 {time(record.expires_at)}。</p>}
    {record.error && <div className="notice notice-error break-all">{record.error}</div>}
    {record.report && <><div className="quality-report-actions">{reportUrl ? <a className="button button-secondary" href={reportUrl} target="_blank" rel="noopener noreferrer">打开公开报告 <Icon name="arrow" size={14} /></a> : <span className="helper">未生成在线链接，本地报告如下。</span>}<button className="button button-secondary" disabled={copy.busy} onClick={() => void copy.run(() => navigator.clipboard.writeText(record.report!.text), () => {})}>复制报告文本</button></div><ErrorNotice message={copy.error} /><details className="quality-report-text"><summary>查看报告文本</summary><pre>{record.report.text}</pre></details></>}
  </article>
}

export default function NodeQuality({ serverId }: { serverId: number }) {
  const resource = useResource<NodeQualityData>(`/api/servers/${serverId}/node-quality`)
  const refresh = useAction()
  const run = useAction()
  const [ipVersion, setIpVersion] = useState('both')
  const [networkMode, setNetworkMode] = useState('low')
  const [uploadReport, setUploadReport] = useState(false)
  const data = resource.data
  const active = data?.reports.some(report => report.status === 'queued' || report.status === 'running')
  return <section className="panel">
    <div className="panel-heading"><h2>IP 质量与节点报告</h2><button className="button button-secondary" disabled={refresh.busy || !data?.ip_addresses.length} onClick={() => void refresh.run(() => api<IpQuality[]>(`/api/servers/${serverId}/node-quality/refresh`, 'POST'), () => resource.reload())}><Icon name="refresh" size={14} />{refresh.busy ? '查询中…' : '刷新 IP 质量'}</button></div>
    <div className="panel-body quality-body"><ErrorNotice message={resource.error || refresh.error || run.error} retry={resource.reload} />
      {!data ? resource.loading && <Loading /> : <>
        <p className="helper">IP 由 Agent 上报，包含设备网卡地址与配置补充的公网出口地址。质量查询会将公网 IP 发送给第三方数据库；缓存仅作参考。</p>
        {!data.ip_addresses.length ? <div className="inline-empty">设备尚未上报 IP 地址，请升级 Agent 或等待设备上报。</div> : <div className="quality-addresses">{data.ip_addresses.map(ip => {
          const result = data.quality.find(item => item.ip === ip)
          return <article key={ip} className="quality-address"><h3 className="mono">{ip}</h3>{result ? <QualityResult result={result} /> : <p className="helper">尚未查询质量。点击“刷新 IP 质量”查询各数据库。</p>}</article>
        })}</div>}
        <div className="quality-runner"><h3>NodeQuality 插件</h3><p className="helper">在当前服务器运行完整测试，包含处理器、磁盘、IP 质量和网络带宽测试，会占用设备资源与流量。默认使用低流量模式并仅保留本地报告；开启公开上传会将节点 IP、硬件和网络测试结果发送到 NodeQuality 生成公开报告。</p>
          {data.plugin_reason && <div className="notice">{data.plugin_reason}</div>}
          <div className="quality-options"><label>测试 IP 版本<select value={ipVersion} onChange={event => setIpVersion(event.target.value)} disabled={run.busy || active}><option value="both">IPv4 与 IPv6</option><option value="ipv4">仅 IPv4</option><option value="ipv6">仅 IPv6</option></select></label><label>网络测试流量<select value={networkMode} onChange={event => setNetworkMode(event.target.value)} disabled={run.busy || active}><option value="low">低流量模式</option><option value="normal">标准流量模式</option></select></label><label>公开报告上传<select value={String(uploadReport)} onChange={event => setUploadReport(event.target.value === 'true')} disabled={run.busy || active}><option value="false">关闭，仅保留本地报告</option><option value="true">上传并生成公开报告</option></select></label><button className="button button-primary" disabled={!data.plugin_ready || run.busy || active} onClick={() => void run.run(() => api<DiagnosticRecord>(`/api/servers/${serverId}/node-quality/reports`, 'POST', { ip_version: ipVersion, network_mode: networkMode, upload_report: uploadReport }), () => { setUploadReport(false); resource.reload() })}><Icon name="activity" size={16} />{run.busy ? '创建任务…' : active ? '报告正在执行' : '一键获取报告'}</button></div>
        </div>
        <div className="quality-history"><h3>最近报告</h3>{data.reports.length ? data.reports.map(record => <ReportResult key={record.id} record={record} />) : <p className="helper">还没有运行过报告。</p>}</div>
      </>}
    </div>
  </section>
}
