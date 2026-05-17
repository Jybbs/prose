<script setup lang="ts">
defineProps<{
  centered ?: boolean
  heading   : string
  kicker   ?: string
}>()
</script>

<template>
  <div :class="['landing-section-heading', { centered }]">
    <Kicker v-if="kicker">{{ kicker }}</Kicker>
    <h2 v-html="heading" />
    <slot />
  </div>
</template>
