import { useCallback, useEffect, useRef, useState } from 'react'
import { api, errorMessage } from '../api'

// Dashboard-only read polling: preserve the last successful snapshot, bound every read,
// and stop both streams on pause, hidden tabs, or navigation away.
// Reachability follows the actual API result, including loopback-only deployments.
export function useDashboardPoll<T>(path: string, interval: number, paused: boolean) {
  const [data, setData] = useState<T>()
  const [error, setError] = useState('')
  const [updatedAt, setUpdatedAt] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)
  const trigger = useRef<() => void>(() => {})
  const reload = useCallback(() => trigger.current(), [])
  useEffect(() => {
    let active = true, controller: AbortController | undefined, timeout: number | undefined
    const cancel = () => { window.clearTimeout(timeout); controller?.abort(); controller = undefined }
    const load = async (manual = false) => {
      if (controller || document.visibilityState !== 'visible' || (paused && !manual)) return
      const request = new AbortController()
      controller = request
      setLoading(true)
      let timedOut = false
      timeout = window.setTimeout(() => { timedOut = true; request.abort() }, 12_000)
      try {
        const result = await api<T>(path, 'GET', undefined, request.signal)
        if (active && controller === request) { setData(result); setUpdatedAt(Date.now()); setError('') }
      } catch (reason) {
        if (active && controller === request && (!request.signal.aborted || timedOut)) setError(timedOut ? '读取超过 12 秒，保留上次快照，请重试。' : errorMessage(reason))
      } finally {
        if (controller === request) { window.clearTimeout(timeout); controller = undefined; if (active) setLoading(false) }
      }
    }
    const refresh = () => {
      if (document.visibilityState === 'visible') void load()
      else { cancel(); setLoading(false) }
    }
    trigger.current = () => { void load(true) }
    if (paused) setLoading(false)
    else void load()
    const timer = paused ? undefined : window.setInterval(refresh, interval)
    document.addEventListener('visibilitychange', refresh)
    window.addEventListener('online', refresh)
    return () => {
      active = false; cancel(); trigger.current = () => {}; window.clearInterval(timer)
      document.removeEventListener('visibilitychange', refresh)
      window.removeEventListener('online', refresh)
    }
  }, [path, interval, paused])
  return { data, error, updatedAt, loading, reload }
}
