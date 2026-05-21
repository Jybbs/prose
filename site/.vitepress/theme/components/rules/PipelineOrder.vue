<script setup lang="ts">
import RuleChip from './RuleChip.vue'

import { data as pipeline } from '../../../data/pipeline.data'
</script>

<template>
  <ol class="pipeline-order-list">
    <li v-for="rule in pipeline.rules" :key="rule.slug" class="pipeline-order-row">
      <span class="pipeline-order-position">{{ String(rule.position).padStart(2, '0') }}</span>
      <RuleChip
        :slug="rule.slug"
        :undocumented="!rule.documented"
        :family-badge="rule.familyBadge ?? undefined"
      />
      <span class="pipeline-order-imperative">{{ rule.imperative }}</span>
    </li>
  </ol>
</template>
