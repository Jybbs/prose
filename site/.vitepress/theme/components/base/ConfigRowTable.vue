<script setup lang="ts">
import type { ConfigRow } from '../../../lib/shared/rule-schema'
import InlineProse        from './InlineProse.vue'

defineProps<{ rows: readonly ConfigRow[] }>()
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
        <td><InlineProse :nodes="row.typeNodes" /></td>
        <td>
          <code v-if="row.default !== 'unset'">{{ row.default }}</code>
          <span v-else>unset</span>
        </td>
        <td><InlineProse :nodes="row.meaningNodes" /></td>
      </tr>
    </tbody>
  </table>
</template>
