import { useState } from 'react'
import type { ReactNode } from 'react'
import type { Server } from '../types'
import { aggregate, filterServers, network, size, speed } from './data'
import { Icon } from './Icon'
import { ServerCard } from './ServerCard'

function Stat({ icon, label, value, unit, children, tone }: { icon: string; label: string; value: ReactNode; unit?: string; children: ReactNode; tone?: string }) {
  return <div className="d-overview-item"><div className="d-overview-label">{label}<span className={`d-stat-icon d-${tone ?? 'good'}`}><Icon name={icon} size={17} /></span></div><div className="d-overview-value"><strong className={tone ? `d-${tone}` : ''}>{value}</strong>{unit && <b>{unit}</b>}</div><div className="d-overview-note">{children}</div></div>
}

export default function Overview({ servers, loading, error, reload }: { servers?: Server[]; loading: boolean; error: string; reload: () => void }) {
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState('all')
  const entries = servers ?? []
  const visible = filterServers(entries, query, filter)
  const online = entries.filter(server => server.online).length
  const upload = aggregate(entries, 'transmit_bytes_per_sec', true), download = aggregate(entries, 'receive_bytes_per_sec', true)
  const completeCounters = entries.filter(server => network(server.latest_metrics, 'transmitted_bytes') !== null && network(server.latest_metrics, 'received_bytes') !== null)
  const sent = aggregate(completeCounters, 'transmitted_bytes'), received = aggregate(completeCounters, 'received_bytes')
  const total = sent.value === null || received.value === null ? null : sent.value + received.value
  const coverage = (count: number) => count ? `${count} / ${entries.length} 台指标有效` : '暂无有效速率数据'
  return <div className="d-home">
    <h1 className="d-sr-only">服务器总览</h1>
    <section className="d-overview d-glass" aria-label="服务器总览">
      <Stat icon="server" label="在线服务器" value={!servers || error ? '—' : online} unit={servers ? `/ ${entries.length} 台` : undefined} tone="good">{error ? '状态刷新失败' : !entries.length ? '等待服务器接入' : online === entries.length ? '全部服务器在线' : `${entries.length - online} 台离线或待接入`}</Stat>
      <Stat icon="database" label="网卡累计流量" value={size(total)}><span className="d-good">↑ {size(sent.value)}</span><span className="d-info">↓ {size(received.value)}</span></Stat>
      <Stat icon="up" label="实时上行" value={error ? '—' : speed(upload.value)} tone="good">{error ? '等待刷新恢复' : coverage(upload.count)}</Stat>
      <Stat icon="down" label="实时下行" value={error ? '—' : speed(download.value)} tone="info">{error ? '等待刷新恢复' : coverage(download.count)}</Stat>
    </section>
    <div className="d-toolbar"><label className="d-search"><Icon name="search" size={16} /><input type="search" aria-label="搜索服务器" placeholder="搜索名称、系统、主机名…" value={query} onChange={event => setQuery(event.target.value)} /></label><div className="d-segmented" role="group" aria-label="服务器状态筛选">{[['all', '全部'], ['online', '在线'], ['offline', '离线'], ['pending', '待接入']].map(([value, label]) => <button key={value} aria-pressed={filter === value} onClick={() => setFilter(value)}>{label}</button>)}</div><span className="d-result-count">{servers ? `${visible.length} 台服务器` : '等待数据'}</span><button className="d-icon-button" aria-label="刷新服务器" title="每 5 秒自动刷新" onClick={reload}><Icon name="refresh" size={17} /></button></div>
    {error && <div className="d-notice d-error" role="alert"><span>{error} {servers && '以下保留最近一次读取的数据。'}</span><button onClick={reload}>重试</button></div>}
    {loading && !servers ? <div className="d-empty" role="status"><span className="spinner" />正在读取服务器…</div> : visible.length ? <section className="d-node-grid" aria-label="服务器列表">{visible.map(server => <ServerCard key={server.id} server={server} unavailable={Boolean(error)} />)}</section> : !error && <div className="d-empty"><Icon name="server" size={32} /><strong>{entries.length ? '没有匹配的服务器' : '还没有服务器'}</strong><p>{entries.length ? '试试其他关键词或状态筛选。' : '在后台添加服务器并接入设备后，运行信息会显示在这里。'}</p>{entries.length ? <button className="d-button" onClick={() => { setQuery(''); setFilter('all') }}>清除筛选</button> : <a className="d-button" href="#/servers">前往服务器管理</a>}</div>}
    {servers?.length ? <p className="d-footnote">每 5 秒刷新 · 实时速率仅汇总在线且采样时间已知、未过期的设备。网卡累计为设备上报的接口计数之和，可能因重启归零；并非代理用户用量。{Math.min(sent.count, received.count) < entries.length ? ` ${entries.length - Math.min(sent.count, received.count)} 台缺少完整网卡计数。` : ''}</p> : null}
  </div>
}
