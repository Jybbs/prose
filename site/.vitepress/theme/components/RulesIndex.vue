<script setup lang="ts">
import RuleChip from './RuleChip.vue'

import { CATEGORY_META } from '../../lib/registries'
import { data as rules } from '../../data/rules.data'

const categories = (['auto-fix', 'lint'] as const).map(slug => ({
  label: CATEGORY_META[slug].label,
  rules: rules.list.filter(r => r.category === slug),
  slug
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
          <td><RuleChip :slug="rule.slug" /></td>
        </tr>
      </tbody>
    </table>
  </template>
</template>
