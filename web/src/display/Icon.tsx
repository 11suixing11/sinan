import type { ReactNode } from 'react'
import { Icon as SharedIcon } from '../components'

const paths: Record<string, ReactNode> = {
  search: <><circle cx="10.5" cy="10.5" r="6.5" /><path d="m16 16 5 5" /></>,
  sun: <><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5" /></>,
  moon: <path d="M20 14A9 9 0 0 1 10 4a9 9 0 1 0 10 10Z" />,
  cpu: <><rect x="6" y="6" width="12" height="12" rx="2" /><rect x="9" y="9" width="6" height="6" rx="1" /><path d="M9 3v3m6-3v3M9 18v3m6-3v3M3 9h3m-3 6h3m12-6h3m-3 6h3" /></>,
  database: <><ellipse cx="12" cy="5" rx="8" ry="3" /><path d="M4 5v14c0 4 16 4 16 0V5M4 12c0 4 16 4 16 0" /></>,
  monitor: <><rect x="3" y="3" width="18" height="13" rx="2" /><path d="M8 21h8m-4-5v5" /></>,
  clock: <><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></>,
  network: <><rect x="9" y="2" width="6" height="5" rx="1" /><rect x="2" y="17" width="6" height="5" rx="1" /><rect x="16" y="17" width="6" height="5" rx="1" /><path d="M12 7v5M5 17v-5h14v5" /></>,
  user: <><circle cx="12" cy="12" r="9" /><circle cx="12" cy="9" r="3" /><path d="M5.5 18a7 7 0 0 1 13 0" /></>,
}

export function Icon({ name, size = 18 }: { name: string; size?: number }) {
  if (!paths[name]) return <SharedIcon name={name} size={size} />
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{paths[name]}</svg>
}

export function OSIcon({ system }: { system?: string }) {
  const os = system?.toLowerCase() ?? ''
  const name = ['ubuntu', 'debian', 'alpine', 'arch', 'fedora', 'centos', 'rocky', 'gentoo', 'windows', 'macos', 'nix', 'alma'].find(value => os.includes(value))
    ?? (os.includes('darwin') || os.includes('mac os') ? 'macos' : 'unknown')
  return <img className="d-os" src={`/display-icons/os-${name}.${name === 'alpine' ? 'webp' : 'svg'}`} width="16" height="16" alt={system || '系统未知'} title={system || '系统未知'} />
}
