import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import type { PageData } from 'vitepress'

import { injectFixtures }       from '../../lib/config/inject-fixtures'
import type { PageFixtureSets } from '../../lib/fixtures/page-cases'
import * as walker              from '../../lib/fixtures/walker'
import { CASES, CRATE }         from '../corpus'

const SETS: PageFixtureSets = { composition: { byRule: {}, cases: [] }, ruleFixtures: {} }

const CANONICAL = CASES.find(c => walker.readFixtureDocs(c.inputPath)?.canonical === true)!
const srcDir    = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-inject-'))

function page(source: string, name: string): PageData {
  fs.writeFileSync(path.join(srcDir, name), source)
  return { filePath: name, frontmatter: {} } as PageData
}

describe('injectFixtures', () => {
  it('injects the case a page names into its frontmatter', async () => {
    const [rule, caseName] = CANONICAL.id.split('/')
    const data = page(`<Fixture rule="${rule}" case="${caseName}" />`, 'named.md')
    await injectFixtures(data, CRATE, SETS, srcDir)
    expect(Object.keys(data.frontmatter.fixtures)).toEqual([CANONICAL.id])
  })

  it('adds no fixtures key where a page names no case', async () => {
    const data = page('# Heading\n\nPlain prose.\n', 'plain.md')
    await injectFixtures(data, CRATE, SETS, srcDir)
    expect(data.frontmatter.fixtures).toBeUndefined()
  })

  it('reads nothing for a page carrying no file path', async () => {
    const data = { frontmatter: {} } as PageData
    await injectFixtures(data, CRATE, SETS, srcDir)
    expect(data.frontmatter.fixtures).toBeUndefined()
  })
})
