<script setup lang="ts">
import { computed } from 'vue'

import Chip from '../base/Chip.vue'

import { data as rules }              from '../../../data/rules.data'
import { lookup }                     from '../../../lib/shared/lookup'
import { CATEGORY_META, DOMAIN_META } from '../../../lib/shared/registries'
import { useCurrentRule }             from '../../../lib/shared/route'

const current = useCurrentRule()

const items = computed(() => {
  if (!current.value?.related.length) return []
  return current.value.related.map(s => lookup(rules.bySlug, s, 'Related rule'))
})

function renderCaption(text: string): string {
  return text.replace(/`([^`]+)`/g, '<code>$1</code>')
}
</script>

<template>
  <div v-if="items.length" class="related-rules-inline">
    <a
      v-for="r in items"
      :key="r.slug"
      class="related-card"
      :data-category="r.category"
      :data-domain="r.domain"
      :href="`/rules/${r.slug}`"
    >
      <span class="related-card-slug">{{ r.slug }}</span>
      <p class="related-card-caption" v-html="renderCaption(r.caption)"></p>
      <footer class="related-card-meta">
        <Chip variant="category-chip" :category="r.category">
          <span class="category-chip-badge" aria-hidden="true">{{ CATEGORY_META[r.category].initial }}</span>
          <span class="category-chip-label">{{ CATEGORY_META[r.category].label }}</span>
        </Chip>
        <Chip variant="domain-chip" :domain="r.domain">
          <span class="domain-chip-badge" aria-hidden="true">{{ DOMAIN_META[r.domain].badge }}</span>
          <span class="domain-chip-label">{{ DOMAIN_META[r.domain].label }}</span>
        </Chip>
      </footer>
    </a>
  </div>
</template>
