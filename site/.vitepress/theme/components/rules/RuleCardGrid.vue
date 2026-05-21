<script setup lang="ts">
import { computed } from 'vue'

import RuleCard from './RuleCard.vue'

import { data as rules }                  from '../../../data/rules.data'
import type { RuleCategory, RuleFamily }  from '../../../lib/shared/registries'

const props = defineProps<{
  category ?: RuleCategory
  family   ?: RuleFamily
}>()

const items = computed(() => rules.list.filter(r => {
  if (props.category && r.category !== props.category) return false
  if (props.family   && r.family   !== props.family)   return false
  return true
}))
</script>

<template>
  <div v-if="items.length" class="rule-card-grid">
    <RuleCard
      v-for="(rule, idx) in items"
      :key="rule.slug"
      :index="idx"
      :rule="rule"
    />
  </div>
  <p v-else class="rule-card-grid-empty">No rules in this group yet.</p>
</template>
