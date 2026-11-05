<script setup lang="ts">
import { onKeyStroke }          from '@vueuse/core'
import { computed, shallowRef } from 'vue'

import { passCensus }       from '../../../lib/rules/pipeline-census'
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { data as rules }    from '../../../lib/rules/rules.data'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { formatFolio }      from '../../../lib/shared/numerals'
import { FAMILY_META }      from '../../../lib/shared/registries'

import { MiddleEllipsis, NibBadge } from '../../../kit'
import RuleTooltipPopup             from './RuleTooltipPopup.vue'

const families = pipeline.rules.map(rule => rule.family)

// Holds the pass a reader points at or focuses, which keeps every pass up to
// it as at rest and grays every later one until the Escape key clears it.
const here   = shallowRef<number | null>(null)
const census = computed(() => passCensus(families, here.value))

const placed = (at: number) => ({ 'is-here': at === here.value, 'is-later': here.value !== null && at > here.value })

function focus(event: FocusEvent, at: number): void {
  if ((event.target as Element).matches(':focus-visible')) here.value = at
}

onKeyStroke('Escape', () => { here.value = null })
</script>

<template>
  <section class="pipeline-order my-32 panel bg-bg-soft px-32 py-22" aria-label="Pipeline order">
    <header class="mb-22 flex flex-wrap items-center justify-between gap-12">
      <div class="flex flex-wrap gap-4" role="group" aria-label="Passes per family">
        <span
          v-for="entry in census"
          :key="entry.family"
          class="pipeline-order-census-chip nib-ground inline-flex items-center gap-5 border-1 border-divider px-6 py-2 font-mono text-label leading-1-2 font-400 uppercase tabular-nums"
          :class="{ 'is-later': entry.count === 0 }"
          :data-family="entry.family"
          role="img"
          :aria-label="`${FAMILY_META[entry.family].label}, ${entry.count} rules`"
        >
          <NibBadge :emblem="familyEmblem(entry.family)" :size="13" />
          <span aria-hidden="true">{{ entry.count }}</span>
        </span>
      </div>
      <span class="pipeline-order-edition ml-auto kicker">
        <template v-if="here === null">{{ pipeline.rules.length }} rules</template>
        <template v-else>pass {{ formatFolio(here + 1) }} of {{ pipeline.rules.length }}</template>
        &middot; <code>crate/src/rules/registry.rs</code>
      </span>
      <ol class="pipeline-order-spine m-0 flex h-10 shrink-0 grow basis-full list-none items-end gap-2 p-0" aria-hidden="true">
        <li
          v-for="(rule, at) in pipeline.rules"
          :key="rule.slug"
          class="m-0 flex h-full flex-1 items-end"
          :class="placed(at)"
          @pointerenter="here = at"
        ></li>
      </ol>
    </header>
    <ol class="m-0 columns-2 gap-x-40 p-0 column-rule-divider max-medium:columns-1">
      <li
        v-for="(rule, at) in pipeline.rules"
        :key="rule.slug"
        class="pipeline-order-entry mt-0 mb-2 list-none break-inside-avoid"
        :class="placed(at)"
        :data-family="rule.family"
      >
        <RuleTooltipPopup :rule="rule.documented ? rules.bySlug[rule.slug] : null">
          <a
            class="pipeline-order-link flex items-center gap-12 rounded-6 px-8 py-9 text-text-1 no-underline transition-colors hover:text-brand-2"
            :href="rule.documented ? rules.bySlug[rule.slug].href : undefined"
            :title="rule.title"
            @focusin="focus($event, at)"
            @pointerenter="here = at"
          >
            <span class="folio">№ {{ formatFolio(rule.position) }}</span>
            <MiddleEllipsis class="pipeline-order-name min-w-0 font-display text-0-92rem font-500 italic" :text="rule.slug" :tail="2" />
            <span class="leader-fill" aria-hidden="true"></span>
            <NibBadge v-if="rule.family" :emblem="familyEmblem(rule.family)" :size="15.2" />
            <span v-else class="pipeline-order-glyph shrink-0 text-md" aria-hidden="true">·</span>
          </a>
        </RuleTooltipPopup>
      </li>
    </ol>
  </section>
</template>
