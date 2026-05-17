<script setup lang="ts">
export interface Row {
  default : string
  key     : string
  meaning : string
  type    : string
}

defineProps<{ rows: Row[] }>()
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
        <td v-html="row.type" />
        <td><code>{{ row.default }}</code></td>
        <td v-html="row.meaning" />
      </tr>
    </tbody>
  </table>
</template>
