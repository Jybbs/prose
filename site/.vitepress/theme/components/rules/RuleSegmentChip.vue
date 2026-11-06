<script setup lang="ts">
import { NibBadge }      from '../../../kit'

import type { RuleSegment } from '../../../lib/rules/rule-view'
import { familyEmblem }     from '../../../lib/shared/emblems'

defineProps<{
  segment     : RuleSegment
  withTooltip : boolean
}>()
</script>

<template>
  <a
    :data-rule="withTooltip ? segment.rule?.slug : undefined"
    :href="segment.rule?.href"
    class="composition-cards-chip box-border grid h-20 items-center justify-items-start bg-chip-ground font-mono leading-1-2 text-text-1 no-underline hover:text-brand-2"
    :data-family="segment.family"
    :title="segment.rule ? undefined : `${segment.slug} (${segment.family ?? 'undocumented'})`"
  >
    <NibBadge v-if="segment.family" class="block justify-self-center text-family-nib-chip group-data-[edits=false]/fixture:text-text-3-chip-31" :emblem="familyEmblem(segment.family)" :size="13" />
    <span class="composition-cards-chip-label">
      <span class="composition-cards-chip-slug text-xxs tracking-0-04em uppercase">{{ segment.slug }}</span>
    </span>
  </a>
</template>
