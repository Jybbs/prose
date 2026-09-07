import fs   from 'node:fs'
import path from 'node:path'

import { enumeratePages }       from '../../lib/og/pages'
import { cardKeyer, RENDERERS } from '../../lib/og/render/card-key'
import { siteDir }              from '../../lib/shared/paths'
import { readPackageVersions }  from '../../lib/shared/version'
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
      'primitives/aligner.md',
      'reference/cli.md',
      'reference/named.md',
      'usage/quick-start.md',
      'integrations/editor.md'
    ]
    expect(enumeratePages(srcDir, pages)).toMatchSnapshot()
  })

  it('attaches the pipeline position for a rule in the pipeline', () => {
    const [page] = enumeratePages(srcDir, ['rules/alignment/alphabetize-siblings.md'])
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

  const renderers = { '@resvg/resvg-js': '1.0.0', satori: '2.0.0' }

  it('gives the same key for the same input', () => {
    const keyOf = cardKeyer(brand, '0.1.0', renderers)
    expect(keyOf('landing')).toBe(cardKeyer(brand, '0.1.0', renderers)('landing'))
  })

  it('gives a different key when the version or the card changes', () => {
    const keyOf = cardKeyer(brand, '0.1.0', renderers)
    expect(keyOf('landing')).not.toBe(cardKeyer(brand, '0.2.0', renderers)('landing'))
    expect(keyOf('landing'))
      .not.toBe(keyOf({ breadcrumb: [], kind: 'usage', outputPath: 'o', title: 'T' }))
  })

  it('gives a different key when a renderer version changes', () => {
    const keyOf = (r: Record<string, string>) => cardKeyer(brand, '0.1.0', r)('landing')
    expect(keyOf(renderers)).not.toBe(keyOf({ ...renderers, satori: '2.1.0' }))
  })

  it('keys on the real renderer pins when none are passed', () => {
    const pinned = readPackageVersions(siteDir(import.meta.url), RENDERERS)
    expect(cardKeyer(brand, '0.1.0')('landing'))
      .toBe(cardKeyer(brand, '0.1.0', pinned)('landing'))
  })
})

describe('RENDERERS', () => {
  const packageOf = (spec: string): string =>
    spec.split('/', spec.startsWith('@') ? 2 : 1).join('/')

  const dir   = path.join(import.meta.dirname, '../../lib/og/render')
  const bare  = /from '([^.'][^']*)'/g
  const specs = new Set(fs.readdirSync(dir)
    .filter(file => /\.(ts|mjs)$/.test(file))
    .flatMap(file => [...fs.readFileSync(path.join(dir, file), 'utf8').matchAll(bare)])
    .map(([, spec]) => packageOf(spec)))

  it.each(RENDERERS)('%s is still imported by the card renderer', name => {
    expect(specs).toContain(name)
  })
})
