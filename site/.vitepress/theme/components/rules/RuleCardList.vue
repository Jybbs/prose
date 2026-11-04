<script setup lang="ts">
import { computed } from 'vue'

import RuleCard from './RuleCard.vue'

import { data as rules }   from '../../../lib/rules/rules.data'
import { useCurrentRule }  from '../../../lib/composables/route'
import { lookup }          from '../../../lib/shared/lookup'
import type { RuleFamily } from '../../../lib/shared/registries'

const props = defineProps<{
  family  ?: RuleFamily
  related ?: true
}>()

const current = useCurrentRule()

const items = computed(() => {
  if (props.related) {
    return current.value?.related.map(s => lookup(rules.bySlug, s, 'Related rule')) ?? []
  }
  return rules.list.filter(r => !props.family || r.family === props.family)
})
</script>

<template>
  <div v-if="items.length" class="flex max-w-960 flex-col gap-6" :class="related ? 'mt-28' : 'mt-24'">
    <RuleCard v-for="rule in items" :key="rule.slug" :rule="rule" />
  </div>
  <p v-else-if="!related" class="text-sm text-text-3">No rules in this group yet.</p>
</template>
