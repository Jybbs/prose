<script setup lang="ts">
defineProps<{ open: boolean }>()
defineEmits<{ toggle: [] }>()
</script>

<template>
  <button
    type="button"
    class="landing-cta-toggle"
    :aria-expanded="open"
    :aria-label="open ? 'Back to current release' : 'Show previous releases'"
    @click="$emit('toggle')"
  >
    {{ open ? '←' : '+' }}
  </button>
</template>
