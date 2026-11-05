<script setup lang="ts">
import { Popup } from '../../../kit'
import RuleCard  from './RuleCard.vue'

import type { RenderedRule } from '../../../lib/rules/rules.data'

defineProps<{ rule: RenderedRule | null }>()
</script>

<template>
  <Popup v-if="rule" class="rule-card-popup" theme="rule-card">
    <template v-slot:trigger="{ props: trigger }">
      <span v-bind="trigger"><slot /></span>
    </template>
    <RuleCard :rule="rule" :clickable="false" />
  </Popup>
  <slot v-else />
</template>
