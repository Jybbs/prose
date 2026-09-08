import * as cacache from 'cacache'

import { conditionalFetch } from '../../lib/shared/conditional-fetch'
import { supportTest }      from '../support'

const makeSource = (dir: string) => ({
  dir,
  fallback : 'fallback',
  key      : 'probe',
  parse    : (payload: unknown) => (payload as { value: string }).value,
  url      : 'https://api.example/probe'
})

const seedStore = (dir: string) =>
  cacache.put(dir, 'probe', JSON.stringify('stored'), { metadata: { etag: 'W/"1"' } })

// Clears the flag the build reads, so every request in this suite goes out.
beforeEach(() => {
  vi.stubEnv('PROSE_OFFLINE_DOCS', '')
})

describe('conditionalFetch', () => {
  supportTest('parses a fresh payload and persists it with the etag', async ({ tmpDir }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(
      new Response('{"value":"fresh"}', { headers: { etag: 'W/"1"' }, status: 200 })
    ))
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('fresh')
    const entry = await cacache.get(tmpDir, 'probe')
    expect(JSON.parse(entry.data.toString())).toBe('fresh')
    expect(entry.metadata).toStrictEqual({ etag: 'W/"1"' })
  })

  supportTest('sends the stored etag and keeps the payload on a 304', async ({ tmpDir }) => {
    await seedStore(tmpDir)
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 304 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('stored')
    expect(fetchMock).toHaveBeenCalledExactlyOnceWith(
      'https://api.example/probe',
      { headers: { 'If-None-Match': 'W/"1"' }, signal: expect.any(AbortSignal) }
    )
  })

  supportTest('omits the conditional header when the stored entry has no etag',
    async ({ tmpDir }) => {
    const fetchMock = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(new Response('{"value":"fresh"}',   { status: 200 }))
      .mockResolvedValueOnce(new Response('{"value":"fresher"}', { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    await conditionalFetch(makeSource(tmpDir))
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('fresher')
    expect(fetchMock).toHaveBeenLastCalledWith(
      'https://api.example/probe',
      { headers: {}, signal: expect.any(AbortSignal) }
    )
  })

  supportTest('retries a transient upstream error and returns the recovery', async ({ tmpDir }) => {
    const fetchMock = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(null, { status: 502 }))
      .mockResolvedValueOnce(new Response('{"value":"recovered"}', { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('recovered')
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  supportTest('keeps the last-good payload when every attempt throws', async ({ tmpDir, warn }) => {
    await seedStore(tmpDir)
    const fetchMock = vi.fn<typeof fetch>().mockRejectedValue(new TypeError('fetch failed'))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('stored')
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(warn).toHaveBeenCalledWith('[data:probe] request failed, keeping the last-good payload')
  })

  supportTest('seeds the fallback when the store is cold and the upstream errors',
    async ({ tmpDir, warn }) => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 500 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('fallback')
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(warn).toHaveBeenCalledWith('[data:probe] upstream returned 500, seeding the fallback')
  })

  supportTest('falls back when a 200 payload rejects at parse', async ({ tmpDir, warn }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(new Response('{}', { status: 200 })))
    const source = { ...makeSource(tmpDir), parse: () => { throw new Error('no value field') } }
    await expect(conditionalFetch(source)).resolves.toBe('fallback')
    expect(warn).toHaveBeenCalledWith('[data:probe] payload rejected (no value field), seeding the fallback')
  })

  supportTest('returns the stored payload offline without a request', async ({ tmpDir }) => {
    await seedStore(tmpDir)
    const fetchMock = vi.fn<typeof fetch>()
    vi.stubGlobal('fetch', fetchMock)
    vi.stubEnv('PROSE_OFFLINE_DOCS', '1')
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('stored')
    expect(fetchMock).not.toHaveBeenCalled()
  })

  supportTest('returns the fallback offline when the store is cold', async ({ tmpDir }) => {
    vi.stubEnv('PROSE_OFFLINE_DOCS', '1')
    await expect(conditionalFetch(makeSource(tmpDir))).resolves.toBe('fallback')
  })
})
