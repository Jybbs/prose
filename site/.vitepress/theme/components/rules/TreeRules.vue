<script setup lang="ts">
import { computed } from 'vue'

import { data as pipeline } from '../../../lib/rules/pipeline.data'

const props = defineProps<{ tree: 'changed' | 'kept' }>()

const rules = computed(() =>
  pipeline.rules.filter(rule => rule.preservesTree === (props.tree === 'kept')))
</script>

<template>
  <ul>
    <li v-for="rule in rules" :key="rule.slug">
      <InlineRuleLink v-if="rule.documented" :slug="rule.slug" />
      <code v-else>{{ rule.slug }}</code>
    </li>
  </ul>
</template>
