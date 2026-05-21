<script setup lang="ts">
import RelatedRulesInline from './RelatedRulesInline.vue'
import RuleConfigTable    from './RuleConfigTable.vue'
import Fixture            from '../fixtures/Fixture.vue'

defineProps<{
  canonical : string
  rule      : string
}>()
</script>

<template>
  <slot />

  <h2 id="configuration" tabindex="-1">
    Configuration
    <a class="header-anchor" href="#configuration" aria-label="Permalink to &quot;Configuration&quot;">&ZeroWidthSpace;</a>
  </h2>
  <slot name="configuration">
    <RuleConfigTable />
  </slot>

  <h2 id="the-canonical-case" tabindex="-1">
    The Canonical Case
    <a class="header-anchor" href="#the-canonical-case" aria-label="Permalink to &quot;The Canonical Case&quot;">&ZeroWidthSpace;</a>
  </h2>
  <slot name="canonical-lead" />
  <Fixture :rule="rule" :case="canonical" />

  <h2 id="more-examples" tabindex="-1">
    More Examples
    <a class="header-anchor" href="#more-examples" aria-label="Permalink to &quot;More Examples&quot;">&ZeroWidthSpace;</a>
  </h2>
  <slot name="more-examples" />

  <h2 id="related" tabindex="-1">
    Related
    <a class="header-anchor" href="#related" aria-label="Permalink to &quot;Related&quot;">&ZeroWidthSpace;</a>
  </h2>
  <RelatedRulesInline />
  <slot name="related-after" />
</template>
