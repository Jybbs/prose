import type { HighlighterCore } from 'shiki/core'

// Builds the one client-side highlighter the magic-move precompiler and the
// sandbox surfaces share, `getSingletonHighlighterCore` returning one instance per set
// of options to both callers. The core and its regex engine load through a
// dynamic import, which places them in a chunk fetched at the first highlight
// and keeps them out of the shared theme chunk.
export async function codeHighlighter(): Promise<HighlighterCore> {
  const [{ getSingletonHighlighterCore }, { createJavaScriptRegexEngine }] = await Promise.all([
    import('shiki/core'),
    import('shiki/engine/javascript')
  ])
  return getSingletonHighlighterCore({
    engine : createJavaScriptRegexEngine(),
    langs  : [import('shiki/langs/python.mjs'), import('shiki/langs/toml.mjs')],
    themes : [import('shiki/themes/github-light.mjs'), import('shiki/themes/github-dark.mjs')]
  })
}
