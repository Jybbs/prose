import { codeToKeyedTokens, createMagicMoveMachine }                from 'shiki-magic-move/core'
import type { KeyedTokensInfo }                                     from 'shiki-magic-move/types'
import { createJavaScriptRegexEngine, getSingletonHighlighterCore } from 'shiki/core'

import { SHIKI_THEMES } from '../shared/constants'

// Commits each code state through one machine so consecutive steps share
// token keys, which is what lets the renderer slide surviving tokens.
export async function precompileMagicMove(states: readonly string[]): Promise<KeyedTokensInfo[]> {
  const highlighter = await getSingletonHighlighterCore({
    engine : createJavaScriptRegexEngine(),
    langs  : [import('shiki/langs/python.mjs')],
    themes : [import('shiki/themes/github-light.mjs'), import('shiki/themes/github-dark.mjs')]
  })
  const machine = createMagicMoveMachine(code =>
    codeToKeyedTokens(highlighter, code, { lang: 'python', themes: SHIKI_THEMES })
  )
  return states.map(state => machine.commit(state).current)
}
