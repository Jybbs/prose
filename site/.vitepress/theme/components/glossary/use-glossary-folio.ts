import { computed, ref, type ComputedRef, type Ref } from 'vue'

import { data as glossary, type RenderedGlossaryEntry } from '../../../data/glossary.data'
import { cycleIndex, filterEntries, groupByInitial }    from '../../../lib/glossary/folio'

const ordered: readonly RenderedGlossaryEntry[] = Object.values(glossary.entries)
  .toSorted((a, b) => a.slug.localeCompare(b.slug, 'en', { sensitivity: 'base' }))

const query    = ref<string>('')
const selected = ref<string>(ordered[0]?.slug ?? '')

const filtered    = computed(() => filterEntries(ordered, query.value))
const grouped     = computed(() => groupByInitial(filtered.value))
const active      = computed(() => ordered.find(e => e.slug === selected.value))
const activeIndex = computed(() => filtered.value.findIndex(e => e.slug === selected.value))

function step(delta: number): void {
  const pool = filtered.value
  const idx  = cycleIndex(activeIndex.value, delta, pool.length)
  if (idx >= 0) selected.value = pool[idx]!.slug
}

interface GlossaryFolio {
  active      : ComputedRef<RenderedGlossaryEntry | undefined>
  activeIndex : ComputedRef<number>
  filtered    : ComputedRef<readonly RenderedGlossaryEntry[]>
  grouped     : ComputedRef<[string, RenderedGlossaryEntry[]][]>
  ordered     : readonly RenderedGlossaryEntry[]
  query       : Ref<string>
  selected    : Ref<string>
  step        : (delta: number) => void
}

export function useGlossaryFolio(): GlossaryFolio {
  return { active, activeIndex, filtered, grouped, ordered, query, selected, step }
}
