import { useState } from 'react'
import { bytes } from '../format'
import { amount, ratio, utcDay } from './data'
import type { TrafficPoint } from './data'

export default function TrafficChart({ title, points }: { title: string; points: TrafficPoint[] }) {
  const [selected, setSelected] = useState<number | null>(null)
  const maximum = points.reduce((max, point) => { const value = amount(point.total); return value !== null && value > max ? value : max }, 0n)
  const current = points.find(point => point.day === selected) ?? points[points.length - 1]
  const observed = points.some(point => point.total !== null)
  return <section className="panel statistics-chart" aria-label={title}>
    <div className="panel-heading"><h2>{title}</h2><span className="subtle">按日汇总 · UTC</span></div>
    <div className="statistics-chart-body">
      <div className="statistics-chart-legend"><span><i className="statistics-up" />上传</span><span><i className="statistics-down" />下载</span><small>日最高 {observed ? bytes(maximum) : '暂无数据'}</small></div>
      {observed ? <>
        <div className="statistics-chart-bars" role="group" aria-label={`${title}每日数据`} style={{ gridTemplateColumns: `repeat(${points.length}, minmax(0, 1fr))` }}>
          {points.map(point => <button key={point.day} type="button" className="statistics-chart-bar" aria-pressed={current?.day === point.day} data-missing={point.total === null} aria-label={`${utcDay(point.day)}，${point.total === null ? '暂无记录' : `上传 ${bytes(point.uploaded)}，下载 ${bytes(point.downloaded)}`}`} onMouseEnter={() => setSelected(point.day)} onFocus={() => setSelected(point.day)} onClick={() => setSelected(point.day)} onKeyDown={event => {
            if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
            event.preventDefault()
            const index = points.indexOf(point)
            const next = event.key === 'Home' ? 0 : event.key === 'End' ? points.length - 1 : Math.max(0, Math.min(points.length - 1, index + (event.key === 'ArrowLeft' ? -1 : 1)))
            event.currentTarget.parentElement?.querySelectorAll('button')[next]?.focus()
          }}>
            {point.total === null ? <span className="statistics-chart-missing" aria-hidden="true">·</span> : <span className="statistics-chart-stack" style={{ height: `${ratio(point.total, maximum)}%` }} aria-hidden="true"><span className="statistics-up" style={{ flexGrow: ratio(point.uploaded, amount(point.total) ?? 0n) }} /><span className="statistics-down" style={{ flexGrow: ratio(point.downloaded, amount(point.total) ?? 0n) }} /></span>}
          </button>)}
        </div>
        <div className="statistics-chart-axis"><span>{utcDay(points[0].day)}</span><span>{utcDay(points[Math.floor(points.length / 2)].day)}</span><span>{utcDay(points[points.length - 1].day)}</span></div>
        <div className="statistics-chart-selection" aria-live="polite"><strong>{current ? `${utcDay(current.day)}（UTC）` : ''}</strong><span>上传 {bytes(current?.uploaded)} · 下载 {bytes(current?.downloaded)}</span>{current?.incomplete && <small>这一天包含不完整采样</small>}</div>
      </> : <div className="statistics-chart-empty">此时间段暂无流量记录，等待数据上报。</div>}
      <p className="helper">悬停、点击或使用左右方向键查看每日数值。空白日期表示暂无记录，当天数据仍在累积。</p>
    </div>
  </section>
}
