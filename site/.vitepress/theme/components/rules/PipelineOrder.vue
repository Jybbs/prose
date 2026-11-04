<script setup lang="ts">
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { data as rules }    from '../../../lib/rules/rules.data'
import { familyEmblem }     from '../../../lib/shared/emblems'
import { formatFolio }      from '../../../lib/shared/numerals'

import MiddleEllipsis from '../base/MiddleEllipsis.vue'
import NibBadge       from '../base/NibBadge.vue'
</script>

<template>
  <section class="my-32 panel bg-bg-soft px-32 py-22" aria-label="Pipeline order">
    <header class="mb-22 flex justify-end pb-18 border-b-double-divider">
      <span class="pipeline-order-edition kicker">
        {{ pipeline.rules.length }} passes &middot; <code>src/rules/registry.rs</code>
      </span>
    </header>
    <ol class="m-0 columns-2 gap-x-40 p-0 column-rule-divider max-medium:columns-1">
      <li
        v-for="rule in pipeline.rules"
        :key="rule.slug"
        class="pipeline-order-entry mt-0 mb-2 list-none break-inside-avoid"
        :data-family="rule.family"
      >
        <RuleTooltipPopper :rule="rule.documented ? rules.bySlug[rule.slug] : null">
          <a
            class="group/link flex items-center gap-12 rounded-6 px-8 py-9 text-text-1 no-underline transition-colors hover:text-brand-2"
            :href="rule.documented ? rules.bySlug[rule.slug].href : undefined"
            :title="rule.title"
          >
            <span class="folio">№ {{ formatFolio(rule.position) }}</span>
            <MiddleEllipsis class="min-w-0 font-display text-0-92rem font-500 text-family-or-text-1-80-text-1 italic transition-colors group-hover/link:text-family-or-brand-1" :text="rule.slug" :tail="2" />
            <span class="leader-fill" aria-hidden="true"></span>
            <NibBadge v-if="rule.family" :emblem="familyEmblem(rule.family)" :size="15.2" />
            <span v-else class="pipeline-order-glyph shrink-0 text-md" aria-hidden="true">·</span>
          </a>
        </RuleTooltipPopper>
      </li>
    </ol>
  </section>
</template>
