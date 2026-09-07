// @vitest-environment happy-dom
import * as shareLink       from '../../lib/sandbox/share-link'
import type { SharedState } from '../../lib/sandbox/share-link'

describe('share-link', () => {
  it('round-trips a source-bearing session through the hash payload', async () => {
    const state: SharedState = { configToml: 'code-line-length = 40\n', source: 'x = 1\n' }
    const payload = await shareLink.encodeShare(state)
    expect(payload).not.toBeNull()
    await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}${payload}`)).resolves.toStrictEqual(state)
  })

  it('round-trips a case-bearing session without a source', async () => {
    const state: SharedState = { case: 'thematic/service_module_full_pipeline', configToml: '' }
    const payload = await shareLink.encodeShare(state)
    await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}${payload}`)).resolves.toStrictEqual(state)
  })

  it('rejects a hash without the version prefix', async () => {
    await expect(shareLink.decodeShare('#other')).resolves.toBeNull()
  })

  it('rejects a well-compressed payload carrying neither source nor case', async () => {
    const encoded = await shareLink.encodeShare({ configToml: '' })
    await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}${encoded}`)).resolves.toBeNull()
  })

  it('rejects a seeded payload whose configToml is not a string', async () => {
    const state   = { configToml: 7, source: 'x = 1\n' } as unknown as SharedState
    const payload = await shareLink.encodeShare(state)
    await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}${payload}`)).resolves.toBeNull()
  })

  it('rejects a payload that does not inflate', async () => {
    await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}not-deflate-data`)).resolves.toBeNull()
  })
})

// Removes a platform method for one case, which sends the encoder's and the
// decoder's feature test down its missing branch.
async function withoutMethod<T extends object>(
  owner : T,
  key   : keyof T & string,
  run   : () => Promise<unknown>
): Promise<unknown> {
  const held = owner[key]
  Reflect.deleteProperty(owner, key)
  try {
    return await run()
  }
  finally {
    Object.defineProperty(owner, key, { configurable: true, value: held, writable: true })
  }
}

describe('share-link on a platform missing a codec', () => {
  it('encodes to null without a compression stream', async () => {
    await withoutMethod(globalThis, 'CompressionStream', async () => {
      await expect(shareLink.encodeShare({ configToml: '', source: 'x = 1\n' })).resolves.toBeNull()
    })
  })

  it('decodes to null without a decompression stream', async () => {
    await withoutMethod(globalThis, 'DecompressionStream', async () => {
      await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}abc`)).resolves.toBeNull()
    })
  })

  it('encodes to null without the base64 encoder', async () => {
    await withoutMethod(Uint8Array.prototype, 'toBase64', async () => {
      await expect(shareLink.encodeShare({ configToml: '', source: 'x = 1\n' })).resolves.toBeNull()
    })
  })

  it('decodes to null without the base64 decoder', async () => {
    await withoutMethod(Uint8Array, 'fromBase64', async () => {
      await expect(shareLink.decodeShare(`${shareLink.HASH_PREFIX}abc`)).resolves.toBeNull()
    })
  })
})
