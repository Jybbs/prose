import { bareInlineNodes, inlineText } from '../../lib/markdown/inline-text'

describe('bareInlineNodes', () => {
  it('leaves a wiki-link and a backticked slug as text and code with no plugin registered', () => {
    expect(bareInlineNodes('[[align-equals]] and `align-equals`')).toStrictEqual([
      { kind: 'text', text: '[[align-equals]] and ' },
      { kind: 'code', text: 'align-equals' }
    ])
  })
})

describe('inlineText', () => {
  it.each([
    ['Pads the space before `:` so entries share one column.', 'Pads the space before : so entries share one column.'],
    ['keeps the whitespace *Prose*’s rules write',              'keeps the whitespace Prose’s rules write'],
    ['nests **bold `code`** inside emphasis',                   'nests bold code inside emphasis'],
    ['joins a\nsoftbreak',                                      'joins a softbreak'],
    ['links [the CLI page](/reference/cli) by its text',        'links the CLI page by its text'],
    ['plain text',                                              'plain text']
  ])('flattens %j', (src, expected) => {
    expect(inlineText(src)).toBe(expected)
  })
})
