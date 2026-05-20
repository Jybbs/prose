<script setup lang="ts">
import Chip     from '../base/Chip.vue'
import RuleChip from './RuleChip.vue'

import { data as pipeline } from '../../../data/pipeline.data'
import { DOMAIN_META, type RuleDomain } from '../../../lib/shared/registries'
</script>

<template>
  <ol class="pipeline-order-list">
    <li v-for="rule in pipeline.rules" :key="rule.slug" class="pipeline-order-row">
      <span class="pipeline-order-position">{{ String(rule.position).padStart(2, '0') }}</span>
      <RuleChip v-if="rule.documented" :slug="rule.slug" />
      <span
        v-else
        class="rule-chip pipeline-order-undocumented"
        :title="`${rule.slug} (undocumented)`"
      >
        <span class="rule-chip-badge" aria-hidden="true">{{ rule.domain ? DOMAIN_META[rule.domain as RuleDomain].badge : '·' }}</span>
        <span class="rule-chip-slug">{{ rule.slug }}</span>
      </span>
      <span class="pipeline-order-imperative">{{ rule.imperative }}</span>
    </li>
  </ol>
</template>

<style scoped>
.pipeline-order-list {
  list-style : none;
  margin     : 24px 0;
  padding    : 0;
  display    : grid;
  gap        : 8px;
}

.pipeline-order-row {
  display       : grid;
  grid-template-columns: auto auto 1fr;
  align-items   : baseline;
  gap           : 14px;
  margin        : 0;
}

.pipeline-order-position {
  font-family    : var(--vp-font-family-mono);
  font-size      : 0.78rem;
  color          : var(--vp-c-text-3);
  letter-spacing : var(--prose-label-tracking);
}

.pipeline-order-imperative {
  font-family : var(--vp-font-family-base);
  font-size   : var(--prose-text-md);
  color       : var(--vp-c-text-2);
  line-height : 1.5;
}

.pipeline-order-undocumented {
  --domain-color : var(--vp-c-text-3);
  color          : var(--vp-c-text-3);
  cursor         : help;
}

@media (max-width: 720px) {
  .pipeline-order-row {
    grid-template-columns: auto 1fr;
    grid-template-rows   : auto auto;
  }
  .pipeline-order-imperative {
    grid-column: 1 / -1;
    padding-left: 38px;
  }
}
</style>
