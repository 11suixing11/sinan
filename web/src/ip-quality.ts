import type { NodeIpQuality, QualityErrorKind } from './types'

export const queryErrorLabels: Record<QualityErrorKind, string> = {
  dns: 'DNS 解析失败', connect: '连接失败', tls: 'TLS 验证或握手失败', timeout: '查询超时',
  http_403: '访问被拒绝（403）', http_429: '请求被限流（429）', http_other: '其他 HTTP 错误',
  non_json: '响应不是 JSON', schema_mismatch: '字段不匹配', body_error: '响应读取失败',
  response_limit: '响应超过上限', request_error: '请求失败，原因未分类', not_public: '未向第三方查询',
  not_attempted: '尚未开始查询', invalid_origin: '查询入口地址无效',
}

export function nodeIpStartError(scope: { serverId: number; fresh: boolean; error: string; data?: NodeIpQuality }, expectedServer: number) {
  if (scope.serverId !== expectedServer) return '已切换服务器，请在当前页面重新提交。'
  if (!scope.fresh || scope.error || !scope.data) return '当前节点自查状态未知，读取恢复后才能创建任务。'
  if (!scope.data.ready) return scope.data.reason || '节点自查尚未就绪，请准备匹配的设备版本和签名制品。'
  if (scope.data.reports.some(record => ['queued', 'running', 'cleaning', 'cancel_requested'].includes(record.status))) return '此服务器已有诊断任务或正在等待清理确认。'
  return ''
}

export function nodeIpCancelError(scope: { serverId: number; fresh: boolean; error: string; data?: NodeIpQuality }, expectedServer: number, jobId: string) {
  if (scope.serverId !== expectedServer || !scope.fresh || scope.error || !scope.data) return '当前任务状态未知，读取恢复后再请求取消。'
  const record = scope.data.reports.find(record => record.id === jobId && record.job.plugin === 'ipquality')
  if (!record || record.agent_completed || !['queued', 'running', 'cleaning'].includes(record.status)) return '任务已经改变，请刷新查看设备确认。'
  if (!scope.data.cancel_supported) return '此设备尚不支持确认式取消，请先升级设备。'
  return ''
}
