import { computed, ref, type ComputedRef, type Ref } from 'vue'

import { data as glossary, type RenderedGlossaryEntry } from '../../../data/glossary.data'

const ordered: readonly RenderedGlossaryEntry[] = Object.values(glossary.entries)
  .toSorted((a, b) => a.slug.localeCompare(b.slug, 'en', { sensitivity: 'base' }))

const query    = ref<string>('')
const selected = ref<string>(ordered[0]?.slug ?? '')

const filtered = computed<RenderedGlossaryEntry[]>(() => {
  const q = query.value.trim().toLowerCase()
  if (q === '') return ordered
  return ordered.filter(e =>
    e.slug.toLowerCase().includes(q) || e.aliases.some(a => a.toLowerCase().includes(q)))
})

const grouped = computed<[string, RenderedGlossaryEntry[]][]>(() =>
  [...Map.groupBy(filtered.value, e => e.initial).entries()].toSorted(([a], [b]) => a.localeCompare(b, 'en', { sensitivity: 'base' })))

const active = computed<RenderedGlossaryEntry | undefined>(() =>
  ordered.find(e => e.slug === selected.value))

const activeIndex = computed<number>(() =>
  filtered.value.findIndex(e => e.slug === selected.value))

function step(delta: number): void {
  const pool = filtered.value
  if (pool.length === 0) return
  const idx = activeIndex.value < 0 ? 0 : (activeIndex.value + delta + pool.length) % pool.length
  selected.value = pool[idx]!.slug
}

interface GlossaryFolio {
  active      : ComputedRef<RenderedGlossaryEntry | undefined>
  activeIndex : ComputedRef<number>
  filtered    : ComputedRef<RenderedGlossaryEntry[]>
  grouped     : ComputedRef<[string, RenderedGlossaryEntry[]][]>
  ordered     : readonly RenderedGlossaryEntry[]
  query       : Ref<string>
  selected    : Ref<string>
  step        : (delta: number) => void
}

export function useGlossaryFolio(): GlossaryFolio {
  return { active, activeIndex, filtered, grouped, ordered, query, selected, step }
}
