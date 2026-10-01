import { useCallback, useEffect, useState } from 'react'
import { api, ApiError, errorMessage } from '../api'
import type { Sample } from './data'

export function useHistory(serverId: number, minutes: number) {
  const [samples, setSamples] = useState<Sample[]>([])
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [revision, setRevision] = useState(0)
  const reload = useCallback(() => setRevision(value => value + 1), [])
  useEffect(() => {
    const controller = new AbortController()
    let active = true, pending = false, lastFullRead = 0
    let saved: Sample[] = []
    setSamples([]); setError(''); setLoading(true)
    const load = async () => {
      if (pending) return
      pending = true
      const since = Date.now() - minutes * 60_000
      const newest = saved[saved.length - 1]?.sampled_at
      const fullRead = Date.now() - lastFullRead >= 60_000
      try {
        // Re-read the full window every minute to include older, delayed uploads.
        const next = await api<Sample[]>(`/api/dashboard/servers/${serverId}/metrics?since=${Math.max(since, !fullRead && newest ? newest - 120_000 : since)}`, 'GET', undefined, controller.signal)
        if (!active) return
        const merged = new Map([...saved, ...next].map(sample => [sample.id, sample]))
        saved = [...merged.values()].filter(sample => sample.sampled_at >= since && sample.sampled_at <= Date.now() + 60_000)
          .sort((left, right) => left.sampled_at - right.sampled_at).slice(-7200)
        if (fullRead) lastFullRead = Date.now()
        setSamples(saved); setError('')
      } catch (reason) { if (active) { setError(errorMessage(reason)); if (reason instanceof ApiError && [401, 403, 404].includes(reason.status)) { saved = []; setSamples([]) } } }
      finally { pending = false; if (active) setLoading(false) }
    }
    void load()
    const refreshVisible = () => { if (document.visibilityState === 'visible') void load() }
    const timer = window.setInterval(refreshVisible, 15_000)
    document.addEventListener('visibilitychange', refreshVisible)
    return () => { active = false; controller.abort(); window.clearInterval(timer); document.removeEventListener('visibilitychange', refreshVisible) }
  }, [serverId, minutes, revision])
  return { samples, error, loading, reload }
}
