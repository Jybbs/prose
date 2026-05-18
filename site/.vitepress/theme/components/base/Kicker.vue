<script setup lang="ts">
defineProps<{ variant?: 'plain' | 'rule' }>()
</script>

<template>
  <p class="kicker" :data-variant="variant ?? 'plain'"><slot /></p>
</template>
