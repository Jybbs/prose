<script setup lang="ts">
import Chip from '../base/Chip.vue'

import { data as rules } from '../../../data/rules.data'
import { CATEGORY_META } from '../../../lib/shared/registries'
import { lookup }        from '../../../lib/shared/lookup'

const props = defineProps<{ slug: string }>()

const entry = lookup(rules.bySlug, props.slug, 'Rule')
const meta  = CATEGORY_META[entry.category]
</script>

<template>
  <Chip
    variant="rule-chip"
    :href="`/rules/${slug}`"
    :category="entry.category"
    :title="`${slug} (${entry.category})`"
  >
    <span class="rule-chip-badge" aria-hidden="true">{{ meta.badge }}</span>
    <code class="rule-chip-slug">{{ slug }}</code>
  </Chip>
</template>
