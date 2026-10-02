import { useCallback, useEffect, useRef, useState } from 'react'
import { api, ApiError, errorMessage } from '../api'

type Snapshot<T> = { path: string | null; data?: T; error: string; updatedAt: number | null; loading: boolean; denied: boolean }
const empty = <T,>(path: string | null): Snapshot<T> => ({ path, error: '', updatedAt: null, loading: Boolean(path), denied: false })

// Every request has its own identity. Hidden pages abort in-flight reads, and a
// late response from an old route or cleared authorization cannot restore data.
export function useDashboardPoll<T>(path: string | null, interval: number, paused: boolean, identity = '') {
  const key = path === null ? null : `${identity}\n${path}`
  const [snapshot, setSnapshot] = useState<Snapshot<T>>(() => empty(key))
  const trigger = useRef<() => void>(() => {})
  const reset = useRef<() => void>(() => {})
  const reload = useCallback(() => trigger.current(), [])
  const clear = useCallback(() => reset.current(), [])
  useEffect(() => {
    let active = true, controller: AbortController | undefined, timeout: number | undefined
    const cancel = () => { window.clearTimeout(timeout); const pending = controller; controller = undefined; pending?.abort() }
    const update = (patch: Partial<Snapshot<T>>) => setSnapshot(current => ({ ...(current.path === key ? current : empty<T>(key)), ...patch }))
    update({ loading: Boolean(path) && !paused })
    const load = async (manual = false) => {
      if (!path || controller || document.visibilityState !== 'visible' || (paused && !manual)) return
      const request = new AbortController(); controller = request
      update({ loading: true })
      let timedOut = false
      timeout = window.setTimeout(() => { timedOut = true; request.abort() }, 12_000)
      try {
        const result = await api<T>(path, 'GET', undefined, request.signal)
        if (active && controller === request && !request.signal.aborted) update({ data: result, updatedAt: Date.now(), error: '', denied: false })
      } catch (reason) {
        if (active && controller === request && (!request.signal.aborted || timedOut)) {
          const denied = reason instanceof ApiError && [401, 403, 404].includes(reason.status)
          update({ error: timedOut ? '读取超过 12 秒，保留上次快照，请重试。' : errorMessage(reason), denied, ...(denied ? { data: undefined, updatedAt: null } : {}) })
        }
      } finally {
        if (controller === request) { window.clearTimeout(timeout); controller = undefined; if (active) update({ loading: false }) }
      }
    }
    const refresh = () => {
      if (document.visibilityState === 'visible') void load()
      else { cancel(); update({ loading: false }) }
    }
    trigger.current = () => { void load(true) }
    reset.current = () => { cancel(); update({ data: undefined, updatedAt: null, loading: false }) }
    if (!paused) void load()
    const timer = paused || !path ? undefined : window.setInterval(refresh, interval)
    document.addEventListener('visibilitychange', refresh)
    window.addEventListener('online', refresh)
    return () => {
      active = false; cancel(); trigger.current = () => {}; reset.current = () => {}; window.clearInterval(timer)
      document.removeEventListener('visibilitychange', refresh); window.removeEventListener('online', refresh)
    }
  }, [path, key, interval, paused])
  return { ...(snapshot.path === key ? snapshot : empty<T>(key)), reload, clear }
}
