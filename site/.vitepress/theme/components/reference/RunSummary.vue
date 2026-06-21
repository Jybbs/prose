<script setup lang="ts">
import type { RenderedLine } from '../../../lib/reference/run-summary'

defineProps<{ line: RenderedLine }>()
</script>

<template>
  <div class="run-summary">
    <div class="run-summary-bar">
      <span v-for="n in 3" :key="n" class="run-summary-dot" aria-hidden="true" />
      <span class="run-summary-title" aria-hidden="true">prose</span>
      <slot name="bar" />
    </div>
    <div class="run-summary-body">
      <div class="run-summary-line">
        <span
          v-if="line.anchor"
          class="run-summary-anchor"
          :data-ube="line.anchorUbe || undefined"
          aria-hidden="true"
        >{{ line.anchor }}</span>
        <span class="run-summary-msg" :data-tint="line.countTint || undefined">{{ line.text }}</span>
      </div>
    </div>
  </div>
</template>
