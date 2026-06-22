import { existsSync } from 'node:fs'

import { defineLoader } from 'vitepress'

import { readFixtureToggle }             from '../lib/fixtures/toggle'
import { fixtureWatchGlobs, readFixtureDocs, walkFixtures } from '../lib/fixtures/walker'
import { lintDecorations }               from '../lib/markdown/lint-decorations'
import { getRenderer, renderFencedHtml } from '../lib/markdown/renderer'
import { discoverRuleSlugs }             from '../lib/rules/discovery'
import { crateDir, rulesDir }            from '../lib/shared/paths'

const crate     = crateDir(import.meta.url)
const ruleHrefs = new Map(discoverRuleSlugs(rulesDir(import.meta.url)).map(r => [r.slug, r.href]))

interface FixtureEntry {
  changesSource    : boolean
  descriptionHtml ?: string
  hasFindings      : boolean
  hasToggle        : boolean
  inputHtml        : string
  outputHtml       : string
}

type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

function descriptionHtml(
  md        : Awaited<ReturnType<typeof getRenderer>>,
  inputPath : string
): string | undefined {
  const text = readFixtureDocs(inputPath)?.description?.trim()
  if (!text) return undefined
  // The card description renders through `v-html`, which never instantiates
  // the `<InlineRuleLink>` component the rule-link plugin emits for a
  // backticked slug, so downgrade it to the plain anchor primitive links
  // already use on this surface.
  return md.render(text).replace(
    /<InlineRuleLink slug="([^"]+)" \/>/g,
    (_, slug) => `<a class="body-link" href="${ruleHrefs.get(slug)!}"><code>${slug}</code></a>`
  )
}

export default defineLoader({
  watch: fixtureWatchGlobs(crate),
  async load(): Promise<FixtureData> {
    const md      = await getRenderer()
    const entries = [...walkFixtures(crate)].filter(({ inputPath }) => existsSync(`${inputPath}.snap`))
    const rows = await Promise.all(entries.map(async ({ rule, caseName, inputPath }) => {
      const { changesSource, findings, hasFindings, hasToggle, inputRaw, output } =
        await readFixtureToggle(inputPath)
      return {
        caseName,
        entry: {
          changesSource,
          descriptionHtml : descriptionHtml(md, inputPath),
          hasFindings,
          hasToggle,
          inputHtml       : renderFencedHtml(md, inputRaw, 'python'),
          outputHtml      : renderFencedHtml(md, output, 'python', lintDecorations(findings))
        },
        rule
      }
    }))
    const out: FixtureData = {}
    for (const { caseName, entry, rule } of rows) (out[rule] ??= {})[caseName] = entry
    return out
  }
})
