import type { AppRoute } from './routes'

export type NavigationItem = {
  page: AppRoute['page']
  path: string
  label: string
  icon: string
  group: string
}

export const navigation: readonly NavigationItem[] = [
  { page: 'statistics', path: '/statistics', label: '统计仪表盘', icon: 'activity', group: '概览' },
  { page: 'dashboard', path: '/dashboard', label: '服务器看板', icon: 'activity', group: '服务器' },
  { page: 'servers', path: '/servers', label: '服务器', icon: 'server', group: '服务器' },
  { page: 'latency', path: '/latency', label: '延迟检测', icon: 'activity', group: '服务器' },
  { page: 'alicloud', path: '/plugins/alicloud', label: '阿里云 CDT', icon: 'activity', group: '云服务插件' },
  { page: 'ddns', path: '/plugins/ddns', label: '动态域名解析', icon: 'nodes', group: 'DDNS 插件' },
  { page: 'singbox-overview', path: '/plugins/sing-box', label: '代理服务', icon: 'box', group: 'sing-box 插件' },
  { page: 'nodes', path: '/plugins/sing-box/nodes', label: '代理节点', icon: 'nodes', group: 'sing-box 插件' },
  { page: 'proxy-users', path: '/plugins/sing-box/users', label: '代理用户', icon: 'users', group: 'sing-box 插件' },
  { page: 'groups', path: '/plugins/sing-box/groups', label: '策略与套餐', icon: 'nodes', group: 'sing-box 插件' },
  { page: 'catalog', path: '/plugins/catalog', label: '插件目录', icon: 'box', group: '系统' },
  { page: 'plugins', path: '/system/plugins', label: '服务器插件', icon: 'server', group: '系统' },
  { page: 'settings', path: '/system/settings', label: '看板与通知', icon: 'activity', group: '系统' },
  { page: 'notifications', path: '/system/notifications', label: '告警通知', icon: 'activity', group: '系统' },
  { page: 'security', path: '/system/administrator', label: '系统管理员', icon: 'lock', group: '系统' },
]

export function currentNavigation(route: AppRoute): NavigationItem | undefined {
  const page = route.page === 'server' ? (route.section === 'ddns' ? 'ddns' : 'servers') : route.page
  return navigation.find(item => item.page === page)
}
