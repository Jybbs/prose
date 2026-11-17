import type { ShikiTransformer } from '@shikijs/types'

import type { LintFinding }          from '../fixtures/lint-findings'
import { lintDecorationTransformer } from './lint-decorations'
import type { TokenClasses }         from './token-classes'
import { wholeLineTokens }           from './whole-line-tokens'

// Lists the transformers the site's Markdown highlighter runs on every code
// block, in the order it runs them.
export function codeTransformers(
  findings     : ReadonlyMap<string, readonly LintFinding[]>,
  tokenClasses : TokenClasses
): ShikiTransformer[] {
  return [lintDecorationTransformer(findings), wholeLineTokens, tokenClasses.transformer()]
}
