<script setup lang="ts">
import { computed } from 'vue'

import { useCurrentRule } from '../../../lib/composables/route'

import TaxonomyChip from './TaxonomyChip.vue'

const rule         = useCurrentRule()
const showFamily   = computed(() => rule.value !== null && rule.value.family !== 'lint')
</script>

<template>
  <div class="rule-chrome">
    <TaxonomyChip axis="category" />
    <TaxonomyChip v-if="showFamily" axis="family" />
  </div>
</template>

<style scoped>
.rule-chrome {
  display       : flex;
  flex-wrap     : wrap;
  align-items   : center;
  gap           : 10px;
  margin-bottom : 12px;
}
</style>
