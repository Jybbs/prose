import {
  decodeLintMeta, encodeLintMeta, lintDecorations, lintDecorationTransformer
} from '../lib/markdown/lint-decorations'
import { replaceTextTokens } from '../lib/markdown/token-split'
import { walkBodyInlines }   from '../lib/markdown/walk'

class StubToken {
  children: StubToken[] | null = null
  content = ''
  level   = 0
  constructor(public type: string, public tag: string, public nesting: number) {}
}

const text = (content: string): StubToken =>
  Object.assign(new StubToken('text', '', 0), { content })

describe('replaceTextTokens', () => {
  it('splits matching text tokens and preserves the rest', () => {
    const out = replaceTextTokens([text('see prose here')], StubToken, /prose/g, () =>
      [Object.assign(new StubToken('html_inline', '', 0), { content: '<b>prose</b>' })])
    expect(out.map(t => ({ content: t.content, type: t.type }))).toEqual([
      { content: 'see ',         type: 'text' },
      { content: '<b>prose</b>', type: 'html_inline' },
      { content: ' here',        type: 'text' }
    ])
  })

  it('skips text inside links when asked', () => {
    const children = [
      new StubToken('link_open', 'a', 1),
      text('prose'),
      new StubToken('link_close', 'a', -1)
    ]
    const out = replaceTextTokens(
      children, StubToken, /prose/g, () => [text('NO')], { skipInsideLinks: true }
    )
    expect(out.map(t => t.content)).toEqual(['', 'prose', ''])
  })
})

describe('walkBodyInlines', () => {
  const inline = (content: string): StubToken =>
    Object.assign(new StubToken('inline', '', 0), { children: [text(content)] })

  it('visits body inlines and skips the inline following a heading', () => {
    const tokens = [
      new StubToken('paragraph_open', 'p', 1), inline('body'),
      new StubToken('heading_open', 'h2', 1),  inline('heading'),
      inline('after heading')
    ]
    const seen: string[] = []
    walkBodyInlines({ tokens }, (_block, children) => seen.push(children[0].content))
    expect(seen).toEqual(['body', 'after heading'])
  })
})

describe('encodeLintMeta', () => {
  it('round-trips decorations through a base64url fence-meta token', () => {
    const decorations = [{ end: 4, properties: { class: 'lint-flag' }, start: 0 }]
    const meta = encodeLintMeta(decorations)
    expect(meta.startsWith('lintdeco-')).toBe(true)
    expect(decodeLintMeta(meta)).toEqual(decorations)
  })
})

describe('lintDecorations', () => {
  it('sorts findings by position and maps them to shiki decorations', () => {
    const findings = [
      { code: 'b', end_location: { column: 6, row: 2 }, location: { column: 3, row: 2 }, message: 'second' },
      {
        code         : 'a',
        end_location : { column: 4, row: 1 },
        fix          : { applicability: 'safe', edits: [{ before: 'x', content: 'y' }] },
        location     : { column: 1, row: 1 },
        message      : 'first'
      }
    ]
    expect(lintDecorations(findings)).toEqual([
      {
        end        : { character: 3, line: 0 },
        properties : { class: 'lint-flag', 'data-before': 'x', 'data-message': 'first', 'data-rule': 'a', 'data-suggested': 'y' },
        start      : { character: 0, line: 0 }
      },
      {
        end        : { character: 5, line: 1 },
        properties : { class: 'lint-flag', 'data-message': 'second', 'data-rule': 'b' },
        start      : { character: 2, line: 1 }
      }
    ])
  })
})

describe('lintDecorationTransformer', () => {
  const preprocess = lintDecorationTransformer.preprocess as unknown as
    (code: string, options: { decorations?: unknown[]; meta?: { __raw?: string } }) => void

  it('decodes the fence-meta token into options.decorations', () => {
    const decorations = [{ end: 3, properties: { class: 'lint-flag' }, start: 0 }]
    const options: { decorations?: unknown[]; meta?: { __raw?: string } } =
      { meta: { __raw: `python ${encodeLintMeta(decorations)}` } }
    preprocess('', options)
    expect(options.decorations).toEqual(decorations)
  })

  it('leaves decorations untouched when no lint token is present', () => {
    const options: { decorations?: unknown[]; meta?: { __raw?: string } } = { meta: { __raw: 'python' } }
    preprocess('', options)
    expect(options.decorations).toBeUndefined()
  })
})
