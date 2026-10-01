import { expect, test } from 'bun:test'
import { convert, costSummary, money, price, quoteDate, remainingCost } from '../src/display/currency'
import type { ExchangeRates } from '../src/display/currency'
import type { Server } from '../src/types'
import { defaultAssets } from '../src/server-assets'

const quote: ExchangeRates = { base: 'CNY', rates: { CNY: 1, USD: .125, EUR: .1 }, rate_dates: { USD: '2026-09-30', EUR: '2026-09-29' }, rate_date: '2026-09-29', source: 'frankfurter', source_url: 'https://frankfurter.dev', fetched_at: 1, attempted_at: 1, next_refresh_at: 2, stale: true, status: 'stale', error_code: 'fetch_failed' }

test('conversion uses CNY quotes, keeps real stale data and never fabricates missing prices', () => {
  expect(convert(10, 'USD', 'CNY', quote)).toBe(80)
  expect(convert(10, 'USD', 'EUR', quote)).toBe(8)
  expect(convert(10, 'USD', 'USD', undefined)).toBe(10)
  expect(convert(10, 'USD', 'CNY', undefined)).toBeNull()
  expect(convert(10, 'GBP', 'CNY', quote)).toBeNull()
  expect(convert(10, 'USD', 'EUR', { ...quote, rates: { USD: 0, EUR: .1 } })).toBeNull()
  for (const value of [null, undefined, '', '-1', 'NaN', '1e3']) expect(price(value)).toBeNull()
  expect(price('0')).toBe(0)
  expect(money(null, 'CNY')).toBe('—')
  expect(quoteDate(quote, 'USD', 'EUR')).toBe('2026-09-29')
  expect(quoteDate(quote, 'GBP', 'CNY')).toBeNull()
})

test('cost totals exclude public and hidden resources, identify partial coverage and separate billing periods', () => {
  const now = Date.UTC(2026, 9, 1)
  const item = (price: string | null, currency = 'CNY', billing_cycle = 30) => ({ id: 1, public_view: false, asset_settings: { ...defaultAssets, price, currency, billing_cycle, expires_at: (now + 15 * 86_400_000) / 1000 } }) as Server
  const rows = [item('10', 'USD'), item('20', 'CNY', 0), item(null), item('30', 'GBP'), { ...item('1000'), public_view: true }, { ...item('1000'), asset_settings: { ...item('1000').asset_settings!, hidden: true } }]
  expect(costSummary(rows, 'CNY', quote, now)).toEqual({ total: 100, recurring: 80, remaining: 40, priced: 3, converted: 2, recurringCount: 1, remainingCount: 1, missingRates: 1, missingPrices: 1 })
  expect(costSummary([item('3', 'GBP')], 'CNY', quote, now).total).toBeNull()
  expect(remainingCost(item('10').asset_settings, now)).toBe(5)
  expect(remainingCost(item('10', 'CNY', 0).asset_settings, now)).toBeNull()
  expect(remainingCost({ ...item('10').asset_settings!, expires_at: null }, now)).toBeNull()
  expect(remainingCost(item('10').asset_settings, now + 60 * 86_400_000)).toBe(0)
})
