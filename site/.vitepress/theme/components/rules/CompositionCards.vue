<script setup lang="ts">
import { useData }       from 'vitepress'
import { computed, ref } from 'vue'

import FixtureNoChange from '../fixtures/FixtureNoChange.vue'
import FixturePairDoc  from '../fixtures/FixturePairDoc.vue'
import FixtureToggle   from '../fixtures/FixtureToggle.vue'
import RuleSegmentChip from './RuleSegmentChip.vue'

import { useHashOpen }                    from '../../../lib/composables/use-hash-open'
import { COMPOSITION_RULE, fixtureEntry } from '../../../lib/fixtures/entry'
import type { FixtureEntry }              from '../../../lib/fixtures/entry'
import type { CompositionCase }           from '../../../lib/rules/composition'
import { data as composition }            from '../../../lib/rules/composition.data'
import { casesForRule, type RuleSegment } from '../../../lib/rules/rule-view'
import { data as rules }                  from '../../../lib/rules/rules.data'
import type { SavedSession }              from '../../../lib/sandbox/session'
import { railPaint }                      from '../../../lib/shared/family-rail'
import type { FixtureTab }                from '../../../lib/shared/fixture-tab'
import InlineProse                        from '../base/InlineProse.vue'

interface CardRow extends FixtureEntry {
  case           : string
  dominantFamily : string | null
  headlinePaint  : string
  railPaint      : string
  sandboxSeed    : SavedSession
  segments       : readonly RuleSegment[]
  titleHtml      : string
}

const props = defineProps<{ rule?: string }>()

const { frontmatter } = useData()

function toCardRow(entry: CompositionCase): CardRow {
  const families = entry.rules.map(slug => rules.bySlug[slug]?.family ?? null)
  return {
    ...fixtureEntry(frontmatter.value, COMPOSITION_RULE, entry.case),
    case           : entry.case,
    dominantFamily : families[0] ?? null,
    headlinePaint  : railPaint(families, 'to right'),
    railPaint      : railPaint(families),
    sandboxSeed    : { configToml: entry.configToml, source: entry.source },
    titleHtml      : entry.titleHtml,
    segments       : entry.rules.map((slug, idx) => ({
      family : families[idx] ?? null,
      index  : idx + 1,
      rule   : rules.bySlug[slug] ?? null,
      slug
    }))
  }
}

// A rule page renders the subset the rule participates in. Each card leaves
// its folio empty so the `prose-fixture-folio` counter numbers it in place,
// continuing the run the fixtures above it started.
const cards = computed<readonly CardRow[]>(() => {
  const participating = props.rule === undefined ? null : new Set(casesForRule(props.rule))
  return composition.cases
    .filter(entry => participating === null || participating.has(entry.case))
    .map(toCardRow)
})

const activeCase = ref<string | null>(null)
const activeTab  = ref<FixtureTab>('after')

function toggle(row: CardRow): void {
  activeCase.value = activeCase.value === row.case ? null : row.case
}

useHashOpen(fragment => {
  if (cards.value.some(row => row.case === fragment)) activeCase.value = fragment
})
</script>

<template>
  <ol class="composition-cards-list m-0 list-none p-0" aria-label="Composition cards">
    <li
      v-for="row in cards"
      :key="row.case"
      :id="row.case"
      class="fixture-card cursor-pointer hover:border-transparent not-data-open:hover:rail-border-on-bg-soft data-open:hover:rail-border-on-bg"
      :data-open="activeCase === row.case || undefined"
      :data-family="row.dominantFamily"
      :data-edits="row.changesSource"
      :style="{ '--rail-paint': row.railPaint, '--headline-paint': row.headlinePaint }"
      @click="toggle(row)"
    >
      <div class="fixture-card-summary-row">
        <button
          type="button"
          class="fixture-card-summary"
          :aria-expanded="activeCase === row.case"
          :aria-controls="`composition-body-${row.case}`"
        >
          <span class="fixture-card-num" aria-hidden="true" />
          <span class="fixture-card-title truncate" v-html="row.titleHtml" />
        </button>
        <div class="inline-flex min-h-22 items-center justify-end">
          <ol
            v-show="activeCase !== row.case"
            class="composition-cards-ticks m-0 flex list-none items-center justify-end gap-3 p-0"
            :aria-label="`${row.segments.length} rules in pipeline order`"
          >
            <li
              v-for="seg in row.segments"
              :key="seg.slug"
              class="composition-cards-tick-item m-0 inline-flex items-center"
              @click.stop
            >
              <RuleSegmentChip :segment="seg" :with-tooltip="false" />
            </li>
          </ol>
          <div v-show="activeCase === row.case" class="inline-flex items-center" @click.stop>
            <FixtureToggle v-if="row.hasToggle" v-model="activeTab" />
            <FixtureNoChange v-else />
          </div>
        </div>
      </div>

      <div
        :id="`composition-body-${row.case}`"
        class="fixture-card-body transition-grid-template-rows duration-grow ease-morph"
        role="region"
        @click.stop
      >
        <div class="fixture-card-body-inner">
          <div class="fixture-card-body-content">
            <template v-if="row.descriptionNodes">
              <div class="fixture-card-desc"><InlineProse :nodes="row.descriptionNodes" /></div>
              <div class="fixture-card-rule" aria-hidden="true" />
            </template>
            <FixturePairDoc
              v-if="activeCase === row.case"
              :active-tab="activeTab"
              :input-html="row.inputHtml"
              :output-html="row.outputHtml"
              :sandbox-seed="row.sandboxSeed"
            />
            <ol
              class="composition-cards-bar mt-12 mb-0 ml-auto flex list-none gap-3 p-0 transition-width delay-grow duration-360 ease-morph w-60 data-open:w-full"
              :data-open="activeCase === row.case || undefined"
              :aria-label="`${row.segments.length} rules in pipeline order`"
            >
              <li
                v-for="seg in row.segments"
                :key="seg.slug"
                class="m-0 block min-w-0 flex-auto"
                @click.stop
              >
                <RuleSegmentChip :segment="seg" :with-tooltip="true" />
              </li>
            </ol>
          </div>
        </div>
      </div>
    </li>
  </ol>
</template>
