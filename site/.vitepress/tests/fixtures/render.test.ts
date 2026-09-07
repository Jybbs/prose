import fs   from 'node:fs'
import path from 'node:path'

import { fixtureId }                           from '../../lib/fixtures/entry'
import { LINT_FINDINGS_FILE }                  from '../../lib/fixtures/lint-findings'
import { fixtureEntries, resetFixtureEntries } from '../../lib/fixtures/render'
import * as walker                             from '../../lib/fixtures/walker'
import { crateDir }                            from '../../lib/shared/paths'

const crate = crateDir(import.meta.url)
const cases = [...walker.walkFixtures(crate)]

const sidecar = (inputPath: string, file: string): boolean =>
  fs.existsSync(path.join(path.dirname(inputPath), file))

const CANONICAL = cases.find(entry => walker.readFixtureDocs(entry.inputPath)?.canonical === true)!
const LINTING   = cases.find(entry => sidecar(entry.inputPath, LINT_FINDINGS_FILE))!

describe('fixtureEntries', () => {
  it('renders a case to its before-and-after panes', async () => {
    const entry = (await fixtureEntries(crate, [CANONICAL.id]))[CANONICAL.id]
    expect(entry.inputHtml).toContain('<pre')
    expect(entry.outputHtml).toContain('<pre')
    expect(entry.hasToggle).toBe(entry.changesSource || entry.hasFindings)
  })

  it('walks the meta description to inline nodes', async () => {
    const entry = (await fixtureEntries(crate, [CANONICAL.id]))[CANONICAL.id]
    expect(entry.descriptionNodes?.length).toBeGreaterThan(0)
  })

  it('renders a case carrying lint findings through the decorating fence', async () => {
    const entry = (await fixtureEntries(crate, [LINTING.id]))[LINTING.id]
    expect(entry.hasFindings).toBe(true)
    expect(entry.hasToggle).toBe(true)
    expect(entry.outputHtml).toContain('<pre')
  })

  it('returns one entry per requested id', async () => {
    const requested = [CANONICAL.id, LINTING.id]
    expect(Object.keys(await fixtureEntries(crate, requested))).toEqual(requested)
  })

  it('returns an empty map where a page reaches no case', async () => {
    expect(await fixtureEntries(crate, [])).toEqual({})
  })

  it('renders a case reached from two pages once', async () => {
    const first  = await fixtureEntries(crate, [CANONICAL.id])
    const second = await fixtureEntries(crate, [CANONICAL.id])
    expect(first[CANONICAL.id]).toBe(second[CANONICAL.id])
  })

  it('renders again after a reset drops the entries', async () => {
    const before = (await fixtureEntries(crate, [CANONICAL.id]))[CANONICAL.id]
    resetFixtureEntries()
    const after = (await fixtureEntries(crate, [CANONICAL.id]))[CANONICAL.id]
    expect(after).not.toBe(before)
    expect(after).toEqual(before)
  })

  it('names a case carrying no input and snapshot pair', async () => {
    const missing = fixtureId('__no_such_rule__', '__no_such_case__')
    await expect(fixtureEntries(crate, [missing]))
      .rejects.toThrow(/has no input and snapshot pair/)
  })
})
