import * as typewriter                from '../../lib/sandbox/typewriter'
import type { TokenLine, TypingSide } from '../../lib/sandbox/typewriter'

vi.mock('../../lib/markdown/highlighter', () => import('../highlighter-stub'))

const LINE: TokenLine = [
  { content: 'ab',  style: 'color:red' },
  { content: '<c>', style: '' }
]

describe('typewriter.lineHtml', () => {
  it.each([
    [0, ''],
    [1, '<span style="color:red">a</span>'],
    [2, '<span style="color:red">ab</span>'],
    [3, '<span style="color:red">ab</span><span style="">&lt;</span>'],
    [Number.POSITIVE_INFINITY, '<span style="color:red">ab</span><span style="">&lt;c&gt;</span>']
  ])('renders the first %s characters as styled spans', (chars, expected) => {
    expect(typewriter.lineHtml(LINE, chars)).toBe(expected)
  })
})

describe('typewriter.tokenLines', () => {
  it('maps each line to styled tokens through the shared highlighter', async () => {
    expect(await typewriter.tokenLines('a = 1\n')).toEqual([
      [{ content: 'a = 1', style: 'color:red' }],
      []
    ])
  })

  it('leaves a token no theme rule matches as an empty style string', async () => {
    expect(await typewriter.tokenLines('  ')).toEqual([[{ content: '  ', style: '' }]])
  })
})

describe('typewriter.typingFrame', () => {
  const SIDE: TypingSide    = { lines: ['a = 1', 'b = 2', 'c = 3'], max: 5, midEnd: 2 }
  const TOKENS: TokenLine[] = SIDE.lines.map(line => [{ content: line, style: 'color:red' }])

  it('holds the lines outside the changed middle and truncates the one inside it', () => {
    const { html, text } = typewriter.typingFrame(TOKENS, SIDE, 1, 2)
    const lines = html.split('\n')
    expect(text).toBe('a = 1\nb \nc = 3')
    expect(lines[0]).toBe('<span style="color:red">a = 1</span>')
    expect(lines[1]).toBe(
      '<span style="color:red">b </span><span class="code-caret" aria-hidden="true"></span>'
    )
    expect(lines[2]).toBe('<span style="color:red">c = 3</span>')
  })

  it('renders an empty line when the tokens run short of the lines', () => {
    const { html, text } = typewriter.typingFrame([], SIDE, 1, 2)
    expect(text).toBe('a = 1\nb \nc = 3')
    expect(html.split('\n')[0]).toBe('')
  })
})

describe('typewriter.typingPlan', () => {
  it.each([
    ['a\nb\nc', 'a\nx\nc', { cur: { max: 1, midEnd: 2 }, floor: 0, next: { max: 1, midEnd: 2 }, prefix: 1 }],
    ['a\nbb',   'a\nbc',   { cur: { max: 2, midEnd: 2 }, floor: 1, next: { max: 2, midEnd: 2 }, prefix: 1 }],
    ['',        'x = 1',   { cur: { max: 0, midEnd: 1 }, floor: 0, next: { max: 5, midEnd: 1 }, prefix: 0 }],
    ['same',    'same',    { cur: { max: 0, midEnd: 1 }, floor: 0, next: { max: 0, midEnd: 1 }, prefix: 1 }],
    ['a',       'a\nb',    { cur: { max: 0, midEnd: 1 }, floor: 0, next: { max: 1, midEnd: 2 }, prefix: 1 }]
  ])('plans the run from %j to %j', (current, next, expected) => {
    expect(typewriter.typingPlan(current, next)).toEqual({
      ...expected,
      cur  : { ...expected.cur,  lines: current.split('\n') },
      next : { ...expected.next, lines: next.split('\n') }
    })
  })

  it('sweeps a bulk change from column zero rather than a shared floor', () => {
    const plan = typewriter.typingPlan('aa\nbb', 'ax\nbx')
    expect(plan.prefix).toBe(0)
    expect(plan.floor).toBe(0)
    expect(plan.cur.max).toBe(2)
  })
})
