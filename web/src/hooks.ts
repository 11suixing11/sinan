import { useCallback, useEffect, useRef, useState } from 'react'
import { api, ApiError, errorMessage } from './api'

export function resourceWriteError(...resources: { fresh: boolean; error: string }[]) {
  const failure = resources.find(resource => resource.error)
  if (failure) return `最新信息读取失败，暂不能修改；草稿已保留。${failure.error}`
  return resources.some(resource => !resource.fresh) ? '正在刷新相关信息，暂不能修改；草稿已保留。' : ''
}

export function useResource<T>(path: string | null, poll = 5000) {
  const [data, setData] = useState<T>()
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [refreshing, setRefreshing] = useState(true)
  const [revision, setRevision] = useState(0)
  const previousPath = useRef<string | null>(null)
  const reload = useCallback(() => { setRefreshing(true); setRevision(value => value + 1) }, [])
  useEffect(() => {
    if (!path) { setData(undefined); setLoading(false); setRefreshing(false); return }
    if (previousPath.current !== path) { setData(undefined); setLoading(true); setError('') }
    previousPath.current = path
    const controller = new AbortController()
    let active = true
    let sequence = 0
    let pending = false
    const load = async () => {
      if (pending) return
      pending = true
      setRefreshing(true)
      const current = ++sequence
      try {
        const result = await api<T>(path, 'GET', undefined, controller.signal)
        if (active && current === sequence) { setData(result); setError('') }
      } catch (error) {
        if (active && current === sequence) {
          setError(errorMessage(error))
          if (path.startsWith('/api/dashboard/') && error instanceof ApiError && [401, 403, 404].includes(error.status)) setData(undefined)
        }
      } finally { pending = false; if (active && current === sequence) { setLoading(false); setRefreshing(false) } }
    }
    void load()
    const timer = poll ? window.setInterval(() => { if (document.visibilityState === 'visible') void load() }, poll) : undefined
    return () => { active = false; controller.abort(); window.clearInterval(timer) }
  }, [path, poll, revision])
  const currentData = previousPath.current === path ? data : undefined
  return { data: currentData, error, loading, refreshing, fresh: currentData !== undefined && currentData !== null && !error && !refreshing, reload }
}

export function useAction() {
  const alive = useRef(true)
  const locked = useRef(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  useEffect(() => { alive.current = true; return () => { alive.current = false } }, [])
  const run = async <T,>(task: () => Promise<T>, success: (value: T) => void) => {
    if (locked.current) return
    locked.current = true
    setBusy(true); setError('')
    try { const result = await task(); if (alive.current) success(result) }
    catch (error) { if (alive.current) setError(errorMessage(error)) }
    finally { locked.current = false; if (alive.current) setBusy(false) }
  }
  return { busy, error, run, clearError: () => setError('') }
}
