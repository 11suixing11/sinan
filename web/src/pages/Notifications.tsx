import { Badge, Empty, ErrorNotice, Loading, PageHeader, Refresh } from '../components'
import { useResource } from '../hooks'
import { time } from '../format'

type Event = { id: number; server_id: number; server_name: string; last_seen: number; opened_at: number; resolved_at: number | null; resolution: string | null; deliveries: { kind: string; status: string; attempts: number; last_error: string | null }[] }
const statuses: Record<string, string> = { pending: '等待发送或重试', sent: '已发送', failed: '发送失败', cancelled: '已取消' }
export default function Notifications() {
  const resource = useResource<Event[]>('/api/notifications', 15_000)
  return <><PageHeader eyebrow="服务器监控" title="离线告警" description="展示最近 200 条事件，已关闭的事件保留 90 天。"><Refresh onClick={resource.reload} /><a className="button button-secondary" href="#/system/settings">通知设置</a></PageHeader><ErrorNotice message={resource.error} retry={resource.reload} />
    <section className="panel">{resource.loading && !resource.data ? <Loading /> : !resource.data?.length ? <Empty icon="check" title="暂无离线告警" description="尚未接入的服务器不会触发告警。" /> : <div className="table-wrap"><table><thead><tr><th>服务器</th><th>状态</th><th>最后在线 / 触发时间</th><th>恢复时间</th><th>Telegram 通知</th></tr></thead><tbody>{resource.data.map(event => <tr key={event.id}><td><a href={`#/servers/${event.server_id}`}>{event.server_name}</a><small>事件 #{event.id}</small></td><td><Badge tone={event.resolved_at ? 'neutral' : 'bad'}>{!event.resolved_at ? '持续离线' : event.resolution === 'recovered' ? '已恢复' : '告警已关闭'}</Badge></td><td>{time(event.last_seen)}<small>{time(event.opened_at)}</small></td><td>{time(event.resolved_at)}</td><td>{!event.deliveries.length ? '未启用' : event.deliveries.map(item => <div key={item.kind}>{item.kind === 'offline' ? '离线' : '恢复'}：{statuses[item.status] ?? item.status}{item.attempts > 0 && `（${item.attempts} 次）`}{item.last_error && <small>{item.last_error}</small>}</div>)}</td></tr>)}</tbody></table></div>}</section></>
}
