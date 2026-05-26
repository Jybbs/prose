<script setup lang="ts">
import { data as pipeline } from '../../../data/pipeline.data'
import { data as rules }    from '../../../data/rules.data'
import { formatFolio }      from '../../../lib/shared/numerals'

import MiddleEllipsis from '../base/MiddleEllipsis.vue'
</script>

<template>
  <section class="pipeline-order" aria-label="Pipeline order">
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
            :href="rule.documented ? `/rules/${rule.slug}` : undefined"
            :title="`${rule.slug}${rule.family ? ` (${rule.family})` : ''}`"
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
