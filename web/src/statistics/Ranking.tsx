import { bytes } from '../format'
import { amount, ratio } from './data'
import type { TrafficRanking } from './data'

export default function Ranking({ title, rows, link }: { title: string; rows: TrafficRanking[]; link?: (row: TrafficRanking) => string }) {
  const maximum = rows.reduce((max, row) => { const value = amount(row.total) ?? 0n; return value > max ? value : max }, 0n)
  return <section className="panel statistics-ranking" aria-label={title}>
    <div className="panel-heading"><h2>{title}</h2><span className="subtle">上传 + 下载 · 前 8</span></div>
    {rows.length ? <ol>{rows.map((row, index) => <li key={row.id}>
      <span className="statistics-rank-number">{index + 1}</span>
      <div className="statistics-rank-info"><div>{link && !row.deleted ? <a href={link(row)}>{row.name}</a> : <strong>{row.name}</strong>}<b>{bytes(row.total)}</b></div>
        <small>{row.deleted ? '已删除 · ' : ''}上传 {bytes(row.uploaded)} · 下载 {bytes(row.downloaded)}{row.incomplete ? ' · 采样不完整' : ''}</small>
        <span className="statistics-rank-meter" aria-hidden="true"><span style={{ width: `${ratio(row.total, maximum)}%` }} /></span>
      </div>
    </li>)}</ol> : <div className="statistics-ranking-empty">此时间段暂无流量排行</div>}
  </section>
}
