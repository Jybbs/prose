<script setup lang="ts">
import RuleCard from './RuleCard.vue'

import type { RenderedRule } from '../../../data/rules.data'

defineProps<{ rule: RenderedRule | null }>()
</script>

<template>
  <VTooltip v-if="rule" theme="rule-card" :no-auto-focus="true" :popper-class="`rule-card-popper fam-${rule.family}`">
    <slot />
    <template #popper>
      <RuleCard :rule="rule" :clickable="false" />
    </template>
  </VTooltip>
  <slot v-else />
</template>
