<script setup lang="ts">
import { useData } from 'vitepress'
import { computed } from 'vue'

import { useGlossaryFolio } from './use-glossary-folio'

const props = defineProps<{ forceRender?: boolean }>()

const { page } = useData()

const onGlossaryPage = computed(() =>
  page.value.relativePath.replace(/\.md$/, '') === 'reference/glossary')

const visible = computed(() => props.forceRender || onGlossaryPage.value)

const { filtered, grouped, ordered, query, selected } = useGlossaryFolio()
</script>

<template>
  <aside v-if="visible" class="glossary-folio-index" aria-label="Glossary index">
    <div class="glossary-folio-search">
      <span class="vp-icon DocSearch-Search-Icon glossary-folio-search-glyph" aria-hidden="true"></span>
      <input
        v-model="query"
        type="search"
        class="glossary-folio-input"
        aria-label="Filter glossary"
      />
      <span class="glossary-folio-meta">{{ filtered.length }} of {{ ordered.length }}</span>
    </div>
    <div v-if="filtered.length === 0" class="glossary-folio-empty">
      No entries match <code>{{ query }}</code>.
    </div>
    <div v-else class="glossary-folio-list">
      <div v-for="[initial, entries] in grouped" :key="initial" class="glossary-folio-block">
        <p class="glossary-folio-letter">{{ initial }}</p>
        <ul class="glossary-folio-rows">
          <li v-for="entry in entries" :key="entry.slug">
            <button
              type="button"
              class="glossary-folio-row"
              :class="{ 'is-active': entry.slug === selected }"
              :data-family="entry.family"
              @click="selected = entry.slug"
            >
              <span class="glossary-folio-row-dot" aria-hidden="true"></span>
              <code class="glossary-folio-row-slug">{{ entry.slug }}</code>
            </button>
          </li>
        </ul>
      </div>
    </div>
  </aside>
</template>
