<script setup lang="ts">
import FixtureCard from '../fixtures/FixtureCard.vue'

import { fixturesForRule } from '../../../lib/rules/rule-view'

const props = defineProps<{ rule: string }>()

const examples = fixturesForRule(props.rule).examples
</script>

<template>
  <FixtureCard
    v-for="ex in examples"
    :key="ex.case"
    :rule="rule"
    :case="ex.case"
    :title-html="ex.titleHtml"
  />
</template>
