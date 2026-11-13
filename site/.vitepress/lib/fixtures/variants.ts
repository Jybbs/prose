import fs from 'node:fs'

import type { MarkdownRenderer } from 'vitepress'

import type { FixtureGauge, FixtureTrail, OutputSteps } from './entry'
import * as walker                                      from './walker'
import { renderFencedHtml }                             from '../markdown/renderer'
import { readPipeline }                                 from '../rules/pipeline'
import { readProseUnder, repoRoot }                     from '../shared/paths'
import { ruleSlug }                                     from '../shared/rule-slug'
import * as ruleSchema                                  from '../shared/rule-schema'

const root = repoRoot(import.meta.url)

// The binary's own schema and rule order, each read once per build.
let defs     : Record<string, ruleSchema.RuleDef> | undefined
let pipeline : readonly string[] | undefined

// Formats `input` under `config` with only the rules `select` names, from a
// scratch directory holding that configuration as its `prose.toml`.
function formatUnder(input: string, config: walker.CaseConfig, select: readonly string[]): string {
  return readProseUnder(root, [
    'format', '--no-cache', '--select', select.join(','), '--stdin', '--stdin-filename', 'case.py'
  ], input, walker.overrideToml(config))
}

function ruleDefault(slug: string, setting: string): number {
  defs ??= ruleSchema.ruleDefsOf(ruleSchema.proseSchema(root))
  const value = defs[slug]?.default[setting]
  if (typeof value !== 'number') throw new TypeError(`${slug} declares no numeric \`${setting}\``)
  return value
}

// Renders each distinct output once, every step naming the output it shows.
async function distinctOutputs(md: MarkdownRenderer, runs: readonly string[]): Promise<OutputSteps> {
  const seen    = new Map<string, number>()
  const steps   = runs.map(output => seen.getOrInsertComputed(output, () => seen.size))
  const outputs = await Promise.all([...seen.keys()].map(output => renderFencedHtml(md, output, 'python')))
  return { outputs, steps }
}

// Steps one numeric setting of a rule's case from zero to twice its default,
// formatting the case under each value with that rule alone.
export async function readGauge(
  md        : MarkdownRenderer,
  rule      : string,
  inputPath : string,
  setting   : string
): Promise<FixtureGauge> {
  const slug   = ruleSlug(rule)
  const base   = ruleDefault(slug, setting)
  const config = walker.readCaseConfig(inputPath)
  const input  = fs.readFileSync(inputPath, 'utf8')
  const table  = config.rules?.[slug]
  const own    = typeof table === 'object' ? table as Record<string, unknown> : {}
  const runs   = Array.from({ length: 2 * base + 1 }, (_, value) => formatUnder(input, {
    ...config, rules: { ...config.rules, [slug]: { ...own, [setting]: value } }
  }, [slug]))
  return { ...await distinctOutputs(md, runs), default: base, setting }
}

// Formats a composition case under each prefix of its rules in pipeline order,
// so each step after the input adds the next rule to `--select`.
export async function readTrail(md: MarkdownRenderer, inputPath: string): Promise<FixtureTrail> {
  pipeline ??= readPipeline(import.meta.url).map(entry => entry.slug)
  const config = walker.readCaseConfig(inputPath)
  const input  = fs.readFileSync(inputPath, 'utf8')
  const rules  = pipeline.filter(slug => config.harness?.rules?.includes(slug))
  const runs   = rules.map((_, at) => formatUnder(input, config, rules.slice(0, at + 1)))
  return { ...await distinctOutputs(md, [input, ...runs]), rules }
}
