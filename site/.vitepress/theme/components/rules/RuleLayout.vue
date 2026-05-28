<script setup lang="ts">
import RuleCardList    from './RuleCardList.vue'
import RuleConfigTable from './RuleConfigTable.vue'
import RuleFixtures    from './RuleFixtures.vue'
import DocHeading      from '../base/DocHeading.vue'
import Fixture         from '../fixtures/Fixture.vue'

import { data as ruleFixtures } from '../../../data/rule-fixtures.data'
import { lookup }               from '../../../lib/shared/lookup'

const props = defineProps<{ rule: string }>()

const canonical = lookup(ruleFixtures, props.rule, 'RuleLayout rule').canonical
</script>

<template>
  <slot />

  <DocHeading id="configuration" title="Configuration" />
  <slot name="configuration">
    <RuleConfigTable />
  </slot>

  <DocHeading id="the-canonical-case" title="The Canonical Case" />
  <slot name="canonical-lead" />
  <Fixture :rule="rule" :case="canonical" />

  <DocHeading id="more-examples" title="More Examples" />
  <RuleFixtures :rule="rule" />

  <DocHeading id="related" title="Related" />
  <RuleCardList related />
  <slot name="related-after" />
</template>
