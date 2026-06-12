<script setup lang="ts">
import type { RenderedRule } from '../../../data/rules.data'

defineProps<{
  segment     : { family: string | null; rule: RenderedRule | null; slug: string }
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
