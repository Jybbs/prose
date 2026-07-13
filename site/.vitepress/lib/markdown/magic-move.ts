import { codeToKeyedTokens, createMagicMoveMachine } from '@shikijs/magic-move/core'
import type { KeyedTokensInfo }                      from '@shikijs/magic-move/types'

import { SHIKI_THEMES }    from '../shared/constants'
import { codeHighlighter } from './highlighter'

// Commits each code state through one machine so consecutive steps share
// token keys, which is what lets the renderer slide surviving tokens.
// `splitTokens` cuts tokens at diff-chunk edges, so a token straddling a
// match boundary still syncs its key and slides rather than cross-fading.
export async function precompileMagicMove(states: readonly string[]): Promise<KeyedTokensInfo[]> {
  const highlighter = await codeHighlighter()
  const machine = createMagicMoveMachine(
    code => codeToKeyedTokens(highlighter, code, { lang: 'python', themes: SHIKI_THEMES }),
    { enhanceMatching: true, splitTokens: true }
  )
  return states.map(state => machine.commit(state).current)
}
