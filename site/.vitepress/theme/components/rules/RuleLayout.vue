<script setup lang="ts">
import { useData }  from 'vitepress'
import { computed } from 'vue'

import CompositionCards from './CompositionCards.vue'
import RuleCardList     from './RuleCardList.vue'
import RuleConfigTable  from './RuleConfigTable.vue'
import RuleFixtures     from './RuleFixtures.vue'
import DocHeading       from '../base/DocHeading.vue'
import Fixture          from '../fixtures/Fixture.vue'

import { fixturesForRule, pageCompositionCases } from '../../../lib/rules/rule-view'

const props = defineProps<{ rule: string }>()

const { frontmatter } = useData()

const canonical = fixturesForRule(props.rule).canonical
const composes  = computed(() => pageCompositionCases(frontmatter.value, props.rule).length > 0)
</script>

<template>
  <slot />

  <DocHeading id="configuration" title="Configuration" />
  <slot name="configuration">
    <RuleConfigTable />
  </slot>

  <template v-if="$slots.facets">
    <DocHeading id="facets" title="Facets" />
    <slot name="facets" />
  </template>

  <DocHeading id="the-canonical-case" title="The Canonical Case" />
  <Fixture :rule="rule" :case="canonical" />

  <DocHeading id="more-examples" title="More Examples" />
  <RuleFixtures :rule="rule" />
  <CompositionCards v-if="composes" :rule="rule" />

  <DocHeading id="related" title="Related" />
  <RuleCardList related />
  <slot name="related-after" />
</template>
