<script setup lang="ts">
import { computed } from 'vue'

import { data as landing } from '../../../data/landing.data'
import { data as rules }   from '../../../data/rules.data'
import { DOMAIN_META }     from '../../../lib/shared/registries'

const clusters = computed(() => {
  const allGroups = rules.byCategory.flatMap(c => c.byDomain)
  return landing.surfaces.map(s => {
    const group = allGroups.find(g => g.domain === s.domain)
    return { ...s, label: DOMAIN_META[s.domain].label, rules: group?.rules ?? [] }
  })
})
</script>

<template>
  <div class="rules-plate">
    <section v-for="cluster in clusters" :key="cluster.domain" class="rules-cluster" :data-domain="cluster.domain">
      <header class="rules-cluster-head">
        <span class="rules-cluster-emoji" aria-hidden="true">{{ cluster.icon }}</span>
        <h2 class="rules-cluster-name">{{ cluster.label }}</h2>
      </header>
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
