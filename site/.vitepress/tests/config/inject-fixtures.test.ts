import fs   from 'node:fs'
import path from 'node:path'

import type { PageData } from 'vitepress'

import { injectFixtures }       from '../../lib/config/inject-fixtures'
import type { PageFixtureSets } from '../../lib/fixtures/page-cases'
import * as walker              from '../../lib/fixtures/walker'
import { CASES, CRATE }         from '../corpus'
import { supportTest }          from '../support'

const SETS: PageFixtureSets = { composition: { byRule: {}, cases: [] }, ruleFixtures: {} }

const CANONICAL = CASES.find(c => walker.readFixtureDocs(c.inputPath)?.canonical === true)!

function page(dir: string, name: string, source: string): PageData {
  fs.writeFileSync(path.join(dir, name), source)
  return { filePath: name, frontmatter: {} } as PageData
}

describe('injectFixtures', () => {
  supportTest('injects the case a page names into its frontmatter', async ({ tmpDir }) => {
    const [rule, caseName] = CANONICAL.id.split('/')
    const data = page(tmpDir, 'named.md', `<Fixture rule="${rule}" case="${caseName}" />`)
    await injectFixtures(data, CRATE, SETS, tmpDir)
    expect(Object.keys(data.frontmatter.fixtures)).toStrictEqual([CANONICAL.id])
  })

  supportTest('adds no fixtures key where a page names no case', async ({ tmpDir }) => {
    const data = page(tmpDir, 'plain.md', 'Heading\n\nPlain prose.\n')
    await injectFixtures(data, CRATE, SETS, tmpDir)
    expect(data.frontmatter.fixtures).toBeUndefined()
  })

  supportTest('reads nothing for a page carrying no file path', async ({ tmpDir }) => {
    const data = { frontmatter: {} } as PageData
    await injectFixtures(data, CRATE, SETS, tmpDir)
    expect(data.frontmatter.fixtures).toBeUndefined()
  })
})
