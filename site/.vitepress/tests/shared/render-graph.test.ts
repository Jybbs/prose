import fs            from 'node:fs'
import { isBuiltin } from 'node:module'
import path          from 'node:path'

import { hash } from 'ohash'

import { siteDir }             from '../../lib/shared/paths'
import { readRenderGraph }     from '../../lib/shared/render-graph'
import { readPackageVersions } from '../../lib/shared/version'
import { supportTest }         from '../support'

describe('readRenderGraph', () => {
  const site = siteDir(import.meta.url)

  it('reaches a module and a package imported only through another module', async () => {
    const graph = await readRenderGraph(path.join(import.meta.dirname, '../../lib/og/render'))
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
    const graph = await readRenderGraph(tmpDir)
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
    const graph = await readRenderGraph(tmpDir)
    expect(Object.keys(graph.modules).toSorted()).toStrictEqual(['entry.ts', 'lib/size.ts'])
  })

  supportTest('moves the key when a module reached through another changes', async ({ tmpDir }) => {
    const write = (file: string, text: string) => fs.writeFileSync(path.join(tmpDir, file), text)
    fs.mkdirSync(path.join(tmpDir, 'lib'))
    write('entry.ts', "import { mid } from './lib/mid'\nexport const entry = mid\n")
    write('lib/mid.ts', "import { formatHex } from 'culori'\nimport { leaf } from './leaf'\n"
                      + 'export const mid = formatHex(leaf)\n')
    write('lib/leaf.ts', "export const leaf = '#ffffff'\n")
    const before = await readRenderGraph(tmpDir)
    write('lib/leaf.ts', "export const leaf = '#000000'\n")
    const after = await readRenderGraph(tmpDir)
    expect(Object.keys(before.modules).toSorted())
      .toStrictEqual(['entry.ts', 'lib/leaf.ts', 'lib/mid.ts'])
    expect(before.packages).toStrictEqual(readPackageVersions(site, ['culori']))
    expect(hash(after)).not.toBe(hash(before))
  })
})
