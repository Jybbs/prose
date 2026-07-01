import { ofetch }             from 'ofetch'
import type { LoaderContext } from 'astro/loaders'

import { replaceStore, type StoreEntry } from './store'

type StoreContext = Pick<LoaderContext, 'logger' | 'meta' | 'parseData' | 'store'>

interface ConditionalSource {
  etagKey   : string
  fallback  : readonly StoreEntry[]
  headers   : Record<string, string>
  label     : string
  toEntries : (payload: unknown) => readonly StoreEntry[]
  url       : string
}

// Reads an external JSON endpoint with an ETag conditional request, the ETag
// kept in the loader `meta` store and the parsed entries in the data `store`
// across builds. A 304 keeps the persisted store, a network failure or the
// offline flag seeds the static fallback only when the store is cold.
export async function conditionalLoad(ctx: StoreContext, source: ConditionalSource): Promise<void> {
  if (process.env.PROSE_OFFLINE_DOCS === '1') return seedIfCold(ctx, source)

  const fallback = (reason: string): Promise<void> => {
    const note = ctx.store.keys().length > 0
      ? 'keeping the cached store'
      : 'seeding the static fallback'
    ctx.logger.warn(`${source.label}: ${reason}, ${note}`)
    return seedIfCold(ctx, source)
  }

  const etag     = ctx.meta.get(source.etagKey)
  const response = await ofetch
    .raw(source.url, {
      headers             : { ...source.headers, ...(etag ? { 'If-None-Match': etag } : {}) },
      ignoreResponseError : true,
      retry               : 2
    })
    .catch(() => null)

  if (response === null) return fallback('request failed')
  if (response.status === 304) return
  if (!response.ok) return fallback(`upstream returned ${response.status}`)

  const etagHeader = response.headers.get('etag')
  if (etagHeader) ctx.meta.set(source.etagKey, etagHeader)
  await replaceStore(ctx, source.toEntries(response._data))
}

async function seedIfCold(ctx: StoreContext, source: ConditionalSource): Promise<void> {
  if (ctx.store.keys().length === 0) await replaceStore(ctx, source.fallback)
}
