export type SubscriptionSource = {
  id: number
  name: string
  kind: 'url' | 'inline'
  source_host: string | null
  url_configured: boolean
  authorization_configured: boolean
  content_configured: boolean
  settings_revision: number
  identity_epoch: number
  refresh_interval_seconds: number
  auto_refresh?: boolean
  user_agent?: string
  traffic?: { upload?: number; download?: number; total?: number; expire?: number; updated_at?: number }
  changes?: { added: number; updated: number; missing: number; unsupported: number }
  stale?: boolean
  archived: boolean
  current_revision_id: number | null
  last_attempt_at: number | null
  last_success_at: number | null
  last_error: string | null
  supported_count: number
  unsupported_count: number
  active_job_id: string | null
  dependency_ids: number[]
}

export type ExternalNodePreview = {
  id: number | null
  source_id: number
  node_version_id: number | null
  source_revision_id: number | null
  identity_epoch: number
  name: string
  protocol: string | null
  server: string | null
  port: number | null
  transport: string | null
  tcp: boolean
  udp: boolean
  selectable: boolean
  present: boolean
  identity_unique: boolean
  adopted?: boolean
  reason: string | null
}

export type SubscriptionHopReference = {
  kind: 'subscription'
  source_id: number
  external_node_id: number
  node_version_id: number
  update_mode: 'follow_node' | 'pinned'
}

export type SubscriptionSourceJob = {
  id: string
  source_id: number
  settings_revision: number
  identity_epoch: number
  state: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled' | 'superseded'
  phase: string
  error_code: string | null
  result_revision_id: number | null
  created_at: number
  started_at: number | null
  finished_at: number | null
}

const reasons: Record<string, string> = {
  source_deleted: '来源已删除',
  unsupported_configuration: '当前节点配置不可用',
  node_not_adopted: '尚未加入节点库',
  node_disabled: '节点已停用',
  source_archived: '来源已归档，不能用于新链路',
  source_replaced: '来源已更换，需要重新选点；原链路保留旧版本',
  node_missing: '所选节点本次缺失，原链路保留旧版本',
  ambiguous_node_identity: '同一端点存在多个身份，不能自动匹配原节点',
  unsupported_proxy_protocol: '此代理协议暂不支持',
  unsupported_or_invalid_proxy_parameter: '代理参数不受支持或校验未通过',
  unsupported_mihomo_parameter: '此节点包含暂不支持的 Mihomo 参数',
  unsupported_uri_parameter: '分享链接包含暂不支持的参数',
  unsupported_ss_plugin: '此节点依赖暂不支持的 Shadowsocks 插件',
  unsupported_proxy_transport: '此传输方式暂不支持',
  provider_url_without_nodes: '配置只有提供方地址，请使用包含具体节点的订阅地址',
  duplicate_yaml_key: '配置中存在重复字段',
  invalid_json_or_limit: 'JSON 内容无效、字段重复或超过结构限制',
  invalid_yaml: 'YAML 内容无效',
  recursive_or_unknown_yaml_alias: 'YAML 包含递归或未定义引用',
  structure_limit: '配置结构或引用展开超过限制',
  scalar_limit: '单项配置内容过长',
  body_limit: '订阅正文超过 2 MiB',
  decompressed_body_limit: '订阅解压后超过 2 MiB',
  node_count_limit: '节点集合为空或超过 5000 项',
  html_response: '订阅返回了网页，请检查订阅地址或认证',
  download_timeout: '获取订阅超时',
  download_failed: '无法获取订阅，请检查地址及认证',
  source_http_failed: '来源未返回成功响应，请检查订阅是否有效',
  source_dns_failed: '无法解析来源主机',
  non_public_source_address: '来源地址解析到了非公网地址，已拒绝访问',
  cross_origin_redirect: '来源跳转到了其他网站，请明确修改订阅地址',
  redirect_limit: '来源跳转次数超过限制',
  invalid_source_url: '订阅地址不符合 HTTPS 公网来源要求',
  invalid_redirect: '来源返回了无效的跳转地址',
  invalid_source_request: '来源请求设置无效，请检查获取认证',
  unsupported_content_encoding: '来源压缩方式暂不支持',
  cancelled: '本次刷新已取消，已有版本保持不变',
  worker_interrupted: '上次刷新被中断，可重新刷新',
  storage_failed: '保存解析结果失败，可重新刷新',
}

export function sourceReason(reason: string | null | undefined): string {
  return reason ? reasons[reason] ?? '订阅内容或节点参数校验未通过' : ''
}

export function sourceStatus(source: SubscriptionSource): string {
  if (source.archived) return '已归档'
  if (source.active_job_id) return source.current_revision_id ? '更新中，保留上次版本' : '正在获取与解析'
  if (source.last_error === 'cancelled') return source.current_revision_id ? '已取消刷新，保留上次版本' : '已取消导入'
  if (source.last_error) return source.current_revision_id ? '更新失败，使用上次版本' : '导入失败'
  if (!source.current_revision_id) return '等待解析'
  return source.supported_count ? '已有可选节点' : '没有可选节点'
}

export function hopReference(node: ExternalNodePreview, updateMode: SubscriptionHopReference['update_mode']): SubscriptionHopReference | null {
  if (!node.selectable || node.id === null || node.node_version_id === null) return null
  return { kind: 'subscription', source_id: node.source_id, external_node_id: node.id, node_version_id: node.node_version_id, update_mode: updateMode }
}
