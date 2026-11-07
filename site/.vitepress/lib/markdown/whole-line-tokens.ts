import type { ShikiTransformer } from '@shikijs/types'

// Clears the tokenizer's time limit, past which a line's remaining text takes
// the color of its last token, so a page's token colors stay the same at any
// load on the machine running the build.
export const wholeLineTokens: ShikiTransformer = {
  name: 'prose:whole-line-tokens',
  preprocess(_code, options) {
    options.tokenizeTimeLimit = 0
  }
}
