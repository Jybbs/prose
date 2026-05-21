<script setup lang="ts">
import RuleChip from './RuleChip.vue'

import { data as rules } from '../../../data/rules.data'
</script>

<template>
  <template v-for="cat in rules.byCategory" :key="cat.category">
    <h2 :id="cat.category">{{ cat.label }}</h2>
    <template v-for="fam in cat.byFamily" :key="fam.family">
      <h3 :id="fam.family">{{ fam.label }}</h3>
      <ul class="rules-index-list">
        <li v-for="rule in fam.rules" :key="rule.slug">
          <RuleChip :slug="rule.slug" />
        </li>
      </ul>
    </template>
  </template>
</template>
