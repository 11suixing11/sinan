import { useState } from 'react'
import { api } from '../api'
import { Badge, ErrorNotice, Icon, Loading } from '../components'
import { time } from '../format'
import { qualityValue } from '../quality'
import { useAction, useResource } from '../hooks'
import type { DiagnosticRecord, IpQuality, NodeQuality as NodeQualityData, QualityDatabase, QualityErrorKind } from '../types'

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
const queryErrorLabels: Record<QualityErrorKind, string> = {
  dns: 'DNS 解析失败', connect: '连接失败', tls: 'TLS 验证或握手失败', timeout: '查询超时',
  http_403: '访问被拒绝（403）', http_429: '请求被限流（429）', http_other: '其他 HTTP 错误',
  non_json: '响应不是 JSON', schema_mismatch: '字段不匹配', body_error: '响应读取失败',
  response_limit: '响应超过上限', request_error: '请求失败，原因未分类', not_public: '未向第三方查询',
  not_attempted: '尚未开始查询', invalid_origin: '查询入口地址无效',
}

function QualityResult({ result }: { result: IpQuality }) {
  const now = Date.now() / 1000
  const known = (database: QualityDatabase) => database.fields.some(field => qualityValue(field, database.database) !== undefined)
  const historical = (database: QualityDatabase) => Boolean(database.historical) || (known(database) && (database.status !== 'succeeded' || (database.fresh_until != null ? database.fresh_until <= now : database.historical == null ? result.expires_at <= now : true)))
  const successes = result.databases.filter(item => item.status === 'succeeded' && known(item) && !historical(item)).length
  const saved = result.databases.filter(item => known(item) && historical(item)).length
  return <div className="quality-result">
    <div className="quality-summary"><Badge tone={successes > 0 && successes === result.databases.length ? 'good' : successes || saved ? 'warm' : 'neutral'}>{successes ? `${successes} / ${result.databases.length} 个数据库有当前成功结果` : saved ? '当前没有有效成功结果' : '质量未知'}{saved > 0 && ` · ${saved} 个历史结果`}</Badge><span className="subtle">最近查询批次 {time(result.last_attempt_at ?? result.checked_at)}</span></div>
    <p className="helper">这些数据库信息来自同一 {result.provider ?? 'check-place'} 查询入口；各数据库的类型、标记和评分分别展示，评分保留上游原值。缺失、空值或类型不能确认的字段表示未知，不推断为零分或干净。</p>
    <div className="quality-databases">{result.databases.map(database => {
      const past = historical(database), hasSaved = known(database)
      const error = database.last_error?.message ?? database.error
      const kind = database.last_error?.kind ?? database.error_kind
      const httpStatus = database.last_error?.http_status ?? database.http_status
      return <details key={database.database} className="quality-database">
      <summary><span>{database.label}</span><Badge tone={hasSaved ? past ? 'warm' : 'good' : 'neutral'}>{hasSaved ? past ? '历史结果' : '本次已查询' : '未知'}</Badge></summary>
      <p className="helper">查询入口：{database.provider ?? 'check-place'} · 目标 IP：{database.target_ip ?? result.ip}</p>
      <p className="helper">{database.attempted_at != null ? `尝试于 ${time(database.attempted_at)}` : database.error_kind === 'not_attempted' ? '该轮尚未开始查询' : '旧记录未保存逐源查询时间'} · {database.elapsed_ms != null ? `耗时 ${database.elapsed_ms} 毫秒` : '耗时未知'}</p>
      {error && <><p className="helper">当前失败类别：{kind ? queryErrorLabels[kind] ?? '原因未分类' : '旧记录未分类'}{httpStatus != null && ` · HTTP ${httpStatus}`}</p><p className="quality-database-error">{error}</p></>}
      {hasSaved ? <><p className="helper">{past ? '正在显示历史结果' : '最近成功结果'} · {database.last_success_at != null ? `上次成功于 ${time(database.last_success_at)}` : '旧记录未保存成功时间'} · {database.fresh_until != null ? database.fresh_until <= now ? '数据已过期' : `数据有效期至 ${time(database.fresh_until)}` : '有效期未知'}</p><dl className="detail-list">{database.fields.map(field => <div key={field.label}><dt>{field.label}</dt><dd>{qualityValue(field, database.database) ?? '未知'}</dd></div>)}</dl></> : <p className="helper">没有已保存的成功结果，信息未知。</p>}
    </details>})}</div>
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
        <p className="helper">IP 由 Agent 上报，包含设备网卡地址与配置补充的公网出口地址。质量查询会将公网 IP 发送给 check-place 查询入口；缓存仅作参考。</p>
        {!data.ip_addresses.length ? <div className="inline-empty">设备尚未上报 IP 地址，请升级 Agent 或等待设备上报。</div> : <div className="quality-addresses">{data.ip_addresses.map(ip => {
          const results = data.quality.filter(item => item.ip === ip)
          return <article key={ip} className="quality-address"><h3 className="mono">{ip}</h3>{results.length ? results.map(result => <QualityResult key={`${ip}/${result.provider ?? 'check-place'}`} result={result} />) : <p className="helper">尚未查询质量。点击“刷新 IP 质量”查询各数据库。</p>}</article>
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
