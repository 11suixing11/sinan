import { api } from '../api'
import { Badge, ErrorNotice, Icon, Loading } from '../components'
import { time } from '../format'
import { useAction, useResource } from '../hooks'
import { qualityValue } from '../quality'
import type { IpQuality, ServerIpInfo as ServerIpInfoData, QualityDatabase, QualityErrorKind } from '../types'

const queryErrorLabels: Record<QualityErrorKind, string> = {
  dns: 'DNS 解析失败', connect: '连接失败', tls: 'TLS 验证或握手失败', timeout: '查询超时',
  http_403: '访问被拒绝（403）', http_429: '请求被限流（429）', http_other: '其他 HTTP 错误',
  non_json: '响应不是 JSON', schema_mismatch: '字段不匹配', body_error: '响应读取失败',
  response_limit: '响应超过上限', request_error: '请求失败，原因未分类', not_public: '未向第三方查询',
  not_attempted: '尚未开始查询', invalid_origin: '查询入口地址无效',
}

function QualityResult({ result }: { result: IpQuality }) {
  const now = Date.now() / 1000
  const known = (database: QualityDatabase) => database.fields.some(field => qualityValue(field) !== undefined)
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
      {hasSaved ? <><p className="helper">{past ? '正在显示历史结果' : '最近成功结果'} · {database.last_success_at != null ? `上次成功于 ${time(database.last_success_at)}` : '旧记录未保存成功时间'} · {database.fresh_until != null ? database.fresh_until <= now ? '数据已过期' : `数据有效期至 ${time(database.fresh_until)}` : '有效期未知'}</p><dl className="detail-list">{database.fields.map(field => <div key={field.label}><dt>{field.label}</dt><dd>{qualityValue(field) ?? '未知'}</dd></div>)}</dl></> : <p className="helper">没有已保存的成功结果，信息未知。</p>}
    </details>})}</div>
  </div>
}


export default function ServerIpInfo({ serverId }: { serverId: number }) {
  const resource = useResource<ServerIpInfoData>(`/api/servers/${serverId}/ip-quality`)
  const refresh = useAction()
  const data = resource.data
  return <section className="panel">
    <div className="panel-heading"><h2>服务器 IP 信息</h2><button className="button button-secondary" disabled={refresh.busy || !data?.ip_addresses.length} onClick={() => void refresh.run(() => api<IpQuality[]>(`/api/servers/${serverId}/ip-quality/refresh`, 'POST'), () => resource.reload())}><Icon name="refresh" size={14} />{refresh.busy ? '查询中…' : '刷新 IP 质量'}</button></div>
    <div className="panel-body quality-body"><ErrorNotice message={resource.error || refresh.error} retry={resource.reload} />
      {!data ? resource.loading && <Loading /> : <>
        <p className="helper">IP 由 Agent 上报，包含设备网卡地址与配置补充的公网出口地址。质量查询会将公网 IP 发送给 check-place 查询入口；缓存仅作参考。</p>
        {!data.ip_addresses.length ? <div className="inline-empty">设备尚未上报 IP 地址，请升级 Agent 或等待设备上报。</div> : <div className="quality-addresses">{data.ip_addresses.map(ip => {
          const results = data.quality.filter(item => item.ip === ip)
          return <article key={ip} className="quality-address"><h3 className="mono">{ip}</h3>{results.length ? results.map(result => <QualityResult key={`${ip}/${result.provider ?? 'check-place'}`} result={result} />) : <p className="helper">尚未查询质量。点击“刷新 IP 质量”查询各数据库。</p>}</article>
        })}</div>}
      </>}
    </div>
  </section>
}
