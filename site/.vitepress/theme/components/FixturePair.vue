<script setup lang="ts">
import '../css/fixture.css'

defineProps<{
  inputHtml  : string
  outputHtml : string
  variant   ?: 'doc' | 'landing'
}>()
</script>

<template>
  <div :class="['fixture-pair', `fixture-pair-${variant ?? 'doc'}`]">
    <div class="fixture-pair-side">
      <div class="fixture-pair-label">
        <span v-if="variant === 'landing'" class="fixture-pair-dot fixture-pair-dot-before" />
        <span>Before</span>
      </div>
      <div class="fixture-pair-code" v-html="inputHtml" />
    </div>
    <div class="fixture-pair-arrow" aria-hidden="true">→</div>
    <div class="fixture-pair-side">
      <div class="fixture-pair-label">
        <span v-if="variant === 'landing'" class="fixture-pair-dot fixture-pair-dot-after" />
        <span>After</span>
      </div>
      <div class="fixture-pair-code" v-html="outputHtml" />
    </div>
  </div>
</template>
