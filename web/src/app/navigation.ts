import type { AppRoute } from './routes'

export type NavigationItem = {
  page: AppRoute['page']
  path: string
  label: string
  icon: string
  group: string
}

type NavigationGroup = { label: string; items: readonly Omit<NavigationItem, 'group'>[] }

// Groups follow the administrator's tasks: observe, manage servers, run the proxy
// business, extend servers with plugins, then configure the panel itself.
const groups: readonly NavigationGroup[] = [
  { label: '概览', items: [
    { page: 'statistics', path: '/statistics', label: '统计仪表盘', icon: 'activity' },
    { page: 'dashboard', path: '/dashboard', label: '服务器看板', icon: 'activity' },
  ] },
  { label: '服务器', items: [
    { page: 'servers', path: '/servers', label: '服务器', icon: 'server' },
    { page: 'latency', path: '/latency', label: '延迟检测', icon: 'activity' },
  ] },
  { label: '代理服务', items: [
    { page: 'singbox-overview', path: '/plugins/sing-box', label: '代理服务', icon: 'box' },
    { page: 'nodes', path: '/plugins/sing-box/nodes', label: '代理节点', icon: 'nodes' },
    { page: 'proxy-users', path: '/plugins/sing-box/users', label: '代理用户', icon: 'users' },
    { page: 'groups', path: '/plugins/sing-box/groups', label: '策略与套餐', icon: 'nodes' },
  ] },
  { label: '扩展插件', items: [
    { page: 'ddns', path: '/plugins/ddns', label: '动态域名解析', icon: 'nodes' },
    { page: 'alicloud', path: '/plugins/alicloud', label: '阿里云 CDT', icon: 'activity' },
    { page: 'plugins', path: '/system/plugins', label: '服务器插件', icon: 'server' },
    { page: 'catalog', path: '/plugins/catalog', label: '插件目录', icon: 'box' },
  ] },
  { label: '系统', items: [
    { page: 'settings', path: '/system/settings', label: '看板与通知', icon: 'activity' },
    { page: 'notifications', path: '/system/notifications', label: '告警通知', icon: 'activity' },
    { page: 'security', path: '/system/administrator', label: '系统管理员', icon: 'lock' },
  ] },
]

export const navigation: readonly NavigationItem[] = groups.flatMap(group => group.items.map(item => ({ ...item, group: group.label })))

export function currentNavigation(route: AppRoute): NavigationItem | undefined {
  const page = route.page === 'server' ? (route.section === 'ddns' ? 'ddns' : 'servers') : route.page
  return navigation.find(item => item.page === page)
}
