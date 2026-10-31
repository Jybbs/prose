<script setup lang="ts">
defineProps<{ variant?: 'plain' | 'rule' | 'wide' }>()
</script>

<template>
  <p class="kicker" :data-variant="variant ?? 'plain'"><slot /></p>
</template>
