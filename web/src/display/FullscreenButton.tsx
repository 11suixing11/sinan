import { useEffect, useState } from 'react'
import { Icon } from './Icon'

export default function FullscreenButton() {
  const [full, setFull] = useState(false)
  const [error, setError] = useState('')
  useEffect(() => {
    const change = () => { setFull(Boolean(document.fullscreenElement)); setError('') }
    document.addEventListener('fullscreenchange', change)
    return () => { document.removeEventListener('fullscreenchange', change) }
  }, [])
  const toggle = async () => {
    setError('')
    try {
      if (document.fullscreenElement) await document.exitFullscreen()
      else await document.querySelector('.server-display')?.requestFullscreen()
    } catch { setError('浏览器未允许全屏，请使用浏览器的全屏功能。') }
  }
  if (!document.fullscreenEnabled) return null
  return <div className="d-fullscreen-control"><button className="d-icon-button" aria-label={full ? '退出全屏看板' : '全屏看板'} title={full ? '退出全屏' : '全屏看板'} onClick={() => void toggle()}><Icon name={full ? 'minimize' : 'maximize'} /></button>{error && <span className="d-fullscreen-error" role="status">{error}</span>}</div>
}
