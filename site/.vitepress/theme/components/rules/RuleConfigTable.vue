<script setup lang="ts">
import { computed } from 'vue'

import { data as configs } from '../../../data/rule-configs.data'
import { useCurrentRule }  from '../../../lib/composables/route'

const current = useCurrentRule()

const rows = computed(() => {
  const rule = current.value
  if (rule?.family === 'alignment') return configs.alignment
  return configs.toggle
})
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
