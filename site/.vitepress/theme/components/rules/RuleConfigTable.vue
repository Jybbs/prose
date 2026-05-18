<script setup lang="ts">
import { RULE_CONFIG_PRESETS, type Row, type RuleConfigPreset } from '../../../lib/rules/config-presets'

const props = defineProps<{
  preset?: RuleConfigPreset
  rows  ?: Row[]
}>()

const resolved = props.rows ?? (props.preset ? RULE_CONFIG_PRESETS[props.preset] : [])
</script>

<template>
  <table>
    <thead>
      <tr>
        <th>Key</th>
        <th>Type</th>
        <th>Default</th>
        <th>Meaning</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="row in resolved" :key="row.key">
        <td><code>{{ row.key }}</code></td>
        <td v-html="row.type" />
        <td><code>{{ row.default }}</code></td>
        <td v-html="row.meaning" />
      </tr>
    </tbody>
  </table>
</template>
