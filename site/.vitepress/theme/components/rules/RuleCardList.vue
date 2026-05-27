<script setup lang="ts">
import { computed, useTemplateRef } from 'vue'

import RuleCard from './RuleCard.vue'

import { data as rules }                 from '../../../data/rules.data'
import { useCurrentRule }                from '../../../lib/composables/route'
import { useRuleCardNameSync }           from './use-rule-card-name-sync'
import { lookup }                        from '../../../lib/shared/lookup'
import type { RuleCategory, RuleFamily } from '../../../lib/shared/registries'

const props = defineProps<{
  category ?: RuleCategory
  family   ?: RuleFamily
  related  ?: true
}>()

const current = useCurrentRule()

const items = computed(() => {
  if (props.related) {
    return current.value?.related.map(s => lookup(rules.bySlug, s, 'Related rule')) ?? []
  }
  return rules.list.filter(r =>
    (!props.category || r.category === props.category) &&
    (!props.family   || r.family   === props.family))
})

const containerRef = useTemplateRef<HTMLElement>('container')
useRuleCardNameSync(containerRef, items)
</script>

<template>
  <div v-if="items.length" ref="container" :class="related ? 'related-rules-inline' : 'rule-card-grid'">
    <RuleCard v-for="rule in items" :key="rule.slug" :rule="rule" />
  </div>
  <p v-else-if="!related" class="rule-card-grid-empty">No rules in this group yet.</p>
</template>
