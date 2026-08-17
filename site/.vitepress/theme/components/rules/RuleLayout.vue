<script setup lang="ts">
import CompositionCards from './CompositionCards.vue'
import RuleCardList     from './RuleCardList.vue'
import RuleConfigTable  from './RuleConfigTable.vue'
import RuleFixtures     from './RuleFixtures.vue'
import DocHeading       from '../base/DocHeading.vue'
import Fixture          from '../fixtures/Fixture.vue'

import { casesForRule, fixturesForRule } from '../../../lib/rules/rule-view'

const props = defineProps<{ rule: string }>()

const canonical = fixturesForRule(props.rule).canonical
const composes  = casesForRule(props.rule).length > 0
</script>

<template>
  <slot />

  <DocHeading id="configuration" title="Configuration" />
  <slot name="configuration">
    <RuleConfigTable />
  </slot>

  <DocHeading id="the-canonical-case" title="The Canonical Case" />
  <Fixture :rule="rule" :case="canonical" />

  <DocHeading id="more-examples" title="More Examples" />
  <RuleFixtures :rule="rule" />
  <CompositionCards v-if="composes" :rule="rule" />

  <DocHeading id="related" title="Related" />
  <RuleCardList related />
  <slot name="related-after" />
</template>
