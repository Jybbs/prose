<script setup lang="ts">
import { CHIPS } from './rule-chips-data'
</script>

<template>
  <div class="rc-baseline">
    <span
      v-for="chip in CHIPS"
      :key="chip.slug"
      class="rc-chip"
      :data-family="chip.family"
    >{{ chip.slug }}</span>
  </div>
</template>
