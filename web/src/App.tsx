import { lazy, Suspense, useEffect, useRef, useState } from 'react'
import type { FormEvent } from 'react'
import { api, ApiError, errorMessage } from './api'
import { Brand, ErrorNotice, Icon, Loading } from './components'
import { useAction } from './hooks'
import Servers from './pages/Servers'
import ServerDetail from './pages/ServerDetail'
import Nodes from './plugins/singbox/Nodes'
import SingboxOverview from './plugins/singbox/Overview'
import { nodeRoute } from './plugins/singbox/nodeRoute'
import ProxyUsers from './plugins/singbox/ProxyUsers'
import Groups from './plugins/singbox/Groups'
import Plugins from './pages/Plugins'
import PluginCatalog from './pages/PluginCatalog'
import { isCatalogPath } from './plugins/catalog'
import Security from './pages/Security'
import Settings from './pages/Settings'
import Notifications from './pages/Notifications'
import LatencyTasks from './pages/LatencyTasks'
import Statistics from './pages/Statistics'
import ServerToolPage from './pages/ServerToolPage'
import { dashboardRoute } from './display/dashboard'

const ServerDisplay = lazy(() => import('./display/ServerDisplay'))

const navigation = [{ path: '/statistics', label: '统计仪表盘', icon: 'activity', group: '概览' }, { path: '/dashboard', label: '服务器看板', icon: 'activity', group: '服务器' }, { path: '/servers', label: '服务器', icon: 'server', group: '服务器' }, { path: '/latency', label: '延迟检测', icon: 'activity', group: '服务器' }, { path: '/plugins/sing-box', label: '代理服务', icon: 'box', group: 'sing-box 插件' }, { path: '/plugins/sing-box/nodes', label: '代理节点', icon: 'nodes', group: 'sing-box 插件' }, { path: '/plugins/sing-box/users', label: '代理用户', icon: 'users', group: 'sing-box 插件' }, { path: '/plugins/sing-box/groups', label: '策略与套餐', icon: 'nodes', group: 'sing-box 插件' }, { path: '/plugins/catalog', label: '插件目录', icon: 'box', group: '系统' }, { path: '/system/plugins', label: '服务器插件', icon: 'server', group: '系统' }, { path: '/system/settings', label: '看板与通知', icon: 'activity', group: '系统' }, { path: '/system/notifications', label: '告警通知', icon: 'activity', group: '系统' }, { path: '/system/administrator', label: '系统管理员', icon: 'lock', group: '系统' }]
function Login({ onLogin, notice }: { onLogin: () => void; notice: string }) {
  const action = useAction()
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const data = new FormData(event.currentTarget)
    const password = String(data.get('password') ?? '')
    const totp_code = String(data.get('totp_code') ?? '')
    void action.run(async () => {
      try { await api('/api/login', 'POST', { password, totp_code }) }
      catch (error) { if (error instanceof ApiError && error.status === 401) throw new Error('密码或验证码不正确、已过期或已使用，请重新输入。'); throw error }
    }, onLogin)
  }
  return <main className="login-page"><div className="login-story"><Brand /><div className="login-story-content"><span className="eyebrow">一处管理，始终有序</span><h1>掌握每一台<br />服务器。</h1><p>从设备接入到节点授权，<br />让网络的每一步都清晰可见。</p><div className="login-benefits"><span><Icon name="server" size={19} />设备主动连接</span><span><Icon name="check" size={19} />配置自动对账</span><span><Icon name="activity" size={19} />流量按用户统计</span></div></div><div className="login-bottom">司南 · 自托管服务器与节点面板</div></div><div className="login-form-side"><div className="login-card"><span className="login-lock"><Icon name="lock" size={23} /></span><h2>欢迎回来</h2><p>输入管理员密码；已启用二步验证时，还需验证器中的验证码。</p><ErrorNotice message={action.error || notice} /><form onSubmit={submit}><label className="field"><span>管理员密码</span><input name="password" type="password" required autoComplete="current-password" autoFocus disabled={action.busy} placeholder="请输入密码" /></label><label className="field"><span>二步验证码</span><input name="totp_code" inputMode="numeric" pattern="[0-9]{6}" maxLength={6} autoComplete="one-time-code" disabled={action.busy} placeholder="未启用时留空" /><small>启用二步验证后，请输入验证器当前的六位数字。</small></label><button className="button button-primary login-submit" disabled={action.busy}>{action.busy ? <><span className="spinner" />正在登录…</> : <>登录面板<Icon name="arrow" size={18} /></>}</button></form><div className="login-hint"><Icon name="lock" size={13} />仅限管理员访问，使用部署时设置的密码。</div></div><div className="login-footer">你的服务器，你的控制权。</div></div></main>
}

export default function App() {
  const [session, setSession] = useState<boolean | null>(null)
  const [publicDashboard, setPublicDashboard] = useState(false)
  const [accessRevision, setAccessRevision] = useState(0)
  const [notice, setNotice] = useState('')
  const [path, setPath] = useState(window.location.hash.slice(1) || '/dashboard')
  const sessionRef = useRef(session); sessionRef.current = session
  const action = useAction()
  useEffect(() => {
    const controller = new AbortController()
    let active = true
    void api<{ authenticated: boolean; public_dashboard: boolean }>('/api/dashboard/access', 'GET', undefined, controller.signal).then(access => {
      if (active) { setSession(access.authenticated); setPublicDashboard(access.public_dashboard) }
    }).catch(error => { if (active) { setSession(false); setPublicDashboard(false); setNotice(errorMessage(error)) } })
    const unauthorized = () => { if (sessionRef.current) setNotice('登录已过期，请重新登录。'); setSession(false); setPublicDashboard(false); setAccessRevision(value => value + 1) }
    const hash = () => setPath(window.location.hash.slice(1) || '/dashboard')
    window.addEventListener('sinan:unauthorized', unauthorized)
    window.addEventListener('hashchange', hash)
    return () => { active = false; controller.abort(); window.removeEventListener('sinan:unauthorized', unauthorized); window.removeEventListener('hashchange', hash) }
  }, [accessRevision])
  const [route] = path.split('?', 2)
  const nodePage = nodeRoute(path)
  const match = route.match(/^\/servers\/([1-9]\d*)(?:\/(ip-info|node-quality|tcp-quality|plugins))?$/)
  const display = dashboardRoute(route)
  const current = navigation.find(item => (item.path === '/plugins/sing-box/nodes' ? nodePage !== null : route === item.path) || (item.path === '/servers' && Boolean(match)) || (item.path === '/plugins/catalog' && isCatalogPath(route)))
  const title = display ? '服务器看板' : current?.label ?? '控制面板'
  useEffect(() => { document.title = `${title} · 司南` }, [title])
  if (session === null) return <div className="boot"><Brand /><Loading /></div>
  if (!session && !(display && publicDashboard)) return <Login notice={notice} onLogin={() => { setNotice(''); setSession(true); setAccessRevision(value => value + 1) }} />
  if (display) return <Suspense fallback={<div className="boot"><Brand /><Loading /></div>}><ServerDisplay key={session ? 'admin' : 'public'} serverId={display.serverId} /></Suspense>
  const page = match && Number.isSafeInteger(Number(match[1]))
    ? match[2] === 'plugins' ? <Plugins key={match[1]} serverId={Number(match[1])} />
      : match[2] ? <ServerToolPage key={`${match[1]}/${match[2]}`} id={Number(match[1])} section={match[2] as 'ip-info' | 'node-quality' | 'tcp-quality'} />
        : <ServerDetail key={match[1]} id={Number(match[1])} />
    : route === '/servers' || route === '/' ? <Servers />
      : route === '/statistics' ? <Statistics />
      : route === '/latency' ? <LatencyTasks />
      : route === '/plugins/sing-box' ? <SingboxOverview />
        : nodePage ? <Nodes serverId={nodePage.serverId} initialKind={nodePage.kind} initialResource={nodePage.resource} initialServerRole={nodePage.serverRole} />
        : route === '/plugins/sing-box/users' ? <ProxyUsers />
          : route === '/plugins/sing-box/groups' ? <Groups />
            : route === '/system/plugins' ? <Plugins />
              : isCatalogPath(route) ? <PluginCatalog />
                : route === '/system/settings' ? <Settings />
                  : route === '/system/notifications' ? <Notifications />
                    : route === '/system/administrator' ? <Security />
                      : <div className="not-found"><h1>这个页面不存在</h1><p>从左侧导航选择一个页面，或回到服务器列表。</p><a className="button button-primary" href="#/servers">返回服务器</a></div>
  return <div className="app-shell"><aside className="sidebar"><a href="#/servers" className="brand-link" aria-label="司南首页"><Brand /></a><div className="nav-caption">控制面板</div><nav aria-label="主导航">{navigation.map((item, index) => <div className="nav-entry" key={item.path}>{navigation[index - 1]?.group !== item.group && <div className="nav-group-label">{item.group}</div>}<a href={`#${item.path}`} className={current?.path === item.path ? 'active' : ''} aria-current={current?.path === item.path ? 'page' : undefined}><Icon name={item.icon} size={20} /><span>{item.label}</span>{current?.path === item.path && <span className="nav-active-dot" />}</a></div>)}</nav><div className="sidebar-bottom"><div className="sidebar-note"><span className="status-dot" /><span>自托管控制面板</span></div><button className="logout-button" disabled={action.busy} onClick={() => void action.run(() => api('/api/logout', 'POST'), () => { setSession(false); setNotice(''); setPublicDashboard(false); setAccessRevision(value => value + 1) })}><span className="admin-avatar">管</span><span><strong>管理员</strong><small>退出登录</small></span><Icon name="logout" size={17} /></button></div></aside><div className="main-layout"><div className="topbar"><div>控制面板<span>/</span><strong>{current?.label ?? '页面不存在'}</strong></div><span className="topbar-status"><span className="status-dot" />管理员会话已登录</span></div><main className="content"><ErrorNotice message={action.error} />{page}</main><footer className="app-footer"><span>司南</span><span>清晰掌握，自在连接。</span></footer></div></div>
}
