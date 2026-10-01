import { useEffect, useState } from 'react'
import { Brand } from '../components'
import { Icon } from './Icon'
import Overview from './Overview'
import ServerView from './ServerView'
import { dashboardHome } from './dashboard'
import FullscreenButton from './FullscreenButton'
import { CurrencyProvider } from './CurrencyContext'
import './display.css'
import './dashboard.css'

function useTheme() {
  const [theme, setTheme] = useState<'light' | 'dark'>(() => {
    try { const saved = localStorage.getItem('sinan-display-theme'); if (saved === 'light' || saved === 'dark') return saved } catch { /* Storage may be disabled. */ }
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  })
  const toggle = () => setTheme(current => {
    const next = current === 'light' ? 'dark' : 'light'
    try { localStorage.setItem('sinan-display-theme', next) } catch { /* The in-memory choice still works. */ }
    return next
  })
  return { theme, toggle }
}

export default function ServerDisplay({ serverId }: { serverId?: number }) {
  const { theme, toggle } = useTheme()
  const [now, setNow] = useState(Date.now)
  useEffect(() => {
    document.body.classList.add('has-server-display')
    const timer = window.setInterval(() => { if (document.visibilityState === 'visible') setNow(Date.now()) }, 5000)
    const visible = () => { if (document.visibilityState === 'visible') setNow(Date.now()) }
    document.addEventListener('visibilitychange', visible)
    return () => { document.body.classList.remove('has-server-display'); window.clearInterval(timer); document.removeEventListener('visibilitychange', visible) }
  }, [])
  return <CurrencyProvider><div className="server-display" data-theme={theme}>
    <a className="d-skip" href="#display-main" onClick={event => { event.preventDefault(); document.getElementById('display-main')?.focus() }}>跳到服务器信息</a>
    <header className="d-site-header"><div className="d-container d-header-inner"><a className="d-brand" href={dashboardHome} aria-label="司南服务器看板"><Brand /></a><div className="d-header-actions"><FullscreenButton /><button className="d-icon-button" aria-label={theme === 'dark' ? '切换浅色主题' : '切换深色主题'} title={theme === 'dark' ? '浅色主题' : '深色主题'} onClick={toggle}><Icon name={theme === 'dark' ? 'sun' : 'moon'} /></button><a className="d-admin-link" href={serverId ? `#/servers/${serverId}` : '#/servers'}><Icon name="user" /><span>进入后台</span></a></div></div></header>
    <main className="d-container d-main" id="display-main" tabIndex={-1}>{serverId ? <ServerView key={serverId} id={serverId} now={now} /> : <Overview now={now} />}</main>
    <footer className="d-container d-site-footer"><span>司南 · 服务器看板</span><div><a href="https://github.com/theLucius7/sinan" target="_blank" rel="noreferrer">项目源码</a><a href="/notices/NodeFlare.txt" target="_blank" rel="noreferrer">界面来源与许可</a></div></footer>
  </div></CurrencyProvider>
}
