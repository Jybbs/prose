<script setup lang="ts">
import { CATEGORY_META }   from '../../lib/categories'
import { data as rules }   from '../../data/rules.data'

const props = defineProps<{ slug: string }>()

const entry = rules.find(r => r.slug === props.slug)
if (!entry) {
  throw new Error(
    `Rule "${props.slug}" not registered. ` +
    `Available rules: ${rules.map(r => r.slug).sort().join(', ')}`
  )
}
const meta = CATEGORY_META[entry.category]
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
