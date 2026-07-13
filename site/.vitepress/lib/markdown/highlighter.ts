import { getSingletonHighlighterCore, type HighlighterCore } from 'shiki/core'
import { createJavaScriptRegexEngine }                       from 'shiki/engine/javascript'

// The one client-side highlighter the magic-move precompiler and the
// sandbox surfaces share. `getSingletonHighlighterCore` dedupes by
// config, so a single instance backs both callers.
export function codeHighlighter(): Promise<HighlighterCore> {
  return getSingletonHighlighterCore({
    engine : createJavaScriptRegexEngine(),
    langs  : [import('shiki/langs/python.mjs'), import('shiki/langs/toml.mjs')],
    themes : [import('shiki/themes/github-light.mjs'), import('shiki/themes/github-dark.mjs')]
  })
}
