import { codeToKeyedTokens, createMagicMoveMachine } from '@shikijs/magic-move/core'
import type { KeyedTokensInfo }                      from '@shikijs/magic-move/types'

import { SHIKI_THEMES }    from '../shared/constants'
import { codeHighlighter } from './highlighter'

export type MagicMoveMachine = ReturnType<typeof createMagicMoveMachine>

// A machine holds the state it last committed, so a caller stepping through
// consecutive states tokenizes only the incoming one and the surviving tokens
// keep their keys across the run. `splitTokens` cuts tokens at diff-chunk
// edges, so a token straddling a match boundary still syncs its key and slides
// rather than cross-fading.
export async function magicMoveMachine(): Promise<MagicMoveMachine> {
  const highlighter = await codeHighlighter()
  return createMagicMoveMachine(
    code => codeToKeyedTokens(highlighter, code, { lang: 'python', themes: SHIKI_THEMES }),
    { enhanceMatching: true, splitTokens: true }
  )
}

// Commits a whole sequence through one machine, for a caller that holds every
// state up front rather than reaching them one interaction at a time.
export async function precompileMagicMove(states: readonly string[]): Promise<KeyedTokensInfo[]> {
  const machine = await magicMoveMachine()
  return states.map(state => machine.commit(state).current)
}
