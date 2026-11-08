import type { InlineNode }   from '../markdown/inline-nodes'
import { lookup }            from '../shared/lookup'

export const COMPOSITION_RULE = 'composition'

// The signals a card reads to decide whether it draws a before-and-after
// toggle, which the build derives alongside the toggle state.
export interface FixtureFlags {
  changesSource : boolean
  hasFindings   : boolean
  hasToggle     : boolean
}

// One fixture case as a page receives it, carrying its title, the two rendered
// code panes, those flags, the description as inline nodes, the configuration the
// case overrides, and the session that opens it in the sandbox. `transformPageData`
// adds to a page's frontmatter only the cases that page renders.
export interface FixtureEntry extends FixtureFlags {
  configHtml       ?: string
  descriptionNodes ?: InlineNode[]
  inputHtml         : string
  outputHtml        : string
  titleHtml         : string
}

// One numeric setting of a rule's case stepped from zero to twice its default,
// `steps` holding for each value the index of the rendered output it gives.
export interface FixtureGauge extends OutputSteps {
  default : number
  setting : string
}

export type FixtureMap = Record<string, FixtureEntry>

// A composition case run under each prefix of its rules in pipeline order,
// `steps` holding the index of the rendered input and then, for each rule, of
// the rendered output that prefix gives.
export interface FixtureTrail extends OutputSteps {
  rules : readonly string[]
}

// Each distinct output a case gives, rendered once, beside the index of the
// output each step shows.
export interface OutputSteps {
  outputs : readonly string[]
  steps   : readonly number[]
}

// Splits the space-separated case names a `FixtureCards` tag lists.
export function caseNames(attribute: string): string[] {
  return attribute.split(/\s+/).filter(Boolean)
}

// Reports whether a page embeds the case through a `Fixture` or
// `FixtureConvergence` tag, reading the ids `transformPageData` records in its
// frontmatter.
export function embedsInline(
  frontmatter : Record<string, unknown>,
  rule        : string,
  caseName    : string
): boolean {
  const inline = (frontmatter.inlineFixtures ?? []) as readonly string[]
  return inline.includes(fixtureId(rule, caseName))
}

export function fixtureEntry(
  frontmatter : Record<string, unknown>,
  rule        : string,
  caseName    : string
): FixtureEntry {
  const fixtures = (frontmatter.fixtures ?? {}) as FixtureMap
  return lookup(fixtures, fixtureId(rule, caseName), 'Fixture case')
}

// Reads the gauge `transformPageData` stepped for one setting of a case.
export function fixtureGauge(
  frontmatter : Record<string, unknown>,
  rule        : string,
  caseName    : string,
  setting     : string
): FixtureGauge {
  const gauges = (frontmatter.fixtureGauges ?? {}) as Record<string, FixtureGauge>
  return lookup(gauges, gaugeId(rule, caseName, setting), 'Fixture gauge')
}

export function fixtureId(rule: string, caseName: string): string {
  return `${rule}/${caseName}`
}

// Reads the trail `transformPageData` ran for one composition case.
export function fixtureTrail(frontmatter: Record<string, unknown>, caseName: string): FixtureTrail {
  const trails = (frontmatter.fixtureTrails ?? {}) as Record<string, FixtureTrail>
  return lookup(trails, fixtureId(COMPOSITION_RULE, caseName), 'Composition trail')
}

// Keys a gauge by its case and the setting it steps, since one case may carry
// a gauge for each of several settings.
export function gaugeId(rule: string, caseName: string, setting: string): string {
  return `${fixtureId(rule, caseName)}#${setting}`
}
