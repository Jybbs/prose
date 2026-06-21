import { resolveToken }   from '../lib/shared/css-token'
import { enumeratePages } from '../lib/og/pages'
import { fixtureDir }     from './support'

describe('resolveToken', () => {
  it('follows a single var() hop to a leaf hex', () => {
    expect(resolveToken('prose-c-family-engine')).toMatch(/^#[0-9a-f]{6}$/i)
  })

  it('returns a non-aliased value directly', () => {
    expect(resolveToken('prose-c-ube')).toMatch(/^#[0-9a-f]{6}$/i)
  })

  it('returns an empty string for an unknown token', () => {
    expect(resolveToken('prose-c-not-a-real-token')).toBe('')
  })
})

describe('enumeratePages', () => {
  const srcDir = fixtureDir('og-pages')

  it('shapes an OgPage per chapter page, skipping index and off-chapter pages', () => {
    const pages = [
      'index.md',
      'blog/post.md',
      'rules/index.md',
      'rules/alignment/index.md',
      'rules/alignment/demo-rule.md',
      'primitives/aligner.md',
      'reference/cli.md',
      'usage/quick-start.md',
      'integrations/editor.md'
    ]
    expect(enumeratePages(srcDir, pages)).toMatchSnapshot()
  })
})
