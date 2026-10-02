import { useState } from 'react'
import { api } from '../api'
import { ErrorNotice } from '../components'
import { time } from '../format'
import { useAction, useResource } from '../hooks'
import ServerProbes from './ServerProbes'

type Command = { spec: { id: string; command: string; timeout_secs: number; expires_at: number }; requested_at: number; result: null | { status: 'succeeded' | 'failed' | 'expired' | 'interrupted'; stdout: string; stderr: string; finished_at: number; timed_out: boolean; truncated: boolean } }

export default function AgentTasks({ serverId, commandsEnabled }: { serverId: number; commandsEnabled: boolean }) {
  const base = `/api/servers/${serverId}`
  const commands = useResource<Command[]>(`${base}/commands`)
  const commandAction = useAction()
  const [command, setCommand] = useState(''), [seconds, setSeconds] = useState(30), [ttl, setTtl] = useState(300)
  const labels = { succeeded: '已完成', failed: '执行失败', expired: '已过期', interrupted: '执行被中断' }
  return <>
    <ServerProbes serverId={serverId} />
    <section className="panel"><div className="panel-heading"><h2>远程命令</h2><span className="subtle">设备服务账号执行</span></div><div className="panel-body"><ErrorNotice message={commands.error || commandAction.error} />
      {!commandsEnabled && <p className="helper">远程命令默认关闭。需要在节点本机的 Agent 配置中启用 allow_remote_commands 并重启后才能使用。</p>}
      <form onSubmit={event => { event.preventDefault(); if (!commandsEnabled) return; void commandAction.run(() => api(`${base}/commands`, 'POST', { command, timeout_secs: seconds, ttl_secs: ttl }), () => { setCommand(''); commands.reload() }) }}>
        <label>命令<textarea required maxLength={16384} rows={4} value={command} onChange={event => setCommand(event.target.value)} placeholder="输入要在设备上执行的命令" /></label><p className="helper">Unix 使用系统 Shell，Windows 使用 PowerShell。命令可能修改设备；执行中断后不会自动重试。</p>
        <div className="form-grid"><label>执行超时（秒）<input required type="number" min={1} max={600} value={seconds} onChange={event => setSeconds(Number(event.target.value))} /></label><label>设备领取期限（秒）<input required type="number" min={1} max={86400} value={ttl} onChange={event => setTtl(Number(event.target.value))} /></label></div>
        <button className="button button-primary" type="submit" disabled={commandAction.busy || !commandsEnabled}>提交命令</button>
      </form></div><div className="panel-body">{commands.data?.length ? commands.data.map(item => <details key={item.spec.id}><summary>{item.result ? labels[item.result.status] : item.spec.expires_at * 1000 < Date.now() ? '领取已过期' : '等待结果'} · {time(item.requested_at)}</summary><pre className="break-all">{item.spec.command}</pre>{item.result && <><p>{item.result.timed_out && '已达到执行超时。'}{item.result.truncated && '输出超过上限，已截断。'}</p><h4>标准输出</h4><pre className="break-all">{item.result.stdout || '（无输出）'}</pre><h4>错误输出</h4><pre className="break-all">{item.result.stderr || '（无输出）'}</pre></>}</details>) : <p className="subtle">尚未执行远程命令。</p>}</div></section>
  </>
}
