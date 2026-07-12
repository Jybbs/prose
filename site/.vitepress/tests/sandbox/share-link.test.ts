// @vitest-environment happy-dom
import { HASH_PREFIX, decodeShare, encodeShare } from '../../lib/sandbox/share-link'
import type { SharedState }                      from '../../lib/sandbox/share-link'

// Deflates an arbitrary value the way `encodeShare` does, so a test can
// hand `decodeShare` a well-compressed payload of the wrong shape.
async function encodeRaw(value: unknown): Promise<string> {
  const bytes  = new TextEncoder().encode(JSON.stringify(value))
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate-raw'))
  const packed = new Uint8Array(await new Response(stream).arrayBuffer())
  let binary = ''
  for (const byte of packed) binary += String.fromCodePoint(byte)
  return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '')
}

describe('share-link', () => {
  it('round-trips a source-bearing session through the hash payload', async () => {
    const state: SharedState = { configToml: 'code-line-length = 40\n', source: 'x = 1\n' }
    const payload = await encodeShare(state)
    expect(payload).not.toBeNull()
    expect(await decodeShare(`${HASH_PREFIX}${payload}`)).toEqual(state)
  })

  it('round-trips a case-bearing session without a source', async () => {
    const state: SharedState = { case: 'thematic/service_module_full_pipeline', configToml: '' }
    const payload = await encodeShare(state)
    expect(await decodeShare(`${HASH_PREFIX}${payload}`)).toEqual(state)
  })

  it('rejects a hash without the version prefix', async () => {
    expect(await decodeShare('#other')).toBeNull()
  })

  it('rejects a well-compressed payload carrying neither source nor case', async () => {
    expect(await decodeShare(`${HASH_PREFIX}${await encodeRaw({ configToml: '' })}`)).toBeNull()
  })

  it('rejects a seeded payload whose configToml is not a string', async () => {
    const payload = await encodeRaw({ configToml: 7, source: 'x = 1\n' })
    expect(await decodeShare(`${HASH_PREFIX}${payload}`)).toBeNull()
  })

  it('rejects a payload that does not inflate', async () => {
    expect(await decodeShare(`${HASH_PREFIX}not-deflate-data`)).toBeNull()
  })
})
