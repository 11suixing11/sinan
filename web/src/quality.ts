import type { QualityField } from './types'

const unavailable = new Set(['null', 'undefined', 'unknown', 'n/a', 'none', 'nan', '-', '未知'])
const rawNumber = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i

export function qualityValue(field: QualityField): string | undefined {
  const value = field.value
  if (value == null || typeof value === 'object') return undefined
  if (typeof value === 'string' && (!value.trim() || unavailable.has(value.trim().toLowerCase()))) return undefined
  if (field.kind === 'boolean' && typeof value !== 'boolean') return undefined
  if (field.kind === 'text' && typeof value !== 'string') return undefined
  if (field.kind === 'country_code' && (typeof value !== 'string' || !/^[a-z]{2}$/i.test(value))) return undefined
  if (['score', 'asn', 'latitude', 'longitude'].includes(field.kind ?? '')) {
    const numericText = typeof value === 'string' && field.kind === 'score' ? value.trim().replace(/ \([^()]+\)$/, '') : value
    if (typeof numericText === 'boolean' || typeof numericText === 'string' && !rawNumber.test(numericText.trim())) return undefined
    const numeric = Number(numericText)
    if (!Number.isFinite(numeric)) return undefined
    if (field.kind === 'score' && numeric < 0 || field.kind === 'asn' && (numeric <= 0 || numeric > 4294967295 || !Number.isInteger(numeric))) return undefined
    if (field.kind === 'latitude' && Math.abs(numeric) > 90 || field.kind === 'longitude' && Math.abs(numeric) > 180) return undefined
  }
  if (typeof value === 'boolean') return value ? '是' : '否'
  if (typeof value === 'number') return Number.isFinite(value) ? String(value) : undefined
  return typeof value === 'string' ? value : undefined
}
