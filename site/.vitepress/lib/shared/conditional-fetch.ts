import * as cacache from 'cacache'

import { errorMessage } from './error-message'

interface ConditionalFetchSource<T> {
  dir      : string
  fallback : T
  headers ?: Record<string, string>
  key      : string
  parse    : (payload: unknown) => T
  url      : string
}

interface FetchStore<T> {
  etag    ?: string
  payload  : T
}

const RETRIES    = 2
const TIMEOUT_MS = 10_000

// Reads an external JSON endpoint through an `If-None-Match` conditional
// request, keeping the ETag and the parsed payload in a `cacache` store
// under `dir` across builds. A 304 returns the stored payload, a failed
// request or bad payload returns the stored payload when the store is warm
// and `fallback` when it is cold, and the offline flag skips the request.
export async function conditionalFetch<T>(source: ConditionalFetchSource<T>): Promise<T> {
  const stored = await readStore(source)
  if (process.env.PROSE_OFFLINE_DOCS === '1') return stored ? stored.payload : source.fallback

  const response = await fetchWithRetry(source, stored?.etag)
  if (response === null) return settle(source, stored, 'request failed')
  if (response.status === 304 && stored) return stored.payload
  if (!response.ok) {
    await response.body?.cancel()
    return settle(source, stored, `upstream returned ${response.status}`)
  }

  try {
    const payload = source.parse(await response.json())
    await writeStore(source, { etag: response.headers.get('etag') ?? undefined, payload })
    return payload
  }
  catch (err) {
    return settle(source, stored, `payload rejected (${errorMessage(err)})`)
  }
}

async function fetchWithRetry<T>(
  source : ConditionalFetchSource<T>,
  etag   : string | undefined
): Promise<Response | null> {
  const headers = { ...source.headers, ...(etag ? { 'If-None-Match': etag } : {}) }
  for (let attempt = 0; ; attempt++) {
    try {
      const response = await fetch(source.url, { headers, signal: AbortSignal.timeout(TIMEOUT_MS) })
      if (response.status < 500 || attempt === RETRIES) return response
      await response.body?.cancel()
    }
    catch {
      if (attempt === RETRIES) return null
    }
  }
}

async function readStore<T>(source: ConditionalFetchSource<T>): Promise<FetchStore<T> | null> {
  try {
    const entry = await cacache.get(source.dir, source.key)
    return {
      etag    : (entry.metadata as { etag?: string } | undefined)?.etag,
      payload : JSON.parse(entry.data.toString()) as T
    }
  }
  catch {
    return null
  }
}

function settle<T>(
  source : ConditionalFetchSource<T>,
  stored : FetchStore<T> | null,
  reason : string
): T {
  const note = stored ? 'keeping the last-good payload' : 'seeding the static fallback'
  console.warn(`[data:${source.key}] ${reason}, ${note}`)
  return stored ? stored.payload : source.fallback
}

async function writeStore<T>(source: ConditionalFetchSource<T>, store: FetchStore<T>): Promise<void> {
  try {
    await cacache.put(source.dir, source.key, JSON.stringify(store.payload), {
      metadata: { etag: store.etag }
    })
  }
  catch {
    // A failed write still leaves the fresh payload with the caller
  }
}
