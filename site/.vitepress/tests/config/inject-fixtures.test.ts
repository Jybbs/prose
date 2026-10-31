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

  supportTest('records the case a page embeds inline', async ({ tmpDir }) => {
    const tag  = `<Fixture rule="${CANONICAL.rule}" case="${CANONICAL.caseName}" />`
    const data = page(tmpDir, 'inline.md', tag)
    await injectFixtures(data, CRATE, SETS, tmpDir)
    expect(data.frontmatter.inlineFixtures).toStrictEqual([CANONICAL.id])
  })

  supportTest('records no inline case where a page only lays out its rule', async ({ tmpDir }) => {
    const set  = { canonical: CANONICAL.caseName, examples: [] }
    const sets = { ...SETS, ruleFixtures: { [CANONICAL.rule]: set } }
    const data = page(tmpDir, 'layout.md', `<RuleLayout rule="${CANONICAL.rule}">`)
    await injectFixtures(data, CRATE, sets, tmpDir)
    expect(Object.keys(data.frontmatter.fixtures)).toStrictEqual([CANONICAL.id])
    expect(data.frontmatter.inlineFixtures).toBeUndefined()
  })

  supportTest('adds no fixtures key where a page names no case', async ({ tmpDir }) => {
    const data = page(tmpDir, 'plain.md', 'Heading\n\nPlain prose.\n')
    await injectFixtures(data, CRATE, SETS, tmpDir)
    expect(data.frontmatter.fixtures).toBeUndefined()
    expect(data.frontmatter.inlineFixtures).toBeUndefined()
  })

  supportTest('reads nothing for a page carrying no file path', async ({ tmpDir }) => {
    const data = { frontmatter: {} } as PageData
    await injectFixtures(data, CRATE, SETS, tmpDir)
    expect(data.frontmatter.fixtures).toBeUndefined()
  })
})
