<script setup lang="ts">
import SectionHeading from '../SectionHeading.vue'

defineProps<{
  centered ?: boolean
  heading   : string
  kicker   ?: string
  variant   : string
}>()
</script>

<template>
  <div :class="['landing-section', `landing-${variant}`]">
    <SectionHeading :centered="centered" :heading="heading" :kicker="kicker">
      <slot name="heading-extra" />
    </SectionHeading>
    <slot />
  </div>
</template>
