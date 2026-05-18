<script setup lang="ts">
import { data as configs } from '../../../data/rule-configs.data'
import type { RuleConfigPreset } from '../../../lib/rules/config-presets'

const props = defineProps<{
  preset: RuleConfigPreset
}>()

const rows = configs[props.preset]
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
      <tr v-for="row in rows" :key="row.key">
        <td><code>{{ row.key }}</code></td>
        <td v-html="row.typeHtml" />
        <td><code>{{ row.default }}</code></td>
        <td v-html="row.meaningHtml" />
      </tr>
    </tbody>
  </table>
</template>
