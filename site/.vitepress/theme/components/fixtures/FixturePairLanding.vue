<script setup lang="ts">
defineProps<{
  inputHtml  : string
  outputHtml : string
}>()
</script>

<template>
  <div class="fixture-pair fixture-pair-landing">
    <div class="fixture-pair-side">
      <div class="fixture-pair-label" data-side="before">
        <span class="fixture-pair-dot" data-side="before" />
        <span>Before</span>
      </div>
      <div class="fixture-pair-code" v-html="inputHtml" />
    </div>
    <div class="fixture-pair-arrow" aria-hidden="true">→</div>
    <div class="fixture-pair-side">
      <div class="fixture-pair-label" data-side="after">
        <span class="fixture-pair-dot" data-side="after" />
        <span>After</span>
      </div>
      <div class="fixture-pair-code" v-html="outputHtml" />
    </div>
  </div>
</template>
