import path from 'node:path'

import { changeValues }                  from '../../lib/fixtures/gauge'
import { readFixtureToggle }             from '../../lib/fixtures/toggle'
import { readGauge, readTrail }          from '../../lib/fixtures/variants'
import * as walker                       from '../../lib/fixtures/walker'
import { getRenderer, renderFencedHtml } from '../../lib/markdown/renderer'
import { readPipeline }                  from '../../lib/rules/pipeline'
import { fixturesDirFrom }               from '../../lib/shared/paths'
import { CRATE }                         from '../corpus'

const md       = await getRenderer()
const inputOf  = (rule: string, caseName: string): string =>
  path.join(fixturesDirFrom(CRATE), rule, caseName, walker.INPUT_FILE)
const WIDE     = inputOf('align_equals', 'wide_target_breaks_run')
const UNPADDED = inputOf('align_colons', 'docstring_entries_off_keeps_unpadded_entries')
const TRAILED  = inputOf('composition', 'overflow_dict_constants_modernize_unions')
const snapshot = async (inputPath: string): Promise<string> =>
  renderFencedHtml(md, (await readFixtureToggle(inputPath)).output, 'python')

const gauge = await readGauge(md, 'align_equals', WIDE, 'max-shift')
const trail = await readTrail(md, TRAILED)

describe('readGauge', () => {
  it('steps the setting from zero to twice its schema default', () => {
    expect(gauge.default).toBe(16)
    expect(gauge.steps).toHaveLength(33)
  })

  it('renders each distinct output once', () => {
    expect(new Set(gauge.steps).size).toBe(gauge.outputs.length)
  })

  it('formats the case at the default value as its snapshot reads', async () => {
    expect(gauge.outputs[gauge.steps[gauge.default]!]).toBe(await snapshot(WIDE))
  })

  it('changes the output at each value the setting moves a row', () => {
    expect(changeValues(gauge.steps)).toStrictEqual([2, 6, 19, 20, 21])
  })

  it('keeps the case override of the other keys of the rule beside the setting it steps', async () => {
    const kept = await readGauge(md, 'align_colons', UNPADDED, 'max-shift')
    expect(kept.outputs[kept.steps[kept.default]!]).toBe(await snapshot(UNPADDED))
  })

  it('names a rule declaring no numeric setting of that name', async () => {
    await expect(readGauge(md, 'align_equals', WIDE, 'enabled')).rejects.toThrow('declares no numeric `enabled`')
  })
})

describe('readTrail', () => {
  it('orders the harness rules by pipeline position', () => {
    const position = new Map(readPipeline(import.meta.url).map(entry => [entry.slug, entry.position]))
    const at       = trail.rules.map(slug => position.get(slug)!)
    expect(at).toStrictEqual(at.toSorted((a, b) => a - b))
    expect(trail.rules.toSorted()).toStrictEqual(walker.readCaseConfig(TRAILED).harness?.rules?.toSorted())
  })

  it('ends on the output the whole rule set writes', async () => {
    expect(trail.outputs[trail.steps.at(-1)!]).toBe(await snapshot(TRAILED))
  })

  it('holds the input and then one step per rule', () => {
    expect(trail.steps).toHaveLength(trail.rules.length + 1)
    expect(trail.steps[0]).toBe(0)
  })
})
