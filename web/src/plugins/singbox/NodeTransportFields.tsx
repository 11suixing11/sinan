import { useState } from 'react'
import { Field } from '../../components'
import type { NodeSettings } from './nodeSettingsForm'

export default function TransportFields({ settings }: { settings?: NodeSettings }) {
  const previous = settings?.transport
  const [type, setType] = useState(previous?.type ?? 'tcp')
  const [flow, setFlow] = useState(settings?.reality?.flow ?? 'vision')
  const [path, setPath] = useState(previous && 'path' in previous ? previous.path : '/')
  const [host, setHost] = useState(previous && 'host' in previous ? previous.host ?? '' : '')
  const [early, setEarly] = useState(previous?.type === 'ws' ? String(previous.max_early_data) : '0')
  const [header, setHeader] = useState(previous?.type === 'ws' ? previous.early_data_header_name : '')
  const [service, setService] = useState(previous?.type === 'grpc' ? previous.service_name : '')
  return <details className="node-advanced"><summary>传输设置</summary><div className="node-fields-grid">
    <Field label="传输方式"><select name="transport_type" value={type} onChange={event => setType(event.target.value as typeof type)}><option value="tcp">TCP（默认）</option><option value="ws">WebSocket</option><option value="httpupgrade">HTTPUpgrade</option><option value="grpc">gRPC</option></select></Field>
    <Field label="VLESS 流控" hint={type === 'tcp' ? 'Vision 仅适用于 TCP 直传。' : '当前传输不使用 Vision。'}><select name="reality_flow" value={type === 'tcp' ? flow : 'none'} disabled={type !== 'tcp'} onChange={event => setFlow(event.target.value as typeof flow)}><option value="vision">Vision（默认）</option><option value="none">无流控</option></select></Field>
    {(type === 'ws' || type === 'httpupgrade') && <>
      <Field label="请求路径"><input name="transport_path" value={path} onChange={event => setPath(event.target.value)} maxLength={2048} required pattern="/.*" placeholder="/" autoComplete="off" /></Field>
      <Field label="请求主机名" hint="留空使用客户端连接地址。"><input name="transport_host" value={host} onChange={event => setHost(event.target.value)} maxLength={253} placeholder="可选" autoComplete="off" /></Field>
    </>}
    {type === 'ws' && <>
      <Field label="提前数据大小（字节）" hint="0 表示关闭，服务端与订阅保持一致。"><input name="max_early_data" type="number" min={0} max={65535} step={1} value={early} onChange={event => setEarly(event.target.value)} /></Field>
      <Field label="提前数据请求头"><input disabled={Number(early) <= 0} name="early_data_header_name" value={header} onChange={event => setHeader(event.target.value)} maxLength={128} placeholder="例如 Sec-WebSocket-Protocol" autoComplete="off" /></Field>
    </>}
    {type === 'grpc' && <Field label="gRPC 服务名称"><input name="service_name" value={service} onChange={event => setService(event.target.value)} maxLength={128} autoComplete="off" /></Field>}
  </div></details>
}
