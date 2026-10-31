<script setup lang="ts">
import Tool from '../base/Tool.vue'

import { data as shells } from '../../../lib/integrations/shell-completions.data'
import { useTabSelect }   from '../../../lib/composables/use-tab-select'
import InlineProse        from '../base/InlineProse.vue'

const { selected: activeSlug, active } = useTabSelect(shells, s => s.slug)
</script>

<template>
  <div class="shell-card relative my-1-5rem card-stack:pr-0">
    <div class="relative">
      <div role="tablist" aria-label="Shell completion targets">
        <button
          v-for="(shell, i) in shells"
          :key="shell.slug"
          type="button"
          role="tab"
          class="shell-card-tab absolute inline-flex w-132 items-center gap-8 rounded-r-6 border-y-1 border-r-1 border-l-none px-12 py-8 text-left whitespace-nowrap transition-background-border-color-and-color card-stack:relative card-stack:top-auto card-stack:right-auto card-stack:mr-4 card-stack:mb-6 card-stack:ml-12 card-stack:w-auto card-stack:rounded-sm card-stack:border-l-1"
          :class="shell.slug === activeSlug ? 'z-5 border-celadon-65-divider bg-celadon-24-bg-soft text-text-1' : 'z-1 border-y-divider border-r-divider bg-bg-soft text-text-2 hover:bg-celadon-wash-bg-soft hover:text-text-1 card-stack:border-l-divider'"
          :aria-selected="shell.slug === activeSlug"
          :style="{ '--idx': i }"
          @focus="activeSlug = shell.slug"
          @mouseenter="activeSlug = shell.slug"
        >
          <span
            class="tool-ring size-22"
            :class="shell.slug === activeSlug ? 'border-celadon-70-divider' : 'border-celadon-28-divider'"
            aria-hidden="true"
          ><Tool :slug="shell.slug" bare /></span>
          <em class="font-display text-sm font-600">{{ shell.name }}</em>
        </button>
      </div>

      <article class="shell-card-modal flex min-h-260 flex-col overflow-hidden panel bg-bg card-stack:min-h-auto">
        <header class="flex items-center gap-12 px-18 py-14">
          <span
            class="shell-card-modal-mark tool-ring size-26 border-celadon-60-divider"
            aria-hidden="true"
          ><Tool :slug="active.slug" bare /></span>
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
