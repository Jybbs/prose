<script setup lang="ts">
import { CHIPS } from './rule-chips-data'
</script>

<template>
  <ul class="rc-dot">
    <li
      v-for="chip in CHIPS"
      :key="chip.slug"
      class="rc-chip"
      :data-family="chip.family"
    >
      <span class="rc-dot-mark" aria-hidden="true" />
      <span class="rc-slug">{{ chip.slug }}</span>
    </li>
  </ul>
</template>
