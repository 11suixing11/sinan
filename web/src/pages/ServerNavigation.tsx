export default function ServerNavigation({ id, active }: { id: number; active: 'overview' | 'ip-info' | 'node-quality' | 'tcp-quality' }) {
  const pages = [{ key: 'overview', path: '', label: '服务器概况' }, { key: 'ip-info', path: '/ip-info', label: 'IP 信息' }, { key: 'node-quality', path: '/node-quality', label: 'NodeQuality 验机' }, { key: 'tcp-quality', path: '/tcp-quality', label: 'TCP 诊断' }]
  return <nav className="server-navigation" aria-label="服务器导航">{pages.map(page => <a key={page.key} href={`#/servers/${id}${page.path}`} className={`button ${active === page.key ? 'button-primary' : 'button-secondary'}`} aria-current={active === page.key ? 'page' : undefined}>{page.label}</a>)}</nav>
}
