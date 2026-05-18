<script setup lang="ts">
import { data as rules } from '../../../data/rules.data'
import { CATEGORY_META } from '../../../lib/shared/registries'
import { lookup }        from '../../../lib/shared/registry'

const props = defineProps<{ slug: string }>()

const entry = lookup(rules.bySlug, props.slug, 'Rule')
const meta  = CATEGORY_META[entry.category]
</script>

<template>
  <a
    :href="`/rules/${slug}`"
    :data-category="entry.category"
    class="rule-chip"
    :title="`${slug} (${entry.category})`"
  >
    <span class="rule-chip-badge" aria-hidden="true">{{ meta.badge }}</span>
    <code class="rule-chip-slug">{{ slug }}</code>
  </a>
</template>
