import { markdownH1 }        from '../../lib/markdown/h1'
import * as decorations      from '../../lib/markdown/lint-decorations'
import { replaceTextTokens } from '../../lib/markdown/token-split'
import { walkBodyInlines }   from '../../lib/markdown/walk'

class StubToken {
  children: StubToken[] | null = null
  content = ''
  level   = 0
  constructor(public type: string, public tag: string, public nesting: number) {}
}

const text = (content: string): StubToken =>
  Object.assign(new StubToken('text', '', 0), { content })

describe('markdownH1', () => {
  it('extracts the first H1 heading', () => {
    expect(markdownH1('# Aligner\n\nbody')).toBe('Aligner')
  })

  it('skips a deeper heading to find the H1', () => {
    expect(markdownH1('## Subhead\n# Top')).toBe('Top')
  })

  it('returns undefined when no H1 is present', () => {
    expect(markdownH1('no heading here')).toBeUndefined()
  })
})

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

  it('returns the text token unchanged when nothing matches', () => {
    const out = replaceTextTokens([text('nothing to see')], StubToken, /xyz/g, () => [text('X')])
    expect(out.map(t => t.content)).toEqual(['nothing to see'])
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

describe('lintDecorations', () => {
  const spanning = (column: number, endColumn: number, row = 1, endRow = row) => ({
    code         : 'a',
    end_location : { column: endColumn, row: endRow },
    location     : { column, row },
    message      : 'm'
  })

  const classOf = (code: string, column: number, endColumn: number): string =>
    String(decorations.lintDecorations([spanning(column, endColumn)], code)[0].properties?.class)

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
    expect(decorations.lintDecorations(findings, 'x = 1\nyyyyy')).toEqual([
      {
        end        : { character: 3, line: 0 },
        properties : {
          class            : 'lint-flag underline-draw',
          'data-message'   : 'first',
          'data-replaced'  : 'x',
          'data-rule'      : 'a',
          'data-suggested' : 'y'
        },
        start      : { character: 0, line: 0 }
      },
      {
        end        : { character: 5, line: 1 },
        properties : { class: 'lint-flag underline-draw', 'data-message': 'second', 'data-rule': 'b' },
        start      : { character: 2, line: 1 }
      }
    ])
  })

  it('orders two findings sharing a row by column', () => {
    const findings = [spanning(4, 6), spanning(1, 3)]
    expect(decorations.lintDecorations(findings, 'x = 1').map(item => item.start))
      .toEqual([{ character: 0, line: 0 }, { character: 3, line: 0 }])
  })

  it('omits the edit attributes when the fix carries neither side', () => {
    const finding = { ...spanning(1, 4), fix: { applicability: 'safe', edits: [{ before: '', content: '' }] } }
    expect(decorations.lintDecorations([finding], 'x = 1')[0].properties)
      .toEqual({ class: 'lint-flag underline-draw', 'data-message': 'm', 'data-rule': 'a' })
  })

  it.each([
    ['a span reaching the whole line',        'x = 1', 1, 6, 'lint-flag lint-flag-line'],
    ['a span opening past the first column',  'x = 1', 3, 6, 'lint-flag underline-draw'],
    ['a span stopping short of the line end', 'x = 1', 1, 4, 'lint-flag underline-draw'],
    ['a span whose end row has no line',      '',      1, 6, 'lint-flag underline-draw']
  ])('classes %s', (_name, code, column, endColumn, expected) => {
    expect(classOf(code, column, endColumn)).toBe(expected)
  })

  it('measures the line end in UTF scalar values rather than code units', () => {
    expect(classOf('x = "🪻"', 1, 8)).toBe('lint-flag lint-flag-line')
  })

  it('measures a multi-row span against its last line', () => {
    expect(decorations.lintDecorations([spanning(1, 6, 1, 2)], 'xx\nx = 1')[0].properties?.class)
      .toBe('lint-flag lint-flag-line')
  })
})

describe('lintDecorationTransformer', () => {
  const findings = new Map([
    ['demo-rule/basic', [{ code: 'a', end_location: { column: 4, row: 1 }, location: { column: 1, row: 1 }, message: 'm' }]]
  ])
  const preprocess = decorations.lintDecorationTransformer(findings).preprocess as unknown as
    (code: string, options: { decorations?: unknown[], meta?: { __raw?: string } }) => void

  it('computes decorations for the fixture the lint token names', () => {
    const options: { decorations?: unknown[], meta?: { __raw?: string } } =
      { meta: { __raw: `python ${decorations.lintFenceMeta('demo-rule/basic')}` } }
    preprocess('', options)
    expect(options.decorations).toEqual([{
      end        : { character: 3, line: 0 },
      properties : { class: 'lint-flag underline-draw', 'data-message': 'm', 'data-rule': 'a' },
      start      : { character: 0, line: 0 }
    }])
  })

  it('measures the row treatment against the fence it preprocesses', () => {
    const options: { decorations?: unknown[], meta?: { __raw?: string } } =
      { meta: { __raw: `python ${decorations.lintFenceMeta('demo-rule/basic')}` } }
    preprocess('x =', options)
    expect(options.decorations).toEqual([{
      end        : { character: 3, line: 0 },
      properties : { class: 'lint-flag lint-flag-line', 'data-message': 'm', 'data-rule': 'a' },
      start      : { character: 0, line: 0 }
    }])
  })

  it('leaves decorations untouched when no lint token is present', () => {
    const options: { decorations?: unknown[], meta?: { __raw?: string } } = { meta: { __raw: 'python' } }
    preprocess('', options)
    expect(options.decorations).toBeUndefined()
  })

  it('throws when the token names a fixture with no findings', () => {
    expect(() => preprocess('', { meta: { __raw: 'python lint=ghost/none' } }))
      .toThrow(/references no fixture findings/)
  })
})
