// @vitest-environment happy-dom
import * as shareLink       from '../../lib/sandbox/share-link'
import type { SharedState } from '../../lib/sandbox/share-link'

describe('share-link', () => {
  it('round-trips a source-bearing session through the hash payload', async () => {
    const state: SharedState = { configToml: 'code-line-length = 40\n', source: 'x = 1\n' }
    const payload = await shareLink.encodeShare(state)
    expect(payload).not.toBeNull()
    expect(await shareLink.decodeShare(`${shareLink.HASH_PREFIX}${payload}`)).toEqual(state)
  })

  it('round-trips a case-bearing session without a source', async () => {
    const state: SharedState = { case: 'thematic/service_module_full_pipeline', configToml: '' }
    const payload = await shareLink.encodeShare(state)
    expect(await shareLink.decodeShare(`${shareLink.HASH_PREFIX}${payload}`)).toEqual(state)
  })

  it('rejects a hash without the version prefix', async () => {
    expect(await shareLink.decodeShare('#other')).toBeNull()
  })

  it('rejects a well-compressed payload carrying neither source nor case', async () => {
    const encoded = await shareLink.encodeShare({ configToml: '' })
    expect(await shareLink.decodeShare(`${shareLink.HASH_PREFIX}${encoded}`)).toBeNull()
  })

  it('rejects a seeded payload whose configToml is not a string', async () => {
    const state   = { configToml: 7, source: 'x = 1\n' } as unknown as SharedState
    const payload = await shareLink.encodeShare(state)
    expect(await shareLink.decodeShare(`${shareLink.HASH_PREFIX}${payload}`)).toBeNull()
  })

  it('rejects a payload that does not inflate', async () => {
    expect(await shareLink.decodeShare(`${shareLink.HASH_PREFIX}not-deflate-data`)).toBeNull()
  })
})
