<script setup lang="ts">
import Kicker from '../base/Kicker.vue'

defineProps<{
  centered ?: boolean
  heading   : string
  kicker   ?: string
}>()
</script>

<template>
  <div :class="['landing-section-heading', { centered }]">
    <Kicker v-if="kicker" variant="rule">{{ kicker }}</Kicker>
    <h2 v-html="heading" />
    <slot />
  </div>
</template>
