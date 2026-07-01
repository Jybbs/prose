import { codeToKeyedTokens, createMagicMoveMachine } from '@shikijs/magic-move/core'
import type { KeyedTokensInfo }                      from '@shikijs/magic-move/types'
import { getSingletonHighlighterCore }               from 'shiki/core'
import { createJavaScriptRegexEngine }               from 'shiki/engine/javascript'

import { SHIKI_THEMES } from '../shared/constants'

// Commits each code state through one magic-move machine so consecutive steps
// share token keys, which lets the typing-demo renderer slide a surviving token
// between states rather than re-rendering the block.
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
