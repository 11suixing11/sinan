import type { ReactNode } from 'react'
import { Brand, ErrorNotice, Icon } from '../components'
import { navigation } from './navigation'
import type { NavigationItem } from './navigation'

type Props = {
  current?: NavigationItem
  busy: boolean
  error: string
  onLogout: () => void
  children: ReactNode
}

export default function AdminShell({ current, busy, error, onLogout, children }: Props) {
  return <div className="app-shell">
    <aside className="sidebar">
      <a href="#/servers" className="brand-link" aria-label="司南首页"><Brand /></a>
      <div className="nav-caption">控制面板</div>
      <nav aria-label="主导航">
        {navigation.map((item, index) => <div className="nav-entry" key={item.path}>
          {navigation[index - 1]?.group !== item.group && <div className="nav-group-label">{item.group}</div>}
          <a href={`#${item.path}`} className={current?.path === item.path ? 'active' : ''}
            aria-current={current?.path === item.path ? 'page' : undefined}>
            <Icon name={item.icon} size={20} /><span>{item.label}</span>
            {current?.path === item.path && <span className="nav-active-dot" />}
          </a>
        </div>)}
      </nav>
      <div className="sidebar-bottom">
        <div className="sidebar-note"><span className="status-dot" /><span>自托管控制面板</span></div>
        <button className="logout-button" disabled={busy} onClick={onLogout}>
          <span className="admin-avatar">管</span>
          <span><strong>管理员</strong><small>退出登录</small></span>
          <Icon name="logout" size={17} />
        </button>
      </div>
    </aside>
    <div className="main-layout">
      <div className="topbar">
        <div>控制面板<span>/</span><strong>{current?.label ?? '页面不存在'}</strong></div>
        <span className="topbar-status"><span className="status-dot" />管理员会话已登录</span>
      </div>
      <main className="content"><ErrorNotice message={error} />{children}</main>
      <footer className="app-footer"><span>司南</span><span>清晰掌握，自在连接。</span></footer>
    </div>
  </div>
}
