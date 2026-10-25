import { themeSheets } from '../support'

const SHEETS = themeSheets()

describe('the theme cascade', () => {
  it('keeps the focus ring, since no stylesheet removes an outline with `!important`', () => {
    const removed = SHEETS.flatMap(({ file, root }) => {
      const lines = [] as string[]
      root.walkDecls(/^outline/, decl => {
        if (decl.important) lines.push(`${file}:${decl.source?.start?.line}`)
      })
      return lines
    })
    expect(removed).toStrictEqual([])
  })

  it('resets a breakpoint declaration on every ancestor-qualified selector that sets it', () => {
    const outranked = SHEETS.flatMap(({ file, root }) => {
      const base = [] as { prop: string, selector: string }[]
      root.each(node => {
        if (node.type !== 'rule') return
        node.walkDecls(decl => {
          for (const selector of node.selectors) base.push({ prop: decl.prop, selector })
        })
      })
      const misses = [] as string[]
      root.walkAtRules('media', media => media.walkRules(rule => rule.walkDecls(decl => {
        for (const selector of rule.selectors) {
          for (const set of base) {
            if (set.prop !== decl.prop || !set.selector.endsWith(` ${selector}`)) continue
            if (rule.selectors.includes(set.selector)) continue
            misses.push(`${file}:${decl.source?.start?.line} ${selector} misses ${set.selector}`)
          }
        }
      })))
      return misses
    })
    expect(outranked).toStrictEqual([])
  })

  it('keeps every outline out of the glossary and rule-card sheets', () => {
    const popups = new Set([
      'components/glossary/styles/glossary.css',
      'components/rules/styles/specimen-callout.css'
    ])
    const declared = SHEETS.filter(({ file }) => popups.has(file)).flatMap(({ file, root }) => {
      const lines = [] as string[]
      root.walkDecls(/^outline/, decl => { lines.push(`${file}:${decl.source?.start?.line}`) })
      return lines
    })
    expect(declared).toStrictEqual([])
  })
})
