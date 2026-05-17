<script setup lang="ts">
import { data as rules } from '../../data/rules.data'

const autoFix = rules.filter(r => r.category === 'auto-fix')
const lint    = rules.filter(r => r.category === 'lint'    )
</script>

<template>
  <h2 id="auto-fix">Auto-Fix</h2>
  <table>
    <thead>
      <tr><th>Rule</th></tr>
    </thead>
    <tbody>
      <tr v-for="rule in autoFix" :key="rule.slug">
        <td><a :href="`/rules/${rule.slug}`"><code>{{ rule.slug }}</code></a></td>
      </tr>
    </tbody>
  </table>

  <h2 id="lint">Lint</h2>
  <table>
    <thead>
      <tr><th>Rule</th></tr>
    </thead>
    <tbody>
      <tr v-for="rule in lint" :key="rule.slug">
        <td><a :href="`/rules/${rule.slug}`"><code>{{ rule.slug }}</code></a></td>
      </tr>
    </tbody>
  </table>
</template>
