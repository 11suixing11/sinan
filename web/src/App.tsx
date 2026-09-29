import { useEffect, useRef, useState } from 'react'
import type { FormEvent } from 'react'
import { api, ApiError, errorMessage } from './api'
import { Brand, ErrorNotice, Icon, Loading } from './components'
import { useAction } from './hooks'
import Servers from './pages/Servers'
import ServerDetail from './pages/ServerDetail'
import Nodes from './pages/Nodes'
import Users from './pages/Users'
import Artifacts from './pages/Artifacts'

const navigation = [{ path: '/servers', label: '服务器', icon: 'server' }, { path: '/nodes', label: '节点', icon: 'nodes' }, { path: '/users', label: '用户', icon: 'users' }, { path: '/artifacts', label: '制品', icon: 'box' }]
function Login({ onLogin, notice }: { onLogin: () => void; notice: string }) {
  const action = useAction()
  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const password = String(new FormData(event.currentTarget).get('password') ?? '')
    void action.run(async () => {
      try { await api('/api/login', 'POST', { password }) }
      catch (error) { if (error instanceof ApiError && error.status === 401) throw new Error('密码不正确，请重新输入。'); throw error }
    }, onLogin)
  }
  return <main className="login-page"><div className="login-story"><Brand /><div className="login-story-content"><span className="eyebrow">一处管理，始终有序</span><h1>掌握每一台<br />服务器。</h1><p>从设备接入到节点授权，<br />让网络的每一步都清晰可见。</p><div className="login-benefits"><span><Icon name="server" size={19} />设备主动连接</span><span><Icon name="check" size={19} />配置自动对账</span><span><Icon name="activity" size={19} />流量按用户统计</span></div></div><div className="login-bottom">司南 · 自托管服务器与节点面板</div></div><div className="login-form-side"><div className="login-card"><span className="login-lock"><Icon name="lock" size={23} /></span><h2>欢迎回来</h2><p>输入管理员密码，进入你的控制面板。</p><ErrorNotice message={action.error || notice} /><form onSubmit={submit}><label className="field"><span>管理员密码</span><input name="password" type="password" required autoComplete="current-password" autoFocus disabled={action.busy} placeholder="请输入密码" /></label><button className="button button-primary login-submit" disabled={action.busy}>{action.busy ? <><span className="spinner" />正在登录…</> : <>登录面板<Icon name="arrow" size={18} /></>}</button></form><div className="login-hint"><Icon name="lock" size={13} />仅限管理员访问，使用部署时设置的密码。</div></div><div className="login-footer">你的服务器，你的控制权。</div></div></main>
}

export default function App() {
  const [session, setSession] = useState<boolean | null>(null)
  const [notice, setNotice] = useState('')
  const [path, setPath] = useState(window.location.hash.slice(1) || '/servers')
  const sessionRef = useRef(session); sessionRef.current = session
  const action = useAction()
  useEffect(() => {
    const controller = new AbortController()
    let active = true
    void api('/api/me', 'GET', undefined, controller.signal).then(() => { if (active) setSession(true) }).catch(error => {
      if (active) { setSession(false); if (!(error instanceof ApiError && error.status === 401)) setNotice(errorMessage(error)) }
    })
    const unauthorized = () => { if (sessionRef.current) { setNotice('登录已过期，请重新登录。'); setSession(false) } }
    const hash = () => setPath(window.location.hash.slice(1) || '/servers')
    window.addEventListener('sinan:unauthorized', unauthorized)
    window.addEventListener('hashchange', hash)
    return () => { active = false; controller.abort(); window.removeEventListener('sinan:unauthorized', unauthorized); window.removeEventListener('hashchange', hash) }
  }, [])
  const match = path.match(/^\/servers\/([1-9]\d*)$/)
  const current = navigation.find(item => path === item.path || (item.path === '/servers' && Boolean(match)))
  useEffect(() => { document.title = `${current?.label ?? '控制面板'} · 司南` }, [current?.label])
  if (session === null) return <div className="boot"><Brand /><Loading /></div>
  if (!session) return <Login notice={notice} onLogin={() => { setNotice(''); setSession(true) }} />
  return <div className="app-shell"><aside className="sidebar"><a href="#/servers" className="brand-link" aria-label="司南首页"><Brand /></a><div className="nav-caption">控制面板</div><nav aria-label="主导航">{navigation.map(item => <a key={item.path} href={`#${item.path}`} className={current?.path === item.path ? 'active' : ''} aria-current={current?.path === item.path ? 'page' : undefined}><Icon name={item.icon} size={20} /><span>{item.label}</span>{current?.path === item.path && <span className="nav-active-dot" />}</a>)}</nav><div className="sidebar-bottom"><div className="sidebar-note"><span className="status-dot" /><span>自托管控制面板</span></div><button className="logout-button" disabled={action.busy} onClick={() => void action.run(() => api('/api/logout', 'POST'), () => { setSession(false); setNotice('') })}><span className="admin-avatar">管</span><span><strong>管理员</strong><small>退出登录</small></span><Icon name="logout" size={17} /></button></div></aside><div className="main-layout"><div className="topbar"><div>控制面板<span>/</span><strong>{current?.label ?? '页面不存在'}</strong></div><span className="topbar-status"><span className="status-dot" />管理员会话已登录</span></div><main className="content"><ErrorNotice message={action.error} />{match && Number.isSafeInteger(Number(match[1])) ? <ServerDetail key={match[1]} id={Number(match[1])} /> : path === '/servers' || path === '/' ? <Servers /> : path === '/nodes' ? <Nodes /> : path === '/users' ? <Users /> : path === '/artifacts' ? <Artifacts /> : <div className="not-found"><h1>这个页面不存在</h1><p>从左侧导航选择一个页面，或回到服务器列表。</p><a className="button button-primary" href="#/servers">返回服务器</a></div>}</main><footer className="app-footer"><span>司南</span><span>清晰掌握，自在连接。</span></footer></div></div>
}
