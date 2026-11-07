import { enumeratePages }       from '../../lib/og/pages'
import * as cardKey             from '../../lib/og/render/card-key'
import type { RenderGraph }     from '../../lib/shared/render-graph'
import { fixtureDir }           from '../support'

describe('enumeratePages', () => {
  const srcDir = fixtureDir(import.meta.dirname)

  it('shapes an OgPage per chapter page, skipping index and off-chapter pages', () => {
    const pages = [
      'index.md',
      'blog/post.md',
      'rules/index.md',
      'rules/alignment/index.md',
      'rules/alignment/demo-rule.md',
      'architecture/aligner.md',
      'reference/cli.md',
      'reference/named.md',
      'guide/quick-start.md',
      'integrations/editor.md'
    ]
    expect(enumeratePages(srcDir, pages)).toMatchSnapshot()
  })

  it('attaches the pipeline position for a rule in the pipeline', () => {
    const [page] = enumeratePages(srcDir, ['rules/alignment/alphabetize-siblings.md'])
    expect(page.pipeline).toMatchObject({ position: expect.any(Number), total: expect.any(Number) })
  })

  it('falls back to internal stability and the titled slug for an undiscovered primitive', () => {
    const [page] = enumeratePages(srcDir, ['architecture/ghost.md'])
    expect(page).toMatchObject({ primitive: { stability: 'internal' }, title: 'Ghost' })
  })
})

const brand = {
  fonts            : [],
  glyph            : 'g',
  titleWithTagline : { aspect: 1, src: 't' },
  wordmark         : { aspect: 1, src: 'w' }
}

describe('cardKeyer', () => {
  const graph = {
    modules  : { 'parts.ts': 'export const A = 1' },
    packages : { '@resvg/resvg-js': '1.0.0', satori: '2.0.0' }
  }

  it('gives the same key for the same input', () => {
    const keyOf = cardKey.cardKeyer(brand, graph, '0.1.0')
    expect(keyOf('landing')).toBe(cardKey.cardKeyer(brand, graph, '0.1.0')('landing'))
  })

  it('gives a different key when the version or the card changes', () => {
    const keyOf = cardKey.cardKeyer(brand, graph, '0.1.0')
    expect(keyOf('landing')).not.toBe(cardKey.cardKeyer(brand, graph, '0.2.0')('landing'))
    expect(keyOf('landing')).not.toBe(keyOf('icon'))
    expect(keyOf('landing'))
      .not.toBe(keyOf({ breadcrumb: [], kind: 'guide', outputPath: 'o', title: 'T' }))
  })

  it('gives a different key when a module source or a package version changes', () => {
    const keyOf = (g: RenderGraph) => cardKey.cardKeyer(brand, g, '0.1.0')('landing')
    expect(keyOf(graph))
      .not.toBe(keyOf({ ...graph, modules: { 'parts.ts': 'export const A = 2' } }))
    expect(keyOf(graph))
      .not.toBe(keyOf({ ...graph, packages: { ...graph.packages, satori: '2.1.0' } }))
  })
})
