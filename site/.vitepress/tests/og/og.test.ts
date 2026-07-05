import { enumeratePages } from '../../lib/og/pages'
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
