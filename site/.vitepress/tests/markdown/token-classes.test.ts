import { addClassToHast, type ShikiTransformer, type ShikiTransformerContext } from 'shiki'

import { codeHighlighter }                        from '../../lib/markdown/highlighter'
import { TokenClasses }                           from '../../lib/markdown/token-classes'
import { SHIKI_COLOR_REPLACEMENTS, SHIKI_THEMES } from '../../lib/shared/constants'

type Element = Parameters<NonNullable<ShikiTransformer['span']>>[0]

const span = (style: string): Element =>
  ({ children: [], properties: { style }, tagName: 'span', type: 'element' })

describe('TokenClasses', () => {
  it('reads one instance for every caller', async () => {
    await expect(TokenClasses.read()).resolves.toBe(await TokenClasses.read())
  })

  it.each([
    '.sd-e1e4e8 { --shiki-dark:#e1e4e8; }\n',
    '.sd-italic { --shiki-dark-font-style:italic; }\n',
    '.sd-line-through { --shiki-dark-text-decoration:line-through; }\n',
    '.sl-62687b { --shiki-light:#62687b; }\n',
    '.sl-underline-line-through { --shiki-light-text-decoration:underline line-through; }\n'
  ])('writes the rule %s', async rule => {
    expect((await TokenClasses.read()).css()).toContain(rule)
  })

  it('moves every declaration a dual-theme highlight writes onto a class the stylesheet holds', async () => {
    const classes     = await TokenClasses.read()
    const highlighter = await codeHighlighter()
    const html        = highlighter.codeToHtml('# note\nx = f"{y!r}" if z else None\n', {
      colorReplacements : SHIKI_COLOR_REPLACEMENTS,
      defaultColor      : false,
      lang              : 'python',
      themes            : SHIKI_THEMES,
      transformers      : [classes.transformer()]
    })
    expect(html).not.toMatch(/<span style=/)
    const names = new Set(html.matchAll(/\bs[dl]-[\da-z-]+/g).map(([name]) => name))
    expect(names.size).toBeGreaterThan(4)
    const css = classes.css()
    expect([...names].filter(name => !css.includes(`.${name} {`))).toStrictEqual([])
  })

  it.each([
    [
      'keeps each declaration no class covers inline',
      '--shiki-dark:#E1E4E8;user-select:none;',
      { class: ['sd-e1e4e8'], style: 'user-select:none' }
    ],
    [
      'drops the style of a span every class covers',
      '--shiki-dark:#E1E4E8;--shiki-light-font-weight:bold',
      { class: ['sd-e1e4e8', 'sl-bold'] }
    ]
  ])('%s', async (_name, style, properties) => {
    const node    = span(style)
    const context = { addClassToHast } as unknown as ShikiTransformerContext
    ;(await TokenClasses.read()).transformer().span?.call(context, node, 1, 0, span(''), { content: '', offset: 0 })
    expect(node.properties).toStrictEqual(properties)
  })
})
