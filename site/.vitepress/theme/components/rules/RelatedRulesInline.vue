<script setup lang="ts">
import { computed } from 'vue'

import TaxonomyChip from '../aside/TaxonomyChip.vue'

import { data as rules }  from '../../../data/rules.data'
import { lookup }         from '../../../lib/shared/lookup'
import { useCurrentRule } from '../../../lib/shared/route'

const current = useCurrentRule()

const items = computed(() => {
  if (!current.value?.related.length) return []
  return current.value.related.map(s => lookup(rules.bySlug, s, 'Related rule'))
})
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
      <p class="related-card-caption" v-html="r.captionHtml"></p>
      <footer class="related-card-meta">
        <TaxonomyChip axis="category" :value="r.category" :linked="false" />
        <TaxonomyChip v-if="r.domain !== 'lint'" axis="domain" :value="r.domain" :linked="false" />
      </footer>
    </a>
  </div>
</template>
