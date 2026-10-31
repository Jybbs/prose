<script setup lang="ts">
defineProps<{ open: boolean }>()
defineEmits<{ toggle: [] }>()
</script>

<template>
  <button
    type="button"
    class="landing-cta-toggle absolute top-8 right-8 z-5 medallion size-26 border-stamp-accent-30-panel-edge bg-stamp-accent-8-panel-bg font-mono text-md leading-1 text-stamp-ink transition-background-border-color-and-transform hover:border-stamp-accent-55-panel-edge hover:bg-stamp-accent-22-bg"
    :aria-expanded="open"
    :aria-label="open ? 'Back to current release' : 'Show previous releases'"
    @click="$emit('toggle')"
  >
    {{ open ? '←' : '+' }}
  </button>
</template>
