<script setup lang="ts">
import { CATEGORY_META } from '../../lib/categories'
import { data as rules } from '../../data/rules.data'

const categories = (['auto-fix', 'lint'] as const).map(slug => ({
  slug,
  label: CATEGORY_META[slug].label,
  rules: rules.filter(r => r.category === slug)
}))
</script>

<template>
  <template v-for="cat in categories" :key="cat.slug">
    <h2 :id="cat.slug">{{ cat.label }}</h2>
    <table>
      <thead>
        <tr><th>Rule</th></tr>
      </thead>
      <tbody>
        <tr v-for="rule in cat.rules" :key="rule.slug">
          <td><a :href="`/rules/${rule.slug}`"><code>{{ rule.slug }}</code></a></td>
        </tr>
      </tbody>
    </table>
  </template>
</template>
