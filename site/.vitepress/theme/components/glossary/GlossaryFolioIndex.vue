<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import NibBadge from '../base/NibBadge.vue'

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
  <aside v-if="visible" class="glossary-folio-index" aria-label="Glossary index">
    <div class="glossary-folio-search-header">
      <span class="glossary-folio-meta">{{ filtered.length }} of {{ ordered.length }}</span>
      <div class="glossary-folio-search panel">
        <svg class="glossary-folio-search-glyph" viewBox="0 0 24 24" aria-hidden="true">
          <g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2">
            <path d="m21 21l-4.34-4.34" />
            <circle cx="11" cy="11" r="8" />
          </g>
        </svg>
        <input
          v-model="query"
          type="search"
          class="glossary-folio-input"
          aria-label="Filter glossary"
        />
      </div>
    </div>
    <div v-if="filtered.length === 0" class="glossary-folio-empty">
      No entries match <code>{{ query }}</code>.
    </div>
    <div v-else class="glossary-folio-list" @keydown="move">
      <div v-for="[initial, entries] in grouped" :key="initial" class="glossary-folio-block">
        <p class="glossary-folio-letter">{{ initial }}</p>
        <ul class="glossary-folio-rows">
          <li v-for="entry in entries" :key="entry.slug">
            <button
              type="button"
              class="glossary-folio-row"
              :class="{ 'is-active': entry.slug === selected }"
              @click="selected = entry.slug"
            >
              <span class="glossary-folio-row-slot" aria-hidden="true"></span>
              <code class="glossary-folio-row-slug">{{ entry.slug }}</code>
              <span
                v-if="entry.families.length"
                class="glossary-folio-row-deck"
                :class="{ 'is-stacked': entry.families.length > 1 }"
                :style="{ '--deck-count': entry.families.length }"
                aria-hidden="true"
              >
                <span
                  v-for="(family, at) in entry.families"
                  :key="family"
                  class="glossary-folio-row-tile"
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
