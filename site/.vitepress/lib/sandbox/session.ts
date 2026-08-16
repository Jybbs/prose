import { sectionRoute }     from '../shared/routes'
import type { SandboxCase } from './pool.data'
import * as shareLink       from './share-link'

export type SavedSession = { configToml: string, source: string }

export const STORAGE_KEY = 'prose-sandbox'

// Deflates a session into a URL-safe hash payload hung off the given base.
async function hashUrl(base: string, state: shareLink.SharedState): Promise<string | null> {
  const payload = await shareLink.encodeShare(state)
  return payload === null ? null : `${base}${shareLink.HASH_PREFIX}${payload}`
}

// Picks a case other than the one showing, or the only one there is.
export function randomOther(count: number, exclude: number): number {
  if (count <= 1) return 0
  const roll = Math.floor(Math.random() * (count - 1))
  return roll >= exclude ? roll + 1 : roll
}

// Builds a link opening the sandbox on a source the pool does not carry.
export function seedUrl(configToml: string, source: string): Promise<string | null> {
  return hashUrl(sectionRoute('sandbox'), { configToml, source })
}

// Resolves the session a share link in the address bar carries, matching a
// compact payload back to its pool case.
export async function sharedSeed(cases: readonly SandboxCase[]): Promise<SavedSession | null> {
  const hash = typeof window === 'undefined' ? '' : window.location.hash
  const shared = await shareLink.decodeShare(hash)
  if (!shared) return null
  const source = shared.source ?? cases.find(entry => entry.id === shared.case)?.source
  return source === undefined ? null : { configToml: shared.configToml, source }
}

// Builds a link reproducing the current session, compact when the source is an
// untouched pool case, leaving the address bar itself alone.
export function shareUrl(
  cases      : readonly SandboxCase[],
  configToml : string,
  source     : string
): Promise<string | null> {
  if (typeof window === 'undefined') return Promise.resolve(null)
  const match = cases.find(entry => entry.source === source)
  return hashUrl(
    window.location.href.split('#')[0],
    match ? { case: match.id, configToml } : { configToml, source })
}
