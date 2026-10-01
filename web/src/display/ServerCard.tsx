import { memo } from 'react'
import type { ProbeOverview } from '../probes'
import ProbeQuality from './ProbeQuality'
import { time, uptime } from '../format'
import type { Server } from '../types'
import { count, fresh, network, number, percentage, ratio, size, speed, status } from './data'
import { Icon, OSIcon } from './Icon'
import AssetInfo, { AssetChips } from './AssetInfo'

export function Metric({ label, value, detail, display }: { label: string; value: number | null; detail: string; display?: string }) {
  const tone = value !== null && value >= 90 ? 'danger' : value !== null && value >= 75 ? 'warning' : 'good'
  return <div className="d-metric">
    <div><span>{label}</span><strong className={value !== null && value >= 75 ? `d-${tone}` : ''}>{display ?? percentage(value)}</strong></div>
    <span className="d-track" aria-hidden="true"><span className={`d-fill d-bg-${tone}`} style={{ width: `${Math.min(100, Math.max(0, value ?? 0))}%` }} /></span>
    <small title={detail}>{detail}</small>
  </div>
}

export const ServerCard = memo(function ServerCard({ server, unavailable, probes, probeError, probeLoading, now }: { server: Server; unavailable: boolean; probes?: ProbeOverview[]; probeError: boolean; probeLoading: boolean; now: number }) {
  const metrics = server.latest_metrics, info = server.static_info
  const state = status(server, unavailable), live = fresh(server) && !unavailable
  const up = network(metrics, 'transmit_bytes_per_sec'), down = network(metrics, 'receive_bytes_per_sec')
  const swapDisabled = metrics.swap_total === 0 && metrics.swap_used === 0
  return <a className={`d-card d-glass ${!server.online ? 'd-offline' : ''}`} href={`#/overview/${server.id}`} aria-label={`${server.name}，${state.label}，查看详情`}>
    <div className="d-card-header"><span className={`d-dot d-bg-${state.tone}`} /><strong title={server.name}>{server.name}</strong><span className={`d-card-state d-${state.tone}`}>{state.label}</span><OSIcon system={info.system} /></div>
    <AssetChips server={server} />
    <div className="d-card-body">
      <div className="d-chips"><span>{info.system ?? '系统尚未上报'}</span><span>{info.arch ?? '架构未知'}</span></div>
      <div className={`d-metrics ${!live ? 'd-historical' : ''}`}>
        <Metric label={`处理器${info.cpu_cores ? ` · ${info.cpu_cores} 核` : ''}`} value={number(metrics.cpu_percent)} detail={[metrics.load_1, metrics.load_5, metrics.load_15].map(value => number(value) === null ? '—' : value!.toFixed(2)).join(' / ')} />
        <Metric label="内存" value={ratio(metrics.memory_used, info.memory_total)} detail={`${size(metrics.memory_used)} / ${size(info.memory_total)}`} />
        <Metric label="磁盘" value={ratio(metrics.disk_used, info.disk_total)} detail={`${size(metrics.disk_used)} / ${size(info.disk_total)}`} />
        <Metric label="交换内存" value={ratio(metrics.swap_used, metrics.swap_total)} display={swapDisabled ? '未启用' : undefined} detail={swapDisabled ? '设备未配置交换空间' : `${size(metrics.swap_used)} / ${size(metrics.swap_total)}`} />
      </div>
      <div className="d-data-grid">
        <div className="d-data"><small>实时速率</small><span className="d-good"><Icon name="up" size={12} />{live ? speed(up) : '—'}</span><span className="d-info"><Icon name="down" size={12} />{live ? speed(down) : '—'}</span></div>
        <div className="d-data"><small>网卡累计</small><span><Icon name="up" size={12} />{size(network(metrics, 'transmitted_bytes'))}</span><span><Icon name="down" size={12} />{size(network(metrics, 'received_bytes'))}</span></div>
        <div className="d-data"><small>系统运行</small><span><Icon name="clock" size={12} />{metrics.uptime_secs === undefined ? '—' : uptime(metrics.uptime_secs)}</span><span><Icon name="network" size={12} />{count(metrics.tcp_connections)} 个连接</span></div>
      </div>
      <ProbeQuality probes={probes} now={now} online={server.online} unavailable={probeError || unavailable} loading={probeLoading} />
      <div className="d-card-foot"><span className={!live ? 'd-warning' : ''}>{unavailable ? '刷新失败 · 保留历史' : server.metrics_stale ? '指标已过期' : !server.online ? '最近上报的数据' : !server.metrics_sampled_at ? '采样时间未知' : '指标正常'}</span><span title={server.metrics_sampled_at ? time(server.metrics_sampled_at / 1000) : undefined}>{server.metrics_sampled_at ? new Date(server.metrics_sampled_at).toLocaleTimeString('zh-CN', { hour12: false }) : '尚无采样时间'}</span></div>
      {!server.online && !unavailable && <div className="d-offline-overlay"><strong>{state.label}</strong><span>{server.last_seen ? `最后在线 ${time(server.last_seen)}` : '等待设备首次接入'}</span></div>}
    </div>
    <AssetInfo server={server} now={now} />
  </a>
})
