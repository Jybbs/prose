<script setup lang="ts">
import InlineProse from '../base/InlineProse.vue'
import ToolIcon    from '../base/ToolIcon.vue'

import { data as shells } from '../../../lib/integrations/shell-completions.data'
import { useTabSelect }   from '../../../lib/composables/use-tab-select'

const { selected: activeSlug, active } = useTabSelect(shells, s => s.slug)
</script>

<template>
  <div class="relative my-1-5rem pr-132 max-card-stack:pr-0">
    <div class="relative">
      <div role="tablist" aria-label="Shell completion targets">
        <button
          v-for="(shell, i) in shells"
          :key="shell.slug"
          type="button"
          role="tab"
          class="shell-card-tab group absolute z-1 inline-flex w-132 items-center gap-8 rounded-r-6 border-y-1 border-r-1 border-l-0 border-divider bg-bg-soft px-12 py-8 text-left whitespace-nowrap text-text-2 transition-colors aria-selected:z-5 aria-selected:border-ube aria-selected:bg-bg aria-selected:text-text-1 max-card-stack:relative max-card-stack:top-auto max-card-stack:right-auto max-card-stack:mr-4 max-card-stack:mb-6 max-card-stack:ml-12 max-card-stack:w-auto max-card-stack:rounded-sm max-card-stack:border-l-1"
          :aria-selected="shell.slug === activeSlug"
          :style="{ '--idx': i }"
          @focus="activeSlug = shell.slug"
          @mouseenter="activeSlug = shell.slug"
        >
          <span class="tool-ring size-22 transition-colors group-aria-selected:text-link"><ToolIcon class="size-12" :slug="shell.slug" /></span>
          <em class="font-display text-sm font-600">{{ shell.name }}</em>
        </button>
      </div>

      <article class="shell-card-modal flex min-h-260 flex-col overflow-hidden panel bg-bg max-card-stack:min-h-auto">
        <header class="flex items-center gap-12 px-18 py-14">
          <span class="tool-ring size-26"><ToolIcon class="size-14" :slug="active.slug" /></span>
          <span class="kicker">{{ active.caption }}</span>
        </header>
        <div class="flex-auto">
          <div class="shell-card-modal-code" v-html="active.codeHtml"></div>
          <p class="shell-card-modal-note px-18 pb-14 text-0-92rem text-text-2"><InlineProse :nodes="active.noteNodes" /></p>
        </div>
      </article>
    </div>
  </div>
</template>
