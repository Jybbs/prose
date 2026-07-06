<script setup lang="ts">
import { ref } from 'vue'

import InlineRuleLink          from '../rules/InlineRuleLink.vue'
import { data as facetGroups } from '../../../lib/reference/facets.data'

const open       = ref<Record<string, boolean>>({})
const toggle     = (family: string): void => { open.value[family] = !open.value[family] }
const facetNoun  = (count: number): string => (count === 1 ? 'facet' : 'facets')
const facetCount = (family: (typeof facetGroups)[number]): number =>
  family.rules.reduce((sum, group) => sum + group.facets.length, 0)
</script>

<template>
  <div class="per-rule-facets">
    <section
      v-for="family in facetGroups"
      :key="family.family"
      class="per-rule-facets-section"
      :data-family="family.family"
    >
      <button
        type="button"
        class="per-rule-facets-head"
        :aria-expanded="open[family.family] ?? false"
        @click="toggle(family.family)"
      >
        <span class="per-rule-facets-chevron" aria-hidden="true"></span>
        <span class="kicker per-rule-facets-label">
          <span v-if="family.badge" class="per-rule-facets-badge" aria-hidden="true">{{ family.badge }}</span>
          {{ family.label }}
          <span class="per-rule-facets-count">{{ facetCount(family) }} {{ facetNoun(facetCount(family)) }}</span>
        </span>
      </button>
      <div v-show="open[family.family]" class="per-rule-facets-body">
        <div v-for="group in family.rules" :key="group.rule" class="per-rule-facets-rule">
          <p class="per-rule-facets-rule-head">
            <InlineRuleLink v-if="family.family !== 'generic'" :slug="group.rule" />
            <span v-else class="per-rule-facets-scope">{{ group.rule }}</span>
          </p>
          <dl class="per-rule-facets-list">
            <div v-for="facet in group.facets" :key="facet.key" class="per-rule-facets-entry">
              <dt class="per-rule-facets-term">
                <span class="per-rule-facets-key">{{ facet.key }}</span>
                <span class="per-rule-facets-badges">
                  <span class="per-rule-facets-type">{{ facet.type }}</span>
                  <span class="per-rule-facets-default">
                    <span class="per-rule-facets-default-label">default</span>
                    <span class="per-rule-facets-default-value">{{ facet.default }}</span>
                  </span>
                </span>
              </dt>
              <dd class="per-rule-facets-meaning" v-html="facet.meaningHtml" />
            </div>
          </dl>
        </div>
      </div>
    </section>
  </div>
</template>
