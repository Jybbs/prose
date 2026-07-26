<script setup lang="ts">
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { data as rules }    from '../../../lib/rules/rules.data'
import { formatFolio }      from '../../../lib/shared/numerals'

import MiddleEllipsis from '../base/MiddleEllipsis.vue'

type PipelineRule = (typeof pipeline.rules)[number]

function label(rule: PipelineRule): string {
  const family = rule.family ? ` (${rule.family})` : ''
  const after  = rule.after.length > 0 ? ` · reads ${rule.after.join(', ')}` : ''
  return `${rule.slug}${family}${after}`
}
</script>

<template>
  <section class="pipeline-order panel" aria-label="Pipeline order">
    <header class="pipeline-order-masthead">
      <span class="kicker pipeline-order-edition">
        {{ pipeline.rules.length }} passes &middot; <code>src/rule.rs</code>
      </span>
    </header>
    <ol class="pipeline-order-columns">
      <li
        v-for="rule in pipeline.rules"
        :key="rule.slug"
        class="pipeline-order-entry"
        :data-family="rule.family"
      >
        <RuleTooltipPopper :rule="rule.documented ? rules.bySlug[rule.slug] : null">
          <a
            class="pipeline-order-link"
            :href="rule.documented ? rules.bySlug[rule.slug].href : undefined"
            :title="label(rule)"
          >
            <span class="folio">№ {{ formatFolio(rule.position) }}</span>
            <MiddleEllipsis class="pipeline-order-name" :text="rule.slug" :tail="2" />
            <span class="pipeline-order-leader" aria-hidden="true"></span>
            <span class="pipeline-order-glyph" aria-hidden="true">{{ rule.familyBadge ?? '·' }}</span>
          </a>
        </RuleTooltipPopper>
      </li>
    </ol>
  </section>
</template>
