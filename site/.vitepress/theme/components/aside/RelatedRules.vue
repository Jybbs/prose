<script setup lang="ts">
import { computed } from 'vue'

import { useCurrentRule } from '../../../lib/route'
import { data as rules } from '../../../data/rules.data'

const current = useCurrentRule()

const related = computed(() => {
  if (!current.value) return []
  return rules
    .filter(r => r.slug !== current.value!.slug && r.category === current.value!.category)
    .slice(0, 5)
})
</script>

<template>
  <div v-if="current && related.length" class="related-rules">
    <p class="related-rules-kicker">Related</p>
    <ul class="related-rules-list">
      <li v-for="rule in related" :key="rule.slug">
        <RuleChip :slug="rule.slug" />
      </li>
    </ul>
  </div>
</template>
