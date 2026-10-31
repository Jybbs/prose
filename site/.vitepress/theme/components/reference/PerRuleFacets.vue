<script setup lang="ts">
import { nextTick, ref } from 'vue'

import InlineRuleLink          from '../rules/InlineRuleLink.vue'
import { useHashOpen }         from '../../../lib/composables/use-hash-open'
import { data as facetGroups } from '../../../lib/reference/facets.data'
import { familyEmblem }        from '../../../lib/shared/emblems'
import { counted }             from '../../../lib/shared/numerals'
import InlineProse             from '../base/InlineProse.vue'
import NibBadge                from '../base/NibBadge.vue'
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
  <div class="my-1-5rem flex flex-col gap-6">
    <section
      v-for="family in facetGroups"
      :key="family.family"
      class="per-rule-facets-section"
      :data-family="family.family"
    >
      <button
        type="button"
        class="per-rule-facets-head flex w-full items-center gap-9 py-7 pl-14 text-left transition-opacity"
        :aria-expanded="open[family.family] ?? false"
        @click="toggle(family.family)"
      >
        <span class="per-rule-facets-chevron size-0 flex-none border-t-4 border-b-4 border-l-5 border-t-transparent border-b-transparent border-l-family" aria-hidden="true"></span>
        <span class="per-rule-facets-label kicker text-family-76-text-1">
          <NibBadge v-if="family.family !== 'generic'" :emblem="familyEmblem(family.family)" :size="13.6" />
          {{ family.label }}
          <span class="per-rule-facets-count text-text-3 before:mx-0-5em before:content-middot">{{ facetCount(family) }}</span>
        </span>
      </button>
      <div v-show="open[family.family]" class="pt-10 pb-2">
        <div v-for="group in family.rules" :key="group.rule" class="border-l-2 border-l-family pl-14 not-first:mt-18">
          <p :id="group.anchor" class="per-rule-facets-rule-head mt-0 mb-10 -ml-14 flex">
            <InlineRuleLink v-if="family.family !== 'generic'" :slug="group.rule" />
            <span v-else class="per-rule-facets-scope inline-block rounded-r-sm bg-family px-10 py-3 font-mono text-0-85em text-bg">{{ group.rule }}</span>
            <PermalinkAnchor :anchor="group.anchor" :label="group.rule" />
          </p>
          <dl class="flex flex-col">
            <div v-for="facet in group.facets" :key="facet.key" class="per-rule-facets-entry py-10 not-first:border-t-1 not-first:border-t-divider first:pt-0 last:pb-0">
              <dt :id="facet.anchor" class="per-rule-facets-term flex flex-wrap items-center gap-x-0-75rem gap-y-0-4rem">
                <span class="per-rule-facets-key font-mono text-md font-600 text-text-1">{{ facet.key }}</span>
                <PermalinkAnchor :anchor="facet.anchor" :label="facet.key" />
                <span class="ml-auto inline-flex items-center gap-0-4rem">
                  <span class="per-rule-facets-type rounded-sm bg-bg-soft px-0-5em py-2 font-mono text-xs text-text-2">{{ facet.type }}</span>
                  <span class="inline-flex items-baseline gap-0-4em rounded-sm bg-family/medium px-0-5em py-2 text-xs">
                    <span class="font-mono text-kicker tracking-label text-text-3 uppercase">default</span>
                    <span class="per-rule-facets-default-value font-mono text-text-1">{{ facet.default }}</span>
                  </span>
                </span>
              </dt>
              <dd class="per-rule-facets-meaning mt-6 max-w-62ch pl-0-9rem text-sm leading-1-6 text-text-2 mobile:pl-0">
                <InlineProse :nodes="facet.meaningNodes" />
              </dd>
            </div>
          </dl>
        </div>
      </div>
    </section>
  </div>
</template>
