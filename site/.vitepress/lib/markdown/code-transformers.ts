import type { ShikiTransformer } from '@shikijs/types'

import type { LintFinding }          from '../fixtures/lint-findings.ts'
import { lintDecorationTransformer } from './lint-decorations.ts'
import type { TokenClasses }         from './token-classes.ts'
import { wholeLineTokens }           from './whole-line-tokens.ts'

// Lists the transformers the site's Markdown highlighter runs on every code
// block, in the order it runs them.
export function codeTransformers(
  findings     : ReadonlyMap<string, readonly LintFinding[]>,
  tokenClasses : TokenClasses
): ShikiTransformer[] {
  return [lintDecorationTransformer(findings), wholeLineTokens, tokenClasses.transformer()]
}
