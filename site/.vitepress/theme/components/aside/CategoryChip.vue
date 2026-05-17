<script setup lang="ts">
import { computed } from 'vue'

import { useCurrentRule } from '../../../lib/route'

const rule  = useCurrentRule()
const badge = computed(() => rule.value?.category === 'lint' ? '🧶' : '🪜')
const label = computed(() => rule.value?.category === 'lint' ? 'Lint' : 'Auto-Fix')
</script>

<template>
  <div v-if="rule" :class="['category-chip', `category-chip-${rule.category}`]">
    <span class="category-chip-badge" aria-hidden="true">{{ badge }}</span>
    <span class="category-chip-label">{{ label }}</span>
  </div>
</template>
