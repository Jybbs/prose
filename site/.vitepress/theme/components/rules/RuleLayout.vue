<script setup lang="ts">
import RuleCardList    from './RuleCardList.vue'
import RuleConfigTable from './RuleConfigTable.vue'
import DocHeading      from '../base/DocHeading.vue'
import Fixture         from '../fixtures/Fixture.vue'

defineProps<{
  canonical : string
  rule      : string
}>()
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
  <slot name="more-examples" />

  <DocHeading id="related" title="Related" />
  <RuleCardList related />
  <slot name="related-after" />
</template>
