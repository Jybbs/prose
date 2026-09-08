<script setup lang="ts">
import { nextTick, ref } from 'vue'

import InlineRuleLink          from '../rules/InlineRuleLink.vue'
import { useHashOpen }         from '../../../lib/composables/use-hash-open'
import { data as facetGroups } from '../../../lib/reference/facets.data'
import { counted }             from '../../../lib/shared/numerals'
import InlineProse             from '../base/InlineProse.vue'
import PermalinkAnchor         from '../base/PermalinkAnchor.vue'

const open       = ref<Record<string, boolean>>({})
const toggle     = (family: string): void => { open.value[family] = !open.value[family] }
const facetCount = (family: (typeof facetGroups)[number]): string =>
  counted(family.rules.reduce((sum, group) => sum + group.facets.length, 0), 'facet')

// The family each anchor sits under, so a hash naming a rule or one of its
// facets expands the section that holds it.
const familyOf = new Map(facetGroups.flatMap(family =>
  family.rules.flatMap(group =>
    [group.anchor, ...group.facets.map(facet => facet.anchor)]
      .map(anchor => [anchor, family.family] as const))))

// The browser reaches a collapsed target while it is still hidden and scrolls
// nowhere, so the section expands first and the scroll follows the paint.
useHashOpen(fragment => {
  const family = familyOf.get(fragment)
  if (family === undefined) return
  open.value[family] = true
  void nextTick(() => document.querySelector(`[id="${fragment}"]`)?.scrollIntoView())
})
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
          <span class="per-rule-facets-count">{{ facetCount(family) }}</span>
        </span>
      </button>
      <div v-show="open[family.family]" class="per-rule-facets-body">
        <div v-for="group in family.rules" :key="group.rule" class="per-rule-facets-rule">
          <p :id="group.anchor" class="per-rule-facets-rule-head">
            <InlineRuleLink v-if="family.family !== 'generic'" :slug="group.rule" />
            <span v-else class="per-rule-facets-scope">{{ group.rule }}</span>
            <PermalinkAnchor :anchor="group.anchor" :label="group.rule" />
          </p>
          <dl class="per-rule-facets-list">
            <div v-for="facet in group.facets" :key="facet.key" class="per-rule-facets-entry">
              <dt :id="facet.anchor" class="per-rule-facets-term">
                <span class="per-rule-facets-key">{{ facet.key }}</span>
                <PermalinkAnchor :anchor="facet.anchor" :label="facet.key" />
                <span class="per-rule-facets-badges">
                  <span class="per-rule-facets-type">{{ facet.type }}</span>
                  <span class="per-rule-facets-default">
                    <span class="kicker per-rule-facets-default-label">default</span>
                    <span class="per-rule-facets-default-value">{{ facet.default }}</span>
                  </span>
                </span>
              </dt>
              <dd class="per-rule-facets-meaning">
                <InlineProse :nodes="facet.meaningNodes" />
              </dd>
            </div>
          </dl>
        </div>
      </div>
    </section>
  </div>
</template>
