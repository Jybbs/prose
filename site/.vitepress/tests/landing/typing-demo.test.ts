import { ENTRIES, PRELUDE, RESET_ROWS, RULES } from '../../lib/landing/typing-demo'

describe('typing-demo source data', () => {
  it('leads with one edit entry per rule, then the trailing config edits', () => {
    expect(ENTRIES.length).toBeGreaterThan(RULES.length)
    expect(ENTRIES.slice(0, RULES.length).map(e => e.slug)).toEqual([...RULES])
  })

  it('anchors each rule edit on a padded assignment', () => {
    const width = Math.max(...RULES.map(r => r.length))
    expect(ENTRIES[0].anchor).toBe(`${RULES[0].padEnd(width)} = `)
  })

  it('renders the prelude with every rule under a [rules] table', () => {
    expect(PRELUDE).toContain('[rules]')
    for (const rule of RULES) expect(PRELUDE).toContain(rule)
  })

  it('dedups reset rows so each anchor appears once', () => {
    const anchors = RESET_ROWS.map(r => r.anchor)
    expect(new Set(anchors).size).toBe(anchors.length)
  })
})
