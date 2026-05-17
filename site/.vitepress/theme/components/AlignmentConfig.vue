<script setup lang="ts">
import RuleConfigTable from './RuleConfigTable.vue'

const rows = [
  { key: 'enabled',          type: 'bool',          default: 'true',    meaning: 'Toggle the rule on or off' },
  { key: 'max-shift',        type: 'positive int',  default: '8',       meaning: 'Ceiling on per-line padding' },
  { key: 'max-shift-policy', type: '<code>"split"</code> | <code>"drop"</code> | <code>"skip"</code>', default: '"split"', meaning: 'How to handle a group whose widest member exceeds <code>max-shift</code>. See <a href="/guide/configuration#per-rule-knobs">the per-rule knobs</a> for the full semantics' }
]
</script>

<template>
  <RuleConfigTable :rows="rows" />
</template>
