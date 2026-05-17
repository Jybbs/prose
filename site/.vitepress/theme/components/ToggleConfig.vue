<script setup lang="ts">
import RuleConfigTable from './RuleConfigTable.vue'

const rows = [
  { key: 'enabled', type: 'bool', default: 'true', meaning: 'Toggle the rule on or off' }
]
</script>

<template>
  <RuleConfigTable :rows="rows" />
</template>
