<script setup lang="ts">
import RuleChip from './RuleChip.vue'

import { data as rules } from '../../../data/rules.data'
</script>

<template>
  <template v-for="cat in rules.byCategory" :key="cat.category">
    <h2 :id="cat.category">{{ cat.label }}</h2>
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
