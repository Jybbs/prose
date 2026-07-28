import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import * as cacache from 'cacache'

import { conditionalFetch } from '../../lib/shared/conditional-fetch'
import { warnTest }         from '../support'

let dir: string

const makeSource = () => ({
  dir,
  fallback : 'fallback',
  key      : 'probe',
  parse    : (payload: unknown) => (payload as { value: string }).value,
  url      : 'https://api.example/probe'
})

const seedStore = () =>
  cacache.put(dir, 'probe', JSON.stringify('stored'), { metadata: { etag: 'W/"1"' } })

beforeEach(() => {
  dir = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-fetch-'))
})

afterEach(() => {
  fs.rmSync(dir, { force: true, recursive: true })
  vi.unstubAllGlobals()
  vi.unstubAllEnvs()
})

describe('conditionalFetch', () => {
  it('parses a fresh payload and persists it with the etag', async () => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(
      new Response('{"value":"fresh"}', { headers: { etag: 'W/"1"' }, status: 200 })
    ))
    await expect(conditionalFetch(makeSource())).resolves.toBe('fresh')
    const entry = await cacache.get(dir, 'probe')
    expect(JSON.parse(entry.data.toString())).toBe('fresh')
    expect(entry.metadata).toEqual({ etag: 'W/"1"' })
  })

  it('sends the stored etag and keeps the payload on a 304', async () => {
    await seedStore()
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 304 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource())).resolves.toBe('stored')
    expect(fetchMock).toHaveBeenCalledExactlyOnceWith(
      'https://api.example/probe',
      { headers: { 'If-None-Match': 'W/"1"' }, signal: expect.any(AbortSignal) }
    )
  })

  it('omits the conditional header when the stored entry has no etag', async () => {
    const fetchMock = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(new Response('{"value":"fresh"}',   { status: 200 }))
      .mockResolvedValueOnce(new Response('{"value":"fresher"}', { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    await conditionalFetch(makeSource())
    await expect(conditionalFetch(makeSource())).resolves.toBe('fresher')
    expect(fetchMock).toHaveBeenLastCalledWith(
      'https://api.example/probe',
      { headers: {}, signal: expect.any(AbortSignal) }
    )
  })

  it('retries a transient upstream error and returns the recovery', async () => {
    const fetchMock = vi.fn<typeof fetch>()
      .mockResolvedValueOnce(new Response(null, { status: 502 }))
      .mockResolvedValueOnce(new Response('{"value":"recovered"}', { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource())).resolves.toBe('recovered')
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  warnTest('keeps the last-good payload when every attempt throws', async ({ warn }) => {
    await seedStore()
    const fetchMock = vi.fn<typeof fetch>().mockRejectedValue(new TypeError('fetch failed'))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource())).resolves.toBe('stored')
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(warn).toHaveBeenCalledWith('[data:probe] request failed, keeping the last-good payload')
  })

  warnTest('seeds the fallback when the store is cold and the upstream errors', async ({ warn }) => {
    const fetchMock = vi.fn<typeof fetch>().mockResolvedValue(new Response(null, { status: 500 }))
    vi.stubGlobal('fetch', fetchMock)
    await expect(conditionalFetch(makeSource())).resolves.toBe('fallback')
    expect(fetchMock).toHaveBeenCalledTimes(3)
    expect(warn).toHaveBeenCalledWith('[data:probe] upstream returned 500, seeding the fallback')
  })

  warnTest('falls back when a 200 payload rejects at parse', async ({ warn }) => {
    vi.stubGlobal('fetch', vi.fn<typeof fetch>().mockResolvedValue(new Response('{}', { status: 200 })))
    const source = { ...makeSource(), parse: () => { throw new Error('no value field') } }
    await expect(conditionalFetch(source)).resolves.toBe('fallback')
    expect(warn).toHaveBeenCalledWith('[data:probe] payload rejected (no value field), seeding the fallback')
  })

  it('returns the stored payload offline without a request', async () => {
    await seedStore()
    const fetchMock = vi.fn<typeof fetch>()
    vi.stubGlobal('fetch', fetchMock)
    vi.stubEnv('PROSE_OFFLINE_DOCS', '1')
    await expect(conditionalFetch(makeSource())).resolves.toBe('stored')
    expect(fetchMock).not.toHaveBeenCalled()
  })

  it('returns the fallback offline when the store is cold', async () => {
    vi.stubEnv('PROSE_OFFLINE_DOCS', '1')
    await expect(conditionalFetch(makeSource())).resolves.toBe('fallback')
  })
})
