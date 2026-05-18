<script setup lang="ts">
import './disclosure.css'

defineProps<{
  open    ?: boolean
  variant ?: 'fixture' | 'motivation'
}>()
</script>

<template>
  <details :class="['disclosure', `disclosure-${variant ?? 'fixture'}`]" :open="open">
    <summary class="disclosure-summary">
      <span class="disclosure-caret" aria-hidden="true">›</span>
      <span class="disclosure-title"><slot name="title" /></span>
    </summary>
    <div class="disclosure-body">
      <slot />
    </div>
  </details>
</template>
