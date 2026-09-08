export const HASH_PREFIX = '#1.'

export type SharedState = { case?: string, configToml: string, source?: string }

export async function decodeShare(hash: string): Promise<SharedState | null> {
  if (!hash.startsWith(HASH_PREFIX)) return null
  if (typeof DecompressionStream === 'undefined') return null
  if (typeof Uint8Array.fromBase64 !== 'function') return null
  try {
    const packed = Uint8Array.fromBase64(hash.slice(HASH_PREFIX.length), { alphabet: 'base64url' })
    const stream = new Blob([packed]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
    const state  = JSON.parse(await new Response(stream).text()) as SharedState
    const seeded = typeof state.source === 'string' || typeof state.case === 'string'
    return seeded && typeof state.configToml === 'string' ? state : null
  } catch {
    return null
  }
}

// Deflates the session into a URL-safe hash payload for a share link,
// returning `null` where the platform lacks either the codec or the base64
// encoder. An untouched pool example encodes as its case id, which keeps a
// config-experiment link short.
export async function encodeShare(state: SharedState): Promise<string | null> {
  if (typeof CompressionStream === 'undefined') return null
  if (typeof Uint8Array.prototype.toBase64 !== 'function') return null
  const bytes  = new TextEncoder().encode(JSON.stringify(state))
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate-raw'))
  const packed = new Uint8Array(await new Response(stream).arrayBuffer())
  return packed.toBase64({ alphabet: 'base64url', omitPadding: true })
}
