import { useCallback, useEffect, useRef, useState } from 'react'
import { api, errorMessage } from './api'

export function useResource<T>(path: string | null, poll = 5000) {
  const [data, setData] = useState<T>()
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [revision, setRevision] = useState(0)
  const previousPath = useRef<string | null>(null)
  const reload = useCallback(() => setRevision(value => value + 1), [])
  useEffect(() => {
    if (!path) { setData(undefined); setLoading(false); return }
    if (previousPath.current !== path) { setData(undefined); setLoading(true); setError('') }
    previousPath.current = path
    const controller = new AbortController()
    let active = true
    let sequence = 0
    let pending = false
    const load = async () => {
      if (pending) return
      pending = true
      const current = ++sequence
      try {
        const result = await api<T>(path, 'GET', undefined, controller.signal)
        if (active && current === sequence) { setData(result); setError('') }
      } catch (error) {
        if (active && current === sequence) setError(errorMessage(error))
      } finally { pending = false; if (active && current === sequence) setLoading(false) }
    }
    void load()
    const timer = poll ? window.setInterval(() => { if (document.visibilityState === 'visible') void load() }, poll) : undefined
    return () => { active = false; controller.abort(); window.clearInterval(timer) }
  }, [path, poll, revision])
  return { data: previousPath.current === path ? data : undefined, error, loading, reload }
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
