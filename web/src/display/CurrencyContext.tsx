import { createContext, useContext, useState } from 'react'
import type { ReactNode } from 'react'
import { api } from '../api'
import { useAction } from '../hooks'
import { currencyCode, displayCurrencies } from './currency'
import type { ExchangeRates } from './currency'
import { Icon } from './Icon'
import { useDashboardPoll } from './useDashboardPoll'

type CurrencyState = { currency: string; setCurrency: (currency: string) => void; quote?: ExchangeRates; error: string; loading: boolean; reload: () => void }
const Context = createContext<CurrencyState>({ currency: 'CNY', setCurrency: () => {}, error: '', loading: false, reload: () => {} })
export const useCurrency = () => useContext(Context)

export function CurrencyProvider({ children }: { children: ReactNode }) {
  const [currency, save] = useState(() => { try { return currencyCode(localStorage.getItem('sinan-display-currency')) } catch { return 'CNY' } })
  const rates = useDashboardPoll<ExchangeRates>('/api/dashboard/exchange-rates', 300_000, false)
  const setCurrency = (value: string) => {
    const code = currencyCode(value); save(code)
    try { localStorage.setItem('sinan-display-currency', code) } catch { /* Keep only the non-sensitive in-memory preference. */ }
  }
  return <Context.Provider value={{ currency, setCurrency, quote: rates.data, error: rates.error, loading: rates.loading, reload: rates.reload }}>{children}</Context.Provider>
}

export function CurrencyControls() {
  const { currency, setCurrency, quote, error, loading, reload } = useCurrency()
  const action = useAction()
  const currencies = [...new Set([...displayCurrencies, currency, ...Object.keys(quote?.rates ?? {}).filter(code => /^[A-Z]{3}$/.test(code))])]
  const stale = quote?.stale || quote?.status === 'stale' || Boolean(error)
  const available = quote?.status !== 'unavailable' && quote?.rate_date
  const label = loading && !quote ? '正在读取汇率' : !available ? '暂无外币汇率' : stale ? '使用上次汇率' : '每日参考汇率'
  const sourceUrl = quote?.source_url?.startsWith('https://') ? quote.source_url : null
  return <div className="d-currency-controls">
    <label>显示币种<select aria-label="显示币种" value={currency} onChange={event => setCurrency(event.target.value)}>{currencies.map(code => <option key={code} value={code}>{code}</option>)}</select></label>
    <div className={`d-exchange-status ${stale || !available ? 'd-warning' : ''}`}><span>{label}{quote?.rate_date ? ` · ${quote.rate_date}` : ''}</span><small>{sourceUrl ? <a href={sourceUrl} target="_blank" rel="noreferrer">{quote?.source === 'frankfurter-ecb' ? 'Frankfurter · ECB' : 'Frankfurter'}</a> : '仅原币金额可用'}{quote?.fetched_at ? ` · 获取于 ${new Date(quote.fetched_at * 1000).toLocaleString('zh-CN', { hour12: false })}` : ''}</small></div>
    <button className="d-feed-button" disabled={action.busy} onClick={() => void action.run(() => api('/api/exchange-rates/refresh', 'POST', {}), reload)}><Icon name="refresh" size={14} />{action.busy ? '正在更新' : '更新汇率'}</button>
    {(error || action.error) && <span className="d-exchange-error" role="status">{action.error || '汇率读取失败，已有报价仍保留。'}<button onClick={reload}>重试读取</button></span>}
  </div>
}
