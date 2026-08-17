<script setup lang="ts">
import type { RuleSegment } from '../../../lib/rules/rule-view'

defineProps<{
  segment     : RuleSegment
  withTooltip : boolean
}>()
</script>

<template>
  <RuleTooltipPopper :rule="withTooltip ? segment.rule : null">
    <a
      :href="segment.rule?.href"
      class="composition-cards-chip"
      :data-family="segment.family"
      :title="segment.rule ? undefined : `${segment.slug} (${segment.family ?? 'undocumented'})`"
    >
      <span class="composition-cards-chip-label">
        <span class="composition-cards-chip-slug">{{ segment.slug }}</span>
      </span>
    </a>
  </RuleTooltipPopper>
</template>
