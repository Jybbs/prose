<script setup lang="ts">
import InlineProse    from '../base/InlineProse.vue'
import { FamilyChip } from '../../../kit'

import { useGlossaryFolio } from '../../../lib/composables/use-glossary-folio'
import { BODY_LINK_CLASS }  from '../../../lib/shared/constants'
import { formatFolio }      from '../../../lib/shared/numerals'

const { active, activeIndex, filtered, step } = useGlossaryFolio()
</script>

<template>
  <div class="" :data-family="active?.primaryFamily">
    <article v-if="active" class="relative mt-24 min-w-0 panel bg-bg pt-16 pr-28 pb-18 pl-32">
      <header class="-mt-16 -mr-28 mb-18 -ml-32 flex min-h-36 items-center justify-between gap-12 rounded-t-base-inner border-b-1 border-b-divider bg-bg-soft p-6">
        <p class="glossary-folio-folio m-0 kicker flex flex-wrap items-center gap-x-10 gap-y-6 pl-6">
          <FamilyChip v-for="family in active.families" :key="family" class="border-0 bg-transparent pl-2" :family="family" :linked="false" />
          <span class="inline-flex h-24 items-center justify-center leading-none text-divider" aria-hidden="true">·</span>
          <span class="inline-flex h-24 items-center justify-center leading-none">{{ activeIndex < 0 ? '–' : formatFolio(activeIndex + 1) }} / {{ formatFolio(filtered.length) }}</span>
        </p>
        <div class="inline-flex h-24 shrink-0 items-center gap-6">
          <button type="button" class="inline-flex size-24 cursor-pointer items-center justify-center rounded-sm border-1 border-divider bg-bg p-0 font-mono text-1rem leading-none font-400 text-text-3 transition-colors hover:text-text-1 hover:border-text-3" aria-label="Previous entry" @click="step(-1)">
            <span class="block -translate-y-0.5 leading-none" aria-hidden="true">‹</span>
          </button>
          <button type="button" class="inline-flex size-24 cursor-pointer items-center justify-center rounded-sm border-1 border-divider bg-bg p-0 font-mono text-1rem leading-none font-400 text-text-3 transition-colors hover:text-text-1 hover:border-text-3" aria-label="Next entry" @click="step(1)">
            <span class="block -translate-y-0.5 leading-none" aria-hidden="true">›</span>
          </button>
        </div>
      </header>
      <h2 class="glossary-folio-headword relative mt-0 mb-14 border-t-0 pt-0 font-display text-1-7rem leading-1-15 font-500 text-text-1">
        <a v-if="active.href" :class="BODY_LINK_CLASS" :href="active.href">{{ active.slug }}</a>
        <span v-else class="relative inline-block after:absolute after:inset-x-0 after:-bottom-2 after:h-1 after:translate-y-4 after:bg-text-3 after:opacity-0 after:transition-translate-and-opacity hover:after:translate-y-0 hover:after:opacity-100">{{ active.slug }}</span>
      </h2>
      <p v-if="active.aliases.length > 0" class="mt-0 flex flex-wrap items-center gap-x-8 gap-y-6 border-b-1 border-b-divider pb-18">
        <span class="mr-8 font-mono text-label tracking-label uppercase text-text-2">also</span>
        <code v-for="alias in active.aliases" :key="alias" class="inline-flex items-center rounded-sm border-none bg-bg-alt px-10 py-4 align-baseline text-meta leading-1-3 tracking-meta text-text-2">{{ alias }}</code>
      </p>
      <div class="text-md leading-1-55 text-text-1 glossary-folio-body"><InlineProse :nodes="active.definitionNodes" /></div>
    </article>
  </div>
</template>
