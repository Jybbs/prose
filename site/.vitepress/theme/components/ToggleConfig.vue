<script setup lang="ts">
import RuleConfigTable from './RuleConfigTable.vue'

const rows = [
  { default: 'true', key: 'enabled', meaning: 'Toggle the rule on or off', type: 'bool' }
]
</script>

<template>
  <RuleConfigTable :rows="rows" />
</template>
