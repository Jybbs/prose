<script setup lang="ts">
import CategoryChip from './CategoryChip.vue'
import DomainChip   from './DomainChip.vue'
</script>

<template>
  <div class="rule-chrome">
    <CategoryChip />
    <DomainChip />
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
