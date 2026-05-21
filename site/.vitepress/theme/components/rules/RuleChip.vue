<script setup lang="ts">
import Chip from '../base/Chip.vue'

import { data as rules } from '../../../data/rules.data'
import { lookup }        from '../../../lib/shared/lookup'
import { FAMILY_META }   from '../../../lib/shared/registries'

const props = defineProps<{ slug: string }>()

const entry = lookup(rules.bySlug, props.slug, 'Rule')
const meta  = FAMILY_META[entry.family]
</script>

<template>
  <Chip
    variant="rule-chip"
    :href="`/rules/${slug}`"
    :category="entry.category"
    :family="entry.family"
    :title="`${slug} (${meta.label.toLowerCase()})`"
  >
    <span class="rule-chip-badge" aria-hidden="true">{{ meta.badge }}</span>
    <span class="rule-chip-slug">{{ slug }}</span>
  </Chip>
</template>
