export const HASH_PREFIX = '#1.'

export type SharedState = { case?: string, configToml: string, source?: string }

export async function decodeShare(hash: string): Promise<SharedState | null> {
  if (!hash.startsWith(HASH_PREFIX) || typeof DecompressionStream === 'undefined') return null
  try {
    const b64    = hash.slice(HASH_PREFIX.length).replaceAll('-', '+').replaceAll('_', '/')
    const bytes  = Uint8Array.from(atob(b64), char => char.codePointAt(0) ?? 0)
    const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
    const state  = JSON.parse(await new Response(stream).text()) as SharedState
    const seeded = typeof state.source === 'string' || typeof state.case === 'string'
    return seeded && typeof state.configToml === 'string' ? state : null
  } catch {
    return null
  }
}

// Deflates the session into a URL-safe hash payload so any sandbox moment
// can travel as a link, returning `null` where the platform lacks the codec.
// An untouched pool example travels as its case id rather than its source,
// keeping the common config-experiment link short.
export async function encodeShare(state: SharedState): Promise<string | null> {
  if (typeof CompressionStream === 'undefined') return null
  const bytes  = new TextEncoder().encode(JSON.stringify(state))
  const stream = new Blob([bytes]).stream().pipeThrough(new CompressionStream('deflate-raw'))
  const packed = new Uint8Array(await new Response(stream).arrayBuffer())
  let binary = ''
  for (const byte of packed) binary += String.fromCodePoint(byte)
  const encoded = btoa(binary).replaceAll('+', '-').replaceAll('/', '_')
  const padding = encoded.indexOf('=')
  return padding === -1 ? encoded : encoded.slice(0, padding)
}
