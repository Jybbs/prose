import { useData }                    from 'vitepress'
import { computed, type ComputedRef } from 'vue'

import { fixtureEntry, type FixtureEntry } from '../fixtures/entry'

// Reads the fixture case a page's frontmatter carries for one rule and case.
export function useFixtureEntry(props: { case: string, rule: string }): ComputedRef<FixtureEntry> {
  const { frontmatter } = useData()
  return computed(() => fixtureEntry(frontmatter.value, props.rule, props.case))
}
