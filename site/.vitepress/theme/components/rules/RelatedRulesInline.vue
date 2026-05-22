<script setup lang="ts">
import { computed } from 'vue'

import RuleCard from './RuleCard.vue'

import { data as rules }  from '../../../data/rules.data'
import { useCurrentRule } from '../../../lib/composables/route'
import { lookup }         from '../../../lib/shared/lookup'

const current = useCurrentRule()

const items = computed(() => {
  if (!current.value?.related.length) return []
  return current.value.related.map(s => lookup(rules.bySlug, s, 'Related rule'))
})
</script>

<template>
  <div v-if="items.length" class="related-rules-inline">
    <RuleCard
      v-for="(rule, idx) in items"
      :key="rule.slug"
      :index="idx"
      :rule="rule"
    />
  </div>
</template>
