<script setup lang="ts">
import RuleConfigTable from './RuleConfigTable.vue'

const rows = [
  { default: 'true',    key: 'enabled',          meaning: 'Toggle the rule on or off',                                                                                                                                                                       type: 'bool' },
  { default: '8',       key: 'max-shift',        meaning: 'Ceiling on per-line padding',                                                                                                                                                                     type: 'positive int' },
  { default: '"split"', key: 'max-shift-policy', meaning: 'How to handle a group whose widest member exceeds <code>max-shift</code>. See <a href="/guide/configuration#per-rule-knobs">the per-rule knobs</a> for the full semantics', type: '<code>"split"</code> | <code>"drop"</code> | <code>"skip"</code>' }
]
</script>

<template>
  <RuleConfigTable :rows="rows" />
</template>
