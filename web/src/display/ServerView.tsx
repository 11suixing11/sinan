import { useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { useResource } from '../hooks'
import { time, uptime } from '../format'
import type { Server } from '../types'
import Chart from './Chart'
import { count, fresh, network, number, percentage, sampleGap, size, speed, status } from './data'
import type { Sample } from './data'
import ProbeCharts from './ProbeCharts'
import { Icon, OSIcon } from './Icon'
import { useHistory } from './useHistory'
import AssetInfo from './AssetInfo'

function InfoGroup({ title, icon, items }: { title: string; icon: string; items: [string, ReactNode][] }) {
  return <section className="d-info-group d-glass"><h2><Icon name={icon} size={16} />{title}</h2><dl>{items.map(([label, value]) => <div key={label}><dt>{label}</dt><dd>{value ?? '—'}</dd></div>)}</dl></section>
}

function ResourceCharts({ server, now }: { server: Server; now: number }) {
  const [minutes, setMinutes] = useState(15)
  const history = useHistory(server.id, minutes)
  const samples = useMemo(() => {
    const byTime = new Map(history.samples.map(sample => [sample.sampled_at, sample]))
    if (server.metrics_sampled_at && server.metrics_sampled_at <= now) byTime.set(server.metrics_sampled_at, { id: 'latest', sampled_at: server.metrics_sampled_at, metrics: server.latest_metrics })
    return [...byTime.values()].filter(sample => sample.sampled_at >= now - minutes * 60_000).sort((a, b) => a.sampled_at - b.sampled_at)
  }, [history.samples, server.metrics_sampled_at, server.latest_metrics, now, minutes])
  const props = { from: now - minutes * 60_000, to: now, gap: sampleGap(samples) }
  const series = (label: string, color: string, read: (sample: Sample) => number | null) => ({ label, color, points: samples.map(sample => ({ at: sample.sampled_at, value: read(sample) })) })
  return <section className="d-resource-charts">
    <div className="d-section-heading"><h2><Icon name="activity" size={17} />资源趋势</h2><div className="d-segmented" role="group" aria-label="资源时间范围">{[[15, '15 分钟'], [60, '1 小时'], [120, '2 小时']].map(([value, label]) => <button key={value} aria-pressed={minutes === value} onClick={() => setMinutes(Number(value))}>{label}</button>)}</div></div>
    {history.error && <div className="d-notice d-error" role="alert"><span>{history.error} 历史采样刷新失败。</span><button onClick={history.reload}>重试</button></div>}
    {history.loading && <p className="d-history-loading" role="status">正在读取历史采样…</p>}
    <div className="d-charts">
      <Chart {...props} title="处理器" maximum={100} format={percentage} series={[series('使用率', 'var(--d-danger-bar)', sample => number(sample.metrics.cpu_percent))]} />
      <Chart {...props} title="内存" maximum={number(server.static_info.memory_total) ?? undefined} format={size} series={[series('已用内存', 'var(--d-success-bar)', sample => number(sample.metrics.memory_used))]} />
      <Chart {...props} title="磁盘" maximum={number(server.static_info.disk_total) ?? undefined} format={size} series={[series('已用磁盘', 'var(--d-warning-bar)', sample => number(sample.metrics.disk_used))]} />
      <Chart {...props} title="网络速率" format={speed} series={[series('上行', 'var(--d-success-bar)', sample => network(sample.metrics, 'transmit_bytes_per_sec')), series('下行', 'var(--d-info)', sample => network(sample.metrics, 'receive_bytes_per_sec'))]} />
    </div>
    <p className="d-footnote">鼠标移到曲线上查看采样，键盘可用左右方向键移动；点击图例可隐藏曲线。空缺不补零，较长采样间隔以断线显示。</p>
  </section>
}

export default function ServerView({ id, now }: { id: number; now: number }) {
  const resource = useResource<Server>(`/api/servers/${id}`)
  const server = resource.data
  if (!server) return <><a className="d-button d-back" href="#/overview"><Icon name="back" />返回总览</a>{resource.error ? <div className="d-notice d-error" role="alert"><span>{resource.error}</span><button onClick={resource.reload}>重试</button></div> : <div className="d-empty" role="status"><span className="spinner" />正在读取服务器…</div>}</>
  const info = server.static_info, metrics = server.latest_metrics, state = status(server, Boolean(resource.error))
  const live = fresh(server) && !resource.error
  return <div className="d-detail">
    <section className="d-detail-hero d-glass"><a href="#/overview" className="d-icon-button" aria-label="返回服务器总览"><Icon name="back" size={20} /></a><div><h1>{server.name}<span className={`d-status d-${state.tone}`}>{state.label}</span></h1><p><OSIcon system={info.system} />{info.system ?? '系统尚未上报'}<span>·</span>{info.arch ?? '架构未知'}</p></div><button className="d-icon-button" aria-label="刷新服务器详情" onClick={resource.reload}><Icon name="refresh" /></button></section>
    {resource.error && <div className="d-notice d-error" role="alert">{resource.error} 以下保留最近一次读取的信息。</div>}
    {!live && <div className="d-notice"><Icon name="clock" size={16} /><span>{resource.error ? '暂时无法读取最新设备状态。' : !server.online ? '设备离线或尚未接入。' : server.metrics_stale ? '指标已过期；在线心跳不代表指标仍在采集。' : '采样时间未知，无法确认指标是否仍然有效。'}以下为最后上报的数据，实时速率暂不显示。</span></div>}
    <div className="d-info-groups">
      <InfoGroup title="硬件信息" icon="cpu" items={[
        ['处理器', info.cpu_model], ['核心 / 架构', `${count(info.cpu_cores)} 核 / ${info.arch ?? '—'}`],
        ['内存 / 磁盘', `${size(info.memory_total)} / ${size(info.disk_total)}`], ['虚拟化', info.virtualization],
      ]} />
      <InfoGroup title="系统信息" icon="server" items={[
        ['主机名', info.hostname], ['运行时间', metrics.uptime_secs === undefined ? undefined : uptime(metrics.uptime_secs)],
        ['内核版本', info.kernel], ['进程 / 连接', `${count(metrics.processes)} / ${count(number(metrics.tcp_connections) !== null && number(metrics.udp_connections) !== null ? metrics.tcp_connections! + metrics.udp_connections! : null)}`],
      ]} />
    </div>
    <div className="d-live-strip d-glass"><div><span>处理器</span><strong>{percentage(metrics.cpu_percent)}</strong></div><div><span>已用内存</span><strong>{size(metrics.memory_used)}</strong></div><div><span>实时上行</span><strong className="d-good">{live ? speed(network(metrics, 'transmit_bytes_per_sec')) : '—'}</strong></div><div><span>实时下行</span><strong className="d-info">{live ? speed(network(metrics, 'receive_bytes_per_sec')) : '—'}</strong></div></div>
    <ResourceCharts server={server} now={now} />
    <AssetInfo server={server} now={now} detail />
    <ProbeCharts id={id} now={now} unavailable={Boolean(resource.error) || !server.online} />
    <section className="d-table-section d-glass"><h2><Icon name="network" size={16} />网络接口</h2>{Object.keys(metrics.network_interfaces ?? {}).length ? <div className="d-table-scroll"><table><thead><tr><th>接口</th><th>累计上传</th><th>累计下载</th><th>上行速率</th><th>下行速率</th></tr></thead><tbody>{Object.entries(metrics.network_interfaces ?? {}).map(([name, metric]) => <tr key={name}><th scope="row">{name}</th><td>{size(metric.transmitted_bytes)}</td><td>{size(metric.received_bytes)}</td><td className="d-good">{live ? speed(metric.transmit_bytes_per_sec) : '—'}</td><td className="d-info">{live ? speed(metric.receive_bytes_per_sec) : '—'}</td></tr>)}</tbody></table></div> : <p className="d-table-empty">设备尚未上报网卡数据。</p>}</section>
    {metrics.disks?.length ? <section className="d-table-section d-glass"><h2><Icon name="database" size={16} />磁盘读写</h2><div className="d-table-scroll"><table><thead><tr><th>磁盘 / 挂载点</th><th>已用 / 容量</th><th>读取 / 写入速率</th><th>读 / 写操作数（每秒）</th><th>等待 / 利用率</th></tr></thead><tbody>{metrics.disks.map((disk, index) => <tr key={`${disk.name}-${index}`}><th scope="row">{disk.name}<small>{disk.mount_point}</small></th><td>{size(disk.used_bytes)} / {size(disk.total_bytes)}</td><td>{speed(disk.read_bytes_per_sec)} / {speed(disk.write_bytes_per_sec)}</td><td>{count(disk.read_iops)} / {count(disk.write_iops)}</td><td>{number(disk.await_ms) === null ? '—' : `${disk.await_ms!.toFixed(1)} ms`} / {percentage(disk.utilization_percent)}</td></tr>)}</tbody></table></div></section> : null}
    {metrics.gpus?.length ? <section className="d-table-section d-glass"><h2><Icon name="cpu" size={16} />图形处理器</h2><div className="d-table-scroll"><table><thead><tr><th>型号</th><th>使用率</th><th>已用显存 / 总显存</th></tr></thead><tbody>{metrics.gpus.map((gpu, index) => <tr key={index}><th scope="row">{gpu.model}</th><td>{percentage(gpu.usage_percent)}</td><td>{size(gpu.memory_used)} / {size(gpu.memory_total)}</td></tr>)}</tbody></table></div></section> : null}
    <div className="d-info-groups"><InfoGroup title="设备状态" icon="activity" items={[
      ['设备版本', info.agent_version], ['运行时版本', info.runtime_version], ['最近设备消息', time(server.last_seen)], ['最后心跳', time(server.last_heartbeat_at)],
    ]} /><InfoGroup title="采样信息" icon="clock" items={[
      ['最后采样', server.metrics_sampled_at ? time(server.metrics_sampled_at / 1000) : '时间未知'], ['交换内存', `${size(metrics.swap_used)} / ${size(metrics.swap_total)}`], ['TCP / UDP 连接', `${count(metrics.tcp_connections)} / ${count(metrics.udp_connections)}`], ['系统负载（1 / 5 / 15 分钟）', [metrics.load_1, metrics.load_5, metrics.load_15].map(value => number(value) === null ? '—' : value!.toFixed(2)).join(' / ')],
    ]} /></div>
    <p className="d-footnote">页面展示设备上报的信息，每 5 秒刷新状态。网卡累计可能因重启归零，不等同于代理用户用量。</p>
  </div>
}
