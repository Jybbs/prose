<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import { railPaint }        from '../../../lib/shared/family-rail'
import { stripSuffix }      from '../../../lib/shared/strip-suffix'
import { useGlossaryFolio } from '../../../lib/composables/use-glossary-folio'

const props = defineProps<{ forceRender?: boolean }>()

const { page } = useData()

const onGlossaryPage = computed(() =>
  stripSuffix(page.value.relativePath, '.md') === 'reference/glossary')

const visible = computed(() => props.forceRender || onGlossaryPage.value)

const { filtered, grouped, ordered, query, selected } = useGlossaryFolio()
</script>

<template>
  <aside v-if="visible" class="mt-20 flex flex-col gap-12" aria-label="Glossary index">
    <div class="flex flex-col gap-4">
      <span class="ml-auto font-mono text-meta tracking-meta whitespace-nowrap text-text-3 uppercase">{{ filtered.length }} of {{ ordered.length }}</span>
      <div class="flex items-center gap-10 panel bg-bg-soft px-14 py-6 transition-colors focus-within:border-ube-55-divider focus-within:ring-3 focus-within:ring-ube/20 hover:bg-bg hover:not-focus-within:border-ube-35-divider">
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
    <div v-else class="glossary-folio-list overflow-y-auto">
      <div v-for="[initial, entries] in grouped" :key="initial" class="not-first:mt-14">
        <p class="mt-0 mb-4 border-b-1 border-b-divider px-4 pb-4 font-mono text-meta font-500 tracking-label text-text-3 uppercase opacity-70">{{ initial }}</p>
        <ul class="m-0 list-none p-0">
          <li v-for="entry in entries" :key="entry.slug" class="mt-0">
            <button
              type="button"
              class="group grid min-h-24 w-full grid-cols-3-1fr items-center gap-10 rounded-sm border-none px-8 py-3 text-left transition-colors not-aria-current:hover:bg-family/10 aria-current:bg-family/22"
              :aria-current="entry.slug === selected || undefined"
              :data-family="entry.primaryFamily"
              :style="{ '--rail-paint': railPaint(entry.families) }"
              @click="selected = entry.slug"
            >
              <span class="glossary-folio-row-rail self-stretch rounded-2" aria-hidden="true"></span>
              <code
                class="min-w-0 rounded-none bg-transparent p-0 font-base text-sm wrap-anywhere text-text-2 transition-none group-hover:text-text-1 group-aria-current:font-600 group-aria-current:text-text-1"
              >{{ entry.slug }}</code>
            </button>
          </li>
        </ul>
      </div>
    </div>
  </aside>
</template>
