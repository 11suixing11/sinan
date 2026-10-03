import { useCallback, useEffect, useRef, useState } from 'react'
import { api, ApiError, errorMessage } from './api'

export const resourceRefreshingMessage = '相关信息正在刷新或刷新失败，请成功刷新后再提交；当前草稿已保留。'
const backgroundNoticeDelay = 250

export function useResource<T>(path: string | null, poll = 5000) {
  const [data, setData] = useState<T>()
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [revision, setRevision] = useState(0)
  const [availability, setAvailability] = useState({ ready: false })
  const previousPath = useRef<string | null>(null)
  const currentPath = useRef(path)
  currentPath.current = path
  const generation = useRef(0)
  const snapshot = useRef<{ path: string | null; valid: boolean; value?: T }>({ path: null, valid: false })
  const reload = useCallback(() => {
    ++generation.current
    snapshot.current.valid = false
    setAvailability({ ready: false }); setLoading(true)
    setRevision(value => value + 1)
  }, [])
  const isCurrent = useCallback(() => currentPath.current === path && snapshot.current.valid && snapshot.current.path === path, [path])
  const getCurrent = useCallback(() => isCurrent() ? snapshot.current.value : undefined, [isCurrent])
  useEffect(() => {
    if (!path) { snapshot.current = { path: null, valid: false }; setData(undefined); setAvailability({ ready: false }); setLoading(false); return }
    if (previousPath.current !== path) { setData(undefined); setLoading(true); setError('') }
    previousPath.current = path
    const controller = new AbortController()
    let active = true
    let sequence = 0
    let pending = false
    let presentation: number | undefined
    const load = async (background = false) => {
      if (pending) return
      pending = true
      const current = ++sequence, epoch = ++generation.current
      snapshot.current.valid = false
      // Writes are blocked immediately, even before React renders. Brief polls
      // retain the current view; a slow read still exposes its unavailable state.
      if (background && snapshot.current.value !== undefined) {
        presentation = window.setTimeout(() => { if (active && epoch === generation.current) setAvailability({ ready: false }) }, backgroundNoticeDelay)
      } else { setAvailability({ ready: false }); setLoading(true) }
      try {
        const result = await api<T>(path, 'GET', undefined, controller.signal)
        if (active && current === sequence && epoch === generation.current) {
          snapshot.current = { path, valid: true, value: result }
          setData(previous => JSON.stringify(previous) === JSON.stringify(result) ? previous : result)
          // Publish completion even for unchanged data: another component may
          // have rendered a disabled action while this request was pending.
          setError(''); setAvailability({ ready: true })
        }
      } catch (error) {
        if (active && current === sequence && epoch === generation.current) {
          setError(errorMessage(error)); setAvailability({ ready: false })
          if (path.startsWith('/api/dashboard/') && error instanceof ApiError && [401, 403, 404].includes(error.status)) setData(undefined)
        }
      } finally { window.clearTimeout(presentation); pending = false; if (active && current === sequence && epoch === generation.current) setLoading(false) }
    }
    void load()
    const timer = poll ? window.setInterval(() => { if (document.visibilityState === 'visible') void load(true) }, poll) : undefined
    return () => { active = false; snapshot.current.valid = false; ++generation.current; controller.abort(); window.clearTimeout(presentation); window.clearInterval(timer) }
  }, [path, poll, revision])
  const currentData = previousPath.current === path ? data : undefined
  return { data: currentData, error, loading, ready: previousPath.current === path && availability.ready,
    refreshing: loading || !isCurrent(), fresh: currentData !== undefined && currentData !== null && !error && isCurrent(),
    reload, isCurrent, getCurrent }
}

export type ResourceState<T> = ReturnType<typeof useResource<T>>

/**
 * Reads several endpoints in one round and publishes changed data together, so
 * sections that combine them never render a mix of old and new responses.
 * Write guards keep the per-resource semantics of useResource: every round,
 * including background polls, invalidates all members at once. A member whose
 * response equals the data already shown is current as soon as it arrives;
 * changed data becomes current when the whole round is published. A failed
 * member stays unavailable while the others remain current.
 */
export function useResourceGroup<T extends Record<string, unknown>>(paths: { [K in keyof T]: string }, poll = 5000): { [K in keyof T]: ResourceState<T[K]> } {
  type Key = keyof T & string
  type Flags = Partial<Record<Key, boolean>>
  const keys = Object.keys(paths) as Key[]
  const signature = keys.map(key => `${key}=${paths[key]}`).join('\n')
  const [values, setValues] = useState<Partial<T>>({})
  const [errors, setErrors] = useState<Partial<Record<Key, string>>>({})
  const [loading, setLoading] = useState(true)
  const [ready, setReady] = useState<Flags>({})
  const [revision, setRevision] = useState(0)
  const generation = useRef(0)
  const snapshot = useRef<{ signature: string; valid: Flags; values: Partial<T> }>({ signature: '', valid: {}, values: {} })
  const currentSignature = useRef(signature)
  currentSignature.current = signature
  const previousSignature = useRef<string | null>(null)
  const latestPaths = useRef(paths)
  latestPaths.current = paths
  const reload = useCallback(() => {
    ++generation.current
    snapshot.current.valid = {}
    setReady({}); setLoading(true)
    setRevision(value => value + 1)
  }, [])
  useEffect(() => {
    if (previousSignature.current !== signature) { snapshot.current = { signature, valid: {}, values: {} }; setValues({}); setErrors({}); setLoading(true) }
    previousSignature.current = signature
    const controller = new AbortController()
    let active = true
    let sequence = 0
    let pending = false
    let presentation: number | undefined
    const keepValid = (current: Flags) => {
      const kept = Object.fromEntries(Object.entries(current).filter(([key]) => snapshot.current.valid[key as Key])) as Flags
      return Object.keys(kept).length === Object.keys(current).length ? current : kept
    }
    const load = async (background = false) => {
      if (pending) return
      pending = true
      const round = ++sequence, epoch = ++generation.current
      const live = () => active && round === sequence && epoch === generation.current
      snapshot.current.valid = {}
      // Writes are blocked immediately; a brief poll keeps its presentation.
      if (background && Object.keys(snapshot.current.values).length) {
        presentation = window.setTimeout(() => { if (live()) setReady(keepValid) }, backgroundNoticeDelay)
      } else { setReady({}); setLoading(true) }
      const requests = keys.map(key => api<unknown>(latestPaths.current[key], 'GET', undefined, controller.signal).then(value => {
        // A response equal to the data shown is current now; changed data waits for the round.
        if (live() && Object.hasOwn(snapshot.current.values, key) && JSON.stringify(value) === JSON.stringify(snapshot.current.values[key])) {
          snapshot.current.valid[key] = true
          setReady(current => current[key] ? current : { ...current, [key]: true })
        }
        return value
      }))
      const results = await Promise.allSettled(requests)
      window.clearTimeout(presentation)
      pending = false
      if (!live()) return
      const nextValues: Partial<T> = { ...snapshot.current.values }
      const valid: Flags = {}
      const nextErrors: Partial<Record<Key, string>> = {}
      results.forEach((result, index) => {
        const key = keys[index]
        if (result.status === 'fulfilled') { nextValues[key] = result.value as T[Key]; valid[key] = true }
        else nextErrors[key] = errorMessage(result.reason)
      })
      snapshot.current = { signature, valid, values: nextValues }
      // Unchanged members keep their identity so dependent memoized views stay stable.
      setValues(previous => {
        const merged: Partial<T> = { ...nextValues }
        for (const key of keys) if (JSON.stringify(previous[key]) === JSON.stringify(nextValues[key])) merged[key] = previous[key]
        return keys.every(key => merged[key] === previous[key]) ? previous : merged
      })
      // Publish completion even for unchanged data: a disabled action may be waiting.
      setErrors(nextErrors); setReady({ ...valid }); setLoading(false)
    }
    void load()
    const timer = poll ? window.setInterval(() => { if (document.visibilityState === 'visible') void load(true) }, poll) : undefined
    return () => { active = false; snapshot.current.valid = {}; ++generation.current; controller.abort(); window.clearTimeout(presentation); window.clearInterval(timer) }
    // Keys and paths are captured through the signature.
  }, [signature, poll, revision])
  const matches = previousSignature.current === signature
  const result = {} as { [K in keyof T]: ResourceState<T[K]> }
  for (const key of keys) {
    const isCurrent = () => currentSignature.current === signature && snapshot.current.signature === signature && snapshot.current.valid[key] === true
    const data = matches ? values[key] as T[Key] | undefined : undefined
    const error = errors[key] ?? ''
    const memberLoading = loading && ready[key] !== true
    result[key] = {
      data, error, loading: memberLoading, ready: matches && ready[key] === true && !error,
      refreshing: memberLoading || !isCurrent(), fresh: data !== undefined && data !== null && !error && isCurrent(),
      reload, isCurrent, getCurrent: () => isCurrent() ? snapshot.current.values[key] as T[Key] : undefined,
    }
  }
  return result
}

/**
 * Reports a click rejected because related reads are still refreshing, instead
 * of ignoring it silently. The action is not deferred: writes and dialogs only
 * use a confirmed snapshot, so the person retries once the refresh completes.
 */
export function useRefreshNotice() {
  const [shownAt, setShownAt] = useState(0)
  useEffect(() => {
    if (!shownAt) return
    const timer = window.setTimeout(() => setShownAt(0), 3000)
    return () => window.clearTimeout(timer)
  }, [shownAt])
  return {
    visible: shownAt !== 0,
    /** Returns true when the action may proceed now; a refresh in flight is reported. */
    allows: (error: string) => {
      if (error === resourceRefreshingMessage) setShownAt(Date.now())
      return !error
    },
  }
}

export function resourceWriteError(...resources: { isCurrent?: () => boolean; fresh?: boolean; error?: string }[]): string {
  const failure = resources.find(resource => resource.error)
  if (failure) return `最新信息读取失败，暂不能修改；草稿已保留。${failure.error}`
  return resources.every(resource => resource.isCurrent ? resource.isCurrent() : resource.fresh === true)
    ? '' : resourceRefreshingMessage
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
