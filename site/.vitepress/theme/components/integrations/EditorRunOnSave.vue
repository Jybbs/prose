<script setup lang="ts">
import Tool from '../base/Tool.vue'

import { data as editors } from '../../../lib/integrations/editor-configs.data'
import { useTabSelect }    from '../../../lib/composables/use-tab-select'
import { formatFolio }     from '../../../lib/shared/numerals'

const { selected: activeSlug } = useTabSelect(editors, e => e.slug)
</script>

<template>
  <div class="my-1-5rem grid grid-cols-232-1fr overflow-hidden panel bg-bg card-stack:grid-cols-1fr">
    <aside class="border-r-1 border-r-divider bg-bg-soft pt-14 pr-14 pb-18 pl-18 card-stack:border-b-1 card-stack:border-r-none card-stack:border-r-current card-stack:border-b-divider">
      <span class="kicker mb-14 block pb-13 border-b-double-divider">Editors &middot; {{ formatFolio(editors.length) }}</span>
      <ul class="m-0 flex list-none flex-col gap-2 p-0 card-stack:flex-row card-stack:flex-wrap card-stack:gap-4" role="tablist">
        <li v-for="(editor, i) in editors" :key="editor.slug" class="mt-0" role="presentation">
          <button
            type="button"
            role="tab"
            class="editor-card-row flex w-full items-center gap-10 rounded-sm border-none p-8 text-left text-text-1 transition-background card-stack:flex-3/10"
            :class="editor.slug === activeSlug ? 'bg-celadon/medium' : 'hover:bg-celadon/7'"
            :aria-selected="editor.slug === activeSlug"
            @focus="activeSlug = editor.slug"
            @mouseenter="activeSlug = editor.slug"
          >
            <span class="folio">№ {{ formatFolio(i + 1) }}</span>
            <span
              class="tool-ring size-22"
              :class="editor.slug === activeSlug ? 'border-celadon-65-divider' : 'border-celadon-28-divider'"
              aria-hidden="true"
            ><Tool :slug="editor.slug" bare /></span>
            <em
              class="min-w-0 overflow-hidden font-display text-md font-500 whitespace-nowrap"
              :class="{ 'text-celadon-60-text-1': editor.slug === activeSlug }"
            >{{ editor.name }}</em>
            <span class="leader-fill card-stack:hidden" aria-hidden="true"></span>
          </button>
        </li>
      </ul>
    </aside>

    <div class="flex min-w-0 flex-col" aria-live="polite">
      <section
        v-for="editor in editors"
        :key="editor.slug"
        v-show="editor.slug === activeSlug"
        class="editor-card-face flex flex-1 flex-col bg-code-block"
      >
        <header class="flex items-baseline gap-10 border-b-1 border-b-divider px-18 py-14">
          <span class="kicker">{{ editor.caption }}</span>
          <span class="editor-card-face-target ml-auto font-mono text-xs text-text-2">{{ editor.target }}</span>
        </header>
        <div class="editor-card-face-code flex flex-1 flex-col" v-html="editor.codeHtml" />
      </section>
    </div>
  </div>
</template>
