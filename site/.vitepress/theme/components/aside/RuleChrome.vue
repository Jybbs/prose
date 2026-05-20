<script setup lang="ts">
import TaxonomyChip from './TaxonomyChip.vue'
</script>

<template>
  <div class="rule-chrome">
    <TaxonomyChip axis="category" />
    <TaxonomyChip axis="domain" />
  </div>
</template>

<style scoped>
.rule-chrome {
  display      : flex;
  flex-wrap    : wrap;
  align-items  : center;
  gap          : 10px;
  margin-bottom: 12px;
}
</style>
