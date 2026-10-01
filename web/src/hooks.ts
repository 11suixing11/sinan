import { useCallback, useEffect, useRef, useState } from 'react'
import { api, ApiError, errorMessage } from './api'

export function useResource<T>(path: string | null, poll = 5000) {
  const [data, setData] = useState<T>()
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [revision, setRevision] = useState(0)
  const [ready, setReady] = useState(false)
  const previousPath = useRef<string | null>(null)
  const currentPath = useRef(path)
  currentPath.current = path
  const generation = useRef(0)
  const snapshot = useRef<{ path: string | null; valid: boolean; value?: T }>({ path: null, valid: false })
  const reload = useCallback(() => {
    ++generation.current
    snapshot.current.valid = false
    setReady(false); setLoading(true)
    setRevision(value => value + 1)
  }, [])
  const isCurrent = useCallback(() => currentPath.current === path && snapshot.current.valid && snapshot.current.path === path, [path])
  const getCurrent = useCallback(() => isCurrent() ? snapshot.current.value : undefined, [isCurrent])
  useEffect(() => {
    if (!path) { snapshot.current = { path: null, valid: false }; setData(undefined); setReady(false); setLoading(false); return }
    if (previousPath.current !== path) { setData(undefined); setLoading(true); setError('') }
    previousPath.current = path
    const controller = new AbortController()
    let active = true
    let sequence = 0
    let pending = false
    const load = async () => {
      if (pending) return
      pending = true
      const current = ++sequence, epoch = ++generation.current
      snapshot.current.valid = false
      setReady(false); setLoading(true)
      try {
        const result = await api<T>(path, 'GET', undefined, controller.signal)
        if (active && current === sequence && epoch === generation.current) {
          snapshot.current = { path, valid: true, value: result }
          setData(result); setError(''); setReady(true)
        }
      } catch (error) {
        if (active && current === sequence && epoch === generation.current) {
          setError(errorMessage(error))
          if (path.startsWith('/api/dashboard/') && error instanceof ApiError && [401, 403, 404].includes(error.status)) setData(undefined)
        }
      } finally { pending = false; if (active && current === sequence && epoch === generation.current) setLoading(false) }
    }
    void load()
    const timer = poll ? window.setInterval(() => { if (document.visibilityState === 'visible') void load() }, poll) : undefined
    return () => { active = false; snapshot.current.valid = false; ++generation.current; controller.abort(); window.clearInterval(timer) }
  }, [path, poll, revision])
  return { data: previousPath.current === path ? data : undefined, error, loading, ready: previousPath.current === path && ready, reload, isCurrent, getCurrent }
}

export type ResourceState<T> = ReturnType<typeof useResource<T>>

export function resourceWriteError(...resources: { isCurrent: () => boolean }[]): string {
  return resources.every(resource => resource.isCurrent()) ? '' : '相关信息正在刷新或刷新失败，请成功刷新后再提交；当前草稿已保留。'
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
