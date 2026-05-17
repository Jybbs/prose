<script setup lang="ts">
defineProps<{
  inputHtml  : string
  outputHtml : string
}>()
</script>

<template>
  <div class="fixture-side">
    <div class="fixture-label">Before</div>
    <div class="fixture-code" v-html="inputHtml" />
  </div>
  <div class="fixture-arrow" aria-hidden="true">→</div>
  <div class="fixture-side">
    <div class="fixture-label">After</div>
    <div class="fixture-code" v-html="outputHtml" />
  </div>
</template>
