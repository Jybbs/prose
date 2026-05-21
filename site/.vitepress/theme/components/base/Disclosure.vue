<script setup lang="ts">
defineProps<{
  id   ?: string
  open ?: true
}>()
</script>

<template>
  <details :id="id" class="disclosure" :open="open">
    <summary class="disclosure-summary">
      <span class="disclosure-caret" aria-hidden="true">›</span>
      <span class="disclosure-title"><slot name="title" /></span>
      <span class="disclosure-actions"><slot name="actions" /></span>
    </summary>
    <div class="disclosure-body">
      <slot />
    </div>
  </details>
</template>
