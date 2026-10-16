import fs            from 'node:fs'
import { isBuiltin } from 'node:module'
import path          from 'node:path'

import { enumeratePages }          from '../../lib/og/pages'
import * as cardKey                from '../../lib/og/render/card-key'
import { siteDir }                 from '../../lib/shared/paths'
import { readPackageVersions }     from '../../lib/shared/version'
import { fixtureDir, supportTest } from '../support'

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
    const keyOf = (g: cardKey.RenderGraph) => cardKey.cardKeyer(brand, g, '0.1.0')('landing')
    expect(keyOf(graph))
      .not.toBe(keyOf({ ...graph, modules: { 'parts.ts': 'export const A = 2' } }))
    expect(keyOf(graph))
      .not.toBe(keyOf({ ...graph, packages: { ...graph.packages, satori: '2.1.0' } }))
  })
})

describe('readRenderGraph', () => {
  const site = siteDir(import.meta.url)

  it('reaches a module and a package imported only through another module', async () => {
    const graph = await cardKey.readRenderGraph()
    expect(Object.keys(graph.modules)).toContain('../../markdown/inert-env.ts')
    expect(graph.packages).toMatchObject(readPackageVersions(site, ['culori']))
    expect(Object.keys(graph.packages).some(isBuiltin)).toBe(false)
  })

  supportTest('follows a dynamic import and keys each package by its name', async ({ tmpDir }) => {
    const write = (file: string, text: string) => fs.writeFileSync(path.join(tmpDir, file), text)
    fs.mkdirSync(path.join(tmpDir, 'lib'))
    write('entry.ts', [
      "import fs from 'node:fs'",
      "import { createElement } from 'satori/jsx'",
      "import type { Leaf } from './lib/types'",
      "import { getSVGViewBox } from '@iconify/utils'",
      "import * as ns from './lib/ns'",
      'export const entry: Leaf[] = [String(fs), String(createElement), String(getSVGViewBox)]',
      "export const later = () => import('./lib/late')",
      "export const hasher = () => import('ohash')",
      'export const space = ns'
    ].join('\n'))
    write('lib/late.ts', 'export const late = 1\n')
    write('lib/ns.ts', 'export const ns = 1\n')
    write('lib/types.ts', 'export type Leaf = string\n')
    const graph = await cardKey.readRenderGraph(tmpDir)
    expect(Object.keys(graph.modules).toSorted())
      .toStrictEqual(['entry.ts', 'lib/late.ts', 'lib/ns.ts'])
    expect(graph.packages)
      .toStrictEqual(readPackageVersions(site, ['@iconify/utils', 'ohash', 'satori']))
  })

  supportTest('keeps a module whose constant the bundler inlines', async ({ tmpDir }) => {
    fs.mkdirSync(path.join(tmpDir, 'lib'))
    fs.writeFileSync(path.join(tmpDir, 'entry.ts'),
      "import { SIZE } from './lib/size'\nexport const area = SIZE * SIZE\n")
    fs.writeFileSync(path.join(tmpDir, 'lib/size.ts'), 'export const SIZE = 180\n')
    const graph = await cardKey.readRenderGraph(tmpDir)
    expect(Object.keys(graph.modules).toSorted()).toStrictEqual(['entry.ts', 'lib/size.ts'])
  })

  supportTest('moves the key when a module reached through another changes', async ({ tmpDir }) => {
    const write = (file: string, text: string) => fs.writeFileSync(path.join(tmpDir, file), text)
    fs.mkdirSync(path.join(tmpDir, 'lib'))
    write('entry.ts', "import { mid } from './lib/mid'\nexport const entry = mid\n")
    write('lib/mid.ts', "import { formatHex } from 'culori'\nimport { leaf } from './leaf'\n"
                      + 'export const mid = formatHex(leaf)\n')
    write('lib/leaf.ts', "export const leaf = '#ffffff'\n")
    const before = await cardKey.readRenderGraph(tmpDir)
    write('lib/leaf.ts', "export const leaf = '#000000'\n")
    const after = await cardKey.readRenderGraph(tmpDir)
    expect(Object.keys(before.modules).toSorted())
      .toStrictEqual(['entry.ts', 'lib/leaf.ts', 'lib/mid.ts'])
    expect(before.packages).toStrictEqual(readPackageVersions(site, ['culori']))
    expect(cardKey.cardKeyer(brand, after, '0.1.0')('landing'))
      .not.toBe(cardKey.cardKeyer(brand, before, '0.1.0')('landing'))
  })
})
