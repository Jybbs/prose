import * as typingDemo from '../../lib/landing/typing-demo'

describe('typing-demo source data', () => {
  it('leads with one edit entry per rule, then the trailing config edits', () => {
    expect(typingDemo.ENTRIES.length).toBeGreaterThan(typingDemo.RULES.length)
    expect(typingDemo.ENTRIES.slice(0, typingDemo.RULES.length).map(e => e.slug)).toEqual([...typingDemo.RULES])
  })

  it('anchors each rule edit on a padded assignment', () => {
    const width = Math.max(...typingDemo.RULES.map(r => r.length))
    expect(typingDemo.ENTRIES[0].anchor).toBe(`${typingDemo.RULES[0].padEnd(width)} = `)
  })

  it('renders the prelude with every rule under a [rules] table', () => {
    expect(typingDemo.PRELUDE).toContain('[rules]')
    for (const rule of typingDemo.RULES) expect(typingDemo.PRELUDE).toContain(rule)
  })

  it('dedups reset rows so each anchor appears once', () => {
    const anchors = typingDemo.RESET_ROWS.map(r => r.anchor)
    expect(new Set(anchors).size).toBe(anchors.length)
  })

  it.each([...typingDemo.RESET_ROWS])('reset row $anchor spans first prelude to last end', row => {
    const matching = typingDemo.ENTRIES.filter(entry => entry.anchor === row.anchor)
    expect(row.prelude).toBe(matching[0].from)
    expect(row.end).toBe(matching.at(-1)?.to)
  })
})
