<script setup lang="ts">
import Fixture from '../fixtures/Fixture.vue'

import { fixturesForRule } from '../../../lib/rules/rule-view'

const props = defineProps<{ rule: string }>()

const examples = fixturesForRule(props.rule).examples
</script>

<template>
  <Fixture
    v-for="ex in examples"
    :key="ex.case"
    :rule="rule"
    :case="ex.case"
    :title="ex.title"
  />
</template>
