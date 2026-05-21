<script setup lang="ts">
import { computed } from 'vue'

import { data as landing } from '../../../data/landing.data'
import { data as rules }   from '../../../data/rules.data'
import { FAMILY_META }     from '../../../lib/shared/registries'

const clusters = computed(() =>
  landing.surfaces.map(s => ({
    ...s,
    label : FAMILY_META[s.family].label,
    rules : rules.byFamily[s.family] ?? []
  }))
)
</script>

<template>
  <div class="rules-plate">
    <section v-for="cluster in clusters" :key="cluster.family" class="rules-cluster" :data-family="cluster.family">
      <a class="rules-cluster-head" :href="`/rules/${cluster.family}/`">
        <span class="rules-cluster-emoji" aria-hidden="true">{{ cluster.icon }}</span>
        <h2 class="rules-cluster-name">{{ cluster.label }}</h2>
      </a>
      <ul class="rules-cluster-specimens">
        <li v-for="rule in cluster.rules" :key="rule.slug">
          <a class="specimen" :href="`/rules/${rule.slug}`">
            <span class="specimen-slug">{{ rule.slug }}</span>
            <span class="specimen-callout" role="tooltip">
              <span class="specimen-callout-slug">{{ rule.slug }}</span>
              <span class="specimen-callout-body" v-html="rule.captionHtml" />
            </span>
          </a>
        </li>
      </ul>
    </section>
  </div>
</template>
