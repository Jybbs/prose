import { COMPOSITION_RULE, fixtureId, gaugeId } from '../../../site/.vitepress/lib/fixtures/entry'
import { LINT_FINDINGS_FILE }                    from '../../../site/.vitepress/lib/fixtures/lint-findings'
import * as render                               from '../../../site/.vitepress/lib/fixtures/render'
import * as walker                               from '../../../site/.vitepress/lib/fixtures/walker'
import { CASES, FIXTURES, hasSidecar }           from '../corpus'

const CANONICAL = CASES.find(entry => walker.readFixtureDocs(entry.inputPath)?.canonical === true)!
const LINTING   = CASES.find(entry => hasSidecar(entry.inputPath, LINT_FINDINGS_FILE))!

describe('fixtureEntries', () => {
  it('renders a case to its before-and-after panes', async () => {
    const entry = (await render.fixtureEntries(FIXTURES, [CANONICAL.id]))[CANONICAL.id]
    expect(entry.inputHtml).toContain('<pre')
    expect(entry.outputHtml).toContain('<pre')
    expect(entry.hasToggle).toBe(entry.changesSource || entry.hasFindings)
  })

  it('titles a case from its meta.toml, rendering backticks as code spans', async () => {
    const titled = CASES.find(entry => walker.fixtureTitle(walker.readFixtureDocs(entry.inputPath))?.includes('`'))!
    const entry  = (await render.fixtureEntries(FIXTURES, [titled.id]))[titled.id]
    expect(entry.titleHtml).toContain('<code>')
    expect(entry.titleHtml).not.toContain('`')
  })

  it('walks the meta description to inline nodes', async () => {
    const entry = (await render.fixtureEntries(FIXTURES, [CANONICAL.id]))[CANONICAL.id]
    expect(entry.descriptionNodes?.length).toBeGreaterThan(0)
  })

  it('renders a case carrying lint findings through the decorating fence', async () => {
    const entry = (await render.fixtureEntries(FIXTURES, [LINTING.id]))[LINTING.id]
    expect(entry.hasFindings).toBe(true)
    expect(entry.hasToggle).toBe(true)
    expect(entry.outputHtml).toContain('<pre')
  })

  it('returns one entry per requested id', async () => {
    const requested = [CANONICAL.id, LINTING.id]
    expect(Object.keys(await render.fixtureEntries(FIXTURES, requested))).toStrictEqual(requested)
  })

  it('returns an empty map where a page reaches no case', async () => {
    await expect(render.fixtureEntries(FIXTURES, [])).resolves.toStrictEqual({})
  })

  it('renders a case reached from two pages once', async () => {
    const first  = await render.fixtureEntries(FIXTURES, [CANONICAL.id])
    const second = await render.fixtureEntries(FIXTURES, [CANONICAL.id])
    expect(first[CANONICAL.id]).toBe(second[CANONICAL.id])
  })

  it('renders again after a reset drops the entries', async () => {
    const before = (await render.fixtureEntries(FIXTURES, [CANONICAL.id]))[CANONICAL.id]
    render.resetFixtureEntries()
    const after = (await render.fixtureEntries(FIXTURES, [CANONICAL.id]))[CANONICAL.id]
    expect(after).not.toBe(before)
    expect(after).toStrictEqual(before)
  })

  it('names a case carrying no input and snapshot pair', async () => {
    const missing = fixtureId('__no_such_rule__', '__no_such_case__')
    await expect(render.fixtureEntries(FIXTURES, [missing]))
      .rejects.toThrow(/has no input and snapshot pair/)
  })
})

describe('fixtureGauges', () => {
  const request = { case: 'wide_target_breaks_run', rule: 'align_equals', setting: 'max-shift' }
  const id      = gaugeId(request.rule, request.case, request.setting)

  it('steps each requested gauge, keyed by its id', async () => {
    const gauges = await render.fixtureGauges(FIXTURES, [{ ...request, id }])
    expect(Object.keys(gauges)).toStrictEqual([id])
    expect(gauges[id].setting).toBe('max-shift')
  })

  it('steps a gauge reached from two pages once', async () => {
    const first  = await render.fixtureGauges(FIXTURES, [{ ...request, id }])
    const second = await render.fixtureGauges(FIXTURES, [{ ...request, id }])
    expect(first[id]).toBe(second[id])
  })
})

describe('fixtureTrails', () => {
  const id = fixtureId(COMPOSITION_RULE, 'overflow_dict_constants_modernize_unions')

  it('runs each requested trail, keyed by its id', async () => {
    const trails = await render.fixtureTrails(FIXTURES, [id])
    expect(Object.keys(trails)).toStrictEqual([id])
    expect(trails[id].steps).toHaveLength(trails[id].rules.length + 1)
  })

  it('runs a trail reached from two pages once', async () => {
    const first  = await render.fixtureTrails(FIXTURES, [id])
    const second = await render.fixtureTrails(FIXTURES, [id])
    expect(first[id]).toBe(second[id])
  })
})
