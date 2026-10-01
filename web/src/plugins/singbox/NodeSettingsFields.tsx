import { useState } from 'react'
import { Field } from '../../components'
import type { Node } from '../../types'

export type NodeSettings = {
  listen: string; public_port: number | null; tcp_fast_open: boolean; tls_alpn: string[];
  reality: { handshake_server: string | null; handshake_port: number; fingerprint: string };
  hysteria2: { up_mbps: number | null; down_mbps: number | null; ignore_client_bandwidth: boolean; obfs_enabled: boolean };
  tuic: { congestion_control: string; auth_timeout_seconds: number | null; heartbeat_seconds: number | null; zero_rtt_handshake: boolean };
  anytls: { idle_session_check_seconds: number | null; idle_session_timeout_seconds: number | null; min_idle_session: number | null };
}
const number = (form: FormData, key: string) => { const value = String(form.get(key) ?? '').trim(); return value ? Number(value) : null }
export function nodeSettingsRequest(form: FormData) {
  const protocol = String(form.get('protocol'))
  const settings = { listen: String(form.get('listen') ?? '::').trim(), public_port: number(form, 'public_port'), tcp_fast_open: form.get('tcp_fast_open') === 'on', tls_alpn: String(form.get('tls_alpn') ?? '').split(',').map(value => value.trim()).filter(Boolean) }
  if (protocol === 'vless-reality') return { ...settings, reality: { handshake_server: String(form.get('handshake_server') ?? '').trim() || null, handshake_port: number(form, 'handshake_port') ?? 443, fingerprint: String(form.get('fingerprint') ?? 'chrome') } }
  if (protocol === 'hysteria2') return { ...settings, hysteria2: { up_mbps: number(form, 'up_mbps'), down_mbps: number(form, 'down_mbps'), ignore_client_bandwidth: form.get('ignore_client_bandwidth') === 'on', obfs_enabled: form.get('obfs_enabled') === 'on', ...(form.get('obfs_password') ? { obfs_password: String(form.get('obfs_password')) } : {}) } }
  if (protocol === 'tuic') return { ...settings, tuic: { congestion_control: String(form.get('congestion_control') ?? 'cubic'), auth_timeout_seconds: number(form, 'auth_timeout_seconds'), heartbeat_seconds: number(form, 'heartbeat_seconds'), zero_rtt_handshake: form.get('zero_rtt_handshake') === 'on' } }
  if (protocol === 'anytls') return { ...settings, anytls: { idle_session_check_seconds: number(form, 'idle_session_check_seconds'), idle_session_timeout_seconds: number(form, 'idle_session_timeout_seconds'), min_idle_session: number(form, 'min_idle_session') } }
  return settings
}

export function ConnectionFields({ node }: { node: Node | 'new' }) {
  const settings = node === 'new' ? undefined : node.settings
  return <>
    <Field label="监听地址" hint="填写服务器本机 IP。:: 监听 IPv6，通常同时接受 IPv4；仅 IPv4 可填 0.0.0.0。"><input name="listen" required defaultValue={settings?.listen ?? '::'} maxLength={45} spellCheck={false} /></Field>
    <Field label="公开端口" hint="客户端连接使用的端口。留空跟随监听端口；有 NAT 映射时填写映射后的端口。"><input name="public_port" type="number" min={1} max={65535} step={1} defaultValue={settings?.public_port ?? ''} placeholder="与监听端口相同" /></Field>
    <label className="node-switch"><input name="enabled" type="checkbox" defaultChecked={node === 'new' || node.enabled !== false} /><span>启用节点<small>停用后保留授权与历史流量，等待设备应用配置后停止监听。</small></span></label>
  </>
}

function Seconds({ name, label, value, hint }: { name: string; label: string; value?: number | null; hint?: string }) {
  return <Field label={label} hint={hint ?? '留空使用运行时默认值。'}><input name={name} type="number" min={1} max={3600} step={1} defaultValue={value ?? ''} placeholder="使用默认值" /></Field>
}

export default function ProtocolSettings({ protocol, node }: { protocol: string; node: Node | 'new' }) {
  const settings = node === 'new' ? undefined : node.settings
  const [obfs, setObfs] = useState(settings?.hysteria2?.obfs_enabled ?? false)
  const [bbr, setBbr] = useState(settings?.hysteria2?.ignore_client_bandwidth ?? false)
  const tls = ['hysteria2', 'tuic', 'anytls', 'naive'].includes(protocol)
  const tcp = !['hysteria2', 'tuic'].includes(protocol)
  return <details className="node-advanced"><summary>协议高级设置</summary><div className="node-fields-grid">
    {tcp && <label className="node-switch"><input name="tcp_fast_open" type="checkbox" defaultChecked={settings?.tcp_fast_open ?? false} /><span>TCP Fast Open<small>默认关闭，需要服务器系统支持。</small></span></label>}
    {tls && <Field label="TLS ALPN" hint={protocol === 'naive' ? '留空使用默认值；自定义仅支持 h2。' : '多个值用英文逗号分隔。留空沿用默认值，Hysteria2 / TUIC 默认为 h3。'}><input name="tls_alpn" maxLength={263} defaultValue={settings?.tls_alpn?.join(', ') ?? ''} placeholder={['hysteria2', 'tuic'].includes(protocol) ? 'h3' : protocol === 'naive' ? 'h2' : '使用默认值'} autoComplete="off" /></Field>}
    {protocol === 'vless-reality' && <>
      <Field label="Reality 握手目标" hint="留空跟随伪装域名。可指定域名或 IP，不含端口与路径。"><input name="handshake_server" defaultValue={settings?.reality?.handshake_server ?? ''} placeholder="与伪装域名相同" maxLength={253} spellCheck={false} /></Field>
      <Field label="Reality 握手端口"><input name="handshake_port" type="number" min={1} max={65535} defaultValue={settings?.reality?.handshake_port ?? 443} required /></Field>
      <Field label="客户端 TLS 指纹" hint="同步写入 sing-box 配置与 VLESS 分享链接。"><select name="fingerprint" defaultValue={settings?.reality?.fingerprint ?? 'chrome'}>{[['chrome', 'Chrome'], ['firefox', 'Firefox'], ['safari', 'Safari'], ['edge', 'Edge'], ['ios', 'iOS'], ['android', 'Android'], ['randomized', '随机']].map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></Field>
    </>}
    {protocol === 'hysteria2' && <>
      <label className="node-switch"><input name="ignore_client_bandwidth" type="checkbox" checked={bbr} onChange={event => setBbr(event.target.checked)} /><span>强制使用 BBR<small>与手动带宽互斥。关闭且留空带宽时沿用运行时协商。</small></span></label>
      <Field label="服务器上传带宽（Mbps）" hint="对应客户端下行；上下行同时填写或同时留空。"><input name="up_mbps" type="number" min={1} max={1000000} step={1} disabled={bbr} defaultValue={settings?.hysteria2?.up_mbps ?? ''} placeholder="不限制" /></Field>
      <Field label="服务器下载带宽（Mbps）" hint="对应客户端上行，订阅自动换算方向。"><input name="down_mbps" type="number" min={1} max={1000000} step={1} disabled={bbr} defaultValue={settings?.hysteria2?.down_mbps ?? ''} placeholder="不限制" /></Field>
      <label className="node-switch"><input name="obfs_enabled" type="checkbox" checked={obfs} onChange={event => setObfs(event.target.checked)} /><span>启用 Salamander 混淆<small>服务端与授权客户端自动使用同一混淆参数。</small></span></label>
      {obfs && <Field label="混淆密码" hint={settings?.hysteria2?.obfs_enabled ? '已配置，留空保留；填写新值后客户端需更新订阅。' : '留空自动生成。也可填写 8 至 256 字节的密码。'}><input name="obfs_password" type="password" minLength={8} maxLength={256} autoComplete="new-password" /></Field>}
    </>}
    {protocol === 'tuic' && <>
      <Field label="拥塞控制"><select name="congestion_control" defaultValue={settings?.tuic?.congestion_control ?? 'cubic'}><option value="cubic">CUBIC（默认）</option><option value="bbr">BBR</option><option value="new_reno">New Reno</option></select></Field>
      <Seconds name="auth_timeout_seconds" label="认证超时（秒）" value={settings?.tuic?.auth_timeout_seconds} />
      <Seconds name="heartbeat_seconds" label="心跳间隔（秒）" value={settings?.tuic?.heartbeat_seconds} hint="同时作用于服务器与客户端。" />
      <label className="node-switch"><input name="zero_rtt_handshake" type="checkbox" defaultChecked={settings?.tuic?.zero_rtt_handshake ?? false} /><span>启用 0-RTT 握手<small>减少握手等待，但存在重放风险；默认关闭。</small></span></label>
    </>}
    {protocol === 'anytls' && <>
      <Seconds name="idle_session_check_seconds" label="闲置会话检查间隔（秒）" value={settings?.anytls?.idle_session_check_seconds} hint="客户端设置，留空使用 30 秒默认值。" />
      <Seconds name="idle_session_timeout_seconds" label="闲置会话超时（秒）" value={settings?.anytls?.idle_session_timeout_seconds} hint="客户端设置，留空使用 30 秒默认值。" />
      <Field label="最少保留闲置会话" hint="客户端设置，留空或 0 不额外保留。"><input name="min_idle_session" type="number" min={0} max={128} step={1} defaultValue={settings?.anytls?.min_idle_session ?? ''} placeholder="0" /></Field>
    </>}
  </div></details>
}
