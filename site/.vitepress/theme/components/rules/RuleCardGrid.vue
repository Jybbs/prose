<script setup lang="ts">
import { computed } from 'vue'

import { data as rules }                 from '../../../data/rules.data'
import type { RuleCategory, RuleDomain } from '../../../lib/shared/registries'

const props = defineProps<{
  category ?: RuleCategory
  domain   ?: RuleDomain
}>()

const items = computed(() => rules.list.filter(r => {
  if (props.category && r.category !== props.category) return false
  if (props.domain   && r.domain   !== props.domain)   return false
  return true
}))
</script>

<template>
  <div v-if="items.length" class="rule-card-grid">
    <a
      v-for="r in items"
      :key="r.slug"
      class="rule-card"
      :data-category="r.category"
      :data-domain="r.domain"
      :href="`/rules/${r.slug}`"
    >
      <span class="rule-card-slug">{{ r.slug }}</span>
      <p class="rule-card-caption" v-html="r.captionHtml"></p>
    </a>
  </div>
  <p v-else class="rule-card-grid-empty">No rules in this group yet.</p>
</template>

