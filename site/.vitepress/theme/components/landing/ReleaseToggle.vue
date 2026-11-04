<script setup lang="ts">
defineProps<{ open: boolean }>()
defineEmits<{ toggle: [] }>()
</script>

<template>
  <button
    type="button"
    class="absolute top-8 right-8 z-5 medallion size-26 border-stamp-accent-30-panel-edge bg-stamp-accent-8-panel-bg font-mono text-md leading-none text-stamp-ink transition-background-border-color-and-scale hover:scale-106 hover:border-stamp-accent-55-panel-edge hover:bg-stamp-accent-22-bg"
    :aria-expanded="open"
    :aria-label="open ? 'Back to current release' : 'Show previous releases'"
    @click="$emit('toggle')"
  >
    {{ open ? '←' : '+' }}
  </button>
</template>
