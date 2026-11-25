import { codeTransformers } from '../../../site/.vitepress/lib/markdown/code-transformers'
import { TokenClasses }     from '../../../site/.vitepress/lib/markdown/token-classes'
import { wholeLineTokens }  from '../../../site/.vitepress/lib/markdown/whole-line-tokens'

describe('codeTransformers', () => {
  it('passes the highlighter the shared wholeLineTokens instance', async () => {
    expect(codeTransformers(new Map(), await TokenClasses.read())).toContain(wholeLineTokens)
  })

  it('runs the lint flags, the whole-line tokens, and the token classes in that order', async () => {
    expect(codeTransformers(new Map(), await TokenClasses.read()).map(transformer => transformer.name))
      .toStrictEqual(['prose:lint-flag', 'prose:whole-line-tokens', 'prose:token-classes'])
  })
})
