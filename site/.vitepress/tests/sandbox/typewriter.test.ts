import { lineHtml, tokenLines, typingPlan } from '../../lib/sandbox/typewriter'
import type { TokenLine }                   from '../../lib/sandbox/typewriter'

vi.mock('../../lib/markdown/highlighter', () => ({
  codeHighlighter: () => Promise.resolve({
    codeToTokens: (text: string) => ({
      tokens: text.split('\n').map(line =>
        line === '' ? [] : [{ content: line, htmlStyle: { color: 'red' } }])
    })
  })
}))

const LINE: TokenLine = [
  { content: 'ab',  style: 'color:red' },
  { content: '<c>', style: '' }
]

describe('lineHtml', () => {
  it.each([
    [0, ''],
    [1, '<span style="color:red">a</span>'],
    [2, '<span style="color:red">ab</span>'],
    [3, '<span style="color:red">ab</span><span style="">&lt;</span>'],
    [Number.POSITIVE_INFINITY, '<span style="color:red">ab</span><span style="">&lt;c&gt;</span>']
  ])('renders the first %s characters as styled spans', (chars, expected) => {
    expect(lineHtml(LINE, chars)).toBe(expected)
  })
})

describe('tokenLines', () => {
  it('maps each line to styled tokens through the shared highlighter', async () => {
    expect(await tokenLines('a = 1\n')).toEqual([
      [{ content: 'a = 1', style: 'color:red' }],
      []
    ])
  })
})

describe('typingPlan', () => {
  it.each([
    ['a\nb\nc', 'a\nx\nc', { curMax: 1, curMidEnd: 2, floor: 0, nextMax: 1, nextMidEnd: 2, prefix: 1 }],
    ['a\nbb',   'a\nbc',   { curMax: 2, curMidEnd: 2, floor: 1, nextMax: 2, nextMidEnd: 2, prefix: 1 }],
    ['',        'x = 1',   { curMax: 0, curMidEnd: 1, floor: 0, nextMax: 5, nextMidEnd: 1, prefix: 0 }],
    ['same',    'same',    { curMax: 0, curMidEnd: 1, floor: 0, nextMax: 0, nextMidEnd: 1, prefix: 1 }],
    ['a',       'a\nb',    { curMax: 0, curMidEnd: 1, floor: 0, nextMax: 1, nextMidEnd: 2, prefix: 1 }]
  ])('plans the run from %j to %j', (current, next, expected) => {
    expect(typingPlan(current, next)).toEqual({
      ...expected,
      curLines  : current.split('\n'),
      nextLines : next.split('\n')
    })
  })

  it('sweeps a bulk change from column zero rather than a shared floor', () => {
    const plan = typingPlan('aa\nbb', 'ax\nbx')
    expect(plan.prefix).toBe(0)
    expect(plan.floor).toBe(0)
    expect(plan.curMax).toBe(2)
  })
})
