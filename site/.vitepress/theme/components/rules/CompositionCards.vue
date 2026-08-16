<script setup lang="ts">
import { computed, ref } from 'vue'

import FixtureNoChange from '../fixtures/FixtureNoChange.vue'
import FixturePairDoc  from '../fixtures/FixturePairDoc.vue'
import FixtureToggle   from '../fixtures/FixtureToggle.vue'
import RuleSegmentChip from './RuleSegmentChip.vue'

import { useHashOpen }                      from '../../../lib/composables/use-hash-open'
import { data as fixturesData }             from '../../../lib/fixtures/fixtures.data'
import type { InlineNode }                  from '../../../lib/markdown/inline-nodes'
import type { CompositionCase }             from '../../../lib/rules/composition'
import { data as composition }              from '../../../lib/rules/composition.data'
import { casesForRule }                     from '../../../lib/rules/rule-view'
import { data as rules, type RenderedRule } from '../../../lib/rules/rules.data'
import type { SavedSession }                from '../../../lib/sandbox/session'
import { railPaint }                        from '../../../lib/shared/family-rail'
import type { FixtureTab }                  from '../../../lib/shared/fixture-tab'
import { inlineCode }                       from '../../../lib/shared/inline-code'
import { lookup }                           from '../../../lib/shared/lookup'
import { formatFolio }                      from '../../../lib/shared/numerals'
import InlineProse                          from '../base/InlineProse.vue'

interface RuleSegment {
  family : string | null
  index  : number
  rule   : RenderedRule | null
  slug   : string
}

interface CardRow {
  case             : string
  changesSource    : boolean
  descriptionNodes : InlineNode[] | undefined
  dominantFamily   : string | null
  hasToggle        : boolean
  headlinePaint    : string
  inputHtml        : string
  num              : string
  outputHtml       : string
  railPaint        : string
  sandboxSeed      : SavedSession
  segments         : readonly RuleSegment[]
  titleHtml        : string
}

const props = defineProps<{ rule?: string }>()

function toCardRow(entry: CompositionCase, num: string): CardRow {
  const families = entry.rules.map(slug => rules.bySlug[slug]?.family ?? null)
  const fixture  = lookup(fixturesData.composition, entry.case, 'CompositionCards case')
  return {
    case             : entry.case,
    changesSource    : fixture.changesSource,
    descriptionNodes : fixture.descriptionNodes,
    dominantFamily   : families[0] ?? null,
    hasToggle        : fixture.hasToggle,
    headlinePaint    : railPaint(families, 'to right'),
    inputHtml        : fixture.inputHtml,
    num,
    outputHtml       : fixture.outputHtml,
    railPaint        : railPaint(families),
    sandboxSeed      : { configToml: entry.configToml, source: entry.source },
    segments         : entry.rules.map((slug, idx) => ({
      family : families[idx] ?? null,
      index  : idx + 1,
      rule   : rules.bySlug[slug] ?? null,
      slug
    })),
    titleHtml        : inlineCode(entry.title)
  }
}

// A rule page renders the subset the rule participates in, keeping each card's
// folio number from the full run so it still names the composition-page entry.
const cards = computed<readonly CardRow[]>(() => {
  const participating = props.rule === undefined ? null : new Set(casesForRule(props.rule))
  return composition.cases
    .map((entry, i) => ({ entry, num: formatFolio(i + 1) }))
    .filter(({ entry }) => participating === null || participating.has(entry.case))
    .map(({ entry, num }) => toCardRow(entry, num))
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
  <ol class="composition-cards-list" aria-label="Composition cards">
    <li
      v-for="row in cards"
      :key="row.case"
      :id="row.case"
      class="fixture-card"
      :class="{ 'is-open': activeCase === row.case }"
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
          <span class="fixture-card-num">{{ row.num }}</span>
          <span class="fixture-card-title" v-html="row.titleHtml" />
        </button>
        <div class="composition-cards-tick-cell">
          <ol
            v-show="activeCase !== row.case"
            class="composition-cards-ticks"
            :aria-label="`${row.segments.length} rules in pipeline order`"
          >
            <li
              v-for="seg in row.segments"
              :key="seg.slug"
              class="composition-cards-tick-item"
              @click.stop
            >
              <RuleSegmentChip :segment="seg" :with-tooltip="false" />
            </li>
          </ol>
          <div v-show="activeCase === row.case" class="composition-cards-toggle-slot" @click.stop>
            <FixtureToggle v-if="row.hasToggle" v-model="activeTab" />
            <FixtureNoChange v-else />
          </div>
        </div>
      </div>

      <div
        :id="`composition-body-${row.case}`"
        class="fixture-card-body"
        role="region"
        @click.stop
      >
        <div class="fixture-card-body-inner">
          <div class="fixture-card-body-content">
            <template v-if="row.descriptionNodes">
              <div class="fixture-card-desc"><InlineProse :nodes="row.descriptionNodes" /></div>
              <div class="fixture-card-rule" aria-hidden="true" />
            </template>
            <div v-if="activeCase === row.case" class="composition-cards-detail">
              <FixturePairDoc
                :active-tab="activeTab"
                :input-html="row.inputHtml"
                :output-html="row.outputHtml"
                :sandbox-seed="row.sandboxSeed"
              />
            </div>
            <ol
              class="composition-cards-bar"
              :class="{ 'is-open': activeCase === row.case }"
              :aria-label="`${row.segments.length} rules in pipeline order`"
            >
              <li
                v-for="seg in row.segments"
                :key="seg.slug"
                class="composition-cards-bar-cell"
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
