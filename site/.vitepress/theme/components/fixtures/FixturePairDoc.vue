<script setup lang="ts">
import type { FixtureTab } from '../../../lib/shared/fixture-tab'

defineProps<{
  activeTab  : FixtureTab
  inputHtml  : string
  outputHtml : string
}>()
</script>

<template>
  <div class="fixture-pair fixture-pair-doc">
    <div class="fixture-pair-panel" v-html="activeTab === 'before' ? inputHtml : outputHtml" />
  </div>
</template>
