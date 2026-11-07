<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import { NibBadge } from '../../../kit'

import { useGlossaryFolio } from '../../../lib/composables/use-glossary-folio'
import { arrowIndex }       from '../../../lib/shared/arrow-keys'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { stripSuffix }      from '../../../lib/shared/strip-suffix'

const props = defineProps<{ forceRender?: boolean }>()

const { page } = useData()

const onGlossaryPage = computed(() =>
  stripSuffix(page.value.relativePath, '.md') === 'reference/glossary')

const visible = computed(() => props.forceRender || onGlossaryPage.value)

const { filtered, grouped, ordered, query, selected } = useGlossaryFolio()

// Moves the focus along every row of the index under the arrow keys, Home,
// and End.
function move(event: KeyboardEvent): void {
  const rows = [...(event.currentTarget as Element).querySelectorAll<HTMLElement>('.glossary-folio-row')]
  const at   = rows.indexOf(event.target as HTMLElement)
  const to   = at < 0 ? undefined : arrowIndex(event.key, at, rows.length)
  if (to === undefined) return
  event.preventDefault()
  rows[to]!.focus()
}
</script>

<template>
  <aside v-if="visible" class="mt-20 flex flex-col gap-12" aria-label="Glossary index">
    <div class="flex flex-col gap-4">
      <span class="ml-auto font-mono text-meta tracking-meta whitespace-nowrap text-text-3 uppercase">{{ filtered.length }} of {{ ordered.length }}</span>
      <div class="flex items-center gap-10 panel bg-bg-soft px-14 py-6 transition-colors hover:bg-bg focus-within:border-link focus-within:inset-ring-1 focus-within:inset-ring-link">
        <svg class="size-14 shrink-0 text-text-2" viewBox="0 0 24 24" aria-hidden="true">
          <g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2">
            <path d="m21 21l-4.34-4.34" />
            <circle cx="11" cy="11" r="8" />
          </g>
        </svg>
        <input
          v-model="query"
          type="search"
          class="h-22 min-w-0 flex-1 border-none text-sm text-text-1 outline-none placeholder:text-text-3 placeholder:opacity-80"
          aria-label="Filter glossary"
        />
      </div>
    </div>
    <div v-if="filtered.length === 0" class="px-4 py-12 text-sm text-text-3">
      No entries match <code class="rounded-none bg-transparent p-0 text-text-2">{{ query }}</code>.
    </div>
    <div v-else class="glossary-folio-list overflow-y-auto" @keydown="move">
      <div v-for="[initial, entries] in grouped" :key="initial" class="not-first:mt-14">
        <p class="mt-0 mb-4 border-b-1 border-b-divider px-4 pb-4 font-mono text-meta font-500 tracking-label text-text-3 uppercase opacity-70">{{ initial }}</p>
        <ul class="m-0 list-none p-0">
          <li v-for="entry in entries" :key="entry.slug" class="mt-0">
            <button
              type="button"
              class="glossary-folio-row group grid min-h-24 w-full grid-cols-8-1fr-auto items-center gap-10 rounded-sm border-none px-8 py-3 text-left focus-visible:focus-ring focus-visible:-outline-offset-2"
              :aria-current="entry.slug === selected || undefined"
              @click="selected = entry.slug"
            >
              <span class="glossary-folio-row-slot block size-6 justify-self-center" aria-hidden="true"></span>
              <code class="glossary-folio-row-slug min-w-0 rounded-none bg-transparent p-0 font-base text-sm wrap-anywhere text-text-2 transition-none group-hover:text-text-1 group-aria-current:font-600 group-aria-current:text-text-1">{{ entry.slug }}</code>
              <span
                v-if="entry.families.length"
                class="glossary-folio-row-deck relative block h-20"
                :class="{ 'is-stacked': entry.families.length > 1 }"
                :style="{ '--deck-count': entry.families.length }"
                aria-hidden="true"
              >
                <span
                  v-for="(family, at) in entry.families"
                  :key="family"
                  class="glossary-folio-row-tile absolute top-0 box-border grid size-20 place-items-center"
                  :data-family="family"
                  :style="{ '--deck-at': at }"
                >
                  <NibBadge :emblem="familyEmblem(family)" :size="13" />
                </span>
              </span>
            </button>
          </li>
        </ul>
      </div>
    </div>
  </aside>
</template>
