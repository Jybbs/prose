<script setup lang="ts">
import Chip     from '../base/Chip.vue'
import RuleChip from './RuleChip.vue'

import { data as pipeline } from '../../../data/pipeline.data'
import { FAMILY_META, type RuleFamily } from '../../../lib/shared/registries'
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
        <span class="rule-chip-badge" aria-hidden="true">{{ rule.family ? FAMILY_META[rule.family as RuleFamily].badge : '·' }}</span>
        <span class="rule-chip-slug">{{ rule.slug }}</span>
      </span>
      <span class="pipeline-order-imperative">{{ rule.imperative }}</span>
    </li>
  </ol>
</template>
