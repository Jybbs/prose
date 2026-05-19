<script setup lang="ts">
import type { FixtureTab } from '../../../lib/shared/fixture-tab'

defineProps<{
  activeTab ?: FixtureTab
  inputHtml  : string
  outputHtml : string
  variant   ?: 'doc' | 'landing'
}>()
</script>

<template>
  <div v-if="variant === 'landing'" class="fixture-pair fixture-pair-landing">
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

  <div v-else class="fixture-pair fixture-pair-doc">
    <div class="fixture-pair-panel" v-html="activeTab === 'before' ? inputHtml : outputHtml" />
  </div>
</template>
