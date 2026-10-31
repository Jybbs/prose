<script setup lang="ts">
import Kicker from '../base/Kicker.vue'

defineProps<{
  centered ?: boolean
  heading   : string
  kicker   ?: string
}>()
</script>

<template>
  <div class="landing-section mt-48 mobile:mt-54">
    <div class="mb-34 gap-24" :class="centered ? 'mx-auto max-w-720 items-center text-center' : 'flex items-end'">
      <Kicker v-if="kicker" variant="rule">{{ kicker }}</Kicker>
      <h2 class="mt-12 font-display leading-1-1 font-500 text-balance text-text-1" v-html="heading" />
      <slot name="heading-extra" />
    </div>
    <slot />
  </div>
</template>
