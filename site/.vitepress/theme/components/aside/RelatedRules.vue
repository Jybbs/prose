<script setup lang="ts">
import { computed } from 'vue'

import Kicker   from '../base/Kicker.vue'
import RuleChip from '../rules/RuleChip.vue'

import { data as rules }  from '../../../data/rules.data'
import { useCurrentRule } from '../../../lib/composables/route'
import { lookup }         from '../../../lib/shared/lookup'

const current = useCurrentRule()

const related = computed(() => {
  if (!current.value) return []
  if (current.value.related.length) {
    return current.value.related.map(slug => lookup(rules.bySlug, slug, 'Related rule'))
  }
  return rules.list
    .filter(r => r.slug !== current.value!.slug && r.category === current.value!.category)
    .slice(0, 5)
})
</script>

<template>
  <div v-if="current && related.length" class="related-rules">
    <Kicker class="related-rules-kicker">Related</Kicker>
    <ul class="related-rules-list">
      <li v-for="rule in related" :key="rule.slug">
        <RuleChip :slug="rule.slug" />
      </li>
    </ul>
  </div>
</template>
