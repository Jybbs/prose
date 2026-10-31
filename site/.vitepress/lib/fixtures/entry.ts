import type { InlineNode } from '../markdown/inline-nodes'
import { lookup }          from '../shared/lookup'

export const COMPOSITION_RULE = 'composition'

// The signals a card reads to decide whether it draws a before-and-after
// toggle, which the build derives alongside the toggle state.
export interface FixtureFlags {
  changesSource : boolean
  hasFindings   : boolean
  hasToggle     : boolean
}

// One fixture case as a page receives it, carrying the two rendered code panes,
// those flags, and the description as inline nodes. `transformPageData` adds to
// a page's frontmatter only the cases that page renders.
export interface FixtureEntry extends FixtureFlags {
  descriptionNodes ?: InlineNode[]
  inputHtml         : string
  outputHtml        : string
}

export type FixtureMap = Record<string, FixtureEntry>

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

export function fixtureId(rule: string, caseName: string): string {
  return `${rule}/${caseName}`
}
