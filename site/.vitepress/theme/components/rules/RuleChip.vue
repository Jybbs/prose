<script setup lang="ts">
import Chip from '../base/Chip.vue'

import { data as rules } from '../../../data/rules.data'
import { lookup }        from '../../../lib/shared/lookup'

const props = defineProps<{
  familyBadge   ?: string
  slug           : string
  undocumented  ?: boolean
}>()

const entry = props.undocumented ? null : lookup(rules.bySlug, props.slug, 'Rule')
const badge = entry?.familyBadge ?? props.familyBadge ?? '·'
const label = entry?.familyLabel?.toLowerCase() ?? 'undocumented'
</script>

<template>
  <Chip
    :variant="undocumented ? 'rule-chip pipeline-order-undocumented' : 'rule-chip'"
    :href="undocumented ? undefined : `/rules/${slug}`"
    :category="entry?.category"
    :family="entry?.family"
    :title="`${slug} (${label})`"
  >
    <span class="rule-chip-badge" aria-hidden="true">{{ badge }}</span>
    <span class="rule-chip-slug">{{ slug }}</span>
  </Chip>
</template>
