<script setup lang="ts">
import Fixture from '../fixtures/Fixture.vue'

import { data as ruleFixtures } from '../../../data/rule-fixtures.data'
import { lookup }               from '../../../lib/shared/lookup'

const props = defineProps<{ rule: string }>()

const examples = lookup(ruleFixtures, props.rule, 'RuleFixtures rule').examples
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
