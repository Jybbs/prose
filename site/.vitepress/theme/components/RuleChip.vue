<script setup lang="ts">
import { computed } from 'vue'

import { data as rules } from '../../data/rules.data'

const props = defineProps<{ slug: string }>()

const entry    = computed(() => rules.find(r => r.slug === props.slug))
const badge    = computed(() => entry.value?.category === 'lint' ? '🧶' : '🪜')
const category = computed(() => entry.value?.category ?? 'auto-fix')
</script>

<template>
  <a
    :href="`/rules/${slug}`"
    :class="['rule-chip', `rule-chip-${category}`]"
    :title="`${slug} (${category})`"
  >
    <span class="rule-chip-badge" aria-hidden="true">{{ badge }}</span>
    <code class="rule-chip-slug">{{ slug }}</code>
  </a>
</template>
