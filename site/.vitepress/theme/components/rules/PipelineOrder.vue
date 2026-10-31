<script setup lang="ts">
import { onKeyStroke }          from '@vueuse/core'
import { computed, shallowRef } from 'vue'

import { passCensus }       from '../../../lib/rules/pipeline-census'
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { data as rules }    from '../../../lib/rules/rules.data'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { formatFolio }      from '../../../lib/shared/numerals'
import { FAMILY_META }      from '../../../lib/shared/registries'

import MiddleEllipsis from '../base/MiddleEllipsis.vue'
import NibBadge       from '../base/NibBadge.vue'

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
  <section class="pipeline-order panel" aria-label="Pipeline order">
    <header class="pipeline-order-masthead">
      <div class="pipeline-order-census" role="group" aria-label="Passes per family">
        <span
          v-for="entry in census"
          :key="entry.family"
          class="pipeline-order-census-chip family-chip nib-ground"
          :class="{ 'is-later': entry.count === 0 }"
          :data-family="entry.family"
          role="img"
          :aria-label="`${FAMILY_META[entry.family].label}, ${entry.count} rules`"
        >
          <NibBadge :emblem="familyEmblem(entry.family)" :size="13" />
          <span aria-hidden="true">{{ entry.count }}</span>
        </span>
      </div>
      <span class="kicker pipeline-order-edition">
        <template v-if="here === null">{{ pipeline.rules.length }} rules</template>
        <template v-else>pass {{ formatFolio(here + 1) }} of {{ pipeline.rules.length }}</template>
        &middot; <code>crate/src/rules/registry.rs</code>
      </span>
      <ol class="pipeline-order-spine" aria-hidden="true">
        <li
          v-for="(rule, at) in pipeline.rules"
          :key="rule.slug"
          :class="placed(at)"
          @pointerenter="here = at"
        ></li>
      </ol>
    </header>
    <ol class="pipeline-order-columns">
      <li
        v-for="(rule, at) in pipeline.rules"
        :key="rule.slug"
        class="pipeline-order-entry"
        :class="placed(at)"
        :data-family="rule.family"
      >
        <RuleTooltipPopper :rule="rule.documented ? rules.bySlug[rule.slug] : null">
          <a
            class="pipeline-order-link"
            :href="rule.documented ? rules.bySlug[rule.slug].href : undefined"
            :title="rule.title"
            @focusin="focus($event, at)"
            @pointerenter="here = at"
          >
            <span class="folio">№ {{ formatFolio(rule.position) }}</span>
            <MiddleEllipsis class="pipeline-order-name" :text="rule.slug" :tail="2" />
            <span class="pipeline-order-leader" aria-hidden="true"></span>
            <NibBadge v-if="rule.family" :emblem="familyEmblem(rule.family)" :size="15.2" />
            <span v-else class="pipeline-order-glyph" aria-hidden="true">·</span>
          </a>
        </RuleTooltipPopper>
      </li>
    </ol>
  </section>
</template>
