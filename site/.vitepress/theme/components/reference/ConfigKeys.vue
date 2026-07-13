<script setup lang="ts">
import { computed } from 'vue'

import { data, type ConfigKeys } from '../../../lib/reference/config-keys.data'
import InlineProse               from '../base/InlineProse.vue'

const props = defineProps<{ section: keyof ConfigKeys }>()

const rows = computed(() => data[props.section])
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
