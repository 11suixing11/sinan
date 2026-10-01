import { useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import type { ProbeOverview } from '../probes'
import type { Server } from '../types'
import { aggregate, network, size, speed } from './data'
import { dashboardCounts, savedSort, savedView, selectServers, snapshotUnavailable } from './dashboard'
import type { DashboardFilter, DashboardSort, DashboardView } from './dashboard'
import { Icon } from './Icon'
import { ServerCard } from './ServerCard'
import ServerTable from './ServerTable'
import { useDashboardPoll } from './useDashboardPoll'

function Stat({ icon, label, value, unit, children, tone }: { icon: string; label: string; value: ReactNode; unit?: string; children: ReactNode; tone?: string }) {
  return <div className="d-overview-item"><div className="d-overview-label">{label}<span className={`d-stat-icon d-${tone ?? 'good'}`}><Icon name={icon} size={17} /></span></div><div className="d-overview-value"><strong className={tone ? `d-${tone}` : ''}>{value}</strong>{unit && <b>{unit}</b>}</div><div className="d-overview-note">{children}</div></div>
}

function readPreference(key: string): string | null {
  try { return localStorage.getItem(`sinan-dashboard-${key}`) } catch { return null }
}
function savePreference(key: string, value: string) {
  try { localStorage.setItem(`sinan-dashboard-${key}`, value) } catch { /* Keep the in-memory choice when storage is disabled. */ }
}
const filters: [DashboardFilter, string][] = [['all', '全部'], ['online', '在线'], ['offline', '离线'], ['pending', '待接入'], ['stale', '指标待更新']]

export default function Overview({ now }: { now: number }) {
  const [paused, setPaused] = useState(false)
  const resource = useDashboardPoll<Server[]>('/api/servers', 5000, paused)
  const probes = useDashboardPoll<ProbeOverview[]>('/api/probes/overview', 15_000, paused)
  const { data: servers, error, loading, updatedAt } = resource
  const [view, setView] = useState<DashboardView>(() => savedView(readPreference('view')))
  const [sort, setSort] = useState<DashboardSort>(() => savedSort(readPreference('sort')))
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<DashboardFilter>('all')
  const [group, setGroup] = useState('')
  const [region, setRegion] = useState('')
  const unavailable = snapshotUnavailable(updatedAt, now, paused, error)
  const probeUnavailable = paused || Boolean(probes.error) || probes.updatedAt === null || now - probes.updatedAt > 45_000
  const byServer = useMemo(() => {
    const grouped = new Map<number, ProbeOverview[]>()
    for (const entry of probes.data ?? []) {
      const items = grouped.get(entry.server_id) ?? []
      items.push(entry); grouped.set(entry.server_id, items)
    }
    return grouped
  }, [probes.data])
  const entries = useMemo(() => (servers ?? []).filter(server => !server.asset_settings?.hidden), [servers])
  const groups = [...new Set(entries.map(server => server.asset_settings?.group_name).filter((value): value is string => Boolean(value)))].sort()
  const regions = [...new Set(entries.map(server => server.asset_settings?.region).filter((value): value is string => Boolean(value)))].sort()
  const visible = useMemo(() => selectServers(entries, query, filter, group, region, sort, unavailable), [entries, query, filter, group, region, sort, unavailable])
  const counts = dashboardCounts(entries)
  const upload = aggregate(entries, 'transmit_bytes_per_sec', true), download = aggregate(entries, 'receive_bytes_per_sec', true)
  const completeCounters = entries.filter(server => network(server.latest_metrics, 'transmitted_bytes') !== null && network(server.latest_metrics, 'received_bytes') !== null)
  const sent = aggregate(completeCounters, 'transmitted_bytes'), received = aggregate(completeCounters, 'received_bytes')
  const total = sent.value === null || received.value === null ? null : sent.value + received.value
  const coverage = (count: number) => count ? `${count} / ${entries.length} 台指标有效` : '暂无有效速率数据'
  const refresh = () => { resource.reload(); probes.reload() }
  const reset = () => { setQuery(''); setFilter('all'); setGroup(''); setRegion('') }
  const changed = Boolean(query || filter !== 'all' || group || region)
  const chooseView = (value: DashboardView) => { setView(value); savePreference('view', value) }
  const chooseSort = (value: string) => { const next = savedSort(value); setSort(next); savePreference('sort', next) }
  const feedLabel = paused ? '自动刷新已暂停' : error ? '连接中断' : unavailable && servers ? '等待新快照' : loading && !servers ? '正在连接' : '每 5 秒自动刷新'
  return <div className="d-home">
    <section className="d-dashboard-hero" aria-label="看板标题与刷新状态">
      <div><div className="d-dashboard-eyebrow"><Icon name="monitor" size={16} />基础设施监控</div><h1>服务器看板</h1><p>集中查看设备状态与网络质量，点击服务器查看历史曲线。</p></div>
      <div className="d-dashboard-feed"><span className={`d-feed-label ${unavailable ? 'd-warning' : 'd-good'}`}><span className={`d-dot d-bg-${unavailable ? 'warning' : 'good'}`} />{feedLabel}</span><small>最近成功读取：{updatedAt === null ? '尚未读取' : new Date(updatedAt).toLocaleTimeString('zh-CN', { hour12: false })}</small><button className="d-feed-button" aria-pressed={paused} onClick={() => setPaused(value => !value)}><Icon name={paused ? 'play' : 'pause'} size={14} />{paused ? '恢复自动刷新' : '暂停自动刷新'}</button></div>
    </section>
    <section className="d-overview d-glass" aria-label="服务器总览">
      <Stat icon="server" label="在线服务器" value={!servers || unavailable ? '—' : counts.online} unit={servers ? `/ ${entries.length} 台` : undefined} tone="good">{unavailable ? '当前在线状态待确认' : !entries.length ? '等待服务器接入' : counts.online === counts.all ? '全部服务器在线' : `${counts.offline} 台离线，${counts.pending} 台待接入`}</Stat>
      <Stat icon="database" label="网卡累计流量" value={size(total)}><span className="d-good">↑ {size(sent.value)}</span><span className="d-info">↓ {size(received.value)}</span></Stat>
      <Stat icon="up" label="实时上行" value={unavailable ? '—' : speed(upload.value)} tone="good">{unavailable ? '等待刷新恢复' : coverage(upload.count)}</Stat>
      <Stat icon="down" label="实时下行" value={unavailable ? '—' : speed(download.value)} tone="info">{unavailable ? '等待刷新恢复' : coverage(download.count)}</Stat>
    </section>
    <div className="d-dashboard-toolbar d-glass">
      <div className="d-toolbar"><label className="d-search"><Icon name="search" size={16} /><input type="search" aria-label="搜索服务器" placeholder="搜索名称、地区、标签、系统…" value={query} onChange={event => setQuery(event.target.value)} /></label>
        <div className="d-segmented" role="group" aria-label="服务器状态筛选">{filters.map(([value, label]) => <button key={value} aria-label={label} aria-pressed={filter === value} onClick={() => setFilter(value)}>{label}<span className="d-filter-count" aria-hidden="true">{!servers || (value !== 'all' && unavailable) ? '—' : counts[value]}</span></button>)}</div>
        <button className="d-icon-button" aria-label="刷新服务器" title={paused ? '读取一次快照，不恢复自动刷新' : '立即刷新服务器与拨测'} onClick={refresh} disabled={loading && probes.loading}><Icon name="refresh" size={17} /></button>
      </div>
      <div className="d-dashboard-options">
        <div className="d-asset-filters"><label>分组<select aria-label="分组" value={group} onChange={event => setGroup(event.target.value)}><option value="">全部分组</option>{groups.map(name => <option key={name} value={name}>{name}</option>)}</select></label><label>地区<select aria-label="地区" value={region} onChange={event => setRegion(event.target.value)}><option value="">全部地区</option>{regions.map(name => <option key={name} value={name}>{name}</option>)}</select></label>
          <label>排序<select aria-label="排序" value={sort} onChange={event => chooseSort(event.target.value)}><option value="default">默认顺序</option><option value="attention">需关注的优先</option><option value="name">按名称</option><option value="cpu">处理器占用优先</option><option value="memory">内存占用优先</option><option value="network">网络速率优先</option></select></label>
        </div>
        <div className="d-dashboard-view"><span className="d-result-count">{servers ? `${visible.length} 台服务器` : '等待数据'}</span>{changed && <button className="d-clear-filters" onClick={reset}>清除筛选</button>}<div className="d-segmented" role="group" aria-label="看板视图"><button aria-pressed={view === 'cards'} onClick={() => chooseView('cards')}><Icon name="grid" size={14} />卡片</button><button aria-pressed={view === 'table'} onClick={() => chooseView('table')}><Icon name="list" size={14} />表格</button></div></div>
      </div>
    </div>
    {paused && <div className="d-notice" role="status"><Icon name="pause" size={16} /><span>已暂停读取服务器与拨测。以下是历史快照，不代表当前在线状态；手动刷新只读取一次。</span></div>}
    {!paused && !error && unavailable && servers && <div className="d-notice" role="status"><span>超过 15 秒未收到新的服务器快照，当前状态与实时速率暂不可确认。</span><button onClick={refresh}>重试</button></div>}
    {error && <div className="d-notice d-error" role="alert"><span>{error} {servers && '以下保留最近一次读取的数据。'}</span><button onClick={refresh}>重试</button></div>}
    {probes.error && <div className="d-notice d-error" role="alert"><span>拨测数据读取失败，延迟与丢包暂不可确认。</span><button onClick={probes.reload}>重试拨测</button></div>}
    {loading && !servers ? <div className="d-empty" role="status"><span className="spinner" />正在读取服务器…</div> : !servers && paused ? <div className="d-empty"><Icon name="pause" size={28} /><strong>尚未读取服务器</strong><p>恢复自动刷新或手动读取一次快照。</p></div> : visible.length ? view === 'cards' ?
      <section className="d-node-grid" aria-label="服务器列表">{visible.map(server => <ServerCard key={server.id} server={server} unavailable={unavailable} probes={probes.data ? byServer.get(server.id) ?? [] : undefined} probeError={probeUnavailable} probeLoading={probes.loading} now={now} />)}</section> :
      <ServerTable servers={visible} unavailable={unavailable} byServer={byServer} probesKnown={probes.data !== undefined} probeError={probeUnavailable} probeLoading={probes.loading} now={now} /> :
      !error && <div className="d-empty"><Icon name="server" size={32} /><strong>{entries.length ? '没有匹配的服务器' : servers?.length ? '服务器已在看板隐藏' : '还没有服务器'}</strong><p>{entries.length ? '试试其他关键词、状态、地区或分组。' : servers?.length ? '在服务器管理中取消展示隐藏后，就会出现在这里。' : '在后台添加服务器并接入设备后，运行信息会显示在这里。'}</p>{entries.length ? <button className="d-button" onClick={reset}>重置筛选条件</button> : <a className="d-button" href="#/servers">前往服务器管理</a>}</div>}
    <p className="d-footnote">汇总覆盖所有未隐藏的服务器，不随下方筛选改变。状态每 5 秒、拨测每 15 秒读取；本页不创建任务或执行诊断。{sort === 'attention' && '关注顺序为离线、指标待更新、待接入或资源占用达到 90%；这只是展示排序，不是告警判定。'}<br />实时速率只统计在线且采样时间已知、未过期的设备。网卡累计是最近上报的接口计数，可能因重启归零，并非代理用户用量。{servers && Math.min(sent.count, received.count) < entries.length ? ` ${entries.length - Math.min(sent.count, received.count)} 台缺少完整网卡计数。` : ''}</p>
  </div>
}
