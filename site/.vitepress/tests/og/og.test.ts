import { enumeratePages } from '../../lib/og/pages'
import { cardKeyer }      from '../../lib/og/render/cache'
import { fixtureDir }     from '../support'

describe('enumeratePages', () => {
  const srcDir = fixtureDir(import.meta.dirname)

  it('shapes an OgPage per chapter page, skipping index and off-chapter pages', () => {
    const pages = [
      'index.md',
      'blog/post.md',
      'rules/index.md',
      'rules/alignment/index.md',
      'rules/alignment/demo-rule.md',
      'primitives/aligner.md',
      'reference/cli.md',
      'reference/named.md',
      'usage/quick-start.md',
      'integrations/editor.md'
    ]
    expect(enumeratePages(srcDir, pages)).toMatchSnapshot()
  })

  it('attaches the pipeline position for a rule in the pipeline', () => {
    const [page] = enumeratePages(srcDir, ['rules/alignment/alphabetize.md'])
    expect(page.pipeline).toMatchObject({ position: expect.any(Number), total: expect.any(Number) })
  })

  it('falls back to internal stability and the titled slug for an undiscovered primitive', () => {
    const [page] = enumeratePages(srcDir, ['primitives/ghost.md'])
    expect(page).toMatchObject({ primitive: { stability: 'internal' }, title: 'Ghost' })
  })
})

describe('cardKeyer', () => {
  const brand = {
    fonts            : [],
    glyph            : 'g',
    titleWithTagline : { aspect: 1, src: 't' },
    wordmark         : { aspect: 1, src: 'w' }
  }

  it('keys stably per input and re-keys when the version or card changes', () => {
    const keyOf = cardKeyer('0.1.0', brand)
    expect(keyOf('landing')).toBe(cardKeyer('0.1.0', brand)('landing'))
    expect(keyOf('landing')).not.toBe(cardKeyer('0.2.0', brand)('landing'))
    expect(keyOf('landing')).not.toBe(keyOf({ breadcrumb: [], kind: 'usage', outputPath: 'o', title: 'T' }))
  })
})
