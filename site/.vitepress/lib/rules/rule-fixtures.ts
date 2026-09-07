import { readFixtureToggle } from '../fixtures/toggle'
import * as walker           from '../fixtures/walker'
import { inlineCode }        from '../shared/inline-code'

interface PendingExample {
  case      : string
  inputPath : string
  title     : string
}

interface RuleExample {
  case      : string
  titleHtml : string
}

export interface RuleFixtureSet {
  canonical : string
  examples  : readonly RuleExample[]
}

export type RuleFixturesData = Record<string, RuleFixtureSet>

// Drops a leading backtick, which would otherwise sort ahead of the letters it
// wraps.
const sortKey = (title: string): string => title.replace(/^`+/, '')

// Collects the canonical case and the previewable examples each rule registers,
// ordering the examples that rewrite their input ahead of the rest and
// alphabetizing by title within each group. A rule registering no canonical case
// is left out.
export async function readRuleFixtures(crate: string): Promise<RuleFixturesData> {
  const byRule: Record<string, { canonical: string | null, examples: PendingExample[] }> = {}
  for (const { rule, caseName, inputPath } of walker.walkFixtures(crate)) {
    const docs = walker.readFixtureDocs(inputPath)
    if (docs === undefined) continue
    const set   = (byRule[rule] ??= { canonical: null, examples: [] })
    const title = walker.fixtureTitle(docs)
    if (docs.canonical === true) {
      set.canonical = caseName
    } else if (docs.previewable === true && title) {
      set.examples.push({ case: caseName, inputPath, title })
    }
  }

  const out: RuleFixturesData = {}
  for (const [rule, { canonical, examples }] of Object.entries(byRule)) {
    if (canonical === null) continue
    const ranked = await Promise.all(examples.map(async example => ({
      ...example,
      hasToggle : (await readFixtureToggle(example.inputPath)).hasToggle
    })))
    ranked.sort((a, b) =>
      Number(b.hasToggle) - Number(a.hasToggle) ||
      sortKey(a.title).localeCompare(sortKey(b.title)))
    out[rule] = {
      canonical,
      examples : ranked.map(({ case: name, title }) =>
        ({ case: name, titleHtml: inlineCode(title) }))
    }
  }
  return out
}
