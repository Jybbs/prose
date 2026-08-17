import fs   from 'node:fs'
import path from 'node:path'

import * as scopes from '../../lib/suppression/scopes'

const page    = fs.readFileSync(
  path.join(import.meta.dirname, '../../../reference/suppression-directives.md'), 'utf8'
)
const anchors = [...page.matchAll(/^## (.+)$/gm)].map(([, heading]) =>
  heading.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replaceAll(/^-|-$/g, ''))

describe('directiveHref', () => {
  it.each(scopes.SCOPE_ORDER)('anchors the %s scope to a heading on the reference page', scope => {
    const [route, anchor] = scopes.directiveHref(scope).split('#')
    expect(route).toBe('/reference/suppression-directives')
    expect(anchors).toContain(anchor)
  })
})

describe('scopeBands', () => {
  it('groups scope-keyed items into the shared band order', () => {
    const bands = scopes.scopeBands([
      { id : 'a', scope : 'line' },
      { id : 'b', scope : 'file' },
      { id : 'c', scope : 'line' }
    ] as const)
    expect(bands.map(b => b.scope)).toEqual([...scopes.SCOPE_ORDER])
    expect(bands.map(b => b.items.map(i => i.id))).toEqual([['b'], [], ['a', 'c'], []])
  })
})
