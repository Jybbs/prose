import type { DecorationItem, ShikiTransformer } from '@shikijs/types'

const META_PREFIX = 'lintdeco-'

// Encodes decorations into a fence-meta token the transformer decodes in
// its preprocess hook, so they travel inside the markdown string rather
// than through module state the config and build realms do not share.
export function encodeLintMeta(decorations: readonly DecorationItem[]): string {
  return META_PREFIX + Buffer.from(JSON.stringify(decorations)).toString('base64url')
}

export const lintDecorationTransformer: ShikiTransformer = {
  name: 'prose:lint-flag',
  preprocess(_code, options) {
    const token = options.meta?.__raw?.split(/\s+/).find(part => part.startsWith(META_PREFIX))
    if (!token) return
    const json = Buffer.from(token.slice(META_PREFIX.length), 'base64url').toString('utf8')
    ;(options.decorations ??= []).push(...(JSON.parse(json) as DecorationItem[]))
  }
}
