import fs   from 'node:fs'
import path from 'node:path'

import loader, { continuing }     from '../../lib/reference/section-glance.data'
import { siteDir }                from '../../lib/shared/paths'
import { FAMILY_ORDER, SECTIONS } from '../../lib/shared/registries'
import { familyRoute }            from '../../lib/shared/routes'

const data      = await loader.load([])
const site      = siteDir(import.meta.url)
const withPages = SECTIONS.filter(({ slug }) => data[slug].length > 0)

describe('section glance data', () => {
  it('lists every rule family page and the composition page under rules', () => {
    expect(data.rules.map(entry => entry.href)).toStrictEqual(expect.arrayContaining([
      ...FAMILY_ORDER.map(familyRoute),
      '/rules/composition/'
    ]))
    expect(data.rules.every(entry => entry.href.endsWith('/'))).toBe(true)
  })

  it('renders a code span in a title as code', () => {
    const source = data.architecture.find(entry => entry.href === '/architecture/source')
    expect(source?.titleNodes).toContainEqual({ kind: 'code', text: 'Source' })
  })

  it('leaves the sandbox, which holds no page beside its index, empty', () => {
    expect(data.sandbox).toStrictEqual([])
  })

  it('lowers the first letter of each description to follow its title', () => {
    for (const entry of SECTIONS.flatMap(({ slug }) => data[slug])) {
      expect(entry.descriptionNodes[0]).toMatchObject({ text: expect.stringMatching(/^(?:[a-z]|[A-Z]{2})/) })
    }
  })

  it.each([['Covers the rules', 'covers the rules'], ['SARIF output', 'SARIF output']])('continues %j as %j', (text, continued) => {
    expect(continuing(text)).toBe(continued)
  })

  it.each(withPages)('$slug opens its index page on its glance', ({ slug }) => {
    const index = fs.readFileSync(path.join(site, slug, 'index.md'), 'utf8')
    expect(index.split('\n').find(line => line.startsWith('## '))).toBe('## The Section at a Glance')
    expect(index).toContain(`<SectionGlance section="${slug}" />`)
  })
})
