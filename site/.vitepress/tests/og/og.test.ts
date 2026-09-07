import fs   from 'node:fs'
import path from 'node:path'

import { enumeratePages }       from '../../lib/og/pages'
import * as cardKey             from '../../lib/og/render/card-key'
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

  const packages = { '@resvg/resvg-js': '1.0.0', satori: '2.0.0' }

  it('gives the same key for the same input', () => {
    const keyOf = cardKey.cardKeyer(brand, '0.1.0', packages)
    expect(keyOf('landing')).toBe(cardKey.cardKeyer(brand, '0.1.0', packages)('landing'))
  })

  it('gives a different key when the version or the card changes', () => {
    const keyOf = cardKey.cardKeyer(brand, '0.1.0', packages)
    expect(keyOf('landing')).not.toBe(cardKey.cardKeyer(brand, '0.2.0', packages)('landing'))
    expect(keyOf('landing'))
      .not.toBe(keyOf({ breadcrumb: [], kind: 'usage', outputPath: 'o', title: 'T' }))
  })

  it('gives a different key when a keyed package version changes', () => {
    const keyOf = (r: Record<string, string>) => cardKey.cardKeyer(brand, '0.1.0', r)('landing')
    expect(keyOf(packages)).not.toBe(keyOf({ ...packages, satori: '2.1.0' }))
  })

  it('keys on the real package pins when none are passed', () => {
    const pinned = readPackageVersions(siteDir(import.meta.url), cardKey.KEYED_PACKAGES)
    expect(cardKey.cardKeyer(brand, '0.1.0')('landing'))
      .toBe(cardKey.cardKeyer(brand, '0.1.0', pinned)('landing'))
  })
})

describe('the card modules', () => {
  const packageOf = (spec: string): string =>
    spec.split('/', spec.startsWith('@') ? 2 : 1).join('/')

  const dir     = path.join(import.meta.dirname, '../../lib/og/render')
  const sources = fs.readdirSync(dir)
    .filter(file => /\.(ts|mjs)$/.test(file))
    .map(file => fs.readFileSync(path.join(dir, file), 'utf8'))

  const imported = (pattern: RegExp, take: (spec: string) => string) =>
    new Set(sources.flatMap(text => [...text.matchAll(pattern)]).map(([, spec]) => take(spec)))

  const packages = imported(/from '([^.'][^']*)'/g, packageOf)
  const outside  = imported(/from '(\.\.?\/[^']*)'/g, spec => `${spec}.ts`)

  const listed = [...cardKey.KEYED_PACKAGES, ...cardKey.UNKEYED_PACKAGES]

  it.each([...packages].filter(name => !name.startsWith('node:')))(
    '%s is either keyed into the cache key or listed as unkeyed', name => {
      expect(listed).toContain(name)
    }
  )

  it.each(listed)('%s is still imported', name => {
    expect(packages).toContain(name)
  })

  it.each([...outside].filter(spec => !spec.startsWith('./')))(
    '%s is hashed into the template digest', spec => {
      expect(cardKey.SHARED_SOURCES).toContain(spec)
    }
  )

  it.each(cardKey.SHARED_SOURCES)('%s is still imported from outside the directory', spec => {
    expect(outside).toContain(spec)
  })
})
